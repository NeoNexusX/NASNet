# 第 08 课：线程与会话工作线程

第 07 课的会话是阻塞的：读一个慢目录时，整个程序都在等。C 库的上下文又不能被多个线程同时使用。本课让每个会话由一个工作线程独占，其他线程通过 channel 发请求、等回复。学完后，程序能同时浏览两个连接，一个卡住不影响另一个。

**状态**：计划 · 仅大纲。学完第 07 课后扩写成完整讲义。

[课程目录](../Course-Syllabus.md) · [上一课](07-libnfs-and-libsmb2.md) · [下一课](09-async-and-tokio.md)

## 新知识

| 知识 | 官方来源 | 在 NASNet 中 |
|---|---|---|
| 闭包怎样捕获变量：`Fn`、`FnMut`、`FnOnce` 与 `move` | [Book 13.1](https://doc.rust-lang.org/book/ch13-01-closures.html) | 把“怎样建立会话”作为闭包交给工作线程 |
| `thread::spawn`、`JoinHandle`，以及签名里的 `Send + 'static` | [Book 16.1](https://doc.rust-lang.org/book/ch16-01-threads.html)、[thread::spawn](https://doc.rust-lang.org/std/thread/fn.spawn.html) | 每个会话一个工作线程 |
| 消息传递：`mpsc`，所有发送端丢弃后接收循环结束 | [Book 16.2](https://doc.rust-lang.org/book/ch16-02-message-passing.html) | 请求枚举；每个请求带一个回复通道 |
| 共享状态：`Arc<Mutex<T>>`，与第 05 课的 `RefCell` 对照 | [Book 16.3](https://doc.rust-lang.org/book/ch16-03-shared-state.html) | 测试替身的尝试记录从 `RefCell` 换成 `Mutex` |
| `Send` 与 `Sync`；裸指针为什么让封装类型自动不是 `Send` | [Book 16.4](https://doc.rust-lang.org/book/ch16-04-extensible-concurrency-sync-and-send.html)、[Nomicon](https://doc.rust-lang.org/nomicon/send-and-sync.html) | 在工作线程内部创建 C 上下文，不写 `unsafe impl Send` |

讲义会引用的真实报错：把第 07 课的会话直接移进 `thread::spawn` 时的 `cannot be sent between threads safely`，以及闭包借用局部变量时的 `closure may outlive the current function`。

## 项目练习

代码放在 `src/session.rs`，测试写在 `tests/course08_worker.rs`。测试只用内存后端和 `fixture` 后端，用 `Barrier` 和 channel 控制先后顺序，不用 `sleep`。

| 题 | 写什么 | 用什么证明 |
|---|---|---|
| P8.1 | 让测试里的 `MemoryBackend` 满足 `Send`：`RefCell` 换成 `Mutex` | 第 05 课的测试照常通过；后端能被移进另一个线程 |
| P8.2 | `SessionWorker::spawn`：接收一个建立会话的闭包，在新线程里调用它，把连接结果传回调用方 | 连接成功和失败两种结果都能传回；会话在工作线程里创建 |
| P8.3 | 请求枚举和工作循环：收到读目录请求，调用会话，把结果发回 | 连续发三个请求，回复和请求一一对应 |
| P8.4 | `SessionHandle::list_directory`：发请求并等回复；工作线程已退出时返回明确的错误 | 句柄可以克隆，也可以在别的线程使用；工作线程退出后调用得到错误而不是 panic |
| P8.5 | 关闭：所有句柄丢弃后工作循环结束，会话在创建它的线程里释放 | 记录释放时的线程 ID，与创建时相同；`join` 能返回 |
| P8.6 | 两个会话并发：一个的读目录被测试卡住，另一个照常返回 | 被卡住的会话放行前，另一个会话已经拿到结果 |

可选示例 E8.1：分别把 `Rc<String>` 和裸指针移进 `thread::spawn`，对照两条编译错误，再换成 `Arc` 看哪一条消失。

## 验收

- `cargo test --locked --test course08_worker` 全部通过，并且连续运行 20 次结果一致。
- 能解释：`thread::spawn` 的闭包为什么要求 `'static`；为什么选“在线程里创建上下文”而不是 `unsafe impl Send`；工作循环靠什么知道该结束。

## 设计说明

- `RemoteBackend` 和 `RemoteSession` 保持同步接口。会话的线程归属由本课的工作线程解决，异步由第 09 课在句柄这一层解决，两个 trait 都不用改。
- 后端要被多个工作线程共用，扩写时确定给 `RemoteBackend` 加 `Send + Sync` 约束的位置。
