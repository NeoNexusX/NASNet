# 第 03 课：封装、生命周期与配置快照

让设备只能通过受检查的方法修改；学习删除时交还所有权、返回引用的生命周期，以及在“合法的模型”和“可序列化的候选数据”之间转换。

**状态**：已完成。

[课程目录](../Course-Syllabus.md) · [上一课](02-validation-and-protocols.md) · [下一课](04-errors-logging-and-toml-storage.md)

## 一、新知识

### 1. 模块与可见性

官方示例（[The Rust Book 7.3](https://doc.rust-lang.org/book/ch07-03-paths-for-referring-to-an-item-in-the-module-tree.html)）：

```rust
mod back_of_house {
    pub struct Breakfast {
        pub toast: String,
        seasonal_fruit: String,
    }

    impl Breakfast {
        pub fn summer(toast: &str) -> Breakfast {
            Breakfast {
                toast: String::from(toast),
                seasonal_fruit: String::from("peaches"),
            }
        }
    }
}

pub fn eat_at_restaurant() {
    let mut meal = back_of_house::Breakfast::summer("Rye");
    meal.toast = String::from("Wheat");               // 公开字段，可以读写
    println!("I'd like {} toast please", meal.toast);
    // meal.seasonal_fruit = String::from("blueberries");  // 错误：字段私有
}
```

- 条目默认私有，只有定义它的模块和子模块能访问；`pub` 让它对模块外可见。
- `pub struct` 只公开类型本身，字段要逐个加 `pub`。有私有字段的结构体，模块外无法直接写字面量构造，只能通过它提供的函数（如 `summer`）。
- 枚举一旦 `pub`，所有变体自动公开。

**在项目中**：`Device` 的字段全部私有，只开放只读访问和经过校验的修改方法，并且只返回 `&[T]`，不返回 `&mut Vec<T>`，所以任何修改都绕不过规则。`Address` 的字段公开，因为它只是候选值，保存进设备时才校验。

### 2. 可变迭代与原位置更新

官方示例（[slice::iter_mut](https://doc.rust-lang.org/std/primitive.slice.html#method.iter_mut)）：

```rust
let x = &mut [1, 2, 4];
for elem in x.iter_mut() {
    *elem += 2;                    // 通过 &mut 引用修改元素本身
}
assert_eq!(x, &[3, 4, 6]);
```

- `iter_mut()` 产生 `&mut T`，用 `*elem` 解引用后读写元素。
- 和 `find` 组合，就能找到要更新的那一个元素并整体替换。

**在项目中**：

```rust
pub fn try_update_address(&mut self, address: Address) -> Result<(), ModelError> {
    address.validate()?;                                       // 先校验新值
    if let Some(existing) = self.addresses.iter_mut().find(|saved| saved.id == address.id) {
        *existing = address;                                   // 原位置替换
        return Ok(());
    }
    Err(ModelError::AddressNotFound(address.id))
}
```

先校验再替换，失败时旧值、位置和数量都不变；不要“先删除再添加”。

### 3. 删除并交还所有权

官方示例（[Vec::remove](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.remove)、[Vec::swap_remove](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.swap_remove)、[Iterator::position](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.position)）：

```rust
let mut v = vec!['a', 'b', 'c'];
assert_eq!(v.remove(1), 'b');            // 返回被移除的元素，后面的元素依次前移
assert_eq!(v, ['a', 'c']);

let mut v = vec!["foo", "bar", "baz", "qux"];
assert_eq!(v.swap_remove(1), "bar");     // 用最后一个元素填补空位，更快但改变顺序
assert_eq!(v, ["foo", "qux", "baz"]);

let a = [1, 2, 3];
assert_eq!(a.iter().position(|&x| x == 2), Some(1));
assert_eq!(a.iter().position(|&x| x == 5), None);
```

- `remove` 把元素**移出** Vec 并返回，调用方得到它的所有权。
- `remove` 保持其余元素的顺序；`swap_remove` 是 O(1)，但会打乱顺序。
- `position` 返回第一个满足条件的下标（`Option<usize>`）。

**在项目中**：删除后设备不再拥有这条记录，所以返回拥有所有权的 `Address`，而不是引用；保存顺序必须不变，所以用 `remove`。

```rust
pub fn remove_address(&mut self, id: &str) -> Result<Address, ModelError> {
    self.addresses
        .iter()
        .position(|saved| saved.id == id)
        .map(|index| self.addresses.remove(index))
        .ok_or_else(|| ModelError::AddressNotFound(id.to_owned()))
}
```

错误要拥有自己的数据：借来的 `id: &str` 需要 `to_owned()` 才能放进错误返回。`ok_or_else` 把 `None` 变成 `Err`。

### 4. 生命周期标注

官方示例（[The Rust Book 10.3](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)）：

```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}

let string1 = String::from("long string is long");
{
    let string2 = String::from("xyz");
    let result = longest(string1.as_str(), string2.as_str());
    println!("The longest string is {result}");   // result 在 string2 有效时使用，合法
}
```

- 返回值可能借自 `x` 也可能借自 `y`，编译器无法自己判断，所以要用 `'a` 声明：返回值的有效期不超过两个参数中较短的那个。
- 生命周期标注只描述借用之间的关系，不会延长任何值的寿命。
- **省略规则**让大多数函数不必手写标注：① 每个引用参数各自得到一个生命周期；② 只有一个输入生命周期时，它被赋给所有输出；③ 方法有 `&self` 或 `&mut self` 时，`self` 的生命周期被赋给所有输出。

**在项目中**：

```rust
pub fn find_connection(&self, id: &str) -> Option<&ProtocolConnection>
// 按规则 ③ 展开后等价于：
pub fn find_connection<'a>(&'a self, id: &str) -> Option<&'a ProtocolConnection>
```

返回的连接引用只借自设备，与 `id` 无关，所以查询字符串可以先释放，拿到的引用仍然可用。

### 5. Clone、to_owned 与 to_vec

官方示例（[slice::to_vec](https://doc.rust-lang.org/std/primitive.slice.html#method.to_vec)）：

```rust
let s = [10, 40, 30];
let x = s.to_vec();          // 从切片复制出一个新的 Vec
```

- `to_owned()` 从借用得到拥有的值（`&str` → `String`）；`to_vec()` 从 `&[T]` 得到 `Vec<T>`，要求 `T: Clone`，逐个复制元素。
- `#[derive(Clone)]` 为类型生成逐字段复制的 `clone()`。复制出的数据与原值相互独立。

### 6. From 与 TryFrom

官方示例（[std::convert::From](https://doc.rust-lang.org/std/convert/trait.From.html)、[TryFrom](https://doc.rust-lang.org/std/convert/trait.TryFrom.html)）：

```rust
let string = "hello".to_string();
let other_string = String::from("hello");     // From<&str> for String
assert_eq!(string, other_string);

struct GreaterThanZero(i32);

impl TryFrom<i32> for GreaterThanZero {
    type Error = &'static str;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value <= 0 {
            Err("GreaterThanZero only accepts values greater than zero!")
        } else {
            Ok(GreaterThanZero(value))
        }
    }
}
```

- `From` 表示一定成功的转换；实现 `From<A> for B` 后自动获得 `a.into()`。
- `TryFrom` 表示可能失败的转换，用关联类型 `type Error` 声明失败类型；实现后可以写 `B::try_from(a)` 或 `a.try_into()`。

**在项目中**：[config.rs](../../src/config.rs) 在合法的 `Device` 和公开字段的候选数据 `DeviceData` 之间转换。

```rust
impl From<&Device> for DeviceData { /* 从借用复制：to_owned、to_vec */ }

impl TryFrom<DeviceData> for Device {
    type Error = ModelError;
    // 消耗快照：Device::new 后逐个 try_add_address、try_add_connection，第一处错误即返回
    fn try_from(data: DeviceData) -> Result<Self, Self::Error> { /* …… */ }
}
```

`From<&Device>` 只拿到借用，必须复制数据；`TryFrom<DeviceData>` 拿到快照的所有权，可以直接移动里面的字符串和列表。恢复复用已有的校验，不会交出半成品。

### 7. Serde 派生

官方示例（[serde.rs](https://serde.rs/derive.html)）：

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Point {
    x: i32,
    y: i32,
}
```

derive 为类型生成序列化与反序列化代码，具体的文本格式（JSON、TOML……）由另一个库决定。

**在项目中**：`DeviceData` 派生了这两个 trait，是可以直接读写的候选数据；`Device` 故意不派生 `Deserialize`，否则反序列化会直接填充私有字段、绕过校验。`ConfigDocument` 在快照外增加 `schema_version`，第 04 课用它读写设备文件。

## 二、示例练习：examples/course03.rs

```sh
cargo run --locked --example course03
```

示例在一个文件里用 `mod todo` 演示私有字段。按提示改动、预测、运行，再恢复：

- **E3.1 私有字段**：取消 main 中 `list.tasks.push(...)` 的注释，读编译错误，说明为什么在 `mod todo` 里面可以访问、外面不行。
- **E3.2 原位置更新**：`try_update` 用 `iter_mut().find()` 拿到 `&mut Task` 后整体替换，更新不存在的 ID 返回错误且列表不变。
- **E3.3 删除并交还所有权**：把 `remove` 中的 `Vec::remove` 换成 `swap_remove`，预测哪条断言失败。
- **E3.4 生命周期标注**：删掉 `find_title` 的 `'a` 标注，读编译器的提示，对照省略规则解释原因。
- **E3.5 快照与恢复**：修改快照不影响原清单；含重复 ID 的快照恢复失败。

## 三、项目练习：src/model/device.rs、src/config.rs

- **P3.1 原位置更新地址**：`try_update_address` 先校验，再按 ID 查找并替换；不存在时返回 `AddressNotFound`。因为先校验，“不存在的 ID + 空 Host”返回的是 `EmptyHost`。测试：`valid_address_update_preserves_its_position`、`invalid_address_update_keeps_original_values`、`missing_address_update_distinguishes_invalid_input`。
- **P3.2 删除地址**：`remove_address` 返回被删除的 `Address`，其余顺序不变；缺失或重复删除返回 `AddressNotFound`。测试：`removing_an_address_returns_ownership_and_preserves_other_order`。
- **P3.3 查找连接与生命周期**：`find_connection` 返回借自设备的引用。测试 `connection_borrow_outlives_the_query_string` 让查询字符串先离开作用域。
- **P3.4 更新与删除连接**：`try_update_connection`、`remove_connection` 与地址的规则相同，失败时协议配置和列表顺序不变。测试：`valid_connection_update_preserves_position_and_other_connections`、`invalid_connection_update_keeps_original_values`、`missing_connection_update_and_removal_keep_saved_values`。
- **P3.5 快照转换**：`From<&Device> for DeviceData` 和 `TryFrom<DeviceData> for Device`，先恢复地址、再恢复连接。快照里有密码，不能打印或写入日志。测试：`snapshot_*`、`restoration_*`、`empty_device_snapshot_restores_empty_lists`。

## 四、验收

```sh
cargo run --locked --example course03
cargo test --locked --test course03_snapshots
```

能回答：

- 删除为什么返回 `Address`，查找为什么返回 `&Address`？
- `find_connection` 返回的引用和查询字符串之间是什么关系？
- `From<&Device>` 为什么必须复制数据，`TryFrom<DeviceData>` 为什么可以直接移动？
- `Device` 为什么不派生 `Deserialize`？
