# 第 01 课：设备模型、所有权、查询与排序

本课方法已实现，复习时阅读现有代码并修改测试验证；不用重新创建旧接口。

[课程目录](../Course-Syllabus.md) · [源码地图](../Code-Reading-Guide.md) · [Rust 测试入门](04-errors-logging-and-toml-storage.md#test-basics)

<a id="part-a"></a>

## A：设备与地址

阅读 [Device](../../src/model/device.rs)、[Address](../../src/model/address.rs)、[独立样例](../../examples/course01.rs)及[共享样例数据](../../examples/common/mod.rs)。

Device 拥有 ID、名称、地址和连接列表；Vec 拥有其中的记录，记录拥有自己的 String。Device 字段私有，对外通过 `id()`、`name()`、`addresses()` 等只读方法观察。Address 是可公开构造的候选值，保存时由设备检查。

### 练习 1：Device::new（阅读）

位置：`src/model/device.rs::Device::new`。

```rust
pub fn new(id: String, name: String) -> Self
```

`new` 是没有 self 的关联函数，Self 代表 Device。传入 String 后所有权移入设备，不能再把原变量当作仍拥有字符串来使用。读清结构体构造中哪些值被移动、哪些字段初始化为空列表。

测试：`tests/course01_model.rs::new_device_owns_its_name_and_starts_empty`。解释它为什么能在构造后读取名称，而调用方不能继续使用已移走的 name。

### 练习 2：添加地址（阅读）

当前入口是 `Device::try_add_address(&mut self, address) -> Result<(), ConfigError>`。它取得候选地址的所有权，在校验和重复 ID 检查成功后追加到 Vec；失败时设备原值不变。具体校验在[第 02 课](02-validation-and-protocols.md)。

`&mut self` 允许修改设备，按值参数让设备可以拥有地址。列表保存插入顺序，不在添加时排序。不要另写绕过校验的添加入口。

### 练习 3：查询数量（阅读）

位置：`Device::address_count(&self) -> usize`。`&self` 只借用设备，调用后设备仍可继续使用。返回的数字是拥有的值，读取它不要求继续借用设备。

### 练习 4：重命名（阅读）

位置：`Device::rename(&mut self, name: String)`。替换名称时接手新 String，保留 ID 和所有地址。

样例先借用 `device.name()` 并打印，再调用 rename。解释只读引用最后一次使用后为什么可以可变借用；临时将引用使用移到 rename 后，预测编译错误，观察后恢复。

<a id="part-b"></a>

## B：查询与优先级

### 练习 5：按 ID 查找（阅读）

```rust
pub fn find_address(&self, id: &str) -> Option<&Address>
```

找到时返回列表元素的引用，不复制 Address；没有匹配项返回 None。返回引用借自设备，设备必须继续存在。`Option` 表达“可能没有”，不能把正常缺失当成必须 panic 的错误。

测试：`address_lookup_borrows_saved_values_and_reports_absence`。解释测试中的引用与设备列表元素之间的关系。

### 练习 6：生成候选顺序（阅读）

```rust
pub fn addresses_by_priority(&self) -> Vec<&Address>
```

返回新的引用列表，按 priority 升序排列，不修改原 Vec。`iter().collect()` 收集的是引用；新 Vec 拥有这些引用，但不拥有 Address。稳定排序保证优先级相同的地址保持保存顺序。

测试：`priority_sort_is_stable_without_changing_saved_order`。它同时断言候选顺序和保存顺序；只断言候选顺序无法证明原配置没被改动。

## 运行与验收

```sh
cargo run --locked --example course01
cargo test --locked --test course01_model
```

样例展示两个地址；测试另外构造同优先级和更高优先级的情况。完成后解释 String 的移动、共享/可变借用、Option、Vec<Address> 与 Vec<&Address>，并修改一个测试输入先预测再运行。

参考：[所有权](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)、[引用与借用](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)、[迭代器](https://doc.rust-lang.org/book/ch13-02-iterators.html)。
