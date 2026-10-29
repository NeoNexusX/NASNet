# 第 04 课：启动、日志、TOML 配置与 Rust 测试

本课使用现有项目完成两件事：读懂配置驱动的启动与日志流程，亲手完成尚未实现的设备存储。`4.2`、`4.4`、`4.8` 已实现，作为阅读与修改练习；其余十个函数仍是 TODO。

## 从哪里开始

1. 阅读 [A：依赖与启动](#part-a)和 [C.1：测试入门](#test-basics)。
2. 完成下面的 **[练习 4.1：TOML 编解码入门](#exercise-4-1)**。题目包含新建文件位置、测试名称、操作步骤和验收命令。
3. 阅读 [C.2～C.4：测试组织、资源与隔离](#test-organization)，再阅读 `4.2`、`4.4`、`4.8`。
4. 按 B 部分各题补测试和实现，完成 C.5 练习，全量回归后进行[整课验收](#acceptance)。

[课程目录](../Course-Syllabus.md) · [源码地图](../Code-Reading-Guide.md) · [上一课](03-encapsulation-and-serialization.md)

<a id="part-a"></a>

## A：依赖、启动、错误与日志

### 依赖分别负责什么

| 依赖 | 用途 | 本课关注点 |
|---|---|---|
| `serde` | 定义 Rust 值与序列化数据模型的转换规则 | `Serialize`、`Deserialize`、字段与枚举属性 |
| `toml` | Rust 候选值与 TOML 文本的编解码 | `to_string_pretty`、`from_str`、编码/解码错误 |
| `tracing` | 产生带级别、字段和 span 的诊断事件 | 事件和操作上下文 |
| `tracing-subscriber` | 接收、过滤、格式化事件 | 两个输出层分别过滤 |
| `tempfile` | 为测试创建独立的临时资源 | 仅开发依赖；`TempDir` 的生命周期 |

TOML 的 `[table]` 表达表，`[[items]]` 表达数组中的表；可选字段为 `None` 时省略，不能写成 `null`。普通 Serde 编码会重新生成排版，不保留人工注释。解析成功只代表类型匹配，恢复业务模型仍需校验。

Cargo 中的日志声明：

```toml
tracing = { version = "0.1", default-features = false, features = ["std"] }
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
```

`version = "0.1"` 允许 `>=0.1.0, <0.2.0`，具体版本由 Cargo.lock 锁定。tracing 默认启用 `std` 和 `attributes`；这里关闭默认组合后只请求 `std`。`attributes` 提供 `#[instrument]`，现有 `info!`、`debug!`、`info_span!` 不需要它。subscriber 的 `env-filter` 支持过滤指令，`fmt` 支持格式化输出；feature 是编译时选择，日志等级是运行时设置。

阅读：[Serde 派生](https://serde.rs/derive.html)、[toml](https://docs.rs/toml/latest/toml/)、[tracing](https://docs.rs/tracing/latest/tracing/)、[分层过滤](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/index.html#per-layer-filtering)。

### 启动调用链

```text
main
  → startup::prepare：解析 --config / --device，确定主文件路径
  → StoreConnection：读取并校验主配置
  → logging::init：打开日志文件，安装一次 subscriber
  → app::run：展示主配置，或按需读取选中的设备
```

- 默认主文件：可执行文件同级的 `config.toml`；缺失时使用内置值，不创建主文件。
- `--config <文件>`：显式文件必须存在；相对参数先相对启动时的 cwd 定位。
- 配置里的相对设备/日志路径：相对主文件目录解析。
- 配置损坏、权限错误或日志初始化失败：向 stderr 报告并退出，不静默使用默认值。

主程序使用 `cargo run --locked -- --config <文件>`；课程样例使用 `cargo run --locked --example course04`；自动验收使用 `cargo test --locked`。这三种入口互相独立。完整文件职责见[源码地图](../Code-Reading-Guide.md)。

### 错误与日志边界

| 类型 | 职责 |
|---|---|
| `ConfigError` | 模型、软件/日志设置或引用规则不合法 |
| `ConfigIoError` | TOML 编解码、模型恢复校验或版本不支持 |
| `StoreError` | 文件操作与配置错误，携带操作、路径和来源 |
| `AppError` | 应用入口接收的失败及退出码；参数错误为 2，其他失败为 1 |

`Display` 生成英文摘要，`Error::source` 借出下层错误，`From` 让 `?` 完成类型转换。借出的 source 受当前错误的生命周期约束；`'static` 不表示返回的引用永久有效。模型和存储只返回错误，应用边界记录与展示一次。TOML 底层错误可能含输入值，正常报告不递归打印 source。

一个 registry 安装两个 `fmt::layer`：文件默认 TRACE，终端默认 INFO；每层自己 `with_filter`。`RUST_LOG` 只覆盖终端过滤，不能作为全局过滤器提前丢弃文件需要的 DEBUG/TRACE。

文件通过 `OpenOptions::append` 和 `Mutex<File>` 同步追加，保留 target 且关闭 ANSI；终端日志写 stderr，命令结果写 stdout。应用致命错误使用专属 target 交给文件层，终端层排除此 target，再由 `report_error` 直接显示一次失败摘要。这样终端等级为 off 时仍能看到操作失败。

同步追加不等于每条事件都已同步到磁盘；当前没有日志轮转。日志不记录密码、配置正文或可能复述输入的原始解析错误。

### 阅读与修改练习 4.A～4.F

| 编号 | 位置与任务 | 验收 |
|---|---|---|
| 4.A | 阅读 main/startup/app/logging/error，画出上述调用链 | 解释各层输入输出、文件副作用和错误传播 |
| 4.B | 在 `tests/course04_cli.rs` 新增“终端 trace、文件 error”的用例 | 两层输出互不限制，断言事件标识与等级 |
| 4.C | 在独立 `examples/course04.rs` 中配置简单 subscriber 并练习嵌套 span | 解释 `enter()` guard 的作用域和事件所属 span |
| 4.D | 阅读 CLI 过滤测试，补一例配置终端等级与 `RUST_LOG` 不同的情况 | 环境只改变子进程终端，文件过滤不变 |
| 4.E | 阅读错误枚举、Display/From/source，跟踪损坏主配置的失败 | 解释具体类型和来源，终端不泄露原始输入 |
| 4.F | 阅读 startup 参数解析，补一个非法参数组合测试 | 验证默认路径、重复选项、缺少值和退出码 |

<a id="part-b"></a>

## B：TOML 入门与分文件存储

<a id="exercise-4-1"></a>

### 练习 4.1：TOML 编解码入门

**动手位置：新建 `tests/course04_toml_basics.rs`。这个文件需要你创建，当前仓库没有替你写好这道练习。** 只使用 `src/config.rs` 中已有的 `SoftwareSettings` 和 toml 库，不调用尚未完成的项目编解码或文件保存函数，也不修改 main。

先读 [C.1：测试入门](#test-basics)，再编写三个 `#[test]` 函数：

| 测试函数名 | 你要写的步骤 | 必须断言的结果 |
|---|---|---|
| `software_settings_round_trip` | 构造 `SoftwareSettings { language: "zh-CN".to_owned() }`；用 `toml::to_string_pretty` 编码；用 `toml::from_str` 解码，目标类型明确为 `SoftwareSettings` | 文本包含 language；恢复后的 language 等于原值 |
| `rejects_unknown_software_field` | 解析含 `language = "zh-CN"` 和 `unknown = true` 的文本 | 返回解码错误，解释 `deny_unknown_fields` 的作用 |
| `rejects_missing_language` | 将空 TOML 文本解析为 `SoftwareSettings` | 返回解码错误，必需字段不能悄悄获得默认值 |

第一个测试可以从下面的框架开始，按表格补齐逻辑：

```rust
use nasnet::config::SoftwareSettings;

#[test]
fn software_settings_round_trip() {
    todo!("4.1: construct, encode, decode, and assert");
}
```

另外两个函数同样使用 `#[test]`。合法输入可用 `expect` 让意外错误导致测试失败；错误场景用 `is_err()` 或错误匹配断言。不要用 `#[should_panic]` 或 `#[ignore]` 让占位代码冒充完成。

在项目根目录运行：

```sh
# 创建文件并写出三个测试函数后，先确认 Cargo 发现了它们
cargo test --locked --test course04_toml_basics -- --list
# 再运行这道练习
cargo test --locked --test course04_toml_basics
```

**验收：列出三个指定测试，运行结果为 `3 passed; 0 failed; 0 ignored`。** 文件未创建时 Cargo 会报告目标不存在；只复制占位框架会失败；看到 `0 tests` 不算通过。完成后解释：Serde、toml 和文件 I/O 各自负责什么，为什么这道练习不需要读写设备文件。

### 文件结构与共同规则

主配置包含 software、logging 与有序的设备 id/path 引用；每台设备独立保存 `ConfigDocument { schema_version, device }`。直接阅读[主文件](../../examples/configuration/config.toml)、[完整设备](../../examples/configuration/devices/home.toml)和[空设备](../../examples/configuration/devices/backup.toml)，以这些文件为格式依据。

设备路径只接受非空、相对、`.toml` 扩展名，`Path::components` 全部为 Normal；主清单的 ID 和路径不得重复。读取设备后核对文档 ID 与引用一致。路径规则按 Path API 的解析结果判断，不是完整的符号链接或多进程隔离。

日志路径可以是明确的绝对路径或相对主配置目录的路径，拒绝 Windows 盘符相对/仅根路径。startup 在打开日志前检查它是否与主配置或登记设备冲突。这里检查路径信息，不加载设备正文。

`StoreConnection` 不缓存设备，不扫描目录自动登记；已登记设备缺失必须报错。运行时连接状态、目录项和传输进度不写入设备配置。

下面每个函数题先写行为测试，再实现；阅读题在现有测试中补一个边界用例。后续测试文件见 [C：测试地图](#test-map)。

<a id="exercise-4-2"></a>

### 练习 4.2：主配置校验（阅读与修改）

位置：`src/config.rs` 的 `AppConfig::validate`、`LoggingSettings::validate` 和 `is_managed_device_path`；测试：`tests/course04_config.rs`。

跟踪版本 → 软件/日志设置 → 设备引用的检查顺序。解释 HashSet 如何发现重复项；新增一个非法语言、日志级别或路径用例，断言具体错误。校验不打开设备文件。

<a id="exercise-4-3"></a>

### 练习 4.3：编码主配置（TODO）

位置：`src/config.rs::encode_app_config`；补到 `tests/course04_config.rs`。

先校验，再编码主文件；只包含软件、日志设置和引用，不嵌入设备正文。与已有 `decode_app_config` 往返后比较字段和引用顺序，不比较注释或整份排版。非法候选必须返回校验错误。

<a id="exercise-4-4"></a>

### 练习 4.4：解码主配置（阅读与修改）

位置：`src/config.rs::decode_app_config`；测试：`tests/course04_config.rs`。

解释 `toml::from_str` 如何确定 AppConfig 类型，以及解析成功后为什么还调用 validate。分别构造语法错误、版本错误和引用错误，说明它们的错误类型与检查顺序。

<a id="exercise-4-5"></a>

### 练习 4.5：编码单设备（TODO）

位置：`src/config.rs::encode_device_toml`；补到 `tests/course04_config.rs`。

用现有 `DeviceData::from` 和 `SCHEMA_VERSION` 构造 ConfigDocument 后编码。设备地址、连接顺序、协议配置和密码必须原样保留；编码结果含密码，不能写入终端或日志。

<a id="exercise-4-6"></a>

### 练习 4.6：解码并恢复单设备（TODO）

位置：`src/config.rs::decode_device_toml`；补到 `tests/course04_config.rs`。

解析 ConfigDocument → 检查版本 → `Device::try_from`。分别验证非法 TOML、未知协议、版本不支持与业务校验错误；合法设备、空列表、可选字段和密码空格应能往返。复用现有快照恢复，不直接反序列化为 Device。

<a id="exercise-4-7"></a>

### 练习 4.7：解析设备路径（TODO）

位置：`src/store.rs::StoreConnection::resolve_device_path`；补到 `tests/course04_store.rs`。

应用共同路径规则后，基于 `config_path.parent()` 组合路径。改变测试子进程的 cwd 不得影响设备定位；非法路径返回错误。这里只处理路径形式，不假定文件存在。

<a id="exercise-4-8"></a>

### 练习 4.8：读取主文件（阅读与修改）

位置：`src/store.rs` 的 `load_app`、`load_app_required`、`new` 和 `from_config_path`；测试：`tests/course04_store.rs`。

`load_app_required` 保留读取操作、路径与来源；`load_app` 只把 NotFound 转成默认值。解释 startup 为何对显式配置选择前者、默认配置选择后者。损坏、空文件和其他 I/O 错误都不能当成缺失；读取主文件不预加载设备、不创建文件。

<a id="exercise-4-9"></a>

### 练习 4.9：保存主文件（TODO）

位置：`src/store.rs::StoreConnection::save_app`；补到 `tests/course04_store.rs`。

编码成功后调用 write_atomic，只更新主配置。先保存其他设备文件的字节，再验证更新软件/日志设置不会改动它们；非法配置不能覆盖旧主文件。

<a id="exercise-4-10"></a>

### 练习 4.10：按引用加载设备（TODO）

位置：`src/store.rs` 中 DeviceStore 的 `load_device`；补到 `tests/course04_store.rs`。

解析路径，只读该设备，解码校验后核对 ID。另一设备文件损坏不能影响健康设备的独立加载；已登记文件缺失须报带路径的 I/O 错误，不能返回空设备。

<a id="exercise-4-11"></a>

### 练习 4.11：保存单设备（TODO）

位置：`src/store.rs` 中 DeviceStore 的 `save_device`；补到 `tests/course04_store.rs`。

先核对引用与设备 ID，再编码并替换指定文件。用字节断言证明主配置和其他设备文件不变；ID 不一致或编码失败时不写入。

<a id="exercise-4-12"></a>

### 练习 4.12：单文件原子替换（TODO）

位置：`src/store.rs::write_atomic` 和 `temporary_path`；补到 `tests/course04_store.rs`。

创建父目录 → 同目录 create_new 临时文件 → write_all → sync_all → 关闭 → rename。解释 static AtomicU64、Relaxed 和 create_new 各自的职责；临时名称不是文件锁。

失败时尽力清理自己创建的临时文件，保留原始错误；不能先删除旧目标或删除冲突文件。测试成功替换和可控失败后的旧内容。当前不实现目录 fsync，不承诺完整掉电保证。

<a id="exercise-4-13"></a>

### 练习 4.13：登记设备（TODO）

位置：`src/store.rs::StoreConnection::register_device`；补到 `tests/course04_store.rs`。

构造候选主配置并校验重复 ID/路径和设备 ID；拒绝覆盖已存在的新设备路径。先保存设备，再保存候选主配置，最后提交内存。主文件保存失败时保留旧主文件与内存，但可能留下未引用设备文件：测试应确认并说明这一状态，不能声称跨文件回滚。

<a id="exercise-4-14"></a>

### 练习 4.14：移除设备引用（TODO）

位置：`src/store.rs::StoreConnection::remove_device_reference`；补到 `tests/course04_store.rs`。

只从候选清单移除并保存，成功后替换内存；存在返回 true，不存在返回 false。设备文件保留，保存失败时内存和原主文件不变。重新关联是显式读取、校验和保存清单，不通过扫描目录认领。

实现顺序：`4.1` → 阅读 `4.2/4.4/4.8` → `4.3/4.5/4.6` → `4.7/4.12` → `4.9～4.11` → `4.13/4.14`。

<a id="part-c"></a>

## C：Rust 自动化测试

<a id="test-basics"></a>

### C.1：测试入门

`cargo test` 编译测试程序，由 Rust 自带的 libtest 收集 `#[test]` 函数并报告结果。测试不由项目 main 调用，普通测试意外 panic 或断言失败就算失败。

每例包含“准备输入 → 调用 API → 断言结果”。例如，`tests/course04_config.rs` 已有：

```rust
#[test]
fn rejects_unknown_console_level() {
    let mut config = nasnet::config::AppConfig::default();
    config.logging.console_level = "verbose".to_owned();
    assert!(matches!(
        config.validate(),
        Err(nasnet::error::ConfigIoError::Validation(
            nasnet::error::ConfigError::InvalidLoggingSettings
        ))
    ));
}
```

- `assert!(条件)`：条件必须为真。
- `assert_eq!(实际值, 预期值)`：检查相等，失败时展示两边。
- `assert!(matches!(结果, 模式))`：检查具体错误枚举；matches! 本身只返回 bool。
- 测试可返回 `Result<(), E>` 并用 `?`；返回 Err 就失败。业务函数返回预期 Err，则由错误断言判定通过。

测试中使用 expect/unwrap 表示该前提不成立就失败。不能用 `#[should_panic]` 让 TODO 占位通过；ignored、filtered out、0 tests 都不等于已经验收。

<a id="test-organization"></a>

### C.2：测试怎样组织

| 类型 | 位置与边界 |
|---|---|
| 单元测试 | `#[cfg(test)]` 测试子模块，可拆到单独文件；能访问所属模块的私有逻辑 |
| 集成测试 | 根目录 `tests/*.rs`，由 Cargo 发现并编译为独立测试程序；使用库的公开 API |
| 文档测试 | API 的 `///` Rust 代码块，由 rustdoc 提取执行 |

`#[test]` 登记函数，`#[cfg(test)]` 控制模块只在测试构建时编译。集成测试不需要手写 main 或再加 cfg(test)。普通课件 Markdown 不会自动执行；某个 unit/doc target 显示 0 tests 时，还需查看实际的集成测试结果。

<a id="test-map"></a>

### C.3：现有测试与辅助资源

| 文件 | 内容 |
|---|---|
| `tests/course01_model.rs` | 所有权、查找、重命名、稳定排序 |
| `tests/course02_protocols.rs` | Host/NFS/SMB 校验和失败保留 |
| `tests/course03_snapshots.rs` | 更新、删除、借用、快照与恢复 |
| `tests/course04_config.rs` | 主配置解码、版本/设置/引用校验、错误来源 |
| `tests/course04_store.rs` | 主文件读取、缺失策略、I/O 上下文、只读范围 |
| `tests/course04_cli.rs` | 真实二进制的参数、路径、日志过滤、追加与错误反馈 |
| `tests/common/` | 只读配置、虚构设备和 TestResult |

练习 4.1 的 `tests/course04_toml_basics.rs` 需要你另行创建；上表中的现有配置测试不等于已完成该题。

开发依赖 `tempfile = "3"` 提供 `tempfile::tempdir()`。TempDir 持有目录生命周期，path() 只借出路径；先结束文件操作和子进程，再让目录离开作用域清理。需要检查清理错误可用 close()。标准库 temp_dir() 只返回临时目录位置。

`tests/common/mod.rs` 中 TestResult 是 `Result<(), Box<dyn Error>>` 的别名，便于测试用 `?` 传播多种错误；业务函数仍使用自己的错误类型。

### C.4：文件、CLI 与日志隔离

libtest 默认可并行运行测试。每例用独立 TempDir，不修改示例或真实配置，不共享固定文件名、测试进程 cwd、环境变量或全局 subscriber。

CLI 测试用 `Command::new(env!("CARGO_BIN_EXE_nasnet"))` 启动项目二进制。Command 的 env/env_remove/current_dir 只设置子进程；output() 等待退出，提供 status/stdout/stderr。每个子进程分别初始化 subscriber；Rust 2024 的 set_var/remove_var 是 unsafe，本课不靠修改测试进程环境来隔离。

逐段阅读 CLI 夹具的 raw_command/command、write_config、log 和 portable_binary。默认路径用例复制二进制到临时目录后从另一 cwd 启动；日志只断言事件标识、级别和次数，不固定时间戳。夹具使用 toml 库生成输入，不代表项目 encode_app_config 已实现。

### C.5：运行与练习 4.T1～4.T4

```sh
cargo test --locked
cargo test --locked --test course04_config
cargo test --locked --test course04_cli file_trace_and_console_info
cargo test --locked rejects_unknown_console_level
cargo test --locked -- --nocapture
```

`--test` 选择测试目标，名称用于筛选函数；`--` 后的参数传给测试运行器。`--nocapture` 用于调试打印，不能替代断言。

| 编号 | 任务与验收 |
|---|---|
| 4.T1 | 阅读并运行 `rejects_unknown_console_level`；临时改成合法输入，解释断言为何失败，再恢复 |
| 4.T2 | 在配置或协议测试中新增一个边界用例，断言具体错误与失败后状态 |
| 4.T3 | 阅读 TempDir/CLI 夹具，验证一个新日志组合且不影响其他测试 |
| 4.T4 | 为下一个函数 TODO 先写行为测试，观察失败，完成实现后执行单目标与全量回归 |

阅读：[测试编写](https://doc.rust-lang.org/book/ch11-01-writing-tests.html)、[测试组织](https://doc.rust-lang.org/book/ch11-03-test-organization.html)、[Cargo 集成测试](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#integration-tests)、[tempfile](https://docs.rs/tempfile/latest/tempfile/)。

<a id="acceptance"></a>

## 整课验收

完成 4.1、六项 A 练习、三个阅读题、十个实现 TODO 及四项测试练习。功能以对应测试和全量回归为证据；理解以你的调用链说明、结果预测和修改验证为证据。

特别检查：单设备保存不碰其他文件、坏配置不被默认值掩盖、缺失设备报错、ID 核对、日志两层独立过滤、发布失败后磁盘与内存状态明确。当前测试只覆盖已实现部分，不代表未完成 TODO、其他平台或真实 NAS 已验收。
