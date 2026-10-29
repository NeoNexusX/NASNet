# 第 03 课：封装、生命周期与配置快照

本课的更新、删除和快照转换已实现。阅读现有实现，并用测试验证失败保留、引用归属与恢复校验。

[课程目录](../Course-Syllabus.md) · [上一课](02-validation-and-protocols.md) · [第 04 课](04-errors-logging-and-toml-storage.md)

<a id="part-a"></a>

## A：封装、更新与删除

阅读 [Device](../../src/model/device.rs)。设备字段私有，只提供只读访问和经过校验的修改入口；不暴露可变切片供调用方绕过规则。

### 练习 13：原位置更新地址（阅读）

```rust
pub fn try_update_address(&mut self, address: Address) -> Result<(), ConfigError>
```

先校验候选，再按 ID 查找；存在时原位置替换，不存在返回 AddressNotFound。校验先于查找，所以“不存在的 ID + 空 Host”返回 EmptyHost。失败保留旧值、位置和数量，不先删除再验证。

### 练习 14：删除并返回所有权（阅读）

```rust
pub fn remove_address(&mut self, id: &str) -> Result<Address, ConfigError>
```

找到时从 Vec 移出并返回 Address，其他元素顺序不变；缺失和重复删除返回 AddressNotFound。返回拥有的数据而非引用，因为设备已不再拥有被删除记录。

比较 Vec::remove 与 swap_remove：后者可能改变保存顺序，不符合这里的契约。错误拥有 String，因此借来的 id 需要 to_owned 后才能随错误返回。

### 练习 15：借用与查询字符串的生命周期（阅读）

```rust
pub fn find_connection(&self, id: &str) -> Option<&ProtocolConnection>
```

返回引用借自设备，与查询字符串的存活时间无关。省略规则对应的关系是：

```rust
pub fn find_connection<'a>(&'a self, id: &str) -> Option<&'a ProtocolConnection>
```

`'a` 描述借用关系，不延长设备生命。样例和测试让查询 String 先离开作用域，仍可读取设备里的连接；设备必须继续存在，且不能在这个引用后续还会使用时可变借用设备。

### 练习 16：更新连接（阅读）

位置：`Device::try_update_connection`。先 `connection.validate()`，再按 ID 定位并原位置替换；缺失返回 ConnectionNotFound，非法协议保留原有错误。失败时协议配置和列表顺序不变。

### 练习 17：删除连接（阅读）

位置：`Device::remove_connection`。规则与删除地址相同：返回整个 ProtocolConnection，保留其他顺序，缺失正常返回错误。借用查找返回引用，删除返回拥有的数据，解释两者为什么不同。

<a id="part-b"></a>

## B：候选快照与恢复

阅读 [config.rs](../../src/config.rs) 中的 DeviceData、ConfigDocument 和两种转换。

DeviceData 的公开字段是候选数据，不代表业务合法；ConfigDocument 在候选快照外增加 schema_version。Device 不派生 Deserialize，避免反序列化直接填充私有列表绕过添加入口。

### 练习 18：从借用生成独立快照（阅读）

位置：`impl From<&Device> for DeviceData`。复制 ID/名称、地址和连接列表，让快照独立拥有数据。字符串使用 to_owned，切片使用 to_vec；后者通过 Clone 复制元素。

修改快照不改变原设备。快照中也有密码，不能打印完整对象或把它写入日志。

### 练习 19：消耗快照并恢复（阅读）

位置：`impl TryFrom<DeviceData> for Device`。`type Error = ConfigError` 声明失败类型。先创建空设备，再按保存顺序逐个通过 try_add_address 和 try_add_connection 恢复；先地址、后连接，返回第一处错误。

From<&Device> 必须复制借用的数据，TryFrom<DeviceData> 则可移动输入。错误时不会交出半份设备。当前 Device::new 不额外校验设备 ID/名称，恢复不擅自新增规则。

### Serde 与格式边界

Serialize/Deserialize 定义结构转换，不替代业务校验。公开候选结构用 deny_unknown_fields；协议枚举通过 protocol/settings 标签区分 nfs/smb。TOML 中 None 省略字段。

本课在内存中检查快照转换。[练习 4.5](04-errors-logging-and-toml-storage.md#exercise-4-5)和[4.6](04-errors-logging-and-toml-storage.md#exercise-4-6)再实现 TOML 编码、解析、版本检查与模型恢复。

## 运行与验收

```sh
cargo run --locked --example course03
cargo test --locked --test course03_snapshots
```

测试覆盖成功更新、非法更新保留、缺失/重复删除、查询字符串先释放、快照独立、恢复时重复 ID/非法字段拒绝、空设备及密码空格保留。完成后解释每个场景的所有权与失败边界，并修改一个用例先预测再运行。

参考：[生命周期](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)、[From/TryFrom](https://doc.rust-lang.org/std/convert/index.html)、[Serde 枚举表示](https://serde.rs/enum-representations.html)。
