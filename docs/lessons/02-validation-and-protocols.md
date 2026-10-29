# 第 02 课：错误处理与独立 NFS/SMB 配置

**状态：功能 ✅ 已完成并验收；完整调用链理解待补验。** 本课包含 A/B 两部分，保留原有讲解与练习要求供复习。

[返回课程目录与 Rust 知识清单](../Course-Syllabus.md) · [第 04 课补课](04-errors-logging-and-toml-storage.md#part-a)

先按[源码地图与前三课补充理解验收](../Code-Reading-Guide.md)阅读本课入口、演示、辅助函数和错误实现，再结合下文复习。已通过的实现不要求重做；理解状态须依据你的解释与小改动单独确认。

## 本课目录

- [A 部分](#part-a)
- [B 部分](#part-b)
- 完整演示：`cargo run --locked -- course02`

<a id="part-a"></a>

## A：配置校验与 Result

以下保留三个已完成练习的要求和说明供复习。它们属于项目配置阶段的基础地址校验部分。

### 先理解返回类型

前一课的 `Option<&Address>` 表达“有结果或没有结果”。这一课的 `Result<(), ConfigError>` 表达“操作成功，或者失败并给出原因”。

```rust
Ok(())                              // 成功，不需要附带其他结果
Err(ConfigError::EmptyHost)          // 失败：Host 为空
Err(ConfigError::DuplicateAddressId(String::from("a1")))
// 失败：ID 重复，并携带这个 ID
```

`()` 是单元类型，`Ok(())` 不是空指针。`Result<T, E>` 中的 `T` 是成功时的数据类型，`E` 是错误类型。这里的成功结果只需要说明检查通过，所以使用 `()`。

`ConfigError` 是已经提供的枚举 `enum`，现在集中定义在 `src/error.rs`。一个枚举值每次属于其中一种变体，例如 `EmptyHost`；`DuplicateAddressId(String)` 还可以拥有一个字符串，用来描述究竟哪个 ID 重复。错误文字的 `Display` 和标准错误接口实现也位于该文件，本轮不用填写。

### 练习 7：检查 Host

位置：`src/model/address.rs` 的 `impl Address`，这是私有方法，由公开 `validate()` 调用，不再单独导出。

```rust
fn validate_host(&self) -> Result<(), ConfigError>
```

按以下顺序检查，遇到第一项错误就返回：

| 条件 | 应返回的错误 |
|---|---|
| `trim()` 后为空 | `ConfigError::EmptyHost` |
| 任意位置包含空白字符，包括首尾空格、制表符、换行 | `ConfigError::HostContainsWhitespace` |
| 包含 `://`、`/` 或反斜杠 | `ConfigError::HostContainsSchemeOrPath` |
| 都没有 | `Ok(())` |

不自动修改输入。`" nas.example.com"` 应报错，不能悄悄去掉空格后通过。IPv6 如 `"2001:db8::10"` 应通过，不能把所有冒号都判为错误。空格和制表符的检查可以先用熟悉的循环。

几个独立的字符串操作示例：

```rust
"   ".trim().is_empty();      // true，trim 返回去掉首尾空白的借用视图
'\t'.is_whitespace();         // true，检查一个字符
"chapter/one".contains('/');  // true，包含这个字符
"part::item".contains("::"); // true，包含这段文字
```

这只是本轮的基础校验规则，不是完整的 IP/域名解析器。它不检查所有非法主机名、端口写法或域名语法，不查询 DNS，也不判断服务器是否存在。完整规则随后续连接输入模型补充。

### 练习 8：复用校验，并传播错误

位置：`src/model/address.rs` 的 `impl Address`。

```rust
pub fn validate(&self) -> Result<(), ConfigError>
```

先检查地址的 `id.trim().is_empty()`；为空时返回 `EmptyAddressId`。然后复用 Host 检查，全部通过后返回 `Ok(())`。不修改输入字符串；优先级 `0` 合法，不加新的优先级限制。

这里开始练习 `?`：当左边返回 `Ok(value)`，继续使用其中的值；返回 `Err(error)`，当前函数立即返回这个错误。当前几个函数的错误类型相同，不需要考虑错误类型转换。

先看一个与地址校验无关的例子：

```rust
fn show_number(text: &str) -> Result<(), std::num::ParseIntError> {
    let number = text.parse::<u32>()?;
    println!("数字是 {number}");
    Ok(())
}
```

`show_number("12")` 会输出数字并成功返回；`show_number("hello")` 会提前返回解析错误。`?` 不会把错误吞掉，也不会自动打印它；展示由调用方负责。

如果被调用函数返回 `Result<(), ConfigError>`，`?` 成功后得到的就是 `()`，不需要用 `let result` 保存。调用完成后可以直接返回 `Ok(())`，这也是本课检查时整理过的一处写法。

练习时先自己用 `match` 理解传播过程，再尝试使用 `?`。

### 练习 9：检查成功后才能修改列表

位置：`src/model/device.rs` 的 `impl Device`。

```rust
pub fn try_add_address(&mut self, address: Address) -> Result<(), ConfigError>
```

要求按以下顺序处理：

1. 调用传入地址的 `validate`，失败时直接传播错误。
2. 复用你已经写好的 `find_address`，检查同一设备内有没有相同 ID。判断依据是完整 ID 字符串。
3. 重复时返回 `DuplicateAddressId`，错误中带上这次传入的 ID；不能覆盖已有地址。
4. 全部通过后才追加地址，返回 `Ok(())`。

失败时，设备的数量、已有地址及其保存顺序都不应改变。不需要复制地址，也不需要 `clone`。传入参数按值接收，所以出错时这个参数也不会自动归还调用方；它没有被保存到设备。

第 3 课已移除旧的 `add_address`，设备字段改为私有；前两课演示也统一使用 `try_add_address`，通过只读方法访问保存数据。公开候选值可以包含错误输入，提交给设备时才校验。

### 运行演示，对照行为

```sh
cargo check --locked
cargo run -- lesson03
```

演示按 Host 场景 → 地址 → 添加地址的顺序观察；Host 场景构造合法 ID 的候选 Address，再经公开 validate 进入私有方法。三个方法均已完成，可以直接运行观察以下结果。

Host 部分应得到：

| 输入 | 预期结果 |
|---|---|
| `192.168.50.10`、`nas.example.com`、`2001:db8::10` | 基础校验通过 |
| 空字符串、只有空格 | Host 为空 |
| `nas example.com`、首尾带空格、包含制表符 | Host 包含空白字符 |
| `smb://nas.example.com`、`nas.example.com/data`、包含反斜杠 | Host 不应包含协议前缀或路径 |

单条地址部分应先接受有效地址，再分别展示空 ID、只有空格的 ID、空 Host、带路径 Host 的错误；优先级为 `0` 的有效地址也应通过。

添加地址部分应得到：

| 操作 | 结果 | 操作后的地址数量 |
|---|---|---|
| 添加 `address-1 / 192.168.50.10` | 成功 | 1 |
| 再添加 `address-1 / nas.example.com` | ID 重复 | 1 |
| 添加 `address-2`，Host 为空 | Host 为空 | 1 |
| 添加 `address-3 / nas.example.com` | 成功 | 2 |

最后保存的仍应是 `address-1 / 192.168.50.10` 和 `address-3 / nas.example.com`，按这个顺序展示。演示仅打印结果，不包含自动断言；需要你对照讲义检查。

整理并检查代码：

```sh
cargo fmt
cargo clippy --all-targets --locked -- -D warnings
```

本小节及协议校验的临时错误豁免和源码学习提示已清理；第 3 课新增的两个“找不到”错误变体也已完成实现并移除临时豁免。地址查询保留循环写法，仅在该方法上允许 Clippy 的 `manual_find` 提示。

第 1 课的两个小节仍可用 `cargo run -- lesson01` 和 `cargo run -- lesson02` 复习。请能解释：`Option` 和 `Result` 分别表达什么、`?` 在错误时返回到哪里，以及为什么修改列表必须放在所有检查之后。

参考：[枚举](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html)、[Result 与错误传播](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)、[str 的字符串操作](https://doc.rust-lang.org/std/primitive.str.html)。

<a id="part-b"></a>

## B：独立的 NFS/SMB 配置

本课开始把设备地址和协议连接组合起来：设备保存地址列表以及多条协议连接；连接根据协议拥有不同的数据。当前只操作内存，不连接服务器、不读取真实密码。

#### 数据怎么组织

连接记录、配置枚举和校验分派位于 `src/model/connection.rs`；NFS 配置位于 `src/model/connection/nfs.rs`，SMB 配置位于 `src/model/connection/smb.rs`。这些类型统一从 `crate::model` 导出：

| 类型 | 保存什么 |
|---|---|
| `NfsConfig` | 导出路径、请求版本、可选端口、AUTH_SYS 身份 |
| `NfsAuthSys` | UID、GID、附加组 |
| `SmbConfig` | 共享名、共享内起始目录、可选端口、账号认证、方言策略 |
| `SmbAuth` | 用户名、可选域、明文密码 |
| `ConnectionConfig` | 一种协议的完整配置：`Nfs(...)` 或 `Smb(...)` |
| `ProtocolConnection` | 连接 ID 和 `ConnectionConfig` |

每个 `Device` 内嵌拥有连接，所以当前结构不重复保存设备 ID；连接的 Host 从所属设备的地址列表选择。多个导出或共享可以复用同一设备地址，但各自保存身份与路径。

NFS 的 UID/GID 是请求身份，不是远程文件属主，不表示服务器授予了某种权限。SMB 使用用户名、域和密码，不复用 NFS 身份字段。`password` 保存原始密码，当前只在内存中；第 04 课在各自设备 TOML 文件中明文保存，应用日志隐藏密码。

版本字段仅表达请求意向：`NfsVersion::V3` / `V4`；`SmbDialect::Auto` 自动协商，`Smb2` / `Smb3` 表示版本系列限制。这里不宣称原生库、服务器或系统挂载已经支持某个值，真实能力在后端接入时检查。

### 带数据的枚举

以前的 `ConfigError` 有些变体携带字符串；现在枚举的变体携带完整结构体：

```rust
pub enum ConnectionConfig {
    Nfs(NfsConfig),
    Smb(SmbConfig),
}
```

一个 `ConnectionConfig` 值每次恰好是一种情况。它拥有对应结构体，而不是同时保存两套配置。

用独立例子理解如何借用内部数据：

```rust
enum Message {
    Text(String),
    Count(u32),
}

fn show(message: &Message) {
    match message {
        Message::Text(text) => println!("文字：{text}"),
        Message::Count(number) => println!("数量：{number}"),
    }
}
```

`message` 是引用，分支里得到的 `text` 也是引用，不会移走字符串。本课用 `match &self.config` 采用同样的借用方式。

### 已完成练习 10：NFS 配置校验

`NfsConfig::validate(&self) -> Result<(), ConfigError>` 按以下顺序检查：

1. `export_path` 必须以 `/` 开头，不能包含 `\0` 空字符。不满足时返回 `InvalidNfsExportPath`。
2. `port` 为 `Some(0)` 时返回 `InvalidPort`；`None` 表示交给后端按规则处理。
3. 通过后返回 `Ok(())`。

`/` 是合法导出路径；空字符串和 `volume1/data` 不合法。路径中的普通空格可以保留，不自动修改输入。UID、GID 为 `0`、附加组为空也都合法，不在格式校验中推断权限。

`u16` 只能表达 `0..=65535`，所以本轮不必再检查大于 65535；接收用户文本和解析数字安排在后续输入边界。

### 已完成练习 11：SMB 配置校验

`SmbConfig::validate` 按以下顺序处理，遇到第一项错误立即返回：

| 顺序 | 条件 | 错误 |
|---|---|---|
| 1 | 共享名去掉首尾空白后为空，或含 `/`、反斜杠、`\0` | `InvalidSmbShareName` |
| 2 | 指定的起始目录以 `/` 或反斜杠开头，或含 `\0` | `InvalidSmbBasePath` |
| 3 | `username.trim()` 为空 | `EmptySmbUsername` |
| 4 | `password.is_empty()` 为真 | `EmptySmbPassword` |
| 5 | 端口是 `Some(0)` | `InvalidPort` |

全部通过返回 `Ok(())`。`team data` 是合法共享名，空格本身不一律禁止；`data/photos` 是路径，不能当成共享名。

`base_path` 为 `None` 或 `Some(String::new())` 表示共享根目录；`Some(String::from("photos/2026"))` 表示共享内相对目录。本轮只检查表中列出的格式规则，不解析 `.` / `..` 或规范化路径，完整路径处理在后端接入时完善。

注意 `base_path` 的类型是 `Option<String>`，当前函数接收 `&self`。需要借用里面的字符串，不能拿走它。可以先用 `if let Some(path) = &self.base_path` 理解“只有存在时才检查”，也可以使用 `match`。

用户名检查 trim 后非空；密码只拒绝空字符串，不 trim，保留首尾空格；域是可选字段。本轮不进行账号认证、完整协议字符串检查或服务器权限判断。

### 已完成练习 12：根据实际协议分派

`ProtocolConnection::validate`：

1. 先检查 `id.trim()`，为空时返回 `EmptyConnectionId`。
2. `match &self.config`，取出当前配置的引用。
3. 调用它自己的 `validate`，保留原错误。可以直接返回分支的 `Result`，也可以用 `?` 后返回 `Ok(())`。

NFS 配置只走 NFS 校验；SMB 配置只走 SMB 校验。格式错误不意味着应自动改成另一种协议。

`Device::try_add_connection` 已提供：先调用这条连接的校验，再查设备内是否有重复连接 ID，全部成功后才保存。失败时连接列表不变；不需要你重写上课已经掌握的添加流程。

### 连接列表与切片

设备里的 `connections` 字段为私有，外部通过方法读取：

```rust
pub fn connections(&self) -> &[ProtocolConnection]
```

`&[ProtocolConnection]` 是只读切片：借用一段连续的连接，可以遍历和查询长度，不创建新 Vec，也不能用它向设备追加或修改连接。这里与 `Vec<&Address>` 不同，切片借用的是原有列表中的一段数据，引用列表则拥有自己的引用数组。

第 3 课已经把设备字段及两个列表私有化，地址同样通过 `addresses()` 返回只读切片；旧的无校验 `add_address` 已移除。公开地址和连接仍可作为候选值构造，保存与更新通过设备的方法进行。

### 运行与观察

```sh
cargo check --locked
cargo run -- lesson04
```

演示依次调用 NFS 校验、SMB 校验和连接添加，三个校验均已完成，可以直接运行并观察下面的结果。

NFS 部分应接受 `/volume1/data`、`/`，拒绝相对路径、零端口和含空字符的路径。SMB 部分应接受合法共享、共享根目录以及含普通空格的共享名，再分别展示共享名、起始目录、空用户名、空密码和零端口的错误。

连接添加部分应得到：

| 操作 | 结果 | 当前连接数量 |
|---|---|---|
| `nfs-data`：合法 NFS 导出 | 成功 | 1 |
| `smb-data`：合法 SMB 共享 | 成功 | 2 |
| 再添加 `nfs-data` | 连接 ID 重复 | 2 |
| 空连接 ID | 连接 ID 为空 | 2 |
| `invalid-nfs`：相对导出路径 | NFS 路径错误 | 2 |
| `invalid-smb`：共享名带路径 | SMB 共享名错误 | 2 |

最后设备仍只有一个地址，连接列表只保存 `nfs-data` 和 `smb-data`，两个配置分别展示。演示只打印结果，不包含自动断言。

整理并检查：

```sh
cargo fmt
cargo clippy --all-targets --locked -- -D warnings
```

协议校验已经完成，错误枚举原有的整段 `allow(dead_code)` 已清理。第 3 课新增的两个不存在错误也已接入实现，临时豁免已清理。前四个演示均保留；模型封装与序列化在第 3 课分两轮继续。

参考：[枚举与结构体组合](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html)、[match](https://doc.rust-lang.org/book/ch06-02-match.html)、[if let](https://doc.rust-lang.org/book/ch06-03-if-let.html)、[切片](https://doc.rust-lang.org/book/ch04-03-slices.html)。
