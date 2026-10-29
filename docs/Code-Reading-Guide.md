# 项目源码阅读与理解验收

这份指南覆盖当前 NASNet 的全部源码和构建入口，与[12 课目录](Course-Syllabus.md)同步。代码无论由谁写，都要能解释职责、调用方、输入输出、所有权、错误与副作用。功能通过与学习者理解分别记录；以下理解状态没有证据前均留空。

[第 04 课补课](lessons/04-errors-logging-and-toml-storage.md#part-a) · [第 04 课文件持久化](lessons/04-errors-logging-and-toml-storage.md#part-b) · [学习方式](Rust-Learning-Plan.md)

## 完整源码地图

| 理解已验收 | 文件与阅读单元 | 必须解释的内容 | 所属课 |
|---|---|---|---|
|  | Cargo.toml / Cargo.lock | package、lib/bin、edition2024、MSRV、依赖范围与锁定版本、features、--locked | 03～04 |
|  | .gitignore | target 与运行配置不提交、应用 Cargo.lock 提交；哪些是正式源码 | 01、04 |
|  | [lib.rs](../src/lib.rs) | 库根如何公开 model/config/error/store，库为什么不依赖 app | 04 |
|  | [main.rs](../src/main.rs) | binary 根、mod app、调用库与退出码；长期课程表 | 04 |
|  | [app.rs](../src/app.rs) | 参数、旧别名、路径转换、可选设备选择、课程分派、成功/失败流程 | 04 |
|  | [app/logging.rs](../src/app/logging.rs) | EnvFilter、fmt、stderr、一次性初始化、report_error、事件与来源链 | 04 |
|  | [error.rs](../src/error.rs) | 四层错误枚举、英文 Display、source、From、上下文和不输出的边界 | 02～04 |
|  | [model.rs](../src/model.rs) | 子模块声明和导出，业务模型与错误定义的位置 | 01、03～04 |
|  | [model/device.rs](../src/model/device.rs) 全部方法 | 字符串/列表归属、合法添加、更新失败不改原值、删除、借用查询 | 01～03 |
|  | [model/address.rs](../src/model/address.rs) | Address 内部私有 Host 校验、公开入口、顺序及本地格式与解析的区别 | 02 |
|  | [model/connection.rs](../src/model/connection.rs) | ProtocolConnection、ConnectionConfig、Serde 标签、校验分派 | 02～03 |
|  | [connection/nfs.rs](../src/model/connection/nfs.rs) | AUTH_SYS、版本、端口、路径；本地合法不代表远端授权 | 02～03 |
|  | [connection/smb.rs](../src/model/connection/smb.rs) | 共享与相对路径、账号/域/密码保真、派生 trait | 02～03 |
|  | [config.rs](../src/config.rs) 已完成部分 | DeviceData、单设备 ConfigDocument、From/TryFrom、encode/decode、Serde 属性 | 03 |
|  | [config.rs](../src/config.rs) 新增部分 | AppConfig、SoftwareSettings、DeviceReference、主配置和单设备 TOML、版本与引用规则 | 04 |
|  | [store.rs](../src/store.rs) 完整模块 | DeviceStore、StoreConnection、主配置/单设备路径、13 项 TODO 中的存储部分、临时写入与登记顺序 | 04 |
|  | [app/demo.rs](../src/app/demo.rs) run/run_lesson02/show_device | 样例构造、借用展示、查找与排序观察 | 01 |
|  | [app/demo.rs](../src/app/demo.rs) run_lesson03/04、协议样例、show_result/show_connections | 合法/非法输入、预期错误后继续、摘要与密码隐藏 | 02 |
|  | [app/demo.rs](../src/app/demo.rs) run_lesson05/06、show_restore_result | 借用作用域、更新候选、JSON 篡改与比较依据 | 03 |
|  | [app.rs](../src/app.rs) 的第 04 课 A/B 流程与辅助代码 | span/事件、课程分支、临时目录、只读分支、跨进程命令和故障注入的归属 | 04 |

每新增、移动或删除源码都要同步本表。第三方库不要求逐行读全部内部实现，但实际 API、feature、错误和资源契约必须读懂。第 04 课先介绍新依赖，不能先复制调用再说以后补。

## 前三课的补验路线

| 课号 | 阅读路线 | 需要提供的理解证据 |
|---|---|---|
| 01 | main/app → app/demo → model 导出 → Device/Address → show_device | 从 name 移入设备到展示借用；预测同优先级插入后的保存顺序和候选顺序，改样例验证 |
| 02 | app/demo → 协议样例 → Address/ProtocolConnection/NFS/SMB 校验 → error → show_result | 跟踪空 SMB 用户名如何返回错误；解释演示为何继续、失败后哪些值不变、日志为何不能打印密码 |
| 03 | run_lesson05/06 → Device 修改 → DeviceData/ConfigDocument → JSON → ConfigIoError | 解释快照和模型边界、引用生命周期、Json/Validation/UnsupportedVersion 的区别、Display 与 source |

不重填已通过的实现；解释不清楚就记录具体位置待补充，不能笼统标记“懂了”。原 SQLite 功能验收另见[记录](reviews/04-sqlite-acceptance.md)，其源码与讲义仅作为归档阅读。

## 第 04 课 A：从应用边界读完整错误链

先读 Cargo.toml 和 lib/main：一个 package 可以同时有 library 和 binary crate。库内 `crate::model` 指 library 根；二进制的 app 通过 `nasnet::model` 等路径使用库。`lib.rs` 不负责启动日志，main 也不放业务校验。

再从 app 的入口追踪：建立 subscriber → 解析课程与目录 → 进入演示/业务调用 → 返回 Result → 应用边界报告。初始化、程序参数和故意展示的错误分别是什么行为，需要逐项指出；`todo!` 是 panic，占位不能当成普通 Result 错误。

接着读 error.rs。enum 表达失败类别，Display 生成英文说明，source 借出底层原因，From 让 `?` 传播时转换到上层错误。错误模块不记录日志、不执行进程退出；AppError::exit_code 只提供建议退出码，由 main 返回给系统；模型和存储也不重复输出。事件记录在 app，subscriber 决定是否显示、如何格式化和写入哪里。report_error 在 ERROR 禁用时用一次 stderr 后备显示英文失败，避免日志过滤吞掉用户反馈；正常日志与后备只选一条路径。

最后读 app/logging.rs 的每个配置调用，解释 `RUST_LOG` 如何改变过滤、默认级别是什么、格式层为什么用 stderr、初始化为何只做一次。错误输出避免重复添加同一来源文字；如果读取来源链，要能指出引用来自当前错误且受其生命周期限制。

六项补课练习与预期见[第 04 课](lessons/04-errors-logging-and-toml-storage.md#part-a)。只有运行演示不能打勾。

## 第 04 课 B：从主配置引用到单台设备文件

1. 先读 toml 库介绍与完整示例 [config.toml](../examples/configuration/config.toml)、[home.toml](../examples/configuration/devices/home.toml)、[backup.toml](../examples/configuration/devices/backup.toml)。解释 table、数组表、字符串、缺失 Option 与 JSON 的差异。
2. 读 AppConfig/SoftwareSettings/DeviceReference 和 ConfigDocument，区分软件设置、引用和完整设备；新代码必须复用已完成的 DeviceData 转换与校验。
3. 追踪 StoreConnection::new、resolve_device_path：相对主文件目录而不是 cwd。说明路径对象不代表文件存在，约定的相对路径与文件 ID 必须核对。
4. 从 load_app 到清单，再从 load_device 到单设备。主文件缺失有默认值，已登记设备缺失必须报错；未选择设备时不读取全部设备。
5. 从 save_device 到 encode_device_toml/write_atomic，指出为什么修改一台不改主配置和其他设备文件。
6. 逐段读临时路径、static AtomicU64/fetch_add/Relaxed；解释 create_new 冲突、write_all、sync_all、关闭和 rename，以及清理不能掩盖原始错误。
7. 跟踪 register_device 的先设备后引用、候选清单和内存提交；解释主文件写失败可能留下孤立文件，单文件替换不能代替多文件事务。
8. 跟踪 remove_device_reference：只解除引用、不物理删除。说明重新关联与覆盖现有设备文件的区别。
9. 读 app::run_toml_course 的完整演示、只读主清单分支和可选 device-id；教学入口日志当前仍默认/RUST_LOG，文件 log_level 接入是待完成的小改动。

第 03 课 JSON 继续作为已完成的 Serde 练习；第 04 课正式文件统一 TOML，不维护两套持久化格式。完整设计和验收矩阵见[现行讲义](lessons/04-errors-logging-and-toml-storage.md#part-b)。

## 理解验收单

| 已通过 | 验收项 | 证据 |
|---|---|---|
|  | 工程结构 | 解释 package/lib/bin、2024、MSRV、模块依赖方向与所有新文件 |
|  | 全部助手代码 | 逐段解释 app、logging、error、临时路径与演示辅助，没有遗漏为黑箱的函数 |
|  | 新依赖 | 解释 tracing/subscriber 与 toml/serde 的分工，完成事件/span 和 TOML 入门练习 |
|  | 错误链 | 从非法 Host/损坏 TOML/文件 I/O 分别追踪到英文日志与退出码 |
|  | 保存与恢复 | 区分主配置和单设备、单文件替换、多文件发布与孤立文件 |
|  | 演示可信度 | 解释比较依据、坏配置注入、跨进程读取以及打印不等于自动断言 |
|  | 亲写与预测 | 完成第 04 课的 6 项阅读练习、1 项新库入门和 13 项函数 TODO，先预测再运行并解释差异 |

理解项需你的解释、小改动和预测证据，助手的静态代码审查不能替代。功能通过但理解有缺口时保留功能状态，继续补充阅读。
