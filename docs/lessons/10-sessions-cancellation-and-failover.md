# 第 10 课：会话状态机、取消与地址切换

前几课的连接是一次性的：连上、读目录、结束。真正的后台要让每个协议连接长期存在：断开后按优先级重新连接，全部地址不可达时定时重试，用户可以随时取消，界面能看到当前状态。本课实现[技术设计](../NAS-Client-Architecture.md)第八节和第十一节的规则。

**状态**：计划 · 仅大纲。学完第 09 课后扩写成完整讲义。

[课程目录](../Course-Syllabus.md) · [上一课](09-async-and-tokio.md)

## 新知识

| 知识 | 官方来源 | 在 NASNet 中 |
|---|---|---|
| 用 enum 表达状态机，把状态转移写成不依赖运行时的纯函数 | [Book 18.3](https://doc.rust-lang.org/book/ch18-03-oo-design-patterns.html) | `SessionState` 和 `transition` |
| `tokio::select!` 与取消安全 | [Tokio 教程：Select](https://tokio.rs/tokio/tutorial/select) | 会话任务同时等待命令、取消信号和重试定时器 |
| `CancellationToken` 与子令牌；丢弃 future 就是取消 | [tokio-util 文档](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html)、[Tokio：Graceful shutdown](https://tokio.rs/tokio/topics/shutdown) | 取消一次连接尝试；关闭全部会话 |
| `watch` 通道：只关心最新值的状态发布 | [tokio::sync::watch](https://docs.rs/tokio/latest/tokio/sync/watch/index.html) | 向界面发布连接状态 |
| 共享状态的两种做法：加锁，或者由一个任务独占并通过消息访问 | [Tokio 教程：Shared state](https://tokio.rs/tokio/tutorial/shared-state) | `SessionManager` 由一个任务独占会话表 |

## 项目练习

代码放在 `src/session.rs`，状态机部分如果超过一屏再拆成 `src/session/state.rs`。测试写在 `tests/course10_sessions.rs`，全部使用暂停的时钟。

| 题 | 写什么 | 用什么证明 |
|---|---|---|
| P10.1 | `SessionState`（未连接、连接中、已连接、等待重试、已停止）和 `transition(状态, 事件)` | 表驱动测试覆盖每一对状态和事件，包括应当被忽略的事件 |
| P10.2 | 会话任务：用 `select!` 驱动状态机，执行连接、读目录和定时 | 从启动到已连接的完整流程；断言状态序列 |
| P10.3 | 取消：连接中取消后回到未连接，不再尝试后面的地址 | 第一个地址卡住时取消，后端的尝试记录只有一个 Host |
| P10.4 | 状态发布：连接 ID、地址 ID、协议、状态和错误 | 订阅者依次看到连接中、已连接；晚订阅的只看到最新状态 |
| P10.5 | 读取时发现连接失效：重新按优先级连接，再重发这次读取；认证和权限错误不重连 | 首选地址中途失效后，读取由第二个地址完成，调用方只看到一次成功 |
| P10.6 | 全部地址不可达时定时重试，间隔逐步加长并有上限 | 推进时钟，断言每次重试发生的时刻 |
| P10.7 | `SessionManager`：按连接 ID 管理会话，同一设备的不同连接各自选地址 | 两条连接分别连上不同地址；关闭管理器后所有会话任务结束 |

## 验收

- `cargo test --locked --test course10_sessions` 全部通过，没有真实等待。
- 能画出状态图，并指出每条边对应 `transition` 的哪个分支。
- 能解释：`select!` 的一个分支完成时，其他分支里的 future 会怎样；为什么读目录可以在重连后重发，而权限修改不可以；修改地址优先级为什么不打断已连接的会话。

## 设计说明

- 读目录、取属性、按范围读取可以安全重发。结果未知的修改操作不自动重发，先查询实际状态，这条规则在以后加入修改操作时落实。
- 重试间隔和上限是内部参数，不进配置文件。
