# NASNet Rust 课程目录

共 12 课。第 01～03 课的模型功能已有回归测试；第 04 课进行中。功能测试与学习者理解分别验收，测试通过不自动代表已理解。

## 课程与知识范围

| 课号 | 内容 | Rust 知识/项目成果 |
|---|---|---|
| 01 | [设备模型、所有权、查询与排序](lessons/01-model-and-ownership.md) | 函数、struct/impl、String/&str、Vec/切片、移动与借用、Option、迭代器与稳定排序 |
| 02 | [错误处理与独立协议配置](lessons/02-validation-and-protocols.md) | enum、match、Result/?、候选校验、NFS/SMB 配置与失败保留 |
| 03 | [封装、生命周期与快照](lessons/03-encapsulation-and-serialization.md) | 私有字段、更新/删除、返回引用、Clone、From/TryFrom、Serde 数据结构 |
| 04 | [启动、日志、TOML 与 Rust 测试](lessons/04-errors-logging-and-toml-storage.md) | lib/bin、版本与 feature、Display/Error/From、路径与文件 I/O、分层日志、#[test] 与测试隔离 |
| 05 | C FFI、指针与资源安全封装 | C ABI、CString/CStr、unsafe 前提、原生句柄与 Drop |
| 06 | 真实 NFS 与 SMB 接入 | 固定 libnfs/libsmb2、绑定与构建、连接和真实目录读取 |
| 07 | 统一远程能力与异步调用 | trait/泛型、动态分派、异步接口、线程调度与 Send/Sync |
| 08 | 会话管理、取消与地址切换 | 状态机、超时、取消、重连、共享状态与事件 |
| 09 | 终端应用、独立后台与 IPC | 进程边界、请求关联、后台启动与退出 |
| 10 | Tauri/Vue 桌面与配置管理 | 前后端数据流、表单、状态与事件订阅 |
| 11 | 远程浏览与系统挂载 | 平台 cfg/API、权限能力、挂载状态与取消 |
| 12 | 自动恢复、构建与交付 | 登录自启、恢复顺序、原生依赖和目标平台打包验证 |

第 05～12 课按实现需要引入新语法和库，开始对应课时再补具体源码、练习与测试；当前不把这些能力记为已实现。

## 当前学习入口

1. 阅读[学习方法](Rust-Learning-Plan.md)和[源码地图](Code-Reading-Guide.md)。
2. 先看[测试入门](lessons/04-errors-logging-and-toml-storage.md#test-basics)，再做 **[练习 4.1](lessons/04-errors-logging-and-toml-storage.md#exercise-4-1)**。
3. 第 04 课的 `4.2/4.4/4.8` 是阅读与修改题，其余函数题按[各题说明](lessons/04-errors-logging-and-toml-storage.md#part-b)实现；不要把已有测试通过视为未完成题目通过。

样例用 `cargo run --locked --example course04`，自动验收用 `cargo test --locked`。主程序只处理项目参数 `--config`、`--device`，不分派课程。
