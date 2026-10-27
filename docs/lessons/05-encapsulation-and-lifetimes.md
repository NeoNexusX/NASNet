# 第 3 课 A：模型封装与生命周期

**状态：本小节的五个方法已完成并检查通过。** 下文保留练习说明供复习；运行 `cargo run --locked -- lesson05` 可以查看完整演示。下一步是第 3 课 B 的配置序列化与恢复时校验。

第 1、2 课已完成。现在已经会校验输入，但旧的公开地址列表允许调用方直接 `push`，绕过校验。本小节把保存数据的入口收进 `Device`，练习更新、删除、借用查找。演示编号为 `lesson05`，不是第 5 课。

第 3 课分两轮：本轮完成模型封装和生命周期基础；下一轮再接配置数据转换、Serde 序列化与恢复时校验。保持一个包、一个 crate，本轮没有新增依赖和课程测试文件。

工程使用 Rust 2018 edition 和 non-mod-rs 布局。本轮方法位于 `src/model/device.rs`；地址与 Host 校验位于 `src/model/address.rs`，统一错误类型位于 `src/model/error.rs`，连接及协议配置位于 `src/model/connection.rs` 和 `src/model/connection/`。`src/model.rs` 只声明模块并统一导出，调用方仍通过 `crate::model` 使用公开接口，设备字段继续保持私有。

## 先理解私有字段和只读接口

`Device` 的字段已改为私有。同级的 `demo` 模块不能再直接访问 `device.addresses`，只能通过公开方法读取：

```rust
pub fn addresses(&self) -> &[Address]
pub fn connections(&self) -> &[ProtocolConnection]
pub fn id(&self) -> &str
pub fn name(&self) -> &str
```

这里省略了函数体，只展示签名。结构体本身是 `pub`，不表示它的每个字段都公开；本项目把保存列表的修改集中在定义 `Device` 的 `model::device` 模块中。[Rust 官方书：可见性与结构体字段](https://doc.rust-lang.org/book/ch07-03-paths-for-referring-to-an-item-in-the-module-tree.html)。

旧的 `add_address` 已移除。添加仍使用已完成的 `try_add_address` / `try_add_connection`；本轮补充更新和删除。读取方法不返回 `&mut Address`、`&mut ProtocolConnection` 或可变切片，调用方就不能通过它们把已保存的 Host 改成空字符串。

`Address`、`ProtocolConnection` 仍可公开构造，代表待提交的候选值。可以先把错误输入放进候选值，再交给设备校验；这不等于错误值已经被保存。返回的已保存记录是共享引用，即使记录的字段公开，也不能通过这个引用修改它。设备 ID 和名称沿用已有规则，本轮不新增名称校验。

## 更新、添加、删除各表达什么

| 操作 | 输入 | 成功结果 | 找不到 ID 时 |
|---|---|---|---|
| 添加 | 拥有的候选值 | 保存到末尾 | 允许添加；重复 ID 报错 |
| 更新 | 拥有的候选值 | 替换同 ID 记录，保留原位置 | 报不存在，不隐式添加 |
| 查找 | 借用的 `&str` | 借出记录引用 | `None` |
| 删除 | 借用的 `&str` | 移出记录并交还所有权 | 报不存在 |

更新使用候选值中的 ID 定位，按完整字符串匹配，不自动 trim，也不修改原 ID。更新同一 ID 不属于重复添加，不应返回 `DuplicateAddressId` / `DuplicateConnectionId`。协议连接更新接收调用方显式给出的配置，不自动选择或切换协议。

先读懂新错误：`AddressNotFound(String)` 和 `ConnectionNotFound(String)`。删除参数 `id: &str` 只是借来的字符串，而错误可能在参数失效后继续使用，所以错误需要拥有自己的字符串。`id.to_owned()` 或 `id.to_string()` 都可以；更新函数已拥有候选值，可以直接移交其中的 ID。

## TODO 13：在原位置更新地址

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

## TODO 14：删除地址并交还所有权

```rust
pub fn remove_address(&mut self, id: &str) -> Result<Address, ConfigError>
```

查到下标后移出那条地址，返回 `Ok(被移出的地址)`；未找到时返回 `AddressNotFound`。空列表、重复删除同一个 ID 都应正常返回错误，不能用 `unwrap` 让查找失败变成 panic。

`Vec::remove(index)` 会返回被移出的元素，并保持其他元素的相对顺序；`swap_remove` 会用末尾元素填补空位，可能改变顺序。地址同优先级时要保留保存顺序，因此这里使用 `remove`。[标准库：Vec::remove](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.remove)。

返回类型是 `Address`，不是 `&Address`：删除后设备已不再拥有那条地址，不能继续从列表借出它。调用方接手所有权，之后可读取、修改，或再次通过校验入口添加。

## TODO 15：借用查找与生命周期

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

## TODO 16、17：连接更新和删除

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

## 运行演示与验收

在项目根目录执行：

```sh
cargo check --locked
cargo fmt -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo run --locked -- lesson05
```

刚开始 `lesson05` 会停在 TODO 13；按 13 → 14 → 15 → 16 → 17 完成后，才能看到完整结果。演示只打印，不包含自动断言。编译成功不意味着五个方法已经实现。

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

完成后应能解释：为什么不暴露可变切片；更新失败为什么不能影响原值；`Option<&ProtocolConnection>` 与 `Result<ProtocolConnection, ConfigError>` 的所有权有什么区别。检查通过后清理对应源码提示和两个新错误变体的临时 `allow(dead_code)`，再进入第 3 课 B。
