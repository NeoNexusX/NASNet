# 第 2 课 A：配置校验与 Result

本小节三个实现已完成并通过演示检查，已清理对应提示。课程合并后，本文件是第 2 课 A，演示命令仍为 `lesson03`。当前练习见[B：独立协议配置](04-protocol-configs.md)。课程测试已移除，仍使用一个包、一个 crate。

以下保留三个已完成练习的要求和说明供复习。它们属于项目配置阶段的基础地址校验部分。

## 先理解返回类型

前一课的 `Option<&Address>` 表达“有结果或没有结果”。这一课的 `Result<(), ConfigError>` 表达“操作成功，或者失败并给出原因”。

```rust
Ok(())                              // 成功，不需要附带其他结果
Err(ConfigError::EmptyHost)          // 失败：Host 为空
Err(ConfigError::DuplicateAddressId(String::from("a1")))
// 失败：ID 重复，并携带这个 ID
```

`()` 是单元类型，`Ok(())` 不是空指针。`Result<T, E>` 中的 `T` 是成功时的数据类型，`E` 是错误类型。这里的成功结果只需要说明检查通过，所以使用 `()`。

`ConfigError` 是已经提供的枚举 `enum`。一个枚举值每次属于其中一种变体，例如 `EmptyHost`；`DuplicateAddressId(String)` 还可以拥有一个字符串，用来描述究竟哪个 ID 重复。错误文字的 `Display` 和标准错误接口实现已提供，本轮不用填写。

## 练习 7：检查 Host

位置：`src/model/validation.rs`。

```rust
pub fn validate_host(host: &str) -> Result<(), ConfigError>
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

## 练习 8：复用校验，并传播错误

位置：`src/model/mod.rs` 的 `impl Address`。

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

## 练习 9：检查成功后才能修改列表

位置：`src/model/mod.rs` 的 `impl Device`。

```rust
pub fn try_add_address(&mut self, address: Address) -> Result<(), ConfigError>
```

要求按以下顺序处理：

1. 调用传入地址的 `validate`，失败时直接传播错误。
2. 复用你已经写好的 `find_address`，检查同一设备内有没有相同 ID。判断依据是完整 ID 字符串。
3. 重复时返回 `DuplicateAddressId`，错误中带上这次传入的 ID；不能覆盖已有地址。
4. 全部通过后才追加地址，返回 `Ok(())`。

失败时，设备的数量、已有地址及其保存顺序都不应改变。不需要复制地址，也不需要 `clone`。传入参数按值接收，所以出错时这个参数也不会自动归还调用方；它没有被保存到设备。

旧的 `add_address` 和公开字段暂时保留，供前两课复习；后续配置录入改用 `try_add_address`。当前还不能宣称所有构造和修改都经过校验，收紧入口是本阶段后续工作。

## 运行演示，对照行为

```sh
cargo check --locked
cargo run -- lesson03
```

演示按 Host → 地址 → 添加地址的顺序调用函数。三个方法均已完成，可以直接运行观察以下结果。

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

本小节对应的临时错误豁免和学习提示已清理；协议练习引入的新错误变体仍暂时使用局部 `allow(dead_code)`。查询与排序小节保留循环写法，仅在该方法上允许 Clippy 的 `manual_find` 提示。

第 1 课的两个小节仍可用 `cargo run -- lesson01` 和 `cargo run -- lesson02` 复习。请能解释：`Option` 和 `Result` 分别表达什么、`?` 在错误时返回到哪里，以及为什么修改列表必须放在所有检查之后。

参考：[枚举](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html)、[Result 与错误传播](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)、[str 的字符串操作](https://doc.rust-lang.org/std/primitive.str.html)。
