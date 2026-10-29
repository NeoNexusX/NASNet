# NASNet Rust 学习方案

起点：有 Python、C/C++ 基础，已接触所有权、借用和引用。目标是能解释、修改并验证 NASNet 的完整 Rust 代码。工程顺序见[项目实现方案](Implementation-Plan.md)，知识进度见[12 课目录](Course-Syllabus.md)。

每课同时要求功能正确和完整理解。关键实现由你亲手完成；助手提供的入口、辅助函数、错误类型、日志、构建设置和演示也全部必读。功能验收与理解验收分别记录，没有你的解释证据就不自动宣布已懂。

## 当前学习顺序

1. 阅读[原 SQLite 作业验收](reviews/04-sqlite-acceptance.md)，保留已经完成的事务、顺序和校验成果。
2. 学习[第 04 课补课](lessons/04-errors-logging-and-toml-storage.md#part-a)：覆盖现有全部源码、Cargo 的 lib/bin、统一错误体系和 `tracing` 日志；完成 6 项解释与小改动练习。
3. 完成[第 04 课 TOML 分文件配置](lessons/04-errors-logging-and-toml-storage.md#part-b)的 TOML 入门练习与 13 项函数实现，主文件保存软件设置和设备引用，每台设备独立一个 TOML。
4. 功能与完整理解都验收后，进入第 05 课 C FFI；后续按当前 12 课目录推进。

旧 SQLite 不再进入当前运行路径；[历史讲义](archive/sqlite-storage.md)不占新课号。新增的补课不能靠助手读过源码代替你的理解验收。

## 每课怎样进行

每课按“代码地图 → 新知识和新库介绍 → 完整调用链阅读 → 亲写实现 → 预测并验证小改动 → 双项验收”推进。新库必须先说明它解决什么问题、与标准库及已有依赖的分工、Cargo 版本与 feature、最小用法、错误和资源行为，再接入项目。

对本轮新增的 `tracing`，先学习事件、级别、结构化字段与 span；对 `tracing-subscriber`，先学习事件收集、过滤、格式化和一次性初始化。`serde` 与 `serde_json` 虽已使用，本轮也回补职责、derive 与解析目标类型。新增 toml 库先讲表与数组表、Option 的省略表示、from_str/to_string_pretty、错误类型及写回不保留注释，再进入正式练习。版本以 Cargo.lock 为准，不能把依赖声明里的兼容范围当成实际锁定版本。

每轮可以填写更多关键逻辑。第 04 课保留 1 项新库入门与 13 个函数实现，覆盖主配置、设备引用、单设备编解码、路径解析、单文件替换及多文件发布顺序。助手提供的辅助函数必须有阅读归属；不能靠把大量流程藏进辅助代码减少练习的实质内容。

遇到困难先给提示；明确要求完整实现时再给完整答案。已经通过的实现不要求重写；缺少理解证据的部分补验阅读。演示的打印不是自动断言，要按讲义逐项核对预期。

## 学习阶段

| 阶段 | 对应课程 | Rust 知识与项目成果 |
|---|---|---|
| 已完成的功能 | 01～03 | 模型、所有权与借用、查询与排序、协议校验、封装、单设备 JSON；完整调用链理解待补验 |
| 当前 A 部分 | 04 | edition 2024、lib/bin、模块职责、Display/Error/From、source、应用边界、事件与 span |
| 当前 B 部分 | 04 | trait、Path/PathBuf、文件 I/O、错误分类、主配置/设备分文件、路径与单文件替换 |
| 原生协议 | 05～06、07 的接口部分 | C ABI、指针、字符串、unsafe、Drop、安全封装、libnfs/libsmb2 |
| 异步与会话 | 07～09 | 任务、线程、通道、Send/Sync、状态、超时、取消、重连和终端版 |
| 后台与桌面 | 09～11 | 进程边界、IPC、Vue/Tauri、命令接口与事件 |
| 系统与交付 | 11～12 | cfg、平台挂载、登录恢复和跨平台构建验证 |

首次出现的语法先学到能解释当前语句，后续课程再深化。返回引用时先说明它借自哪里、谁需要继续存活；复杂生命周期、高级泛型和宏开发按实际需求补充。

## 工程与验收约定

当前一个 package 包含 library crate 和 binary crate，使用 Rust 2024 edition。`lib.rs` 声明可复用模块，`main.rs` 进入 `app`，应用编排调用 `nasnet` 库。无需为了“新架构”提前拆 workspace 或多个服务。

统一错误集中在 `src/error.rs`，按校验、配置、存储、应用分层；模型和业务逻辑通过 `Result` 传播失败，不自行打印。应用边界负责用英文显示和记录错误，避免同一错误每层都输出。日志可以记录操作、阶段、数量与错误分类，不记录密码和完整配置。

每次检查同步清理已通过实现中的练习题号与过时教学提示，保留接口契约、约束和必要注释。未完成 TODO 保留；完整讲解放在讲义。长期课程表保留在 `src/main.rs` 并同步更新。

不自动新增课程测试文件。采用格式、Clippy、演示与明确预期验收；真实协议与挂载仍须在 NAS 和目标系统验证。每次记录命令、结果和限制；理解记录文件/函数、你的解释要点、预测和小改动结果。

[完整源码阅读指南](Code-Reading-Guide.md)必须覆盖未来前端、构建、IPC 和平台适配代码。第三方内部源码无需全部逐行研读，但本项目使用的 API 契约、feature、资源和错误行为必须读懂。

## 推荐阅读

结合当前函数复习 Rust Book 的[所有权](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)、[模块](https://doc.rust-lang.org/book/ch07-03-paths-for-referring-to-an-item-in-the-module-tree.html)、[错误处理](https://doc.rust-lang.org/book/ch09-00-error-handling.html)和[迭代器](https://doc.rust-lang.org/book/ch13-02-iterators.html)。先读第 04 课介绍，再按需查第三方库 API，不必先通读整本书或整库源码。
