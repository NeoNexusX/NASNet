# NASNet Rust 课程

通过一步步实现跨平台 NAS 客户端 NASNet 来学习 Rust。适合已有 Python、C/C++ 基础的学习者；目标是能解释、修改并验证项目中的每一段代码。

[源码地图](Code-Reading-Guide.md) · [技术设计](NAS-Client-Architecture.md)

## 每一课怎样学

讲义位于 `docs/lessons/`。第 01–04 课已学完，合并成一份复习大纲。第 05 课是完整讲义。第 06–10 课先有设计大纲，学完上一课时再扩写成完整讲义。完整讲义按以下方式学习。

1. **新知识**。先用《The Rust Programming Language》、标准库文档或依赖的官方文档介绍本课需要的新 API，说明输入、返回值和常见错误，再指出它在 NASNet 中的用法。
2. **项目练习（P）**。主要练习放在 `src/` 和 `tests/`。讲义给出要验证的行为和预期结果，必要时提供测试骨架或共用数据，由学习者补全测试用例和断言。
3. **可选示例（E）**。需要单独观察 API、日志输出或编译器行为时，再在 `examples/` 放一段短练习。保留已有的完整示例供复习。
4. **验收**。运行讲义末尾和 [README](../README.md) 中的检查命令，并能回答讲义里的问题。

`E5.1` 是第 05 课的可选示例，`P5.3` 是第 05 课第 3 道项目练习。源码中的 `todo!()` 标注对应的项目题号。

约定：

- 源码注释保留设计原因、调用约束、易错的边界以及失败时的数据保证。代码能直接表达的步骤交给命名和代码结构，API 用法、实现步骤与练习说明集中在讲义。
- 关键逻辑由学习者亲手写。卡住时先要提示，明确要求时才给完整实现。
- 功能以测试为证据，理解以“讲得清调用链、预测得出结果、改动后能验证”为证据。示例能运行、助手写过代码，都不算完成。

## 课程目录

| 课 | 主题 | 新知识 | 状态 |
|---|---|---|---|
| [01–04](lessons/01-04-foundations.md) | 已学基础：设备模型、校验、TOML 配置、文件存储、命令行与日志 | 所有权与借用、enum 与模式匹配、`Result` 与错误链、模块与可见性、生命周期、Serde、文件 I/O、tracing、测试 | 已完成 · 合并为大纲 |
| [05](lessons/05-traits-and-remote-backends.md) | 统一远程能力与可替换实现 | trait 与默认实现、泛型与单态化、trait 对象与动态分派、`?Sized` 与 dyn 兼容、`RefCell` 模拟对象、`assert_matches!`、过滤与稳定排序 | **进行中** |
| [06](lessons/06-unsafe-and-c-ffi.md) | unsafe 与 C FFI | unsafe 的前提、`unsafe extern "C"`、`CString`/`CStr`、`NonNull`、用 `Drop` 管理句柄、构建脚本与 cc | 计划 · 仅大纲 |
| [07](lessons/07-libnfs-and-libsmb2.md) | 接入 libnfs 与 libsmb2 | CMake 构建原生库、bindgen、Cargo feature、两个库的同步接口、错误码映射、需要真机的测试 | 计划 · 仅大纲 |
| [08](lessons/08-threads-and-session-worker.md) | 线程与会话工作线程 | 闭包与 `move`、`thread::spawn`、`mpsc`、`Arc`/`Mutex`、`Send`/`Sync` | 计划 · 仅大纲 |
| [09](lessons/09-async-and-tokio.md) | async 与 Tokio | `Future` 与 `async`/`await`、运行时与任务、同步与异步的衔接、`oneshot`、超时与暂停时钟 | 计划 · 仅大纲 |
| [10](lessons/10-sessions-cancellation-and-failover.md) | 会话状态机、取消与地址切换 | enum 状态机、`select!` 与取消安全、`CancellationToken`、`watch`、重连与重试 | 计划 · 仅大纲 |

第 05–10 课分两段，每段结束时有一个可以演示的结果：

| 课 | 做什么 | 学完能演示 |
|---|---|---|
| 05–07 | 先定义可测试的接口，再用一个小 C 库练 FFI，最后接入真实协议 | 命令行列出真实 NAS 上的目录，首选地址不通时自动换下一个 |
| 08–10 | 先用线程隔离 C 上下文，再包装成 async，最后做成长期存在的会话 | 断线后自动重连、超时和取消都有测试，测试不需要真实等待 |

三个贯穿这几课的设计决定：

- 第 05 课的 `RemoteBackend`、`RemoteSession` 一直保持同步接口。第 08 课让一个工作线程独占 C 上下文，第 09 课在句柄这一层包装成 async。
- 第 06 课用仓库内的小 C 库，第 09–10 课用 Tokio 的暂停时钟，所以除第 07 课的真机验收外，都不需要 NAS。
- 第 07 课先接通 NFS，再用同一套接口接入 SMB。

第 10 课之后还有后台进程与本地 IPC、Tauri 与 Vue 桌面界面、远程浏览、系统挂载、自动恢复与交付，学到第 10 课时再设计。

**下一步**：[第 05 课](lessons/05-traits-and-remote-backends.md)，从项目练习 P5.1 开始。先补模拟后端的测试，再实现地址回退、协议分派和目录整理。
