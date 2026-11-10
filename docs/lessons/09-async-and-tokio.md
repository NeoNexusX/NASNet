# 第 09 课：async 与 Tokio

后台要同时管理多个连接、给连接加超时、定时重试，还要响应界面的请求。为每件事开一个线程去等待并不合适。本课把第 08 课的会话句柄改成 async：等待目录结果时不占线程，连接可以加超时，而且测试不用真的等待。

**状态**：计划 · 仅大纲。学完第 08 课后扩写成完整讲义。

[课程目录](../Course-Syllabus.md) · [上一课](08-threads-and-session-worker.md) · [下一课](10-sessions-cancellation-and-failover.md)

## 新知识

| 知识 | 官方来源 | 在 NASNet 中 |
|---|---|---|
| `Future`、`async`/`await`，future 不被等待就不执行 | [Book 第 17 章](https://doc.rust-lang.org/book/ch17-00-async-await.html) | `SessionHandle::list_directory` 变成 `async fn` |
| 运行时与任务：`#[tokio::main]`、`tokio::spawn`、`join!` | [Tokio 教程：Spawning](https://tokio.rs/tokio/tutorial/spawning) | 程序入口建立运行时；同时读两个连接的目录 |
| 不能在 async 里阻塞；同步代码与异步代码怎样衔接 | [Tokio：Bridging with sync code](https://tokio.rs/tokio/topics/bridging) | 阻塞的 C 调用留在第 08 课的工作线程里 |
| 异步通道：`mpsc` 发请求，`oneshot` 收回复 | [Tokio 教程：Channels](https://tokio.rs/tokio/tutorial/channels) | 工作线程一侧用 `blocking_recv`，调用方一侧 `await` |
| 时间：`timeout`、`sleep`，测试里暂停并推进时钟 | [tokio::time](https://docs.rs/tokio/latest/tokio/time/index.html)、[Tokio：Unit testing](https://tokio.rs/tokio/topics/testing) | 单个地址的连接超时；`#[tokio::test(start_paused = true)]` |
| span 跨越 `.await`：用 `Instrument`，不用 `enter()` | [tracing：Instrument](https://docs.rs/tracing/latest/tracing/trait.Instrument.html) | 连接过程的日志 span |

还要讲清一个取舍：trait 里的 `async fn` 不能用于 `dyn`，所以第 05 课的两个 trait 保持同步，异步只出现在句柄这一层。

## 项目练习

代码继续放在 `src/session.rs`，测试写在 `tests/course09_async.rs`。

| 题 | 写什么 | 用什么证明 |
|---|---|---|
| P9.1 | 把工作线程的通道换成 Tokio 的 `mpsc` 和 `oneshot`，`list_directory` 改成 `async fn` | 第 08 课的场景改写成 async 测试后全部通过 |
| P9.2 | 带超时的单地址连接：超时返回 `RemoteError::TimedOut` | 暂停时钟的测试：后端永不返回，推进时钟后得到超时 |
| P9.3 | async 版的按优先级连接，复用 `allows_next_address` | 第一个地址超时、第二个成功；测试瞬间完成，并断言尝试顺序 |
| P9.4 | 超时后的清理：放弃仍卡在 C 调用里的工作线程，它返回后自行退出并释放会话 | 测试放行被卡住的后端后，会话被释放，没有线程残留 |
| P9.5 | 程序入口建立运行时，列目录命令走 async 路径，连接过程带 span | 第 07 课的端到端测试照常通过；日志里连接事件带着所属的 span |
| P9.6 | 同时读取两个连接的目录 | 两个各需 1 秒（虚拟时间）的读取，总共用 1 秒而不是 2 秒 |

可选示例 E9.1：在单线程运行时里启动两个任务，一个调用 `std::thread::sleep`，一个调用 `tokio::time::sleep`，观察谁挡住了谁。

## 验收

- `cargo test --locked --test course09_async` 全部通过，整组测试在 1 秒内结束。
- 能解释：调用 `async fn` 而不 `.await` 会发生什么；为什么阻塞调用不能直接写在任务里；超时后那个还没返回的 C 调用去了哪里。

## 设计说明

- Tokio 只打开用到的 feature：`rt-multi-thread`、`macros`、`sync`、`time`；`test-util` 只放在开发依赖里。
- 超时是内部参数，不进配置文件，也不出现在界面上，和[技术设计第八节](../NAS-Client-Architecture.md)一致。
