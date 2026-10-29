# 历史练习：SQLite 保存与恢复（已归档，不占当前课号）

**归档说明：原 SQLite 作业的四项实现已完成功能验收，原 store.rs 仅格式检查未通过；详见[验收记录](../reviews/04-sqlite-acceptance.md)与[原始源码](../reviews/sqlite-store.rs.txt)。完整理解仍待补验。**

当前项目已按需求改为单一 config.json，不再依赖 SQLite。现行第 04 课合并了[结构、错误与日志](../lessons/04-errors-logging-and-toml-storage.md#part-a)和[JSON 文件持久化](../lessons/04-errors-logging-and-toml-storage.md#part-b)。下文保留当时教学设计和历史 API，不是当前运行说明；其中 TODO 与课程状态描述属于历史上下文。

[返回课程目录与 Rust 知识清单](../Course-Syllabus.md) · [原前置课程](../lessons/03-encapsulation-and-serialization.md)

## 本课目录

0. [必读代码、阅读顺序与理解验收](../Code-Reading-Guide.md)：包括助手编写的入口、错误包装、演示和依赖配置。
1. 新知识：自定义 trait、连接所有权、文件路径、SQL 参数和事务。
2. 数据结构与存储约定。
3. TODO 4.1：打开数据库并建表。
4. TODO 4.2：事务保存完整列表。
5. TODO 4.3：读取与恢复校验。
6. TODO 4.4：按 ID 删除。
7. 运行、故障场景和跨进程验收。
8. 完整代码理解验收与进入下一课的条件。

本课新增知识在总目录中暂不打勾。先读完整调用链，再填写并验收功能与理解，最后更新为 ✅。实现位置是 归档的 `docs/reviews/sqlite-store.rs.txt`，演示入口为 `cargo run --locked -- course04`。

## 1. 从 JSON 到磁盘

第 03 课已经能把一台设备转为字符串，并从字符串校验恢复。本课增加 `SqliteStore`，负责把多个设备的字符串保存到 SQLite 文件，再按原顺序取回。设备内的地址、连接和账号密码随 JSON 一起保存，加载时继续调用 `decode_device`。

当前阶段每台设备一行 JSON，不拆成地址、协议和认证子表。这样先完成事务和持久化闭环，同时保持已有模型规则。后续确实需要按子字段查询或迁移时，再调整存储结构。JSON 的 `schema_version` 仍由第 03 课检查；本课不实现数据库结构迁移。

SMB 账号密码按此前选择明文保存，保留原始空格和字符。演示只使用虚构密码，不输出完整 JSON 或密码。

## 2. 本课的新 Rust 知识

### 自己定义 trait

```rust
pub trait DeviceStore {
    fn save_all(&mut self, devices: &[Device]) -> Result<(), StoreError>;
    fn load_all(&self) -> Result<Vec<Device>, StoreError>;
    fn delete_device(&mut self, id: &str) -> Result<bool, StoreError>;
}
```

上一课实现了标准库已经定义的 `From`、`TryFrom`。这次 `DeviceStore` 是我们定义的接口约定，`impl DeviceStore for SqliteStore` 为 SQLite 提供实现。trait 内只有签名，具体行为写在 impl 中。调用这些 trait 方法时，要让 `DeviceStore` 在调用方的作用域内。

`save_all` 接收切片，借用所有设备，无需复制整个列表。`&mut self` 允许发起事务；`load_all` 返回拥有数据的 `Vec<Device>`，加载结果不借用数据库连接。删除返回 `bool`：删除成功为 true，目标不存在为 false，数据库失败用 Err 表达。

### Path、连接与作用域

`SqliteStore` 拥有一个私有 `Connection`。`open` 接收 `&Path`，避免强制把文件路径转换成 UTF-8 字符串；`Connection::open(path)` 可以接收路径。新文件的父目录需要已经存在，演示已负责创建。

连接离开作用域后会关闭。演示用 `drop(store)` 提前释放连接，再重新打开同一个文件，观察内容是否保留。这里只学习已有资源类型的自动释放，自己实现 `Drop` 和 C 句柄管理留到第 05 课。

### SQL 参数与事务

SQL 中的 `?1`、`?2`、`?3` 是参数位置，使用 `rusqlite::params![...]` 传入值。不要用 `format!` 把设备 ID 或 JSON 拼进 SQL；演示特意使用带单引号的 ID 和密码来检查参数传递。

`connection.transaction()` 借用连接创建事务。所有删除和插入都通过事务执行，成功才 `commit()`；未提交时事务默认回滚，因此 `?` 提前返回也能撤销本次修改。参见 [rusqlite 事务文档](https://docs.rs/rusqlite/0.37.0/rusqlite/struct.Transaction.html)。

`StoreError` 和错误转换已提供：数据库错误、配置错误、下标转换错误可通过 `?` 传播；`DeviceIdMismatch` 表达数据库 ID 和 JSON 内 ID 不一致。演示中的 `Box<dyn Error>` 也必须读懂：它拥有具体错误值，通过统一的 Error 接口向入口返回不同来源的失败。`From`、`Display`、`source` 各自作用及当前动态分派用法见[源码阅读指南](../Code-Reading-Guide.md)；进一步的泛型和动态分派设计安排在后续课程。

## 3. 表结构与行为约定

TODO 4.1 使用下面的建表 SQL：

```sql
CREATE TABLE IF NOT EXISTS devices (
    id TEXT PRIMARY KEY NOT NULL,
    position INTEGER NOT NULL UNIQUE CHECK (position >= 0),
    config_json TEXT NOT NULL
);
```

| 字段 | 含义 |
|---|---|
| `id` | 对应 `Device::id()`，同一数据库中不可重复 |
| `position` | 输入设备列表的位置，决定恢复顺序 |
| `config_json` | `encode_device` 产生的完整文档，包含版本号 |

设备内部两个列表的顺序保存在 JSON 中；数据库 `position` 只负责设备之间的顺序。设备 ID 和名称沿用模型已有规则，不在本课新增 trim 或非空校验。

`save_all` 表示**整体替换当前设备列表**，传空切片会清空。重复设备 ID 视为错误，失败时旧列表必须完整保留。不要使用 `INSERT OR REPLACE` 或 `INSERT OR IGNORE` 隐藏重复项。

## 4. TODO 4.1：打开数据库并初始化

依次打开连接、调用 `execute_batch` 建表，最后返回 `Self { connection }`。全程用 `?` 传播错误。`IF NOT EXISTS` 使已有表保留，不要在打开时删除旧表或清空记录。

依赖已加入 `rusqlite 0.37` 并启用 `bundled`：由依赖编译随附 SQLite，使用项目锁文件；不要求另装 SQLite 服务，但首次构建需要本机 C 编译器。工程自身仍不需要手写 FFI。参见 [rusqlite 使用文档](https://docs.rs/rusqlite/0.37.0/rusqlite/)。

## 5. TODO 4.2：事务保存完整列表

依次完成以下步骤，不把 DELETE 放在事务外：

1. 从连接创建事务。
2. 通过事务执行 `DELETE FROM devices`，无参数可传 `[]`。
3. 用 `devices.iter().enumerate()` 按输入顺序遍历。
4. 每个设备先调用 `encode_device`；位置用 `i64::try_from(position)?` 转换。Rust 2018 需要导入 `std::convert::TryFrom`。
5. 执行 `INSERT INTO devices (id, position, config_json) VALUES (?1, ?2, ?3)`，值使用参数传入。
6. 循环全部成功后提交事务，返回 `Ok(())`。

`?` 的失败路径必须退出整个保存函数。若第二个设备的 ID 与第一个重复，前面的 DELETE 和第一条 INSERT 都应回滚。`commit()` 会消费事务；提交后不再使用它。

## 6. TODO 4.3：按顺序加载并校验

使用 `prepare` 准备下面的查询，再调用 `query_map([], ...)`：

```sql
SELECT id, config_json FROM devices ORDER BY position ASC
```

闭包只读取 SQL 行，可以返回 `Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))`。这里第一项泛型参数 `_` 让编译器推断列索引类型，第二项指定字段读取为 String。

在闭包外遍历行结果，先用 `?` 解出 `(id, json)`，再调用 `decode_device(&json)?`。不把业务解码塞进只返回 `rusqlite::Result` 的闭包，避免混淆错误类型。确认 `device.id()` 和数据库 id 相同，否则返回 `StoreError::DeviceIdMismatch`；正确记录才能放入局部结果列表。

最后一次性返回整个 Vec。空数据库返回空列表；中途遇到坏 JSON、非法 Host 或 ID 不一致时返回错误，不静默跳过坏行。完成后删除 `DeviceIdMismatch` 的临时 `allow(dead_code)`。

`Statement` 借用连接，查询结果又依赖 Statement。把查询和遍历放在当前函数内完成，最终只返回拥有数据的 Device，避免让行引用逃出作用域。

## 7. TODO 4.4：删除指定设备

使用 `DELETE FROM devices WHERE id = ?1`，传入借用的 id。`execute` 返回影响行数，把“大于 0”转换为返回的 bool。

删除是单条 SQL，由 SQLite 保证该语句的原子性，不要求额外创建事务。不重新编号其他设备；position 中留下间隙不影响 `ORDER BY`。删除不存在的设备返回 `Ok(false)`，真正的数据库错误仍用 `?` 传播。

## 8. 运行与验收

```sh
cargo check --locked
cargo fmt -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo run --locked -- course04
```

当前代码可编译，但会停在 TODO 4.1。填写顺序建议为 4.1 → 4.2 → 4.3 → 4.4；演示会在已完成打开后先检查空库，因此未填写 4.3 时会先停在那里。四项完成后应看到：

| 场景 | 预期 |
|---|---|
| 打开新库 | 0 台设备 |
| 保存后关闭连接，再重新打开 | 2 台设备 |
| 设备顺序 | `nas-z'1`、`nas-a`，与 ID 字母序不同 |
| 完整配置往返比较 | true，含地址/连接顺序、协议字段和密码 |
| 重复 ID 的整批保存 | 数据库错误，旧配置完整保留为 true |
| 首次删除 / 再次删除 | true / false |
| 删除后剩余 | 只有 `nas-a` |
| 保存空列表 | 0 台设备 |
| 损坏 JSON | `StoreError::Config` |
| 合法 JSON 里 Host 为空 | `StoreError::Config`，内部为模型校验错误 |
| SQL ID 与 JSON ID 不一致 | `DeviceIdMismatch` |
| 演示结束 | 恢复并保留两台合法设备 |

演示不新增课程测试文件，只在系统临时目录创建独立数据库。故障注入只修改本次生成的演示数据库；最终打印跨进程加载命令。复制该命令重新启动程序，例如：

```sh
cargo run --locked -- course04 "/实际打印的目录/demo.sqlite3"
```

传入已有路径时只走加载展示分支，不执行保存、删除或故障注入。新进程必须能读取同样两台设备；这是与上一课仅内存往返不同的验收项。当前 TODO 尚未填写，不能将编译通过记为持久化已完成。

完成后应能解释：为什么保存需要事务；为什么 query_map 的闭包只读字段；加载为什么仍要校验；连接关闭后数据为什么还在；整体替换与逐条更新的区别。

## 9. 理解验收与推进条件

四个 TODO 以外，`main`、`run_course04`、`device_jsons`、示例构造函数、完整错误实现和 Cargo 配置都是本课必读内容。按[完整阅读路线与七项理解验收](../Code-Reading-Guide.md)逐项解释、预测并完成小改动；由助手提供代码不构成跳过阅读的理由。

功能通过后保留该结果，理解未确认的项目继续留空。两项都通过再进入第 05 课。当前新增阅读任务尚未验收，不能自动标记已理解。
