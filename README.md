# NASNet

跨平台 NAS 客户端，同时是一套循序渐进的 Rust 课程。

**[课程目录](docs/Course-Syllabus.md)** · [当前：第 05 课](docs/lessons/05-traits-and-remote-backends.md) · [源码地图](docs/Code-Reading-Guide.md) · [技术设计](docs/NAS-Client-Architecture.md)

## 运行

```sh
cargo run --locked                                                   # 使用可执行文件旁的 config.toml
cargo run --locked -- --config examples/configuration/config.toml    # 指定主配置
cargo run --locked -- --config examples/configuration/config.toml --device nas-home   # 同时读取选中的设备文件
```

所选的主配置文件缺失时，程序会用内置默认值生成文件。已有文件直接读取，内容有误时报告错误。

配置格式、路径规则和日志选项见[第 01–04 课](docs/lessons/01-04-foundations.md)。

## 检查

```sh
cargo test --locked --all-targets
cargo fmt -- --check
cargo clippy --all-targets --locked -- -D warnings
```

第 05 课的五个测试骨架会先标记为忽略，每题补全后启用对应测试。项目使用 Rust 2024 edition，最低 Rust 1.99。
