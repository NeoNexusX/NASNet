# 第 06 课：unsafe 与 C FFI

libnfs 和 libsmb2 都是 C 库。接入它们之前，先用仓库里一个约 100 行的小 C 库练完整的 FFI 流程：声明 C 函数、传字符串、管理句柄、把错误码变成 `RemoteError`。这个 C 库在内存里放一棵固定的目录树，接口形状模仿 libnfs，所以本课不需要 NAS。学完后，第 05 课的 `connect_by_priority` 和 `list_directory_sorted` 能跑在一个真正用 C 写的后端上。

**状态**：计划 · 仅大纲。学完第 05 课后扩写成完整讲义，题目的契约以扩写后的讲义为准。

[课程目录](../Course-Syllabus.md) · [上一课](05-traits-and-remote-backends.md) · [下一课](07-libnfs-and-libsmb2.md)

## 新知识

| 知识 | 官方来源 | 在 NASNet 中 |
|---|---|---|
| unsafe 允许的五件事；`unsafe` 块、`unsafe fn`、`# Safety` 文档与 `// SAFETY:` 注释 | [Book 20.1](https://doc.rust-lang.org/book/ch20-01-unsafe-rust.html) | 每个 unsafe 块写明依赖的前提和由谁保证 |
| `unsafe extern "C"` 声明、`#[repr(C)]`、不透明类型 | [Nomicon FFI](https://doc.rust-lang.org/nomicon/ffi.html)、[Reference 外部块](https://doc.rust-lang.org/reference/items/external-blocks.html) | 私有模块 `ffi`：手写 7 个函数声明和 2 个不透明句柄 |
| C 字符串：`CString`、`CStr`、`c_char`、`c_int` | [std::ffi](https://doc.rust-lang.org/std/ffi/index.html) | Host 和路径传给 C；错误文本和条目名从 C 读回并立刻复制 |
| 句柄的所有权：`NonNull`、`Drop`、用生命周期表达“目录句柄不能比上下文活得久” | [Book 15.3](https://doc.rust-lang.org/book/ch15-03-drop.html)、[NonNull](https://doc.rust-lang.org/std/ptr/struct.NonNull.html) | `Context` 和 `Dir<'_>` 两个封装类型 |
| 构建脚本与 cc crate | [Cargo 构建脚本](https://doc.rust-lang.org/cargo/reference/build-scripts.html)、[cc 文档](https://docs.rs/cc) | 包根目录的 `build.rs` 编译 `native/dirlist/dirlist.c` |

讲义会引用的真实报错：把含 `\0` 的字符串交给 `CString::new` 的 `NulError`；对临时 `CString` 取 `as_ptr()` 的悬垂指针警告；在 edition 2024 里不写 `unsafe extern` 的编译错误。

## 项目练习

代码放在 `src/remote/fixture.rs`，`ffi` 是其中的私有内联模块。测试写在 `tests/course06_ffi.rs`。C 库额外提供 `dl_live_handles()`，返回还没释放的句柄数，测试用它检查资源是否释放。

| 题 | 写什么 | 用什么证明 |
|---|---|---|
| P6.1 | `build.rs` 编译并链接 C 库；在 `ffi` 模块里声明函数 | 调用 `dl_init`、`dl_destroy` 的冒烟测试能链接、能运行 |
| P6.2 | `Context::new` 与 `Drop`：空指针变成错误，离开作用域时释放 | 创建并丢弃 100 个 `Context` 后，存活句柄数回到 0 |
| P6.3 | `Context::connect(host)`：把 `&str` 转成 C 字符串，读取 C 库的错误文本 | 已知 Host 成功；未知 Host 返回 `NameResolution`；Host 含 `\0` 时不调用 C 函数 |
| P6.4 | `Dir`：打开、逐项读取、关闭目录，把条目复制成 `DirectoryEntry` | 条目内容正确；读到一半提前返回时目录句柄也被关闭 |
| P6.5 | 错误码映射：负的 errno 到 `RemoteError` | 表驱动测试覆盖 C 库会返回的每个错误码 |
| P6.6 | 为 `FixtureBackend`、`FixtureSession` 实现第 05 课的两个 trait | 用第 05 课的 `connect_by_priority`、`list_directory_sorted` 读出 C 库里的目录树 |

可选示例 E6.1：去掉 `Dir` 上的生命周期参数，让它在 `Context` 之后释放，观察为什么编译器不再拦截，以及运行时会发生什么。

## 验收

- `cargo test --locked --test course06_ffi` 全部通过，三条检查命令通过。
- 能指着每个 unsafe 块说出：它依赖什么前提，前提由哪一行代码保证。
- 能解释：C 返回的条目名为什么要立刻复制成 `String`；`Dir` 为什么借用 `Context`；`Drop` 里为什么不能返回错误。

## 设计说明

- 从本课起，构建需要 C 编译器。
- Miri 不能执行外部 C 函数，所以本课用存活句柄计数检查泄漏。AddressSanitizer 需要 nightly，作为可选项介绍。
- 这个 C 库只用于教学和离线测试。第 07 课学了 Cargo feature 后，把它放到 `fixture` feature 后面。
