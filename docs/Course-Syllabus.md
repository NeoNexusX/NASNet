# NASNet Rust 课程

通过一步步实现跨平台 NAS 客户端 NASNet 来学习 Rust。适合已有 Python、C/C++ 基础的学习者；目标是能解释、修改并验证项目中的每一段代码。

[源码地图](Code-Reading-Guide.md) · [技术设计](NAS-Client-Architecture.md)

## 每一课怎样学

每课一个讲义文件（`docs/lessons/`），都按同样的顺序推进：

1. **新知识**：先读官方文档中的示例（讲义附来源链接），再看同一知识在项目里怎样用。
2. **示例练习（E）**：在 `examples/courseNN.rs` 里用一个与项目无关的小例子练习这些知识点，用 `cargo run --locked --example courseNN` 运行。
3. **项目练习（P）**：把同样的知识用到 `src/`。每题先读相关的现有代码，在 `tests/` 里写行为测试并看到它失败，再实现到通过。
4. **验收**：运行讲义末尾的命令，通过 [README](../README.md) 中的全量检查，并能回答讲义里的问题。

编号：`E4.2` 是第 04 课第 2 道示例练习，`P4.5` 是第 04 课第 5 道项目练习。源码中的 `TODO P4.5` 与讲义里的题目一一对应。

约定：

- 关键逻辑由学习者亲手写。卡住时先要提示，明确要求时才给完整实现。
- 已完成的课保留参考实现：复习时可以删掉方法体重写，再用示例和测试验证。
- 功能以测试为证据，理解以“讲得清调用链、预测得出结果、改动后能验证”为证据。示例能运行、助手写过代码，都不算完成。

## 课程目录

| 课 | 主题 | 新知识 | 状态 |
|---|---|---|---|
| [01](lessons/01-model-and-ownership.md) | 设备模型、查询与排序 | struct/impl、所有权与移动、借用、Vec 与切片、Option、迭代器、稳定排序 | 已完成 |
| [02](lessons/02-validation-and-protocols.md) | 校验与 NFS/SMB 配置 | enum、match/if let/matches!、Result 与 ?、自定义错误、字符串检查 | 已完成 |
| [03](lessons/03-encapsulation-and-serialization.md) | 封装、生命周期与配置快照 | 模块可见性、iter_mut、remove 与所有权、生命周期、Clone、From/TryFrom、Serde 派生 | 已完成 |
| [04](lessons/04-errors-logging-and-toml-storage.md) | 测试、TOML 配置、文件存储与日志 | Serde 与 TOML、错误链、Path、文件 I/O 与原子替换、tracing、测试、模块与 lib/bin | **进行中** |
| 05 | 统一远程能力与可替换实现 | trait、泛型、动态分派；先用模拟后端定义最小的目录读取契约 | 计划 |
| 06 | C FFI 与资源安全封装 | C ABI、CString/CStr、unsafe 的前提、原生句柄与 Drop | 计划 |
| 07 | 真实 NFS/SMB 与异步调用 | libnfs/libsmb2 绑定与构建、真实目录读取、线程调度、Send/Sync | 计划 |
| 08 | 会话、取消与地址切换 | 状态机、超时、取消、重连、共享状态与事件 | 计划 |
| 09 | 终端应用、后台与 IPC | 进程边界、请求关联、后台启动与退出 | 计划 |
| 10 | Tauri/Vue 桌面与配置管理 | 前后端数据流、表单、状态与事件订阅 | 计划 |
| 11 | 远程浏览与系统挂载 | 平台 cfg/API、权限能力、挂载状态与取消 | 计划 |
| 12 | 自动恢复、构建与交付 | 登录自启、恢复顺序、原生依赖与各平台打包验证 | 计划 |

第 05–07 课的顺序是“先定义可测试的接口 → 再安全封装原生资源 → 最后接入真实协议并安排异步调用”：先得到不依赖 NAS 的接口和测试，再处理指针、原生构建和调度。第 07 课先接通一种协议，再复用同一边界接入另一种。第 05–12 课开始时再补具体的讲义、示例和测试。

**下一步**：[第 04 课](lessons/04-errors-logging-and-toml-storage.md)，从示例练习 E4.1 开始。
