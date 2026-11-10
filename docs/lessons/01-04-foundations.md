# 第 01–04 课：已学基础（合并）

前四课写出了 NASNet 不联网的部分：内存中的设备模型、TOML 配置、文件存储，以及能读配置、写日志的命令行程序。

**状态**：已完成。代码和测试保持原样。本文只保留复习用的大纲，以及后面的课还要查的项目规则。

[课程目录](../Course-Syllabus.md) · [下一课](05-traits-and-remote-backends.md) · [源码地图](../Code-Reading-Guide.md)

## 一、学过什么

| 原课 | Rust 知识 | 写出的代码 |
|---|---|---|
| 01 设备模型、查询与排序 | struct 与 `impl`、所有权与移动、共享借用与可变借用、`Vec` 与切片、`Option`、迭代器、稳定排序 | [device.rs](../../src/model/device.rs)：`Device::new`、`rename`、`addresses`、`find_address`、`addresses_by_priority` |
| 02 校验与 NFS/SMB 配置 | 带数据的 enum、`match`/`if let`/`matches!`、`Result` 与 `?`、自定义错误、字符串检查 | [address.rs](../../src/model/address.rs)、[connection.rs](../../src/model/connection.rs)、[nfs.rs](../../src/model/connection/nfs.rs)、[smb.rs](../../src/model/connection/smb.rs) 的 `validate`；[error.rs](../../src/model/error.rs) 的 `ModelError`；`try_add_address`、`try_add_connection` |
| 03 封装、生命周期与配置快照 | 模块可见性与私有字段、`iter_mut`、删除并交还所有权、生命周期、`Clone`、`From`/`TryFrom`、Serde 派生 | `try_update_*`、`remove_*`、`find_connection`；[config.rs](../../src/config.rs) 的 `DeviceData` 及其与 `Device` 的互相转换 |
| 04 测试、TOML 配置、文件存储与日志 | Serde 与 TOML、错误链（`Display`、`source`、`From`、`?`）、`Path`/`PathBuf`、文件读写与原子替换、tracing、单元测试与集成测试、模块与 lib/bin | [config.rs](../../src/config.rs) 的编解码；[store.rs](../../src/store.rs) 的 `StoreConnection`；[app.rs](../../src/app.rs)、[startup.rs](../../src/app/startup.rs)、[logging.rs](../../src/app/logging.rs) |

第 04 课的项目题号在测试文件里仍然使用：

| 题号 | 内容 | 测试 |
|---|---|---|
| P4.1–P4.3 | 主配置编码、设备文件的编码与解码 | [course04_config.rs](../../tests/course04_config.rs) |
| P4.4–P4.5 | 解析设备路径、读取单个设备 | [course04_store.rs](../../tests/course04_store.rs) |
| P4.6–P4.8 | 原子替换、保存主配置、保存单设备 | 同上 |
| P4.9–P4.10 | 登记设备、移除设备引用 | 同上 |
| P4.11–P4.13 | 启动链路、`--device` 端到端、重启后保留配置 | [course04_cli.rs](../../tests/course04_cli.rs) |

可选示例 E4.1（[course04.rs](../../examples/course04.rs)）用一个歌单程序观察嵌套 span 和日志过滤。

## 二、后面还会用到的项目规则

### 模型

- `Device` 的字段都是私有的。地址和连接只能通过 `try_add_*`、`try_update_*`、`remove_*` 修改。这些方法先校验、查重，成功后才改动设备；删除时把被删的条目交还给调用方。
- 地址的 `priority` 数值越小越优先。`addresses_by_priority` 用稳定排序，同优先级的地址按保存顺序排列，`addresses()` 仍返回保存顺序。
- Host 只做本地格式检查：不能为空，不能含空白，不能含 `://`、`/` 或 `\`。不查 DNS，也不连网。
- NFS 和 SMB 各有自己的配置类型和校验，由 `ConnectionConfig` 枚举区分。SMB 密码原样保存，不 trim。

### 配置文件

```text
examples/configuration/
├── config.toml        主配置：schema_version、[software]、[logging]、有序的 [[devices]] 引用
└── devices/
    ├── home.toml      一台设备的完整配置：schema_version 加 [device]
    └── backup.toml
```

- 读取顺序固定为“解析 → 业务校验”。所有配置类型都加了 `deny_unknown_fields`，多余的键会报错。
- `schema_version` 不等于当前版本就拒绝，比当前新或旧都一样。
- `Device` 不派生 `Deserialize`。设备文件先解析成 `DeviceData`，再经 `Device::try_from` 逐项添加，所以非法值进不了内存。
- 设备引用的路径相对主配置所在的目录，必须以 `.toml` 结尾，而且每一段都是普通名称。`..`、绝对路径和盘符都会被 `is_managed_device_path` 拒绝。
- 命令行 `--config` 的相对路径相对启动时的工作目录。
- `[logging]` 有三项：`console_level` 和 `file_level` 取 `trace`、`debug`、`info`、`warn`、`error`、`off` 之一；`file_path` 是相对路径时相对主配置所在的目录。

### 错误分层

每层只包装下一层。错误类型和它的 `From` 实现写在接收方的模块里。

| 类型 | 位置 | 表达的失败 | 向上一层 |
|---|---|---|---|
| `ModelError` | `src/model/error.rs` | 地址、协议和模型操作规则 | `ConfigIoError::ModelValidation` |
| `ConfigError` | `src/config.rs` | 软件设置、日志设置、设备引用规则 | `ConfigIoError::Validation`，或直接变成 `AppError::Config` |
| `ConfigIoError` | `src/config.rs` | TOML 编解码、版本不支持、上面两类规则 | `StoreError::Config`，补上文件路径 |
| `StoreError` | `src/store.rs` | 文件操作失败（带操作名和路径）、配置内容错误（带路径） | `AppError::Store` |
| `AppError` | `src/app.rs`，只在 bin 中 | 参数、配置、存储、I/O、日志初始化 | `app::run` 打印摘要并决定退出码 |

- `?` 只调用一次 `From::from`。需要补充操作名或路径时用 `map_err`。
- 配置里有密码，所以每一层的 `Display` 只写摘要，不拼接下层错误的文本。终端报告也不递归打印 `source`。

### 写入保证

- `write_atomic` 在目标的同一目录用 `create_new` 建临时文件，依次 `write_all`、`sync_all`、关闭，再 `rename` 覆盖目标。任何一步失败都只删自己的临时文件，目标保持旧内容。
- 只有 `ErrorKind::NotFound` 表示文件不存在。主配置缺失时用默认值创建；已登记的设备文件缺失时报错。其他错误都不能用默认值掩盖。
- 新增设备时先写设备文件，再写主配置。主配置写失败只会留下一个没被引用的设备文件。
- 内存里的 `AppConfig` 先改 `clone()` 出来的候选副本，磁盘写入成功后才整体替换。
- 移除设备只去掉引用，不删设备文件。
- 当前不提供目录 fsync、多文件事务、多个写入者合并和配置文件监听。

### 启动与日志

```text
main → app::run（唯一打印失败摘要、决定退出码的地方）
         → execute
             1. startup::prepare   解析参数，读取或初始化主配置，确定日志路径并检查冲突
             2. logging::init      创建日志目录，打开日志文件，安装 subscriber
             3. show               打印主配置；有 --device 时只读取这一个设备
```

- 前一步失败，后面的步骤不会执行。参数错误的退出码是 2，其他失败是 1。
- 库不打印，也不记日志。命令结果写 stdout，终端日志写 stderr。
- 一个 registry 装两个 `fmt::layer`，每层各自过滤。`RUST_LOG` 只覆盖终端层。
- 命令失败的事件使用专用 target `nasnet::app_error`，只进文件层；终端由 `app::run` 打印一次摘要。
- 日志不记录密码、配置正文，也不记录可能复述输入的解析错误。当前没有日志轮转。
- span 的 guard 要用具名变量保存。写成 `let _ = span.enter();` 会立刻退出 span。

### 测试手法

- 集成测试放在 `tests/`，只能使用库的公开 API。共用代码放在 [tests/common/mod.rs](../../tests/common/mod.rs)。
- 文件测试每例一个 `tempfile::tempdir()`，示例文件用 `include_str!` 读入后写进临时目录，从不改动 `examples/configuration/`。
- 端到端测试用 `Command::new(env!("CARGO_BIN_EXE_nasnet"))` 启动真实程序，检查退出码、stdout、stderr 和日志文件。
- 测试默认并行运行：不共享文件名，不修改测试进程的工作目录和环境变量。
- 先看到测试失败，再实现到通过。`0 tests`、`ignored`、`filtered out` 都不算通过。

## 三、复习

```sh
cargo test --locked --test course04_config --test course04_store --test course04_cli
cargo run --locked -- --config examples/configuration/config.toml --device nas-home
```

能解释以下问题，就可以继续第 05 课：

- `find_address` 返回的引用为什么不能在 `try_add_address` 之后继续使用。
- 为什么 `Device` 不直接派生 `Deserialize`。
- 单设备保存为什么不碰其他文件；登记设备时主配置写失败会留下什么。
- 损坏的配置为什么不能被默认值掩盖。
- 日志的两层为什么分别过滤，而不是把过滤器放在整个 registry 上。
