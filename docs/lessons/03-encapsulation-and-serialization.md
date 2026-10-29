# 第 03 课：模型封装、生命周期与配置序列化

**状态：功能 ✅ 已完成并验收；完整调用链理解待补验。** 本课包含 A/B 两部分，保留原有讲解与练习要求供复习。

[返回课程目录与 Rust 知识清单](../Course-Syllabus.md) · [第 04 课补课](04-errors-logging-and-toml-storage.md#part-a)

先按[源码地图与前三课补充理解验收](../Code-Reading-Guide.md)阅读本课入口、演示、辅助函数和错误实现，再结合下文复习。已通过的实现不要求重做；理解状态须依据你的解释与小改动单独确认。

## 本课目录

- [A 部分](#part-a)
- [B 部分](#part-b)
- 完整演示：`cargo run --locked -- course03`

<a id="part-a"></a>

## A：模型封装与生命周期

第 1、2 课已完成。现在已经会校验输入，但旧的公开地址列表允许调用方直接 `push`，绕过校验。本小节把保存数据的入口收进 `Device`，练习更新、删除、借用查找。演示编号为 `lesson05`，不是第 5 课。

本课 A 部分讲模型封装和生命周期基础，B 部分讲配置转换、Serde 序列化与恢复时校验；两部分功能均已验收，完整理解待补验。当前保持一个 package 内的 lib/bin 分工，不新增课程测试文件。

工程当前使用 Rust 2024 edition 和同名模块文件/目录并列布局。本轮方法位于 `src/model/device.rs`；地址与 Host 校验位于 `src/model/address.rs`，统一错误类型位于 `src/error.rs`，连接及协议配置位于 `src/model/connection.rs` 和 `src/model/connection/`。`src/model.rs` 只声明模块并统一导出，库内通过 `crate::model`、二进制应用通过 `nasnet::model` 使用公开接口，设备字段继续保持私有。

### 先理解私有字段和只读接口

`Device` 的字段已改为私有。调用方的 `app/demo` 模块不能再直接访问 `device.addresses`，只能通过公开方法读取：

```rust
pub fn addresses(&self) -> &[Address]
pub fn connections(&self) -> &[ProtocolConnection]
pub fn id(&self) -> &str
pub fn name(&self) -> &str
```

这里省略了函数体，只展示签名。结构体本身是 `pub`，不表示它的每个字段都公开；本项目把保存列表的修改集中在定义 `Device` 的 `model::device` 模块中。[Rust 官方书：可见性与结构体字段](https://doc.rust-lang.org/book/ch07-03-paths-for-referring-to-an-item-in-the-module-tree.html)。

旧的 `add_address` 已移除。添加仍使用已完成的 `try_add_address` / `try_add_connection`；本轮补充更新和删除。读取方法不返回 `&mut Address`、`&mut ProtocolConnection` 或可变切片，调用方就不能通过它们把已保存的 Host 改成空字符串。

`Address`、`ProtocolConnection` 仍可公开构造，代表待提交的候选值。可以先把错误输入放进候选值，再交给设备校验；这不等于错误值已经被保存。返回的已保存记录是共享引用，即使记录的字段公开，也不能通过这个引用修改它。设备 ID 和名称沿用已有规则，本轮不新增名称校验。

### 更新、添加、删除各表达什么

| 操作 | 输入 | 成功结果 | 找不到 ID 时 |
|---|---|---|---|
| 添加 | 拥有的候选值 | 保存到末尾 | 允许添加；重复 ID 报错 |
| 更新 | 拥有的候选值 | 替换同 ID 记录，保留原位置 | 报不存在，不隐式添加 |
| 查找 | 借用的 `&str` | 借出记录引用 | `None` |
| 删除 | 借用的 `&str` | 移出记录并交还所有权 | 报不存在 |

更新使用候选值中的 ID 定位，按完整字符串匹配，不自动 trim，也不修改原 ID。更新同一 ID 不属于重复添加，不应返回 `DuplicateAddressId` / `DuplicateConnectionId`。协议连接更新接收调用方显式给出的配置，不自动选择或切换协议。

先读懂新错误：`AddressNotFound(String)` 和 `ConnectionNotFound(String)`。删除参数 `id: &str` 只是借来的字符串，而错误可能在参数失效后继续使用，所以错误需要拥有自己的字符串。`id.to_owned()` 或 `id.to_string()` 都可以；更新函数已拥有候选值，可以直接移交其中的 ID。

### 已完成练习 13：在原位置更新地址

位置：`src/model/device.rs`。

```rust
pub fn try_update_address(&mut self, address: Address) -> Result<(), ConfigError>
```

按顺序处理：

1. 调用候选地址的 `validate`。错误直接返回，不修改设备。
2. 按候选值中的 ID 查找原记录的位置。
3. 找不到返回 `AddressNotFound`；找到则用候选值替换该位置。
4. 返回 `Ok(())`。数量和保存顺序保持不变。

这里明确规定**校验在查找之前**：不存在的 ID 配上空 Host，应先返回 `EmptyHost`。这与已有添加函数先校验候选值的顺序一致。

可以用 `iter().position(...)` 得到下标，结束只读查找后再修改列表。不需要复制整个地址列表，也不需要 `clone`。不要先删除旧值再校验新值，否则失败时原配置会丢失。

### 已完成练习 14：删除地址并交还所有权

```rust
pub fn remove_address(&mut self, id: &str) -> Result<Address, ConfigError>
```

查到下标后移出那条地址，返回 `Ok(被移出的地址)`；未找到时返回 `AddressNotFound`。空列表、重复删除同一个 ID 都应正常返回错误，不能用 `unwrap` 让查找失败变成 panic。

`Vec::remove(index)` 会返回被移出的元素，并保持其他元素的相对顺序；`swap_remove` 会用末尾元素填补空位，可能改变顺序。地址同优先级时要保留保存顺序，因此这里使用 `remove`。[标准库：Vec::remove](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.remove)。

返回类型是 `Address`，不是 `&Address`：删除后设备已不再拥有那条地址，不能继续从列表借出它。调用方接手所有权，之后可读取、修改，或再次通过校验入口添加。

### 已完成练习 15：借用查找与生命周期

```rust
pub fn find_connection(&self, id: &str) -> Option<&ProtocolConnection>
```

按完整 ID 查找，找到返回设备内连接的引用，找不到返回 `None`。可以使用 `iter().find(...)`；这次尝试用迭代器完成查找。

这个函数同时接收两个引用：设备的引用 `&self`，以及查询字符串 `id`。返回值借自设备，和查询字符串是否还存在无关。方法中的生命周期省略规则让上面的签名等价于：

```rust
pub fn find_connection<'a>(&'a self, id: &str) -> Option<&'a ProtocolConnection>
```

`'a` 描述设备借用与返回引用之间的有效期关系，不会让设备活得更久。本轮保留省略写法即可，先看懂关系。[Rust 官方书：引用与生命周期](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)。

演示里特意让查询字符串先离开作用域：

```rust
let borrowed_connection = {
    let query_id = String::from("nfs-data");
    device.find_connection(&query_id)
};
```

这可以成立，因为引用指向设备中的连接。设备必须继续存在；如果后面还要使用这个引用，就不能在中间可变借用设备来更新或删除连接。最后一次使用这个引用之后，才可以再次修改设备，这也是前面改名演示中已经用过的借用规则。

### 已完成练习 16、17：连接更新和删除

```rust
pub fn try_update_connection(
    &mut self,
    connection: ProtocolConnection,
) -> Result<(), ConfigError>

pub fn remove_connection(&mut self, id: &str) -> Result<ProtocolConnection, ConfigError>
```

更新连接复用 `connection.validate()`，因此前一课的 NFS/SMB 校验和错误传播继续生效。先校验，再定位，最后在原位置替换；不存在返回 `ConnectionNotFound`。非法 NFS 路径、空 SMB 用户名等错误必须保留，失败后原协议配置和位置不变。

删除连接与删除地址的所有权规则相同：未找到返回 `ConnectionNotFound`，找到后移出并返回整个值，其余连接顺序不变。借用查找返回的是引用；删除返回的是拥有的数据，注意两个接口的差别。

更新参数按值传入，即使失败，候选值也不会自动回到调用方。这里只要求设备原有数据不变，不要求把输入退还；不需要为了保留输入而添加 `Clone`。

### 运行演示与验收

在项目根目录执行：

```sh
cargo check --locked
cargo fmt -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo run --locked -- lesson05
```

本节实现已验收，演示可完整运行；以下表格保留为复习标准。

地址部分应得到：

| 操作 | 结果 |
|---|---|
| 更新 `address-1` 为 `192.168.50.20`、优先级 1 | 成功，仍位于第一项，数量为 3 |
| 同 ID 改为空 Host、优先级 99 | `EmptyHost`，Host 和优先级仍为上次成功保存的值 |
| 更新不存在的 ID，Host 合法 | `AddressNotFound`，数量不变 |
| 更新不存在的 ID，Host 为空 | `EmptyHost`，体现先校验 |
| 查看同优先级排序 | `address-1`、`address-2`、`address-3` |
| 删除 `address-1` | 取回地址，其余顺序为 `address-2`、`address-3` |
| 再删 `address-1` | `AddressNotFound`，剩余值不变 |

连接部分应得到：

| 操作 | 结果 |
|---|---|
| 查找 `nfs-data`、`smb-data`、`missing` | 前两项找到，最后一项 `None` |
| 查询字符串先释放，再读取找到的连接 | 正常打印 `nfs-data` |
| 更新 `nfs-data` 为 V4、`/volume1/archive`、端口 2049 | 成功，原位置和 ID 不变 |
| 同 ID 改为相对路径 | `InvalidNfsExportPath`，保留上次成功的完整配置 |
| 更新 `smb-data` 为 SMB3、共享 `team data` | 成功，原位置不变 |
| 同 ID 改为空用户名 | `EmptySmbUsername`，原共享、方言和认证均不变 |
| 更新不存在的连接 ID，NFS 配置合法 | `ConnectionNotFound` |
| 更新不存在的连接 ID，NFS 路径非法 | `InvalidNfsExportPath`，体现先校验 |
| 删除 `nfs-data` | 取回连接，其余顺序为 `smb-data`、`smb-backup` |
| 再删 `nfs-data` | `ConnectionNotFound` |
| 从空设备删除地址和连接 | 分别返回不存在错误，数量仍为 0 / 0 |

最终设备保留 2 个地址、2 条 SMB 连接，分别按上表中的剩余顺序保存。删除、更新均不改变另一类列表；前四个演示仍应能运行。

本轮没有新增持久化测试文件；`cargo test --locked` 仍显示 `0 tests`。此前放在 `target` 的 51 项临时校验用例只覆盖旧的配置规则，不代表本轮更新/删除功能通过。

完成后应能解释：为什么不暴露可变切片；更新失败为什么不能影响原值；`Option<&ProtocolConnection>` 与 `Result<ProtocolConnection, ConfigError>` 的所有权有什么区别。对应源码提示和两个新错误变体的临时豁免已清理；配置序列化见下方 B 部分。

<a id="part-b"></a>

## B：配置序列化与恢复时校验

这一轮把内存中的一个设备转为 JSON，再恢复为设备。只在内存中处理字符串，主配置与设备 TOML 分文件保存及恢复安排在第 04 课。沿用此前选择，SMB 的 `password` 直接保存明文，不能 trim；演示使用虚构值，输出不展示密码或完整 JSON。

### 为什么需要配置快照

`Device` 维护私有列表，添加地址和连接时负责业务校验。若直接给它派生 `Deserialize`，自动生成的代码可以填入私有列表，跳过这些添加入口。字段私有并不意味着自动反序列化会调用你的校验函数。

因此新增 `src/config.rs`：`DeviceData` 是拥有数据的候选快照，`ConfigDocument` 为快照加上 `schema_version`。保存路径是 `&Device → DeviceData → ConfigDocument → String`；恢复路径是 `&str → ConfigDocument → DeviceData → Result<Device, ...>`。

`Address`、`ProtocolConnection` 和协议配置已经是公开候选值，本轮为它们派生 `Clone`、`Serialize` 和 `Deserialize`，避免重复维护整套字段。`Device` 不派生序列化 trait。快照仍不可信：例如空 Host 可以进入 `DeviceData`，只有恢复入口检查通过后才能成为设备保存的数据。

本轮只支持一个设备，设备 ID 和名称沿用现有 `Device::new` 的规则，不新增非空约束。地址和连接 ID 在各自列表内保持唯一；不同设备间的 ID 管理留给后续存储层。

### 新语法与依赖

`Cargo.toml` 已加入 `serde`（启用 `derive`）和 `serde_json`，具体版本写入 `Cargo.lock`。`Serialize` 把 Rust 值交给编码器，`Deserialize` 从输入重建 Rust 候选值；它们负责数据结构转换，不等于业务校验。参见 [Serde 官方派生说明](https://serde.rs/derive.html)。

连接配置使用 `#[serde(tag = "protocol", content = "settings", rename_all = "snake_case")]`，NFS 和 SMB 的 JSON 带显式协议标签。参见 [Serde 枚举表示](https://serde.rs/enum-representations.html)。下面是结构片段，不是完整设备文档：

```json
{
  "id": "nfs-data",
  "config": {
    "protocol": "nfs",
    "settings": {
      "export_path": "/volume1/data",
      "version": "V3",
      "port": null,
      "authentication": { "uid": 1000, "gid": 1000, "supplementary_groups": [100] }
    }
  }
}
```

结构体使用 `deny_unknown_fields` 拒绝多余字段；必需字段缺失会产生 JSON 错误。`Option` 字段缺失或为 `null` 时得到 `None`，这是本轮允许的行为。枚举无法识别的协议和版本同样产生解析错误。配置文档版本当前固定为 1，本轮不实现迁移。

`From<&Device>` 表示一定能完成的快照转换；`TryFrom<DeviceData>` 表示可能因数据非法而失败的恢复。`type Error = ConfigError` 声明恢复失败使用哪一种错误。当前工程采用 Rust 2024，`TryFrom` 已在 prelude 中；现有显式导入仍合法，不要求为了升级重写已验收逻辑。

### 已完成练习 18：从借用生成独立快照

位置：`impl From<&Device> for DeviceData`。

通过 `id()`、`name()`、`addresses()` 和 `connections()` 读取设备，构造 `Self`。字符串引用用 `to_owned()`，两个切片用 `to_vec()` 复制为拥有的列表。新增的 `Clone` 派生让列表中的元素可以被复制，包含字符串和协议配置。

不要移动设备字段，也不要按优先级重新排序。快照必须保持原保存顺序，两个相同优先级地址的先后也不变。调用之后，原设备仍可读取和继续使用；快照可以独立修改。

这部分占位语句已在完成练习时移除；实现现在返回拥有数据的独立快照。

### 已完成练习 19：消费快照，校验后恢复

位置：`impl TryFrom<DeviceData> for Device`。

用快照的 ID 和名称创建局部设备，然后按原顺序处理地址，最后处理连接。每项分别传给 `try_add_address` 或 `try_add_connection`，用 `?` 遇到错误立即返回。全部成功才返回 `Ok(device)`。

这里接收拥有的 `DeviceData`，所以可以把各字段移动到设备，不需要再 clone。Rust 允许按字段分别移走结构体内容，前提是之后不再使用整个已部分移动的结构体。

恢复函数内的局部设备尚未交给调用方；如果后面一项失败，已加入的前几项随局部设备一起释放。调用方只获得错误，不会获得半份设备。不要忽略错误，不要用 `unwrap` 把非法配置变为 panic，也不要给 `Device` 增加绕过校验的公开列表入口。

检查顺序沿用现有规则：先地址、再连接；每条先本地校验、再检查重复 ID。返回第一处错误即可。

### 已完成练习 20：生成 JSON

位置：`encode_device`。

用 `SCHEMA_VERSION` 和 `DeviceData::from(device)` 构造 `ConfigDocument`，调用 `serde_json::to_string_pretty(&document)`，成功时返回字符串。这个函数返回 `Result<String, ConfigIoError>`，因此需要用 `?` 转换并传播 Serde 错误，再包成 `Ok(...)`。

不要输出 JSON 到终端，因为里面包含原始密码。当前演示仅显示生成成功、往返比较结果和隐藏密码后的设备摘要。

### 已完成练习 21：解析、检查版本并恢复

位置：`decode_device`。

先用 `serde_json::from_str` 解析为 `ConfigDocument`，显式标注目标类型。若版本不等于 `SCHEMA_VERSION`，返回 `ConfigIoError::UnsupportedVersion`，不要继续恢复。否则调用 `Device::try_from`，成功时返回设备。

`ConfigIoError` 现已集中定义在 `src/error.rs`（config 模块保留重导出），已区分三种情况：JSON 格式/类型错误、模型校验错误、不支持的版本。两个 `From` 实现已提供，让 `?` 能自动把 `serde_json::Error` 和 `ConfigError` 包装到对应变体。保留错误类型，避免将所有失败都改成一个字符串。`UnsupportedVersion` 的临时 `allow(dead_code)` 已清理。

版本判断发生在完整文档解析之后、模型恢复之前。若文档结构本身错误，会先返回 JSON 错误；若结构合法、版本为 99，则返回版本错误。

### 运行与验收

```sh
cargo check --locked
cargo fmt -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo run --locked -- lesson06
```

四项实现已完成，`lesson06` 可完整运行，下面各项输出均已核对。编译、格式和严格 Clippy 检查通过，前五个演示也可正常运行；未新增课程测试文件。

| 场景 | 预期结果 |
|---|---|
| 合法快照恢复 | 设备 ID/名称不变，2 个地址、2 条连接 |
| 同优先级地址顺序 | `address-1`、`address-2` |
| 连接顺序 | `nfs-data`、`smb-data` |
| JSON 往返再编码 | 完整 JSON 相同为 `true`，包括协议配置、可选字段和列表顺序 |
| 密码首尾空格 | 原样保留为 `true` |
| 空 Host | `Validation(EmptyHost)` |
| 重复地址 ID | `Validation(DuplicateAddressId(...))` |
| 重复连接 ID | `Validation(DuplicateConnectionId(...))` |
| 相对 NFS 路径 | `Validation(InvalidNfsExportPath)` |
| 空 SMB 用户名/密码 | 分别为 `EmptySmbUsername` / `EmptySmbPassword` 校验错误 |
| 零端口 | `Validation(InvalidPort)` |
| 配置版本为 99 | `UnsupportedVersion(99)` |
| 空地址/连接 ID | 分别为 `EmptyAddressId` / `EmptyConnectionId` 校验错误 |
| 截断 JSON、缺少必需字段、未知协议 | `Json(...)` |
| 空设备往返 | 0 个地址、0 条连接 |
| 演示末尾的原设备 | 仍为原来的 2 个地址、2 条连接 |

演示使用 `Display` 打印中文错误信息；上表以变体名说明准确的错误类别。非法输入若打印“意外成功”，说明恢复流程漏了检查；编译通过不能替代行为验收。旧的 `lesson01`～`lesson05` 也应继续运行。

完成后应能解释：快照为什么需要复制而恢复可以移动；反序列化为什么还需校验；`?` 如何找到错误转换；失败时为什么不会交出半份设备。
