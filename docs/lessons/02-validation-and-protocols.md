# 第 02 课：错误处理与独立的 NFS/SMB 配置

用枚举区分 NFS 和 SMB 两种协议配置，用 `Result` 报告校验失败，并保证失败时设备数据保持不变。

**状态**：已完成。

[课程目录](../Course-Syllabus.md) · [上一课](01-model-and-ownership.md) · [下一课](03-encapsulation-and-serialization.md)

## 一、新知识

### 1. 枚举

官方示例（[The Rust Book 6.1](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html)）：

```rust
enum IpAddr {
    V4(u8, u8, u8, u8),
    V6(String),
}

let home = IpAddr::V4(127, 0, 0, 1);
let loopback = IpAddr::V6(String::from("::1"));

enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
    ChangeColor(i32, i32, i32),
}
```

- 每个变体可以携带不同类型、不同数量的数据，也可以不带数据。一个枚举值在某一时刻恰好是其中一个变体。
- 和 C 的 enum 不同，Rust 的枚举更像“带标签的联合体”：变体和它的数据绑在一起，不可能读错成另一个变体的数据。

**在项目中**：[connection.rs](../../src/model/connection.rs) 用枚举表达“一条连接恰好是一种协议”，不需要在一个结构体里放 `nfs`、`smb` 两个可选字段再去猜用的是哪个。

```rust
pub enum ConnectionConfig {
    Nfs(NfsConfig),
    Smb(SmbConfig),
}
```

### 2. match、if let 与 matches!

官方示例（[The Rust Book 6.2](https://doc.rust-lang.org/book/ch06-02-match.html)、[6.3](https://doc.rust-lang.org/book/ch06-03-if-let.html)）：

```rust
enum Coin {
    Penny,
    Nickel,
    Dime,
    Quarter,
}

fn value_in_cents(coin: Coin) -> u8 {
    match coin {
        Coin::Penny => 1,
        Coin::Nickel => 5,
        Coin::Dime => 10,
        Coin::Quarter => 25,
    }
}

fn plus_one(x: Option<i32>) -> Option<i32> {
    match x {
        None => None,
        Some(i) => Some(i + 1),     // 匹配的同时把里面的值绑定到 i
    }
}

let config_max = Some(3u8);
if let Some(max) = config_max {     // 只关心一种情况时用 if let
    println!("The maximum is configured to be {max}");
}
```

[`matches!`](https://doc.rust-lang.org/std/macro.matches.html) 判断一个值是否符合模式，返回 `bool`：

```rust
let foo = 'f';
assert!(matches!(foo, 'A'..='Z' | 'a'..='z'));

let bar = Some(4);
assert!(matches!(bar, Some(x) if x > 2));
```

- `match` 必须覆盖所有情况（穷尽检查）。以后给枚举新增变体时，编译器会指出每一处漏掉的 match。
- 分支按顺序尝试；`_` 匹配剩下的所有情况；`if` 守卫可以给模式附加条件。

**在项目中**：`ProtocolConnection::validate` 用 match 把校验分派到对应协议；测试里大量使用 `assert!(matches!(result, Err(ModelError::EmptyHost)))` 精确检查错误变体。

### 3. Result 与 ?

官方示例（[The Rust Book 9.2](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)）：

```rust
use std::fs::File;
use std::io::{self, Read};

fn read_username_from_file() -> Result<String, io::Error> {
    let mut username_file = File::open("hello.txt")?;
    let mut username = String::new();
    username_file.read_to_string(&mut username)?;
    Ok(username)
}
```

- `Result<T, E>` 只有 `Ok(T)` 和 `Err(E)` 两种值。可以预料、调用方可以处理的失败用 `Result` 返回；panic 只留给程序本身的 bug。
- `?` 放在 `Result` 后面：是 `Ok` 就取出里面的值继续执行，是 `Err` 就立刻把错误返回给调用方。`?` 只能用在返回 `Result`（或 `Option`）的函数里。
- `Result<(), E>` 表示“成功时没有额外数据”，成功返回 `Ok(())`。
- `unwrap()`、`expect("说明")` 遇到 `Err` 会 panic，适合测试和确定不会失败的地方。

**在项目中**：

```rust
pub fn validate(&self) -> Result<(), ModelError> {
    if self.id.trim().is_empty() {
        return Err(ModelError::EmptyAddressId);
    }
    self.validate_host()?;      // Host 不合法时，这里直接返回它的错误
    Ok(())
}
```

### 4. 自定义错误类型

官方示例（[std::fmt::Display](https://doc.rust-lang.org/std/fmt/trait.Display.html)）：

```rust
use std::fmt;

struct Point {
    x: i32,
    y: i32,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

let origin = Point { x: 0, y: 0 };
assert_eq!(format!("The origin is: {origin}"), "The origin is: (0, 0)");
```

- 实现 `Display` 后，值就能用 `{}` 格式化；`#[derive(Debug)]` 让它能用 `{:?}` 输出调试信息。
- `write!` 把格式化后的文本写进 formatter，返回 `fmt::Result`。
- 错误类型通常是一个枚举，每个变体代表一种失败；实现 `Display` 和 `std::error::Error` 后，它就能和标准库的错误统一处理（第 04 课继续）。

**在项目中**：[model/error.rs](../../src/model/error.rs) 中的 `ModelError` 列出所有模型规则的失败。

```rust
#[derive(Debug)]
pub enum ModelError {
    EmptyHost,
    DuplicateAddressId(String),
    // ……
}

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyHost => write!(formatter, "Host must not be empty or whitespace only"),
            Self::DuplicateAddressId(id) => write!(formatter, "Duplicate address ID: {id}"),
            // ……
        }
    }
}

impl std::error::Error for ModelError {}
```

### 5. 字符串检查

官方示例（[str::trim](https://doc.rust-lang.org/std/primitive.str.html#method.trim)、[str::contains](https://doc.rust-lang.org/std/primitive.str.html#method.contains)）：

```rust
let s = "\n Hello\tworld\t\n";
assert_eq!("Hello\tworld", s.trim());

let bananas = "bananas";
assert!(bananas.contains("nana"));
assert!(!bananas.contains("apples"));
```

常用组合：`s.trim().is_empty()`（空或只有空白）、`s.chars().any(char::is_whitespace)`（任意位置有空白）、`s.contains("://")`、`s.starts_with('/')`、`s.contains('\0')`。`trim` 返回新的切片，不修改原字符串。

**在项目中**：校验只读取输入，不 trim、不改写用户的值。SMB 密码的首尾空格必须原样保留。

### 6. Option 字段与“先校验，后修改”

`port: Option<u16>` 中，`None` 表示使用协议默认端口，`Some(0)` 是明确写错的值，检查写成 `self.port == Some(0)`。

修改集合的方法按“校验候选 → 检查冲突 → 修改”的顺序写，任何一步失败都直接返回，集合保持原样：

```rust
pub fn try_add_address(&mut self, address: Address) -> Result<(), ModelError> {
    address.validate()?;
    if self.find_address(&address.id).is_some() {
        return Err(ModelError::DuplicateAddressId(address.id));
    }
    self.addresses.push(address);
    Ok(())
}
```

按值接收的候选值在失败时被丢弃，不会退回给调用方；这里保证的是设备里的数据不变。

## 二、示例练习：examples/course02.rs

```sh
cargo run --locked --example course02
```

用待办任务和提醒方式练习校验。按提示改动、预测、运行，再恢复：

- **E2.1 错误枚举与 Display**：改写 `TaskError` 一个变体的 Display 文本，运行后找到它出现的位置。
- **E2.2 用 match 分派**：给 `Reminder` 新增一个变体（例如 `Push(String)`），看编译器在哪里报 match 没有覆盖全部变体。
- **E2.3 检查顺序与 ?**：main 里有一个 ID 和邮箱都错误的任务，解释为什么得到的是 `EmptyId`。
- **E2.4 Option 表示未指定**：`due_day` 为 `None` 合法、`Some(0)` 不合法；说明 `parse_due_day` 怎样用 `map_err` 加 `?` 把解析失败转换成 `InvalidDueDay`。
- **E2.5 先校验后修改**：重复 ID 被拒绝后，断言列表长度和原有元素都没变。

## 三、项目练习：src/model/

阅读 [address.rs](../../src/model/address.rs)、[connection.rs](../../src/model/connection.rs)、[nfs.rs](../../src/model/connection/nfs.rs)、[smb.rs](../../src/model/connection/smb.rs) 和 [device.rs](../../src/model/device.rs)。每条 `ProtocolConnection` 保存一个 ID 和一种协议配置，复用设备的地址列表，不重复保存 Host 或优先级。

#### P2.1 地址校验

`Address::validate` 先检查 ID，再调用私有的 `validate_host`。只做本地格式检查，不解析 DNS，也不连接网络；合法的 Host 包括 IPv4、IPv6 和域名。

| 输入 | 错误 |
|---|---|
| ID 为空或只有空白 | `EmptyAddressId`（先于 Host 检查） |
| Host 为空或只有空白 | `EmptyHost` |
| Host 中间或首尾有空格、制表符、换行 | `HostContainsWhitespace` |
| Host 带 `smb://` 等前缀，或含 `/`、`\` | `HostContainsSchemeOrPath` |

测试：`accepts_ip_addresses_and_domain_names`、`rejects_empty_hosts`、`rejects_whitespace_in_hosts`、`rejects_schemes_and_paths_in_hosts`、`validates_address_id_before_host`。

#### P2.2 NFS 配置

`NfsConfig::validate`：`export_path` 以 `/` 开头且不含 `\0`；显式端口不能为 0。`NfsAuthSys` 的 uid、gid 和附加组是请求身份，不等于服务器上的文件属主；校验也不确认服务器的版本或授权。

测试：`accepts_supported_nfs_versions_and_optional_ports`、`rejects_invalid_nfs_export_paths`、`rejects_explicit_zero_ports_for_both_protocols`。

#### P2.3 SMB 配置

`SmbConfig::validate`：

- `share_name` 非空，且不含 `/`、`\`、`\0`；
- `base_path` 为 `None` 或空字符串表示共享根目录，其他值不能以 `/`、`\` 开头，也不能含 `\0`；
- `username` 不能只有空白；`password` 不能是空字符串，但首尾空格原样保留；
- 显式端口不能为 0。

测试：`smb_accepts_share_root_and_relative_base_paths`、`smb_rejects_invalid_share_names`、`smb_rejects_absolute_or_null_containing_base_paths`、`smb_rejects_empty_credentials_but_preserves_password_whitespace`。

#### P2.4 连接校验分派

`ProtocolConnection::validate` 先检查连接 ID，再 match 到对应协议的 `validate`。测试：`connection_id_is_validated_before_protocol_settings`。

#### P2.5 失败时设备不变

`Device::try_add_address` 和 `try_add_connection` 都按“校验 → 查重 → push”执行。同一设备内连接 ID 必须唯一，不同设备可以重复。测试：`failed_address_additions_preserve_saved_values`、`failed_connection_additions_preserve_saved_connections`。

## 四、验收

```sh
cargo run --locked --example course02
cargo test --locked --test course02_protocols
```

能回答：

- 跟踪一个非法的 SMB 用户名：错误在哪里产生，经过哪些 `?` 回到调用方？
- 为什么用枚举表达协议，而不是在一个结构体里放 nfs、smb 两个 `Option` 字段？
- `port` 为 `None` 和 `Some(0)` 分别表示什么？
- `try_add_connection` 失败时，设备的哪些状态保证不变？传入的候选值去了哪里？
