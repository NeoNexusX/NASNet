# 第 04 课：完整源码、错误、日志与 TOML 分文件配置

**状态：教学方案与配套接口已调整，六项 A 部分练习、一个新库练习及十三个函数 TODO 待完成。** 课程仍为 12 课，第 04 课保留 A/B 两部分。TOML 文件存储尚未实现，不因类型和示例已提供而标记完成。

[返回课程目录与知识清单](../Course-Syllabus.md) · [上一课](03-encapsulation-and-serialization.md) · [源码阅读指南](../Code-Reading-Guide.md)

## 本课目录

1. [A：结构、错误与日志](#part-a)，先认识新依赖与所有助手代码，完成 4.A～4.F。
2. [B：软件主配置与独立设备文件](#part-b)，先完成 4.1 的 TOML 小练习，再填写 4.2～4.14。
3. [整课验收](#acceptance)，检查文件影响范围、按需加载、失败状态与完整理解。

```sh
cargo run --locked -- course04 --intro
# 后续函数填写前会停在 TODO 4.8（load_app）
cargo run --locked -- course04
# 填写后：只读主配置与引用，不提前加载全部设备
cargo run --locked -- course04 examples/configuration
# 填写后：额外加载选中的一台设备
cargo run --locked -- course04 examples/configuration nas-home
```

<a id="part-a"></a>

## A：项目结构、统一错误与日志

### 1. 先认识本轮的依赖

| 库 | 它负责什么 | 本项目如何用 | 本次是否新增 |
|---|---|---|---|
| `serde` | 在 Rust 类型和序列化数据模型之间转换 | derive `Serialize` / `Deserialize`，定义字段和枚举规则 | 已有，回补介绍 |
| `serde_json` | 第 03 课已完成的 JSON 编解码 | 保留作历史练习；新磁盘方案使用 TOML | 已有 |
| `toml` | 把 Rust 类型与 TOML 文档相互转换 | 主配置与单设备文件的 `from_str` / `to_string_pretty` | 新增，先学习再填写 |
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

    let span = tracing::info_span!("load_configuration", file = "config.toml");
    let _entered = span.enter();
    tracing::info!(devices = 2, "configuration loaded");
}
```

这个独立例子使用已经见过的 expect 显式处理初始化失败，避免隐藏返回类型；项目正式入口使用 AppError 返回和统一报告。先安装 subscriber，才有接收与显示事件的处理器。`INFO` 是事件级别；`devices = 2` 是可独立查询的字段；最后的字符串是消息。span 创建后通过 enter 进入作用域，guard 存活期间发出的事件关联该上下文。guard 离开作用域结束进入状态。

### TOML 新库：先比较，再使用

TOML 适合手工编辑配置，支持 `#` 注释、`[table]` 与 `[[array_of_tables]]`；它与 JSON 都可以经由 Serde 转换成 Rust 类型。本轮使用 `toml = "1.1"`，具体版本由 Cargo.lock 锁定。[toml 库文档](https://docs.rs/toml/latest/toml/)说明了其 Serde 支持；[TOML 语法规范](https://toml.io/en/v1.0.0)可查表、数组与字符串规则。

先完成 4.1 的小练习，再接入存储：

```rust
let text = toml::to_string_pretty(&settings)?;
let restored: SoftwareSettings = toml::from_str(&text)?;
```

这是放在返回相应 Result 的函数中的片段，不是另一个程序入口。编码错误是 `toml::ser::Error`，解码错误是 `toml::de::Error`；两者已在 error 模块分别包装。TOML 不提供 JSON 的 null，可选字段为 None 时采用省略字段的表示，不能机械写成 `port = null`。

通过普通 Serde 编码后再写回文件，会重新生成布局，不能承诺保留人工注释、空行和键顺序；本轮接受这一行为。需要保留原有排版时，再学习专门的编辑方式，当前不额外引入库。

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
| `ConfigIoError` | JSON 复习 / TOML 与恢复边界 | TOML 编码/解码、模型校验、不支持版本；JSON 变体保留供第 03 课使用 |
| `StoreError` | 配置文件操作 | 操作名、路径、I/O 原因，或配置错误 |
| `AppError` | 应用入口统一接收 | 配置/存储错误、参数、日志初始化 |

用一个具体路径理解：load_device 解析到非法 Host → ConfigError → ConfigIoError::Validation → StoreError::ConfigFile（包含路径）→ AppError::Store → main 调用 report_error。每层只表达对应责任，不逐层记录日志。

- `Display` 决定 `{error}` 的英文文字；它不会执行校验，也不自行发日志。
- `std::error::Error::source` 借出下层错误，保留具体原因链；没有来源的变体返回 None。引用来自当前 error，`'static` 约束错误类型不含短期借用，并不是让借出的引用永久有效。
- `From` 告诉 `?` 怎样将下层错误转换为当前返回类型。读取文件时还需要 `StoreError::io(operation, path, error)`，因为单个 io::Error 本身没有业务操作和配置路径。
- `AppError::kind` 用于结构化日志字段；`exit_code` 区分参数/未知课程的 2 和其他错误的 1。

所有程序错误文字与日志事件文字使用英文，讲义和课程说明保留中文。TOML 编解码错误的 Display 使用安全英文摘要，不复述配置正文；JSON 复习错误只显示行列。source 保留各库的具体错误类型，不能直接全量写进日志。**report_error 在 ERROR 启用时记录安全的顶层 Display 与 kind；过滤禁用 ERROR 时改用一次 stderr 显示，不递归打印 source**，因为 JSON/TOML 解码错误的底层文字可能包含输入值。

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
| 4.F | 跟踪 app::run 的 OsString/Unicode 检查、参数个数与目录借用；尝试未知课程和额外参数 | 解释 `course04 [directory] [device-id]` 与 `--intro` 怎样分支；说明未来启动时先读主配置、采用 software.log_level，RUST_LOG 如何覆盖它 |

然后补验前三课完整调用链，问题见[源码指南](../Code-Reading-Guide.md)。已经通过的模型实现不用重写。


### Address 校验的边界也要补读

当前 Host 规则只服务于 Address，不需要独立公开。`Address::validate()` 先检查自身 ID，再调用私有 `self.validate_host()`。拆成小方法是为可读性，放回 impl 并不等于把所有检查塞进一个大函数；可见性和函数拆分是两个决定。演示改为构造候选 Address 并调用公开 validate，没有绕过 ID 规则。

<a id="part-b"></a>

## B：主配置与单设备 TOML

### 1. 格式和文件组织分别解决什么

TOML 改变文本表示与手工编辑体验；每台设备一个文件改变 I/O 粒度。即使仍使用 JSON，拆文件也能只读一台；改成 TOML 也不会自动产生缓存、局部解析或多文件事务。本课统一选 TOML，避免维护两套正式格式；第 03 课 JSON 代码继续作为已完成的 Serde 练习。

```text
配置目录/
├── config.toml
└── devices/
    ├── home.toml
    └── backup.toml
```

可运行阶段的配置目录由用户传入，不默认把工作目录当配置根。相对设备路径基于 **config.toml 所在目录**，与启动时 cwd 无关。完整虚构样例在 [examples/configuration](../../examples/configuration/config.toml)。

主配置示例：

```toml
schema_version = 1

[software]
language = "zh-CN"
log_level = "info"

[[devices]]
id = "nas-home"
path = "devices/home.toml"

[[devices]]
id = "nas-backup"
path = "devices/backup.toml"
```

`config.toml` 负责软件自身设置与设备文件清单，不内嵌地址、协议或密码。引用中的 id 用于选择与核对身份；path 指向独立设备文件。devices 数组的顺序决定列表展示顺序，不依赖目录扫描结果。

设备文件示例：

```toml
schema_version = 1

[device]
id = "nas-home"
name = "Home NAS"
connections = []

[[device.addresses]]
id = "lan"
host = "192.168.50.10"
priority = 1
```

实际 [home.toml](../../examples/configuration/devices/home.toml) 还包含独立 SMB 配置和虚构密码；[backup.toml](../../examples/configuration/devices/backup.toml) 展示空列表。读这些文件时要解释 TOML 的表归属，避免把字段写进前一个 table；空 connections 必须属于 device。

主文件与每个设备文件各自有 schema_version。使用现有 `ConfigDocument { schema_version, device }` 作为单设备文档，继续复用 DeviceData 转换与 Device::try_from 校验。现有 serde 的协议标签也要通过 TOML 实际往返验证，不能仅因 JSON 可用就假设 TOML 完全一样。

### 2. 读写与内存策略

| 操作 | 读取/写入范围 |
|---|---|
| 启动读取软件设置和设备列表 | 只读 config.toml |
| 选择某台设备 | 根据引用只读对应设备文件，校验后在应用层保存该设备对象 |
| 再次查询已加载设备 | 应用层读取已有对象；存储对象本身不缓存 |
| 修改单台设备 | 只重写该设备文件，不重写主配置和其他设备 |
| 修改软件设置 | 只重写 config.toml |
| 新增设备 | 先写新设备文件，再向主配置登记引用 |
| 移除设备 | 只更新主配置中的引用，保留设备文件 |
| 明确重新加载 | 重新读取选定文件并校验，成功后替换内存对象 |

AppConfig 只持有软件设置和 DeviceReference 清单，不持有全部 Device。StoreConnection 只持有路径，不隐式缓存、不每次查询都读取全部设备。ConfigManager/会话层未来可管理已加载设备，需明确缓存与重载规则；本轮不自动监听外部编辑、不为文件变化偷偷覆盖内存。

运行中的连接状态、目录项、传输进度属于运行时数据，不随每次变化保存到设备文件。SMB 密码仍按既有选择明文原样保存，日志不打印密码、正文或错误库可能复述的输入。

### 3. 路径和身份约束

本课先采用相对配置目录的受管理路径：非空、非绝对、扩展名为 .toml，只接受 Path::components 解析后得到的 Normal 组件，拒绝 ParentDir、RootDir、Prefix 和 CurDir。这个规则按 Path API 的解析结果判断，不承诺识别所有等价的原始路径写法。文件名由引用明确指定，不把设备 ID 未经处理直接拼接成路径。不自动扫描目录注册文件。

主清单中设备 ID 和路径不能重复；各设备内地址和连接 ID 仍只要求局部唯一。读取设备后必须核对文档内 id 和引用 id 一致。加载已登记但缺失的设备文件必须返回错误，不能虚构空设备或静默移除引用。

上述是首轮命名与路径规则，不等于完成对不受信任目录、符号链接或并发替换的全面隔离。当前配置目录由本用户单个后台管理；若以后支持外部路径或多写入者，先扩展契约和验收再增加能力。

### 4. 单文件替换与多文件一致性

`write_atomic` 每次只保护一个文件：先完整编码，再在同目录 create_new 临时文件，write_all、sync_all、关闭后 rename。失败尽力清理自己创建的临时文件，不能先删除旧目标，不能删掉冲突的既存文件。目录 fsync 尚未实现，不宣称完整跨平台掉电保证。

多个 TOML 文件**没有天然整体事务**。本课不提供容易误导的 save_all：

- 更新已有设备：只替换一个设备文件。
- 新增设备：先写新文件，再发布主配置引用，最后更新内存 AppConfig。新路径已有文件时拒绝覆盖。
- 如果新文件写完而主配置保存失败：保留旧主配置与内存，返回错误；可能留下未引用文件，不谎称全部回滚。重试前应查看或显式重新关联，不能自动覆盖它。
- 移除引用：先保存候选主配置，成功后更新内存；不自动删除设备文件，避免把解除关联和物理删除混为一个操作。
- 重新关联已有文件是显式操作：先加载并验证，再提交主配置；不能把目录里所有 TOML 都当设备。

示例程序只在默认临时目录演示登记和移除。用户传入目录的分支只读主配置及可选的一台设备，不修改 examples/configuration。

### 5. 练习总表

A 部分 4.A～4.F 保留。B 部分有一个新库练习和十三个函数实现，允许分几轮填写，但整课数量不变。

| 编号 | 文件/接口 | 目标 |
|---|---|---|
| 4.1 | toml 库最小练习 | 阅读库介绍与 Cargo feature，使用 SoftwareSettings 做一次编码/解码；解释与 Serde 的分工 |
| 4.2 | AppConfig::validate | 主版本、软件设置、引用 ID/路径、重复项与路径形式 |
| 4.3 | encode_app_config | 验证后编码主文件，不包含设备正文 |
| 4.4 | decode_app_config | 解析主 TOML 后验证 |
| 4.5 | encode_device_toml | 复用 DeviceData 与 ConfigDocument，编码一个设备 |
| 4.6 | decode_device_toml | 解析、版本、Device::try_from 校验 |
| 4.7 | resolve_device_path | 使用主配置所在目录解析相对路径，拒绝不支持的形式 |
| 4.8 | load_app | 只读主配置；仅主文件 NotFound 使用默认软件设置和空引用 |
| 4.9 | save_app | 只保存软件设置与引用，不碰设备文件 |
| 4.10 | load_device | 只读指定引用文件，检查文档 ID 一致 |
| 4.11 | save_device | 核对 ID，编码并只替换该文件 |
| 4.12 | write_atomic | 单文件临时写入、同步、替换和错误清理 |
| 4.13 | register_device | 候选检查、拒绝覆盖、先设备后清单、成功后提交内存 |
| 4.14 | remove_device_reference | 候选清单删除、保存成功后提交内存、不物理删除 |

建议顺序：4.1 → 4.2～4.6 → 4.7、4.12 → 4.8～4.11 → 4.13、4.14。未填写时默认演示先停在 load_app 的 TODO 4.8。Cargo check 通过只证明框架类型正确。

### 6. 各实现的关键提示

`AppConfig::validate` 用 HashSet 检查引用 ID 和路径，用 SCHEMA_VERSION 检查版本；软件 language 不为空，log_level 只接受 trace/debug/info/warn/error/off。主配置验证不打开设备文件。路径规则同时用于单设备入口，直接构造非法引用也不能绕过检查。

主文件与设备文件分别调用 `toml::from_str` / `toml::to_string_pretty`。TOML 解码/编码错误保留不同类型和 source，转换进入 ConfigIoError；错误英文摘要不输出原文。DeviceData 不代表可信设备，恢复仍必须经过校验入口。

`resolve_device_path` 先检查路径组件和扩展名，再用 config_path.parent() 组合，不能依赖 cwd。它只解析路径，不说明文件存在；已存在设备丢失时由 load_device 报带路径和操作的 I/O 错误。

`load_app` 在主文件缺失时返回 AppConfig::default，但不悄悄创建文件；主文件存在却为空或坏格式时应失败。`load_device` 没有这个默认行为，丢文件就是错误。

`save_app` / `save_device` 先编码成功，再调用同一个 write_atomic。I/O 错误用 StoreError::io 保留操作、路径和来源；文件中的 TOML 解析/校验错误用 StoreError::config(path, error) 包装，使日志知道是哪一个文件无效。编码前的校验仍保留类型化来源，不在这些逻辑中记录日志。

`write_atomic` 的临时名用目标文件名、进程 ID 与 AtomicU64 序号组合。逐段理解 static、fetch_add 和 Relaxed；这不是文件锁，create_new 才处理真实冲突。写入/同步失败时要先关闭文件，再清理，清理失败不能覆盖原始错误。

登记和移除引用使用候选 AppConfig：现有 DTO 派生 Clone 可以构造候选，保存成功才替换调用者的 config。不要为了方便直接 push/remove 当前生效清单，再在保存失败后忘记还原。

### 7. 软件日志设置如何接入

当前教学入口仍先用 RUST_LOG 或 info 初始化日志，不能声称 software.log_level 已经实际生效。A 部分 4.F 的后续小改动需要你把真实启动组织为：确定主配置路径 → load_app → 选择 RUST_LOG（若提供）否则 software.log_level → 安装 subscriber → 加载所选设备/业务操作。配置加载失败且 subscriber 未建立时，由 main 用英文 stderr 报告。

软件日志设置和设备配置不同：一个影响程序诊断行为，一个影响 NAS 访问。主配置提供 log_level，tracing-subscriber 决定如何应用；不能让 Device 或 StoreConnection 初始化全局 subscriber。

### 8. 验收矩阵

| 场景 | 必须得到的结果 |
|---|---|
| examples/configuration 主配置 | 2 个引用、软件设置正确；尚未选择设备时不读取它们 |
| 只选择 nas-home | 只读 home.toml，不要求读取 backup.toml |
| 另一个设备文件损坏 | 主清单加载与健康设备单独加载仍可成功；坏设备被选择时才报错 |
| 已登记设备文件缺失 | 英文 I/O 错误，不返回空设备 |
| 单设备更新 | config.toml 和其他设备文件字节不变 |
| 软件设置更新 | 所有设备文件字节不变 |
| 引用 ID 与文档 ID 不同 | 英文身份不一致错误 |
| 路径与 ID 重复、路径越出约定形式 | 验证失败，旧文件与内存不变 |
| 地址/协议非法、空密码、版本不支持 | 恢复校验拒绝；不打印敏感值 |
| 新增设备后主清单保存失败 | 返回错误、原主清单/内存不变；说明可能留下未引用文件 |
| 新增路径已存在 | 拒绝覆盖，不自动认领或改写文件 |
| 移除引用/重复移除 | true / false；设备文件保留 |
| TOML 注释与排版 | 不作为写回保真承诺，比较解析值而非人工注释 |
| 外部重新加载 | 明确请求才更新已加载对象，失败保留原对象 |
| 日志设置优先级 | 4.F 完成后验证 RUST_LOG 覆盖主配置 level；当前代码尚未实现该接线 |

不新增课程测试文件，按临时目录演示逐项预测和核对；示例目录仅含虚构配置。功能与理解均需针对你的实际实现验收，JSON 与 SQLite 历史报告不能替代这一轮。

<a id="acceptance"></a>

## 整课验收

先介绍 toml 再使用，逐段读完整代码，完成 A 部分练习与 B 部分十三个 TODO。解释软件设置、设备引用、设备对象与磁盘文件四者关系；跟踪一条单设备读取、修改和登记失败路径。功能和理解分别记录，全部通过后再进入第 05 课 C FFI。
