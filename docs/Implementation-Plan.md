# NASNet 项目实现方案

与[12 课大纲](Course-Syllabus.md)、[技术设计](NAS-Client-Architecture.md)、[源码地图](Code-Reading-Guide.md)同步。第 01～03 课功能已验收；当前第 04 课保留架构、错误和日志补课，将存储教学改成 **config.toml 保存软件设置和设备引用，每台设备独立一个 TOML 文件**。

原 SQLite 实现与单 JSON 设计分别在[SQLite 验收记录](reviews/04-sqlite-acceptance.md)、[单 JSON 历史方案](archive/single-json-storage.md)中保留。历史验收不表示新 TOML 功能通过。

## 当前代码与本课目标

一个 Cargo package，Rust 2024 edition，最低 Rust 1.85；lib.rs 公开模型、配置、错误、存储，main/app 负责入口、编排和日志。模型/存储不输出日志，错误英文展示集中在 error.rs，tracing-subscriber 只在应用层初始化。

`Address::validate` 先检查 ID，再调用 impl 内私有的 validate_host；Host 没有独立业务类型和外部复用需求时，不额外公开接口。

`config.rs` 中已有 DeviceData / ConfigDocument 的 JSON 转换继续用于第 03 课复习。本课新增 AppConfig、SoftwareSettings、DeviceReference；软件主配置不再内嵌全部 DeviceData。toml 库和错误包装已接入、文档已先介绍，但十三个函数 TODO 尚未填写。

`StoreConnection` 指向目录下 config.toml，不是打开的文件句柄。load_app 只读主文件；load_device/save_device 按引用只操作一个设备。配置根目录是相对路径基准，不能受 cwd 影响。本课先约定受管理的相对 .toml 路径，不支持自动扫描注册或任意外部路径。

应用层示例支持 `course04 <directory> [device-id]`：不指定设备时只看软件设置和引用，指定后只加载那一台。StoreConnection 不缓存；应用层可保留成功加载的设备，显式 reload 后才替换，运行时状态和目录数据不当作配置频繁写回。

软件 log_level 的实际启动接线属于 A 部分待完成小改动。现有教学入口仍先按 RUST_LOG/info 建立 subscriber，不能因为字段存在就宣称其已生效。

## 写入边界

- 软件设置更新只保存 config.toml。
- 设备更新只保存对应设备 TOML。
- 单文件保存共用 write_atomic：编码成功后写同目录临时文件，sync_all，关闭后 rename；不先删除旧文件，不误删冲突文件。
- 新增先写新设备文件，再保存候选主配置，最后提交内存。主文件失败可留下未引用文件，必须明确报告；没有多文件事务。
- 移除仅解除引用，保存成功后提交内存，设备文件不自动删除。重新关联是显式操作，不扫描目录自动认领。

当前一个后台写入者，未实现多进程合并/锁协议、目录 fsync 或所有平台掉电保证。缺失主文件可返回默认设置；已登记设备缺失、ID 不一致、坏 TOML 与权限错误必须报告，不能静默跳过或用空配置覆盖。

## 里程碑

| 阶段 | 课次 | 结果 |
|---|---|---|
| M1/M2 | 01 | 内存模型、借用查询、稳定排序，功能已过 |
| M3 | 02～03 A | 协议校验、合法修改入口，功能已过 |
| M4 | 03 B、04 | JSON 学习成果已过；TOML 软件主文件与设备分文件接口待完成 |
| M5 | 05～06、07 接口 | C 安全封装与真实 NFS/SMB |
| M6 | 07～09 | 调度、会话、取消、重连与终端版 |
| M7 | 09～11 界面 | 独立后台、IPC、桌面浏览 |
| M8 | 11 挂载、12 | 系统挂载、自启恢复与跨平台交付 |

原生协议与挂载需要 NAS 和目标平台验证，模拟演示不可代替。

## 教学与检查

先介绍 toml 的数据模型和两个错误类型，再看 Cargo 依赖与最小例子，再填写[第 04 课](lessons/04-errors-logging-and-toml-storage.md)的 4.2～4.14。原有六项阅读/修改练习保留，4.1 是独立的新库入门练习。新库不能未经讲解直接塞到辅助代码里。

```sh
cargo check --locked
cargo fmt -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo run --locked -- course04 --intro
# 实现前预期停在 TODO 4.8
cargo run --locked -- course04
```

examples/configuration 仅为虚构示例。功能和理解都必须针对当前实际实现验收，不提前打勾；不新增课程测试文件，使用示例、临时目录、明确预期与静态检查。助手提供的辅助函数、错误包装、示例和日志也全部必读。
