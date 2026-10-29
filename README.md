# NASNet

跨平台 NAS 客户端。当前实现设备模型、候选配置校验、主配置读取和双输出日志；设备文件编解码/写入、真实协议访问和系统挂载尚未完成。

[课程目录](docs/Course-Syllabus.md) · **[开始练习 4.1](docs/lessons/04-errors-logging-and-toml-storage.md#exercise-4-1)** · [源码地图](docs/Code-Reading-Guide.md) · [技术设计](docs/NAS-Client-Architecture.md)

## 目录

- `src/`：项目实现。
- `examples/course01.rs`～`course04.rs`、`examples/common/`：独立课程样例。
- `examples/configuration/`：虚构 TOML 配置。
- `tests/`：Rust 自动测试及测试夹具。

## 运行配置

默认读取可执行文件同级的 `config.toml`；通过 cargo run 启动时通常在 `target/debug/`。默认文件缺失时使用内置值，不创建主文件；`--config` 显式指定的文件必须存在。相对设备与日志路径基于主文件目录。

[主文件示例](examples/configuration/config.toml)保存软件设置、日志设置和有序的设备引用；完整设备另存于 `devices/*.toml`。日志默认终端 INFO、文件 TRACE、路径 `logs/nasnet.log`。`RUST_LOG` 仅覆盖终端过滤。日志写 stderr 和追加文件，命令结果写 stdout；日志不记录密码或完整配置。

```sh
cargo run --locked
cargo run --locked -- --config examples/configuration/config.toml
# 完成设备加载后使用
cargo run --locked -- --config examples/configuration/config.toml --device nas-home
```

## 示例与检查

```sh
cargo run --locked --example course01
cargo run --locked --example course02
cargo run --locked --example course03
cargo run --locked --example course04
cargo test --locked
cargo fmt -- --check
cargo clippy --all-targets --locked -- -D warnings
```

Rust 2024 edition，最低 Rust 1.85；一个 package 含 lib/bin。`tempfile` 仅为开发依赖，文件/CLI 测试使用独立临时目录和子进程，不改写示例或真实配置。学习方法见[学习方案](docs/Rust-Learning-Plan.md)，待实现内容见[实现计划](docs/Implementation-Plan.md)。
