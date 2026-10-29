# NASNet

跨平台 NAS 客户端，按 Rust 学习进度逐步实现。第 01～03 课功能已通过，完整理解待补验；第 04 课合并完整源码、统一错误、日志与 TOML 分文件配置。

## 课程目录与知识清单

[12 课大纲与 Rust 已学清单](docs/Course-Syllabus.md) · [完整源码阅读路线](docs/Code-Reading-Guide.md)

| 课号 | 讲义 | 状态 |
|---|---|---|
| 01 | [设备模型、所有权、查询与排序](docs/lessons/01-model-and-ownership.md) | 功能 ✅，理解待补验 |
| 02 | [错误处理与独立协议配置](docs/lessons/02-validation-and-protocols.md) | 功能 ✅，理解待补验 |
| 03 | [封装、生命周期与 JSON 序列化基础](docs/lessons/03-encapsulation-and-serialization.md) | 功能 ✅，理解待补验 |
| 04 | [错误、日志与 TOML 分文件配置](docs/lessons/04-errors-logging-and-toml-storage.md) | 6 项阅读练习、1 项新库练习、13 个函数 TODO |
| 05～12 | [C FFI 到跨平台交付](docs/Course-Syllabus.md) | 计划中 |

新增 toml 库的用途、Serde 分工、最小用法、错误和限制先在第 04 课说明，再进入实现。所有助手提供的入口、类型、错误、日志和示例也必须读懂，不能只填写 TODO。

## 配置方案

```text
配置目录/
├── config.toml             软件设置与设备文件引用
└── devices/
    ├── home.toml           一台设备的地址、协议与身份
    └── backup.toml         另一台设备
```

[主文件示例](examples/configuration/config.toml)保存软件 language/log_level 和有序的 id/path 清单。相对设备路径基于主文件目录；选择设备时只加载它的文件，单设备保存不改主文件和其他设备。新增先写设备后登记引用；移除只解除引用、保留文件。多个文件没有整体事务，失败可能留下未引用文件。

SMB 密码按既有选择在设备 TOML 中明文原样保存，日志不打印正文或密码。TOML 正常写回不承诺保留人工注释与排版。软件 log_level 的启动接线仍是补课练习，当前教学入口使用 RUST_LOG/info。

第 03 课已有 JSON 编解码保留作 Serde 复习，不作为新的正式文件存储方案。[SQLite 成果](docs/reviews/04-sqlite-acceptance.md)与[旧单 JSON 设计](docs/archive/single-json-storage.md)仅归档。

## 代码结构

Rust 2024 edition，最低 Rust 1.85；单 package 含 lib/bin，模块使用同名文件与目录并列布局。

| 部分 | 责任 |
|---|---|
| lib.rs / model | 业务数据和规则；Host 检查在 Address 的私有方法内 |
| config.rs | 既有 JSON 练习、AppConfig/DeviceReference、单设备 TOML 转换 |
| store.rs | StoreConnection、主配置与单设备独立读写、单文件替换 |
| error.rs | 类型化错误、英文 Display、来源链与转换 |
| main.rs / app.rs | 入口、参数、流程与退出 |
| app/logging.rs | tracing-subscriber 初始化、过滤与安全错误报告 |
| app/demo.rs | 已有演示与虚构样例，全部必读 |

StoreConnection 是文件存储入口，ProtocolConnection 是 NAS 协议配置，彼此不混用。库层只返回错误，应用层记录与显示。

## 运行与检查

```sh
cargo check --locked
cargo fmt -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo run --locked -- course04 --intro
RUST_LOG=debug cargo run --locked -- course04 --intro

# TOML TODO 未填写时预期在 4.8 停止
cargo run --locked -- course04

# 填写后：只读主文件及引用
cargo run --locked -- course04 examples/configuration
# 填写后：再读取选中的一台设备
cargo run --locked -- course04 examples/configuration nas-home
```

默认 course01；course01～03 和旧 lesson01～06 可复习已完成内容。当前 13 个函数 TODO 尚未实现，不能把示例合法或编译通过视为 TOML 功能已完成。

更多：[学习方式](docs/Rust-Learning-Plan.md)、[实现计划](docs/Implementation-Plan.md)、[技术设计](docs/NAS-Client-Architecture.md)。
