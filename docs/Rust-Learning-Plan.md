# Rust 学习方法

起点：有 Python、C/C++ 基础，已经接触所有权、借用和引用。目标是能解释、修改并验证 NASNet 的实现。

[课程目录](Course-Syllabus.md) · [源码地图](Code-Reading-Guide.md) · [当前练习 4.1](lessons/04-errors-logging-and-toml-storage.md#exercise-4-1)

## 每道题怎样进行

1. 确定题目编号、动手文件、相关 API 与验收命令。
2. 先学习新增语法或依赖：用途、与现有模块的分工、最小用法、版本/feature、错误和资源生命周期。
3. 阅读调用链和已有测试，写出成功、失败及边界情况的预测。
4. 编写或修改行为测试，观察有意改错时的失败，再实现目标功能。
5. 运行对应测试与全量回归，解释结果和失败后状态。

关键逻辑由学习者亲写。遇到困难先给提示，明确要求完整实现后再提供答案；已实现的阅读题通过解释、补边界测试和小改动验收。示例、夹具和辅助函数也属于必读代码。

## 现在做什么

先读[第 04 课 C.1](lessons/04-errors-logging-and-toml-storage.md#test-basics)，再按[4.1 完整题目](lessons/04-errors-logging-and-toml-storage.md#exercise-4-1)创建指定测试文件。完成后按该课给出的顺序阅读启动基础实现、补齐设备存储。

## 验收约定

功能以可重复的测试为证据，理解以调用链说明、预测和修改验证为证据。记录具体文件/函数和命令结果；不因为助手写过代码、样例运行成功或出现 `0 tests` 就勾选完成。

`src/` 服务项目，`examples/` 放独立样例，`tests/` 放自动验收。文件测试使用临时目录，CLI 测试使用子进程环境和工作目录，不改写真实配置。真实协议、权限和挂载仍需 NAS 与目标系统验证。

检查命令统一为 `cargo test --locked`、`cargo fmt -- --check`、`cargo clippy --all-targets --locked -- -D warnings`；测试方法见[第 04 课 C](lessons/04-errors-logging-and-toml-storage.md#part-c)。
