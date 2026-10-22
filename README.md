# NASNet

统一管理 NAS 设备、多个地址、NFS/SMB 连接和系统挂载的跨平台客户端，按 Rust 学习进度逐步实现。

设备与地址管理、查找、稳定排序，以及基础地址校验均已完成。当前继续配置阶段：加入独立的 NFS/SMB 数据结构、设备连接列表和演示，三个协议校验函数留给你完成。课程测试代码已删除，使用演示观察结果；存储、网络访问和挂载尚未接入。

## 学习与实现路线

- [12 课课程大纲](docs/Course-Syllabus.md)：每课的 Rust 知识、动手范围和完成标准；完整安排也在 [main.rs](src/main.rs) 顶部。目前第 1 课完成，第 2 课进行中。
- [Rust 学习方案](docs/Rust-Learning-Plan.md)：从所有权和借用出发，按需学习结构体、集合、错误处理、FFI 与异步。
- [项目实现方案](docs/Implementation-Plan.md)：从内存模型到真实协议访问、桌面界面和系统挂载的交付顺序。
- [NASNet 技术方案](docs/NAS-Client-Architecture.md)：完整功能范围、地址规则、协议与权限模型、跨平台构建。

## 当前课程：第 2 课，错误处理与独立 NFS/SMB 配置

当前使用一个包、一个二进制 crate，业务代码按 `model` 和 `demo` 两个模块组织：

```text
NASNet/
├── Cargo.toml
├── Cargo.lock
└── src/
    ├── main.rs          12 课完整安排与程序入口
    ├── demo.rs          第 1、2 课共四个小节的演示
    └── model/
        ├── mod.rs          设备、地址与连接列表
        ├── validation.rs   错误类型与已完成的 Host 校验
        └── connection.rs   独立协议配置，TODO 10、11、12
```

第 2 课 A 小节的基础校验已经完成。现在阅读[B 小节：独立协议配置](docs/lessons/04-protocol-configs.md)，按 TODO 10 → 11 → 12 的顺序填写：

- [协议配置](src/model/connection.rs)：`NfsConfig::validate`、`SmbConfig::validate`、`ProtocolConnection::validate`。
- [设备模型](src/model/mod.rs)：提供校验后添加连接的入口；连接列表私有，通过切片只读访问。
- [演示模块](src/demo.rs)：展示 NFS/SMB 的不同配置、错误原因和连接保存结果。

只需 Rust 1.85 或更新版本。在项目根目录执行：

```sh
cargo check --locked
cargo fmt -- --check
cargo clippy --all-targets --locked -- -D warnings

# 复习已完成的三个小节（演示编号保留）：
cargo run -- lesson01
cargo run -- lesson02
cargo run -- lesson03

# 当前：第 2 课 B 小节，沿用演示编号 lesson04：
cargo run -- lesson04
```

当前三个协议校验函数留有 `todo!()`：编译可以通过，运行协议配置演示会在尚未填写的地方停止。当前没有自动测试；运行演示后请对照讲义的预期结果检查。`cargo check` 成功不代表校验逻辑已经实现。

`lesson01`～`lesson04` 是保留的演示小节编号，分别对应新第 1 课 A/B、第 2 课 A/B。默认 `cargo run` 运行第 1 课 A。程序目前只操作内存，不访问 NAS、不读取真实密码。NFS 使用 AUTH_SYS 身份，SMB 使用账号及密码凭据引用，两个协议不共用认证字段。协议版本字段仅表达配置意向，后端与服务器能力会在接入时验证。

已有小节：[第 1 课 A：模型与借用](docs/lessons/01-device-and-borrowing.md)、[第 1 课 B：查找与排序](docs/lessons/02-address-lookup-and-priority.md)、[第 2 课 A：基础校验](docs/lessons/03-validation-and-result.md)。

Rust 调用 C 和 C 资源安全封装安排在新第 5 课，届时加入相应练习。
