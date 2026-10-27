# NASNet

统一管理 NAS 设备、多个地址、独立 NFS/SMB 连接和系统挂载的跨平台客户端，按 Rust 学习进度逐步实现。

**第 1、2 课和第 3 课 A 已完成并检查通过。** 地址与协议校验、重复 ID 检查、连接保存，以及更新、删除与借用查找均已完成。下一步是第 3 课 B：配置序列化与恢复时校验；程序目前只操作内存。

## 学习路线

- [12 课课程大纲](docs/Course-Syllabus.md)：每课的知识、动手范围和完成标准；完整安排也在 [main.rs](src/main.rs) 顶部。
- [Rust 学习方案](docs/Rust-Learning-Plan.md)：按阶段填写关键实现，遇到概念再补充学习。
- [项目实现方案](docs/Implementation-Plan.md)：从内存模型到协议访问、桌面界面和系统挂载的交付顺序。
- [NASNet 技术方案](docs/NAS-Client-Architecture.md)：完整功能范围、地址规则、协议和跨平台设计。

## 工程风格

项目采用 Rust 2018 edition 和 non-mod-rs 布局。模块入口与子模块目录并列，不使用 `mod.rs`：

```text
src/
├── main.rs
├── demo.rs
├── model.rs                  模块声明和统一导出
└── model/
    ├── device.rs             设备及其地址、连接列表的管理
    ├── address.rs            地址模型和 Host 校验
    ├── connection.rs         连接记录、配置枚举和校验分派
    ├── connection/
    │   ├── nfs.rs            NFS 配置、身份、版本及校验
    │   └── smb.rs            SMB 配置、身份、方言及校验
    └── error.rs              统一错误类型及其展示
```

`main.rs` 用 `mod model;` 声明模块，`model.rs` 只声明子模块并通过 `pub use` 统一导出。`connection.rs` 声明 NFS、SMB 子模块并重导出配置类型。调用方仍从 `crate::model` 引入类型；内部模块直接引用类型定义所在的模块。后续新增模块也沿用同名 `.rs` 文件配合同名子目录的布局。

单条地址、连接和协议配置各自负责本地校验；`Device` 负责所属列表的重复 ID 检查、查找和增删改。`error.rs` 只定义失败原因和错误展示。NFS/SMB 配置模块不执行网络连接，真实协议调用在后续后端模块中接入。

`edition = "2018"` 指定语言版本规则；最低编译器版本仍为 Rust 1.85。

## 已完成：第 3 课 A

对照[模型封装与生命周期](docs/lessons/05-encapsulation-and-lifetimes.md)，可以复习 [src/model/device.rs](src/model/device.rs) 中已完成的五个方法：

| 状态 | 方法 | 已实现的行为 |
|---|---|---|
| 已完成 | `try_update_address` | 先校验，再在原位置替换 |
| 已完成 | `remove_address` | 查位置、保持顺序、交还所有权 |
| 已完成 | `find_connection` | 借用查找、返回引用的生命周期 |
| 已完成 | `try_update_connection` | 复用协议校验、失败保留原配置 |
| 已完成 | `remove_connection` | 删除连接并交还所有权 |

设备字段和两个列表已私有化；通过 `id()`、`name()`、`addresses()`、`connections()` 读取。旧的无校验 `add_address` 已移除，前四个演示改用已完成的 `try_add_address`，仍可运行。公开的 `Address`、`ProtocolConnection` 是待提交候选值；已保存记录只能通过设备提供的方法修改，不提供可变引用。

本课后半部分再加入配置数据转换、Serde 序列化和恢复时校验；本轮不引入依赖、磁盘存储或新的课程测试文件。

## 运行与检查

需要 Rust 1.85 或更新版本。打开 PowerShell，先进入项目目录，再运行最近完成的小节：

```powershell
Set-Location 'C:\Users\13427\Desktop\NASNet'
cargo run --locked -- lesson05
```

`cargo run` 编译并运行程序；`--locked` 要求沿用现有 `Cargo.lock`；`--` 后的 `lesson05` 是交给程序的参数，由 `main.rs` 选择演示。

`lesson05` 会依次展示地址更新、地址删除、借用查找连接、连接更新和连接删除。空 Host、非法 NFS 路径、空 SMB 用户名和重复删除等失败是刻意演示的校验结果；观察失败后的保存数据是否保持原样。最终应留下 2 个地址（`address-2`、`address-3`）和 2 条连接（`smb-data`、`smb-backup`）。演示结束后自动退出，每次运行都重新创建内存数据。

修改代码后，可以在项目根目录执行检查和复习演示：

```sh
cargo check --locked
cargo fmt -- --check
cargo clippy --all-targets --locked -- -D warnings

# 最近完成：第 3 课 A，沿用演示小节编号 lesson05
cargo run --locked -- lesson05

# 复习已完成的四个小节
cargo run --locked -- lesson01
cargo run --locked -- lesson02
cargo run --locked -- lesson03
cargo run --locked -- lesson04
```

五个方法已完成，编译、格式和 Clippy 检查均通过，五个演示均可完整运行；`lesson05` 输出已对照讲义检查。项目未内置自动测试，`cargo test --locked` 的 `0 tests` 不代表逻辑已经验收。

`lesson01`～`lesson05` 是小节编号：01/02 对应第 1 课 A/B，03/04 对应第 2 课 A/B，05 对应第 3 课 A。默认 `cargo run` 仍运行 `lesson01`。

已有讲义：[模型与借用](docs/lessons/01-device-and-borrowing.md)、[查找与排序](docs/lessons/02-address-lookup-and-priority.md)、[基础校验](docs/lessons/03-validation-and-result.md)、[独立协议配置](docs/lessons/04-protocol-configs.md)。

NFS 使用 AUTH_SYS 身份，SMB 使用账号和密码凭据引用；不读取真实密码、不访问 NAS。版本字段只表达请求意向，后端和服务器能力在协议接入阶段验证。C FFI 与资源封装安排在第 5 课。
