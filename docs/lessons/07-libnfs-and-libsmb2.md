# 第 07 课：接入 libnfs 与 libsmb2

把第 06 课的做法用到真实的协议库上。先接通 NFS，再用同一套接口接 SMB。学完后，程序能列出真实 NAS 上的一个目录；地址回退、协议分派和目录整理仍然是第 05 课写的那几个函数。

**状态**：计划 · 仅大纲。学完第 06 课后扩写成完整讲义。库函数的返回值和错误码以锁定的库版本为准，扩写时在真机上记录。

[课程目录](../Course-Syllabus.md) · [上一课](06-unsafe-and-c-ffi.md) · [下一课](08-threads-and-session-worker.md) · [技术设计第十三节](../NAS-Client-Architecture.md)

## 新知识

| 知识 | 官方来源 | 在 NASNet 中 |
|---|---|---|
| 用 CMake 构建第三方 C 库：固定源码版本、`links` 键、链接指令、静态与动态链接 | [Cargo 构建脚本](https://doc.rust-lang.org/cargo/reference/build-scripts.html)、[cmake 文档](https://docs.rs/cmake) | `build.rs` 构建固定版本的 libnfs 和 libsmb2，不联网取源码 |
| bindgen：只生成用到的声明，生成结果随源码保存 | [bindgen 用户指南](https://rust-lang.github.io/rust-bindgen/) | `src/nfs/ffi.rs`、`src/smb/ffi.rs` |
| Cargo feature 与条件编译 `#[cfg(feature = "…")]` | [Cargo features](https://doc.rust-lang.org/cargo/reference/features.html) | `nfs`、`smb`、`fixture` 三个 feature；检查命令加 `--all-features` |
| libnfs 的同步接口：建立上下文、设置身份和版本、挂载导出、读目录、取错误文本 | [libnfs.h](https://github.com/sahlberg/libnfs/blob/master/include/nfsc/libnfs.h) | `NfsBackend`、`NfsSession` |
| libsmb2 的同步接口：建立上下文、设置账号、连接共享、读目录 | [libsmb2.h](https://github.com/sahlberg/libsmb2/blob/master/include/smb2/libsmb2.h) | `SmbBackend`、`SmbSession` |
| 需要外部环境的测试：`#[ignore]` 加环境变量 | [Book 11.2](https://doc.rust-lang.org/book/ch11-02-running-tests.html) | 真机测试默认跳过，设置 `NASNET_TEST_NFS`、`NASNET_TEST_SMB` 后运行 |

## 项目练习

代码放在 `src/nfs.rs`、`src/smb.rs`，各带一个生成的 `ffi.rs`。测试写在 `tests/course07_protocols.rs`。

| 题 | 写什么 | 用什么证明 |
|---|---|---|
| P7.1 | `build.rs` 在 `nfs` feature 打开时用 CMake 构建 libnfs；把第 06 课的 C 库移到 `fixture` feature | 开和不开 feature 都能构建；不开时不需要 CMake |
| P7.2 | 用 bindgen 生成 `nfs::ffi`，只保留用到的函数和类型 | 生成的布局测试通过；文件头记录库版本和生成命令 |
| P7.3 | `NfsBackend::connect`：把 `NfsConfig` 的导出路径、版本、端口、UID/GID 翻译成库调用 | 单元测试检查参数翻译；真机测试能建立会话 |
| P7.4 | `NfsSession::list_directory` 和错误映射 | 映射的表驱动测试；真机测试中路径不存在得到 `NotFound`，指向文件得到 `NotADirectory` |
| P7.5 | 命令行新增列目录：选设备、选连接、给路径，经 `Backends::connect` 和 `list_directory_sorted` 打印结果；`AppError` 增加连接失败和读取失败 | 端到端测试使用 `fixture` 后端；真机上手动验证一次 |
| P7.6 | SMB 重复 P7.1–P7.4：构建、绑定、`SmbBackend`、`SmbSession` | 同上；另外断言密码不出现在日志和错误文本中 |
| P7.7 | 给两种协议的连接设置库自身的超时 | 真机测试：连一个不响应的地址，在限定时间内返回 `TimedOut` 并尝试下一个地址 |

## 验收

- 不带 feature 和带 `--all-features` 的三条检查命令都通过。
- 在真实 NAS 上分别用 NFS 和 SMB 列出一个目录。再各做一次错误场景：断开首选地址、写错 SMB 密码、访问不存在的路径，记录得到的错误。
- 能解释：认证失败为什么不换地址；绑定文件为什么随源码保存而不是每次构建时生成；两个库的许可证对链接方式有什么要求。

## 设计说明

- 真机验收需要一台可访问的 NAS，或者在本机用容器运行 NFS 和 Samba 服务。扩写讲义前先确定用哪一种。
- libnfs 和 libsmb2 都采用 LGPL-2.1。静态还是动态链接在扩写时按许可证和打包要求确定。
- 本课只用同步接口。C 上下文不能被多个线程同时使用，这个限制留到第 08 课处理。
