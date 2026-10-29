# 源码阅读地图

[课程目录](Course-Syllabus.md) · [学习方法](Rust-Learning-Plan.md) · **[练习 4.1](lessons/04-errors-logging-and-toml-storage.md#exercise-4-1)**

阅读时说明职责、调用方、输入输出、所有权、错误和副作用。下表覆盖现有实现、样例、测试及辅助函数；逐课的任务与验收以讲义为准。

## 项目实现

| 文件 | 阅读重点 | 课程 |
|---|---|---|
| [Cargo.toml](../Cargo.toml)、[Cargo.lock](../Cargo.lock)、[.gitignore](../.gitignore) | package/lib/bin、edition/MSRV、版本/feature、开发依赖、产物忽略 | 03～04 |
| [lib.rs](../src/lib.rs) | 公开模块与库边界 | 04 A |
| [main.rs](../src/main.rs) | 配置 → 日志 → 应用，错误报告与退出码 | 04 A |
| [app.rs](../src/app.rs) | 主配置展示、设备选择与业务事件 | 04 A |
| [app/startup.rs](../src/app/startup.rs) | OsString、参数、current_exe、缺失策略、reject_log_path_conflicts/comparison_path | 04 A |
| [app/logging.rs](../src/app/logging.rs) | registry/layer、独立过滤、RUST_LOG、OpenOptions/Mutex<File>、Logging 与错误 target | 04 A |
| [error.rs](../src/error.rs) | 四层错误、Display/source/From、上下文与安全摘要 | 02～04 |
| [model.rs](../src/model.rs) | 模块声明与重导出 | 01～03 |
| [model/device.rs](../src/model/device.rs) | 结构体、借用、查询/排序、合法修改、删除交还所有权 | 01～03 |
| [model/address.rs](../src/model/address.rs) | ID 与私有 Host 校验顺序 | 02 |
| [model/connection.rs](../src/model/connection.rs) | 协议枚举、Serde 标签与校验分派 | 02～03 |
| [nfs.rs](../src/model/connection/nfs.rs)、[smb.rs](../src/model/connection/smb.rs) | 协议配置、身份、路径、端口与密码原值 | 02～03 |
| [config.rs](../src/config.rs) | 快照 From/TryFrom；AppConfig/LoggingSettings 校验、解码、路径规则与编解码 TODO | 03、04 B |
| [store.rs](../src/store.rs) | DeviceStore、构造入口、主文件读取、临时路径与剩余存储 TODO | 04 B |

启动路线：main → startup → store/config → logging → app → 可选设备读取。模型/存储返回错误，应用边界记录和显示；加载主文件不预先读取设备。

## 独立样例与测试

| 文件 | 阅读重点 |
|---|---|
| [course01](../examples/course01.rs)、[course02](../examples/course02.rs)、[course03](../examples/course03.rs)、[course04](../examples/course04.rs) | 对应课程的最小 API 用法与失败传播 |
| [examples/common/mod.rs](../examples/common/mod.rs) | 虚构设备构造和隐藏密码的展示 |
| [course01_model](../tests/course01_model.rs)、[course02_protocols](../tests/course02_protocols.rs)、[course03_snapshots](../tests/course03_snapshots.rs) | 输入、具体错误、保存状态和快照边界的断言 |
| [course04_config](../tests/course04_config.rs)、[course04_store](../tests/course04_store.rs) | 正常/错误/边界输入、文件上下文、只读范围 |
| [course04_cli](../tests/course04_cli.rs) | Cli 夹具、Command、二进制定位、环境/cwd/全局 subscriber 隔离和日志断言 |
| [tests/common/mod.rs](../tests/common/mod.rs)、[model.rs](../tests/common/model.rs) | include_str! 只读夹具、TestResult 与设备测试数据 |
| [主配置](../examples/configuration/config.toml)、[home](../examples/configuration/devices/home.toml)、[backup](../examples/configuration/devices/backup.toml) | TOML 表归属、引用、协议标签、可选字段和空列表 |

练习 4.1 要求新建的测试文件不在上表中；文件名与步骤见[题目](lessons/04-errors-logging-and-toml-storage.md#exercise-4-1)。

阅读一个测试时，先预测输入会触发哪条规则、失败后哪些状态应不变，再运行确认；不要只看测试名称或通过数量。
