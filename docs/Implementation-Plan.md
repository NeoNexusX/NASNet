# 实现计划

产品目标与边界见[技术设计](NAS-Client-Architecture.md)，学习步骤见[课程目录](Course-Syllabus.md)。本文只记录当前实现状态和下一步工作。

## 当前状态

| 范围 | 状态 | 验证位置 |
|---|---|---|
| 设备/地址/协议模型，更新、删除与快照恢复 | 已实现 | tests/course01_model.rs、course02_protocols.rs、course03_snapshots.rs |
| 主配置校验、解码与读取 | 已实现 | tests/course04_config.rs、course04_store.rs |
| 默认/显式配置路径，双输出日志与错误边界 | 已实现 | tests/course04_cli.rs |
| 主配置保存与设备 TOML 编解码 | 待实现 | [4.3](lessons/04-errors-logging-and-toml-storage.md#exercise-4-3)、[4.5](lessons/04-errors-logging-and-toml-storage.md#exercise-4-5)、[4.6](lessons/04-errors-logging-and-toml-storage.md#exercise-4-6) |
| 设备路径、单设备读取/保存、原子替换 | 待实现 | [4.7](lessons/04-errors-logging-and-toml-storage.md#exercise-4-7)、[4.9～4.12](lessons/04-errors-logging-and-toml-storage.md#exercise-4-9) |
| 设备登记与引用移除 | 待实现 | [4.13](lessons/04-errors-logging-and-toml-storage.md#exercise-4-13)、[4.14](lessons/04-errors-logging-and-toml-storage.md#exercise-4-14) |
| 真实协议、会话、后台、桌面与挂载 | 后续阶段 | [第 05～12 课](Course-Syllabus.md) |

已实现指功能代码和对应回归存在；学习者理解、其他平台与真实 NAS 验证另外记录。

## 接下来怎样实现

1. 完成[练习 4.1](lessons/04-errors-logging-and-toml-storage.md#exercise-4-1)，掌握直接使用 toml 库。
2. 先完成编码/解码与路径，再完成单文件替换和保存，最后完成登记/移除。每个函数题先补测试，具体契约以[第 04 课 B](lessons/04-errors-logging-and-toml-storage.md#part-b)为准。
3. 设备存储和理解验收完成后，再进入原生协议接入；不提前添加未需要的模块或运行时功能。

## 存储验收重点

- 启动只读主文件；按需读取选定设备，不因其他设备损坏而失败。
- 单设备保存只更新该文件；修改软件/日志设置只更新主文件。
- 原子替换失败保留旧内容，临时文件冲突不误删。
- 新增先写设备后发布引用；主文件保存失败可能留下未引用文件，原内存不提前提交。
- 移除只解除引用，保留设备文件；坏配置、缺失设备和权限错误必须报告。

当前不提供多文件事务、多写入者合并、目录 fsync 或配置文件监听。这些能力需要独立契约与验证后再扩展。
