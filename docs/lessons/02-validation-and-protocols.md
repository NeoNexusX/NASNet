# 第 02 课：错误处理与独立 NFS/SMB 配置

本课校验已实现。阅读输入、检查顺序与失败后状态，通过正式测试复习。

[课程目录](../Course-Syllabus.md) · [上一课](01-model-and-ownership.md) · [源码地图](../Code-Reading-Guide.md)

<a id="part-a"></a>

## A：Result 与地址校验

阅读 [Address](../../src/model/address.rs)、[Device](../../src/model/device.rs)和 [ConfigError](../../src/error.rs)。`Result<T, E>` 用 Ok 表示成功、Err 表示失败原因；`?` 遇到 Err 提前返回，成功时取出值。校验返回 `Result<(), ConfigError>`，表示成功时没有额外数据。

### 练习 7：检查 Host（阅读）

位置：`Address` impl 内的私有 `validate_host`，从公开 `Address::validate` 进入。

检查顺序：空或仅空白 → 含空白字符 → 含协议前缀或路径。合法候选包括 IPv4、IPv6 和域名；这是本地格式校验，不查询 DNS、不探测 NAS，也不证明远端可连接。

| 输入 | 错误类别 |
|---|---|
| 空字符串、只有空格 | EmptyHost |
| 中间/首尾空格、制表符或换行 | HostContainsWhitespace |
| smb:// 前缀、斜杠或反斜杠路径 | HostContainsSchemeOrPath |

Host 规则只服务 Address，因此保持私有，不需要独立公开函数。验证不修改或 trim 输入。

### 练习 8：检查地址 ID（阅读）

位置：`Address::validate`。先拒绝空或仅空白 ID，再检查 Host，不访问设备列表。错误输入可存在于候选值中，只有提交保存时才必须通过校验。

测试：`validates_address_id_before_host`。ID 和 Host 同时无效时，应先得到 EmptyAddressId。

### 练习 9：成功后才修改列表（阅读）

位置：`Device::try_add_address`。先调用候选 validate，再查重复 ID，最后 push。重复返回 DuplicateAddressId，不覆盖旧记录；任何失败都保留数量、原值和顺序。

函数按值接收候选地址，失败时它不会自动退回调用方。这里保证的是设备数据不变，不要求归还输入。

<a id="part-b"></a>

## B：协议配置与校验分派

阅读 [连接模型](../../src/model/connection.rs)、[NFS](../../src/model/connection/nfs.rs)和 [SMB](../../src/model/connection/smb.rs)。设备保存地址列表；每条 ProtocolConnection 保存一个 ID 和一种协议配置，不重复保存 Host 或地址优先级。

```rust
pub enum ConnectionConfig {
    Nfs(NfsConfig),
    Smb(SmbConfig),
}
```

match 按变体取得对应配置。枚举让一条连接恰好拥有一种协议配置；不需要用多个可选字段猜测协议。

### 练习 10：NFS 校验（阅读）

位置：`NfsConfig::validate`。export_path 必须以 `/` 开始且不含空字符；显式端口不能是 0，None 表示未指定。NfsVersion 表达版本，NfsAuthSys 保存 UID、GID 与附加组。

访问身份与服务器文件属主、权限和导出规则是不同概念。当前校验不确认服务器版本、认证方式或远端授权。

### 练习 11：SMB 校验（阅读）

位置：`SmbConfig::validate`。

- share_name 非空且不含路径分隔符或空字符。
- base_path 为 None 或空字符串时表示共享根；其他值是共享内相对路径，不能含空字符。
- username 不能仅为空白，password 不能是空字符串；密码按原值保存，不 trim。
- 显式端口不能为 0。

SmbAuth 与 NfsAuthSys 分开表达身份；SMB 共享名、路径和认证信息不放到设备地址里。样例只显示隐藏密码的摘要。

### 练习 12：连接校验与添加（阅读）

位置：`ProtocolConnection::validate` 和 `Device::try_add_connection`。前者先检查连接 ID，再按协议调用校验；后者先 validate、再检查设备内重复连接 ID，最后追加。

测试分别检查空连接 ID、重复 ID、非法协议字段以及失败后原列表不变。不同设备可以使用相同连接 ID，同设备内必须唯一。

## 运行与验收

```sh
cargo run --locked --example course02
cargo test --locked --test course02_protocols
```

例子只展示一个 Result 分支和合法协议配置。输入矩阵与失败保留由测试断言；不要求样例打印每种输入。`connections()` 返回只读切片，可遍历，不允许外部直接追加。

完成后跟踪一次非法 SMB 用户名怎样返回 ConfigError，解释 `?` 的提前返回、match 分派、Option 端口、检查顺序与保存副作用。

参考：[Result](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)、[枚举与模式匹配](https://doc.rust-lang.org/book/ch06-00-enums.html)。
