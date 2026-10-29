# 历史方案：单个 JSON 配置（已由 TOML 分文件方案取代）

本文件记录旧练习设计，不是现行实现要求。当前阅读[第 04 课](../lessons/04-errors-logging-and-toml-storage.md)。

# 第 04 课：完整源码、统一错误、日志与 JSON 持久化

**状态：六项阅读/小改动练习与八个关键 TODO 待完成，功能和理解均待验收。** 原第 04、05 课合并为一课，先理解现有代码和新库，再亲写单个 `config.json` 的持久化。原 SQLite 作业[功能已通过](../reviews/04-sqlite-acceptance.md)，历史成果保留。

[返回 12 课目录与知识清单](../Course-Syllabus.md) · [上一课](../lessons/03-encapsulation-and-serialization.md) · [完整源码地图](../Code-Reading-Guide.md)

## 本课目录与学习顺序

1. [A：项目结构、错误链与 tracing](#part-a)：先介绍新库，逐段阅读所有相关源码，完成 4.A～4.F 六项练习。
2. [B：单个 config.json 保存与恢复](#part-b)：完成 TODO 4.1～4.8，观察完整读写与失败路径。
3. [整课验收](#acceptance)：功能与理解分别记录，两项都通过再进入第 05 课 C FFI。

```sh
# A 部分可独立运行，不进入未填写的存储代码
cargo run --locked -- course04 --intro
RUST_LOG=debug cargo run --locked -- course04 --intro

# 完整运行 A/B；尚未填写时在 TODO 4.4 停止
cargo run --locked -- course04

# 完成后从目录下的 config.json 加载
cargo run --locked -- course04 "/path/to/config-directory"
```

<a id="part-a"></a>

## A：项目结构、统一错误与日志

### 1. 先认识本轮的依赖

| 库 | 它负责什么 | 本项目如何用 | 本次是否新增 |
|---|---|---|---|
| `serde` | 在 Rust 类型和序列化数据模型之间转换 | derive `Serialize` / `Deserialize`，定义字段和枚举规则 | 已有，回补介绍 |
| `serde_json` | 把 Serde 数据模型编解码为 JSON | `to_string_pretty`、`from_str`、故障演示中的 `Value` | 已有，回补介绍 |
| `tracing` | 产生带级别、字段和 span 上下文的诊断事件 | 应用编排中的 info/debug/warn/error 和课程 span | 新增 |
| `tracing-subscriber` | 接收、过滤和格式化 tracing 事件 | 读取 RUST_LOG，fmt 输出到 stderr | 新增 |

`serde` 不负责写文件，`serde_json` 也不会自行保存配置。解析成功只表示符合目标类型，业务模型仍要校验。先复习[Serde 数据模型](https://serde.rs/data-model.html)和[serde_json 基本用法](https://docs.rs/serde_json/latest/serde_json/)，再定位 `config.rs` 的派生、Serde 属性和编解码调用。

`tracing` 将“发生了什么”表达为 event，将“一段操作的上下文”表达为 span；subscriber 负责处理这些诊断数据。`tracing` 自身不会替你选择终端格式或输出位置。先读[tracing 的核心概念](https://docs.rs/tracing/latest/tracing/)和[subscriber 的 fmt 说明](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/)，再看项目代码。

Cargo.toml 的实际声明：

```toml
tracing = { version = "0.1", default-features = false, features = ["std"] }
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
```

`version` 表示兼容版本范围，Cargo.lock 才记录本次锁定的具体版本。tracing 禁用默认 feature 后启用 `std`，本项目使用宏创建 span，不引入属性宏的用法；subscriber 的 `env-filter` 提供过滤指令，`fmt` 提供格式化层。没有新加 `thiserror`：错误 trait 本轮手写，以便把转换和来源链读清楚。

先读懂这个最小例子，再与 app 对照。它是独立教学片段，**不要在已经初始化的主程序里再次调用初始化**：

```rust
fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .try_init()
        .expect("Failed to install the tracing subscriber");

    let span = tracing::info_span!("load_configuration", file = "config.json");
    let _entered = span.enter();
    tracing::info!(devices = 2, "configuration loaded");
}
```

这个独立例子使用已经见过的 expect 显式处理初始化失败，避免隐藏返回类型；项目正式入口使用 AppError 返回和统一报告。先安装 subscriber，才有接收与显示事件的处理器。`INFO` 是事件级别；`devices = 2` 是可独立查询的字段；最后的字符串是消息。span 创建后通过 enter 进入作用域，guard 存活期间发出的事件关联该上下文。guard 离开作用域结束进入状态。

### 2. 先把新的工程入口读清楚

Rust 的 edition 控制语言规则，`rust-version` 指最低编译器版本；依赖版本又是另一件事。当前采用 Rust 2024 edition，最低版本 1.85。2024 的语言变化见[官方 edition 指南](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)。本轮不要求背下全部变化，先解释 Cargo.toml 中这两个字段。

一个 Cargo package 可以有 library crate 和 binary crate。[Cargo 标准布局](https://doc.rust-lang.org/cargo/guide/project-layout.html)中，lib.rs 是库根、main.rs 是二进制根；这不是两个 package。

| 文件 | 责任 | 必须读懂的点 |
|---|---|---|
| `src/lib.rs` | 公开 model/config/error/store | 哪些是可复用接口，为什么不依赖 app |
| `src/main.rs` | 保留长期课程表、启动日志、调用应用、退出 | 初始化失败的 stderr 后备输出；运行失败的统一报告与退出码 |
| `src/app.rs` | 参数与流程编排、第04课 A/B | OsString、Unicode 检查、可选目录、span、match、Result |
| `src/app/demo.rs` | 原有01～03课演示与样例 | 所有构造、展示与预期错误分支都必读 |
| `src/app/logging.rs` | 日志初始化和 report_error | 过滤、格式化、stderr、全局安装与安全错误输出 |
| `src/error.rs` | 集中定义分层错误 | Display、source、From 和退出分类 |

库内 `crate::model` 从 library 的根找模块；二进制的 app 用 `nasnet::model` 使用同一个 package 的库。模块仍用同名 `.rs` 与子目录并列，无需 `mod.rs`。结构按职责和依赖方向组织，不需要预先拆成多个包。

`StoreConnection` 是文件存储入口；`ProtocolConnection` 是 NAS 协议配置。前者持有 PathBuf，后者持有 NFS/SMB 配置，避免用泛称 Connection 混淆。两者目前都不代表已连接真实 NAS。

### 3. 错误体系独立成层

依次读 error.rs 中四种错误，不只看最后的英文文字：

| 类型 | 表达什么失败 | 例子 |
|---|---|---|
| `ConfigError` | 模型规则或模型操作 | 空 Host、重复地址 ID、非法共享路径 |
| `ConfigIoError` | JSON 与恢复边界 | JSON 格式/类型、模型校验、不支持版本 |
| `StoreError` | 配置文件操作 | 操作名、路径、I/O 原因，或配置错误 |
| `AppError` | 应用入口统一接收 | 配置/存储错误、参数、日志初始化 |

用一个具体路径理解：文件 load_all 解析到非法 Host → ConfigError → ConfigIoError::Validation → StoreError::Config → AppError::Store → main 调用 report_error。每层只表达对应责任，不逐层记录日志。

- `Display` 决定 `{error}` 的英文文字；它不会执行校验，也不自行发日志。
- `std::error::Error::source` 借出下层错误，保留具体原因链；没有来源的变体返回 None。引用来自当前 error，`'static` 约束错误类型不含短期借用，并不是让借出的引用永久有效。
- `From` 告诉 `?` 怎样将下层错误转换为当前返回类型。读取文件时还需要 `StoreError::io(operation, path, error)`，因为单个 io::Error 本身没有业务操作和配置路径。
- `AppError::kind` 用于结构化日志字段；`exit_code` 区分参数/未知课程的 2 和其他错误的 1。

所有程序错误文字与日志事件文字使用英文，讲义和课程说明保留中文。ConfigIoError::Json 的 Display 仅显示安全的行列信息，不复述原始 JSON；source 仍保留 serde_json 错误供代码识别。**report_error 在 ERROR 启用时记录安全的顶层 Display 与 kind；过滤禁用 ERROR 时改用一次 stderr 显示，不递归打印 source**，因为某些 JSON 类型错误的底层文字可能包含输入值。

本课理解 source，不代表要把整条来源链写进日志。不要使用 `?error`、`{:?}` 或完整 JSON 来调试含密码的数据。

### 4. 日志初始化逐段阅读

打开 app/logging.rs 的 init：

1. `std::env::var("RUST_LOG")` 区分存在、未设置、非 Unicode。
2. 存在则 `EnvFilter::try_new` 解析；非法指令返回英文初始化错误，程序退出 1。未设置使用 info。
3. `fmt()` 建立格式化 subscriber，`with_env_filter` 装过滤器。
4. `with_writer(std::io::stderr)` 将诊断输出写到 stderr；普通课程展示继续写 stdout。
5. `with_ansi(false)` 关闭颜色转义，`with_target(false)` 隐藏默认格式中的 target。
6. `try_init()` 安装全局 subscriber；重复安装返回错误，不悄悄忽略。初始化失败时还不能依赖 tracing 输出，main 用 stderr 报告。

打开 app.rs：`info_span!("course", name = course)` 保留课程上下文，`let _entered = span.enter()` 的变量维持 guard 到作用域结束。`course started/completed` 是应用事件；预期的非法 Host 示例在应用层捕获后用 warn 记录，不把模型返回错误的契约改成成功。

运行：

```sh
cargo run --locked -- course04 --intro
RUST_LOG=debug cargo run --locked -- course04 --intro
RUST_LOG=warn cargo run --locked -- course04 --intro
cargo run --locked -- unknown-course
cargo run --locked -- course04 --intro extra-directory
RUST_LOG=off cargo run --locked -- unknown-course
```

默认 info 看不到 debug 示例；debug 能看到；warn 隐藏 info 保留警告。unknown-course 和多余目录返回英文错误、退出码 2。A 部分的预期 warning 不导致应用失败。日志过滤只决定处理哪些事件，不改变业务是否执行。若 `RUST_LOG=off` 禁用 ERROR，report_error 仍在同一个应用边界用 eprintln! 显示一次英文失败；日志过滤不能让用户完全看不到操作失败。请同时阅读 tracing::enabled! 分支。

### 5. 六项亲手补课练习

助手已经提供可运行基线。这六项由你修改、解释、预测并核对；不是只口头说“看过”。可以逐项完成，不把实验性的无用输出留在项目中。

| 编号 | 你来完成 | 先写出的预测与验收证据 |
|---|---|---|
| 4.A | 画出 main → app → library 和 logging/error 的调用关系，列出每个现有文件的责任；解释 `crate::` 与 `nasnet::` | 指出哪些模块接触终端、磁盘，哪些只返回数据；逐段解释助手代码 |
| 4.B | 在 run_bridge_course 新增一个 debug 事件，字段含练习编号和设备数量，消息用英文 | 分别在 info/debug 下运行，预测并核对可见性；不记录配置对象或密码 |
| 4.C | 为补课中的一小段演示创建嵌套 span，先进入再发事件，另在 guard 结束后发事件 | 预测两个事件分别关联哪些 span；解释 guard 所有权与作用域 |
| 4.D | 用 `RUST_LOG=warn`、`debug` 和一条明显非法过滤指令运行，解释 init 的所有分支 | 过滤不改变课程执行；非法指令初始化失败、英文 stderr、退出码 1 |
| 4.E | 针对虚构的 EmptyHost 和不含敏感值的 JSON 格式错误，解释 Display、From、source 的各层；用小片段通过 source 识别有无下层错误 | 逐层说出具体枚举类型；正常 report_error 仍不递归打印来源、不 Debug 整份配置 |
| 4.F | 跟踪 app::run 的 OsString/Unicode 检查、参数个数与目录借用；尝试未知课程和额外参数 | 解释 `course04 [directory]` 与 `course04 --intro` 怎样选择分支；错误到日志/显示及退出码的完整路径 |

然后补验前三课完整调用链，问题见[源码指南](../Code-Reading-Guide.md)。已经通过的模型实现不用重写。

<a id="part-b"></a>

## B：单一 config.json 保存与恢复

### 1. 先认识文件、格式与模型的分工

`serde` 定义 Rust 类型怎样转换成通用数据模型，`serde_json` 处理 JSON 文本；两个库已在第 03 课使用，职责与最小用法已在[补课](../lessons/04-errors-logging-and-toml-storage.md#part-a)回补。B 部分**不额外新增依赖**：文件读写、路径、HashSet、原子计数都来自标准库。

先读 [std::fs](https://doc.rust-lang.org/std/fs/index.html)、[Path / PathBuf](https://doc.rust-lang.org/std/path/index.html)、[Write::write_all](https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all)的接口说明。Path 是借用的路径，PathBuf 拥有路径内容；它们本身不是打开的文件。

| 层 | 责任 | 本课接口 |
|---|---|---|
| model | 业务数据与规则，保持合法修改入口 | Device::try_from 复用已有添加校验 |
| config | 快照、JSON、版本与全量 ID 规则 | ConfigFile、encode_devices、decode_devices |
| store | 读取和替换唯一文件 | DeviceStore、StoreConnection |
| error | 错误类别、英文文字与转换 | ConfigError/ConfigIoError/StoreError/AppError |
| app | 编排、用户摘要、日志与验收场景 | run_json_course；模型与存储不输出日志 |

`StoreConnection::new(directory)` 固定组合 `directory/config.json`。参数 `impl Into<PathBuf>` 接受能转换为拥有路径的类型：传 PathBuf 会移动，传 &Path 会复制路径表示，`directory.into()` 后由对象拥有最终路径。它不打开连接、不提前创建文件；`config_path()` 只读借出最终路径。网络配置仍叫 `ProtocolConnection`，避免将文件存储入口与网络混淆。

### 2. 配置根结构与行为契约

第 03 课的单设备 `ConfigDocument { schema_version, device }` 及已验收函数保留。正式磁盘文件用新的 `ConfigFile { schema_version, devices }`：

```json
{
  "schema_version": 1,
  "devices": [
    {
      "id": "nas-z",
      "name": "Main NAS",
      "addresses": [
        { "id": "lan", "host": "192.168.1.20", "priority": 1 }
      ],
      "connections": []
    }
  ]
}
```

后续应用配置也在同一个根文档中按明确字段扩展，而不是另存多个配置文件。当前 `deny_unknown_fields` 会拒绝未声明字段，所以每次扩展仍要同步结构、版本约定、讲义与验收。

数组顺序就是保存顺序。设备 ID 在整份文档内唯一；地址和协议连接 ID 只在所属设备内唯一，不要求跨设备全局唯一。ID 不新增 trim 或改写。密码按既有要求明文原样保存，空格与字符不变，日志不输出完整 JSON。

`save_all` 整体替换列表，空列表保存合法空配置。`load_all` 缺文件返回空列表，存在但为空/损坏/无权限必须错误。加载失败不返回半份设备。删除不存在返回 false，不触碰文件；加载坏配置失败时不能用空列表覆盖。

当前按一个后台写入者设计。替换流程先写同目录临时文件，再 rename；成功后只有 config.json 是正式配置。临时文件不是第二份配置来源。

### 3. TODO 总表与建议顺序

| 编号 | 文件与函数 | 你负责实现 |
|---|---|---|
| 4.1 | config.rs：validate_device_ids | 全文件设备 ID 唯一性 |
| 4.2 | config.rs：encode_devices | 整份配置快照与 JSON |
| 4.3 | config.rs：decode_devices | 解析、版本、模型恢复与重复 ID |
| 4.4 | store.rs：load_all | 读取文件，仅 NotFound 为空 |
| 4.5 | store.rs：save_all | 先验证、临时写入、整体替换 |
| 4.6 | store.rs：delete_device | 加载、查找、删除、重新保存 |
| 4.7 | store.rs：write_temporary | create_new、完整写入、同步和失败清理 |
| 4.8 | store.rs：replace_config | 同目录替换、失败清理与保留旧文件 |

先完成 4.1～4.4，再写 4.7/4.8，最后组合 4.5/4.6。每个 TODO 都删除占位 `let _ = ...` 和 todo!，保留必要行为契约。不要把关键逻辑转交给新增依赖。

### 4. TODO 4.1：重复设备 ID

使用 `HashSet`，按输入顺序插入 device.id()。insert 返回 false 表示已出现；返回 `ConfigIoError::Validation(ConfigError::DuplicateDeviceId(...))`，或利用已有 From 转换。只为报错复制 ID，检查过程不移动设备、不改输入、不排序。

解释 HashSet 的用途与 Vec 的区别：这里关心是否已经出现，不用它决定保存顺序。顺序仍来自原 devices 切片。

### 5. TODO 4.2：整份编码

先 validate_device_ids；成功后构造 schema_version 为 SCHEMA_VERSION、devices 为逐条 DeviceData::from 的 ConfigFile，再调用 serde_json::to_string_pretty。

可以使用 iter/map/collect，也可显式循环构造 Vec。解释快照怎样拥有原有字段副本，为什么产生的字符串包含密码、不能记录到日志。该函数只在内存工作，不接触文件。

### 6. TODO 4.3：整份解析与恢复

显式解析为 ConfigFile，先检查 schema_version。版本不支持立即错误；支持才逐项 Device::try_from，借助已完成的地址/协议校验恢复。将成功设备放在局部 Vec，全部成功后再次检查设备 ID，最后一次性返回。

可使用 `collect::<Result<Vec<_>, _>>()`，也可先用循环理解。前者收集成功值，遇 Err 返回错误，并不产生 Vec<Result<...>> 或跳过失败。任一配置非法都不返回前半列表。

### 7. TODO 4.4：加载唯一文件

用 fs::read_to_string(&self.config_path) 读取：成功时 decode_devices；NotFound 返回 Vec::new；其他 io::Error 用 StoreError::io 保留操作、路径和来源。

存在但空文本会在 JSON 层失败，不能和缺文件混在一起。目录权限、非 UTF-8 内容和把目录当作文件也不能返回空列表。加载本身不创建目录或文件。

### 8. TODO 4.7：写入临时文件

先学习 [OpenOptions::create_new](https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.create_new)、[File::sync_all](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all)：

1. `OpenOptions::new().write(true).create_new(true).open(path)`，成功才拥有本次新建文件。
2. `write_all(contents)`，不能假设一次 write 写完所有字节。
3. `sync_all()`，将该文件内容与元数据同步请求交给系统。
4. 写入或同步失败时保留原始错误，关闭文件后尽力删除本次创建的临时文件；返回 StoreError::io。
5. 若 open 因名字冲突失败，本次没有创建文件，不能删除已有冲突文件。成功返回前同样让文件句柄关闭，之后才替换。

`?` 在这里不一定够：如果直接提前返回，会跳过你需要安排的清理。可以先保留写入/同步结果，drop(file)，再根据结果清理并返回。清理失败不能掩盖原始失败。

唯一临时路径辅助函数也必读：进程 ID 与 AtomicU64 序列减少名字冲突，static 使计数器跨本进程中的函数调用保存，fetch_add 原子地取得旧值并加一；Ordering::Relaxed 只需要进程内序号，不负责业务内存同步，也不是文件锁或多进程写安全。进程 ID 可复用，旧残留文件也可能碰巧同名；`create_new` 才最终拒绝已存在路径。当前不要求设计通用并发算法。

### 9. TODO 4.8：替换 config.json

读 [fs::rename](https://doc.rust-lang.org/std/fs/fn.rename.html)，使用 temporary → destination。临时文件必须在同目录，且已写完关闭。成功就是本次正式配置替换完成点；失败尽力清理临时文件，返回原 rename 错误，保留目标。

不能先删除 destination 再 rename，那会制造没有旧配置的失败窗口。也不能以 copy 后删除替代同目录替换。当前未实现目录 fsync，不能宣称完整掉电持久性；跨平台的具体文件占用和替换失败仍需要在目标系统验证。

### 10. TODO 4.5：组合完整保存

先 encode_devices，确定内容合法；然后处理父目录并 create_dir_all。用已提供的 temporary_path 得到同目录路径，再 write_temporary，最后 replace_config。

所有 I/O 用带操作与路径的 StoreError 返回。父目录无法确定时使用既有 InvalidConfigPath。检查、编码失败不写任何文件；临时写入失败不碰旧文件；替换失败保留旧配置。只有 rename 成功后目标内容才改变。

### 11. TODO 4.6：删除并持久化

先 load_all，再 position 找 ID。不存在直接 Ok(false)；找到则 remove，并用 save_all 保存剩余设备，成功才 Ok(true)。保持剩余顺序。

如果加载失败，必须传播原错误；如果保存失败，返回错误，由保存流程保留原配置。存储对象只持有路径，不存在需要与失败文件同步的内存缓存。

### 12. 运行与实际覆盖

```sh
cargo check --locked
cargo fmt -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo run --locked -- course04
```

TODO 未填时，默认演示会先停在 load_all 的 TODO 4.4。占位 panic 不是统一错误报告，也不代表存储功能已完成。

无目录参数时，app::run_json_course 创建独立临时目录：首次加载、两台样例设备往返、顺序、重复设备 ID、失败后旧内容、删除/重复删除、空列表、损坏 JSON 和未知版本。结束打印跨进程读取命令。首次设备样例包含两个同优先级地址、NFS/SMB 连接与带首尾空格和单引号的虚构密码，应对照实际 persistence_sample 阅读，不从输出摘要推断原值。完成 TODO 后需要确认这一流程的全部预期。

带目录参数时，仅创建路径对象并加载其中的 config.json，打印设备 ID/名称后返回，后面的保存和坏文件注入不执行：

```sh
cargo run --locked -- course04 "/path/to/existing-directory"
```

传入的是目录，不是 JSON 文件路径。读取不存在配置按接口返回空列表；它不会因此替用户创建空配置。临时演示目录保留便于跨进程核对。

### 13. 功能验收表

| 场景 | 预期 | 验收方式 |
|---|---|---|
| 缺少配置 | 空列表，不创建文件 | 默认演示与目录内容 |
| 正常往返与重建存储对象 | 全字段与设备/地址/连接顺序保持 | 整份编码比较及查看样例 |
| 跨进程读取 | 能读取上次保存的配置 | 使用演示给出的目录命令 |
| 重复设备 ID 保存 | 返回英文错误，旧文件内容不变 | 已有演示；比较磁盘内容 |
| 不同设备同一局部 ID | 允许，不错误归类为全局重复 | 你补充样例 |
| 损坏、空文本、未知字段、未知版本 | 返回配置错误 | 已有损坏/版本演示，其余由你补充 |
| 非法 Host/协议、重复地址/连接 ID | 重新执行模型校验，拒绝整份文件 | 你在临时目录补充故障输入 |
| 密码含首尾空格及特殊字符 | 原样恢复，日志不出现密码 | 已有完整 JSON 比较覆盖虚构样例；再补充字段比较，不打印密码 |
| 删除、重复删除、空列表 | true / false / 合法空配置 | 已有演示 |
| 写临时文件冲突 | 返回错误，不删除冲突文件，不改旧配置 | 你直接调用 write_temporary 构造场景 |
| 替换失败 | 返回原错误，清理本次临时文件，目标保持 | 你在临时目录直接调用 replace_config 验证 |
| 无权限/错误路径 | 返回 I/O 错误，不能伪装为空配置 | 你构造可复现路径场景 |

表中“已有演示”表示已提供观察流程，不表示 TODO 未填写时已经实测通过；打印 boolean 也需要核对必须为 true。无需新增测试文件，可在临时演示中预测、比较并解释实际结果。

理解验收还包括助手写的临时路径、演示样例、错误包装和应用边界。能从保存/失败/重新加载追踪整条链，并说明每一步所有权与副作用后，再将本课知识和理解标 ✅。

<a id="acceptance"></a>

## 整课验收

六项阅读与修改练习、八个持久化 TODO 属于同一课。先运行 `course04 --intro` 核对日志与错误，再完成实现并运行 `course04` 及跨进程读取。按 B 部分的功能表逐项检查，还要能解释 A/B 的完整调用链、错误与副作用。

编译和演示通过不能自动替代理解验收；保留已通过的功能成果，缺少解释证据的项目留空。整课功能与理解均通过后，再进入第 05 课 C FFI。
