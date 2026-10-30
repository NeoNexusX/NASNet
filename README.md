# NASNet

跨平台 NAS 客户端，同时是一套循序渐进的 Rust 课程。

**[课程目录](docs/Course-Syllabus.md)** · [当前：第 04 课](docs/lessons/04-errors-logging-and-toml-storage.md) · [源码地图](docs/Code-Reading-Guide.md) · [技术设计](docs/NAS-Client-Architecture.md)

## 运行

```sh
cargo run --locked                                                   # 使用可执行文件旁的 config.toml，不存在时用内置默认值
cargo run --locked -- --config examples/configuration/config.toml    # 指定主配置
cargo run --locked -- --config examples/configuration/config.toml --device nas-home   # 完成 P4.5 后可用
```

配置格式、路径规则和日志选项见[第 04 课](docs/lessons/04-errors-logging-and-toml-storage.md)。

## 检查

```sh
cargo test --locked
cargo fmt -- --check
cargo clippy --all-targets --locked -- -D warnings
```

Rust 2024 edition，最低 Rust 1.85。
