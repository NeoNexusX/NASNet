# 第 2 课 B：独立的 NFS/SMB 配置

本小节三个协议校验已完成，编译、格式、Clippy、演示及 51 项临时边界用例均检查通过。本文件保留第 2 课 B 的要求供复习，演示编号仍为 `lesson04`。当前练习见[第 3 课 A：模型封装与生命周期](05-encapsulation-and-lifetimes.md)；继续使用一个包、一个 crate，不引入第三方依赖或课程测试文件。

本课开始把设备地址和协议连接组合起来：设备保存地址列表以及多条协议连接；连接根据协议拥有不同的数据。当前只操作内存，不连接服务器、不读取真实密码。

### 数据怎么组织

连接记录、配置枚举和校验分派位于 `src/model/connection.rs`；NFS 配置位于 `src/model/connection/nfs.rs`，SMB 配置位于 `src/model/connection/smb.rs`。这些类型统一从 `crate::model` 导出：

| 类型 | 保存什么 |
|---|---|
| `NfsConfig` | 导出路径、请求版本、可选端口、AUTH_SYS 身份 |
| `NfsAuthSys` | UID、GID、附加组 |
| `SmbConfig` | 共享名、共享内起始目录、可选端口、账号认证、方言策略 |
| `SmbAuth` | 用户名、可选域、密码凭据引用 |
| `ConnectionConfig` | 一种协议的完整配置：`Nfs(...)` 或 `Smb(...)` |
| `ProtocolConnection` | 连接 ID 和 `ConnectionConfig` |

每个 `Device` 内嵌拥有连接，所以当前结构不重复保存设备 ID；连接的 Host 从所属设备的地址列表选择。多个导出或共享可以复用同一设备地址，但各自保存身份与路径。

NFS 的 UID/GID 是请求身份，不是远程文件属主，不表示服务器授予了某种权限。SMB 使用用户名、域和密码凭据引用，不复用 NFS 身份字段。`password_ref` 只是凭据标识，不是密码；系统凭据存储还未接入。

版本字段仅表达请求意向：`NfsVersion::V3` / `V4`；`SmbDialect::Auto` 自动协商，`Smb2` / `Smb3` 表示版本系列限制。这里不宣称原生库、服务器或系统挂载已经支持某个值，真实能力在后端接入时检查。

## 带数据的枚举

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

## 已完成练习 10：NFS 配置校验

`NfsConfig::validate(&self) -> Result<(), ConfigError>` 按以下顺序检查：

1. `export_path` 必须以 `/` 开头，不能包含 `\0` 空字符。不满足时返回 `InvalidNfsExportPath`。
2. `port` 为 `Some(0)` 时返回 `InvalidPort`；`None` 表示交给后端按规则处理。
3. 通过后返回 `Ok(())`。

`/` 是合法导出路径；空字符串和 `volume1/data` 不合法。路径中的普通空格可以保留，不自动修改输入。UID、GID 为 `0`、附加组为空也都合法，不在格式校验中推断权限。

`u16` 只能表达 `0..=65535`，所以本轮不必再检查大于 65535；接收用户文本和解析数字安排在后续输入边界。

## 已完成练习 11：SMB 配置校验

`SmbConfig::validate` 按以下顺序处理，遇到第一项错误立即返回：

| 顺序 | 条件 | 错误 |
|---|---|---|
| 1 | 共享名去掉首尾空白后为空，或含 `/`、反斜杠、`\0` | `InvalidSmbShareName` |
| 2 | 指定的起始目录以 `/` 或反斜杠开头，或含 `\0` | `InvalidSmbBasePath` |
| 3 | `username.trim()` 为空 | `EmptySmbUsername` |
| 4 | `password_ref.trim()` 为空 | `EmptyPasswordRef` |
| 5 | 端口是 `Some(0)` | `InvalidPort` |

全部通过返回 `Ok(())`。`team data` 是合法共享名，空格本身不一律禁止；`data/photos` 是路径，不能当成共享名。

`base_path` 为 `None` 或 `Some(String::new())` 表示共享根目录；`Some(String::from("photos/2026"))` 表示共享内相对目录。本轮只检查表中列出的格式规则，不解析 `.` / `..` 或规范化路径，完整路径处理在后端接入时完善。

注意 `base_path` 的类型是 `Option<String>`，当前函数接收 `&self`。需要借用里面的字符串，不能拿走它。可以先用 `if let Some(path) = &self.base_path` 理解“只有存在时才检查”，也可以使用 `match`。

用户名和凭据引用暂时只检查非空；域是可选字段。本轮不进行账号认证、凭据查找、完整协议字符串检查或服务器权限判断。

## 已完成练习 12：根据实际协议分派

`ProtocolConnection::validate`：

1. 先检查 `id.trim()`，为空时返回 `EmptyConnectionId`。
2. `match &self.config`，取出当前配置的引用。
3. 调用它自己的 `validate`，保留原错误。可以直接返回分支的 `Result`，也可以用 `?` 后返回 `Ok(())`。

NFS 配置只走 NFS 校验；SMB 配置只走 SMB 校验。格式错误不意味着应自动改成另一种协议。

`Device::try_add_connection` 已提供：先调用这条连接的校验，再查设备内是否有重复连接 ID，全部成功后才保存。失败时连接列表不变；不需要你重写上课已经掌握的添加流程。

## 连接列表与切片

设备里的 `connections` 字段为私有，外部通过方法读取：

```rust
pub fn connections(&self) -> &[ProtocolConnection]
```

`&[ProtocolConnection]` 是只读切片：借用一段连续的连接，可以遍历和查询长度，不创建新 Vec，也不能用它向设备追加或修改连接。这里与 `Vec<&Address>` 不同，切片借用的是原有列表中的一段数据，引用列表则拥有自己的引用数组。

第 3 课已经把设备字段及两个列表私有化，地址同样通过 `addresses()` 返回只读切片；旧的无校验 `add_address` 已移除。公开地址和连接仍可作为候选值构造，保存与更新通过设备的方法进行。

## 运行与观察

```sh
cargo check --locked
cargo run -- lesson04
```

演示依次调用 NFS 校验、SMB 校验和连接添加，三个校验均已完成，可以直接运行并观察下面的结果。

NFS 部分应接受 `/volume1/data`、`/`，拒绝相对路径、零端口和含空字符的路径。SMB 部分应接受合法共享、共享根目录以及含普通空格的共享名，再分别展示共享名、起始目录、空用户名、空凭据引用和零端口的错误。

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

协议校验已经完成，错误枚举原有的整段 `allow(dead_code)` 已清理。第 3 课新增的两个不存在错误仅在各自变体上临时豁免，留待新练习完成后清理。前四个演示均保留；模型封装与序列化在第 3 课分两轮继续。

参考：[枚举与结构体组合](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html)、[match](https://doc.rust-lang.org/book/ch06-02-match.html)、[if let](https://doc.rust-lang.org/book/ch06-03-if-let.html)、[切片](https://doc.rust-lang.org/book/ch04-03-slices.html)。
