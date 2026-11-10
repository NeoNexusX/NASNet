# 源码地图

[课程目录](Course-Syllabus.md) · [技术设计](NAS-Client-Architecture.md)

读一个文件时说清楚：它负责什么、谁调用它、输入输出是什么、谁拥有数据、可能返回哪些错误、有没有副作用。

## 结构与依赖方向

```text
src/
├── lib.rs          库入口：公开 model、config、store、remote
├── model.rs        设备、地址、协议模型
│   └── model/      address、device、connection（nfs、smb）、error
├── config.rs       快照转换、主配置校验、TOML 编解码，以及它们的错误类型
├── store.rs        主配置与设备文件的读写，以及 StoreError
├── remote.rs       远程访问契约（两个 trait）、按优先级连接、目录整理，以及它们的错误类型
├── main.rs         进程入口：把参数交给 app::run
└── app.rs          bin 私有：启动流程、打印配置、AppError 与退出码
    └── app/        startup（参数与主配置）、logging（日志）
examples/           独立示例练习
└── configuration/  虚构的主配置与设备文件，TOML 格式以它们为准
tests/              项目练习的自动测试
└── common/         测试共用的主配置和返回类型
```

依赖只能向下：app → store → config → model；remote 只依赖 model。模型和配置不读写文件，库不输出日志；只有 app 打印到终端并决定退出码。错误类型怎样逐层包装见[第 01–04 课的“错误分层”](lessons/01-04-foundations.md)。

文件只在内容能独立成段时才拆分：错误类型和产生它的代码放在同一个文件；`ModelError` 被 model 的多个文件共用，所以单独放在 `model/error.rs`。

## 项目文件

| 文件 | 职责 | 副作用 | 课 |
|---|---|---|---|
| [model/device.rs](../src/model/device.rs) | Device：私有列表、只读访问、查找与排序、校验后修改、删除交还所有权 | 无 | 01–03 |
| [model/address.rs](../src/model/address.rs) | Address 与 ID、Host 校验 | 无 | 01–02 |
| [model/connection.rs](../src/model/connection.rs) | ProtocolConnection、协议枚举与校验分派 | 无 | 02 |
| [nfs.rs](../src/model/connection/nfs.rs)、[smb.rs](../src/model/connection/smb.rs) | 协议配置、身份与校验 | 无 | 02 |
| [model/error.rs](../src/model/error.rs) | ModelError | 无 | 02 |
| [config.rs](../src/config.rs) | DeviceData/DeviceConfigDocument 快照转换；AppConfig、LoggingSettings 校验；TOML 编解码；ConfigError、ConfigIoError | 无 | 03–04 |
| [store.rs](../src/store.rs) | StoreConnection：主配置读取与首次创建、设备路径解析、按需读写设备、原子替换；StoreError（操作名、路径与来源） | 读写文件；不打印、不记日志 | 04 |
| [remote.rs](../src/remote.rs) | RemoteBackend、RemoteSession 两个 trait；按地址优先级连接、按协议选择后端、整理目录；RemoteError、ConnectError | 无；真实的网络访问在第 07 课接入 | 05 |
| [main.rs](../src/main.rs) | 调用 app::run | 读取进程参数 | 04 |
| [app.rs](../src/app.rs) | 启动流程 startup → logging → show；打印主配置、按需加载选中的设备；AppError、唯一的失败报告和退出码 | stdout 结果、stderr 失败摘要、读取设备文件 | 04 |
| [app/startup.rs](../src/app/startup.rs) | 解析参数；准备主配置并检查日志路径 | 读取或初始化主配置 | 04 |
| [app/logging.rs](../src/app/logging.rs) | 两层独立过滤的 subscriber，记录命令失败 | 创建日志目录、追加写日志 | 04 |

## 示例、测试与夹具

| 文件 | 内容 |
|---|---|
| [course04](../examples/course04.rs) | 第 04 课可选示例 E4.1，观察日志过滤和嵌套 span |
| [course04_config](../tests/course04_config.rs) | 主配置校验与解码、错误来源；P4.1–P4.3：主配置编码往返、设备文件的编码与解码 |
| [course04_store](../tests/course04_store.rs) | 主配置读取与首次创建；P4.4–P4.10：设备路径、读取单个设备、原子替换、保存、登记与移除 |
| [course04_cli](../tests/course04_cli.rs) | 用子进程运行真实程序：参数、路径、日志过滤与追加、失败反馈；P4.12–P4.13：按需读取设备和重新启动时保留配置 |
| [course05](../examples/course05.rs) | 第 05 课可选示例 E5.1，观察 dyn 兼容性 |
| [course05_remote](../tests/course05_remote.rs) | 离线测试夹具和 P5.1–P5.5 的测试骨架。每题补完后启用对应测试 |
| [common/mod.rs](../tests/common/mod.rs) | `TestResult` 与共用的主配置、设备配置样例 |

读测试时先预测：输入会触发哪条规则？失败后哪些状态应该保持不变？再运行确认，不要只看测试名称和通过数量。
