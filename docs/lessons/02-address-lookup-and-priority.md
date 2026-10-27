# 第 1 课 B：地址查找与优先级排序

本小节的查找与排序已完成并通过 6 项验收，测试代码已移除。课程合并后，本文件是第 1 课 B，演示命令仍为 `lesson02`。以下说明供复习，当前练习见[第 3 课 A：封装与生命周期](05-encapsulation-and-lifetimes.md)。

这一课分两段做：先用 `Option` 表达“查得到或查不到”，完成练习 5；再学习引用列表和简单排序，完成练习 6。两个实现和第二课演示均已保留。

查找和排序方法位于 `src/model/device.rs`，地址模型位于 `src/model/address.rs`；调用方仍从 `crate::model` 引入 `Device` 和 `Address`。

## 练习 5：按 ID 查找地址

接口已经提供：

```rust
pub fn find_address(&self, id: &str) -> Option<&Address>
```

| 部分 | 含义 |
|---|---|
| `&self` | 只读借用设备，查询不能拿走或修改地址 |
| `id: &str` | 借用要查询的 ID，不要求调用方创建新的 `String` |
| `Option<&Address>` | 找到时返回地址的引用，找不到时没有结果 |

Rust 标准库的 `Option<T>` 有两种情况：`Some(value)` 表示有值，`None` 表示没有值。这里的 `T` 是 `&Address`，所以 `Some` 里面放的是借来的地址引用。

先看一个与地址查询无关的小例子：

```rust
let selection: Option<i32> = Some(7);

match selection {
    Some(number) => println!("选中了 {number}"),
    None => println!("没有选中任何数字"),
}
```

`match` 根据当前是哪种情况执行对应分支。`Some(number)` 同时把里面的值取名为 `number`，供这个分支使用。

回到查询：先用熟悉的 `for` 循环，只读遍历地址列表；找到符合 ID 的那条地址就返回；全部检查后仍没有结果，再返回 `None`。这一题先不要求用迭代器的查找方法。

要返回的是设备里原本保存的地址，不需要复制一个地址，也不能移走地址。返回的引用在使用期间，设备和那条地址都必须仍然有效。

完成时通过的两项验收分别检查：找得到保存的地址，且引用确实指向它；空列表或不存在的 ID 返回 `None`。

## 练习 6：生成候选地址列表

接口已经提供：

```rust
pub fn addresses_by_priority(&self) -> Vec<&Address>
```

这个方法生成一个新的列表，列表里放的是地址引用。设备仍拥有真正的 `Address`，新列表只决定按什么顺序查看它们。

需求是：

1. `priority` 越小，候选顺序越靠前，`0` 也是合法的优先级。
2. 优先级相同时，保持设备中原先保存的先后顺序。
3. 原来的 `self.addresses` 顺序和内容保持不变。
4. 空地址列表返回空列表。

例如，保存顺序是 `A(2)、B(1)、C(1)`，返回的候选顺序应为 `B、C、A`，设备中的保存顺序仍然是 `A、B、C`。

### 引用列表怎么理解

用整数举例：

```rust
let numbers = vec![30, 10, 20];
let view: Vec<&i32> = numbers.iter().collect();
```

`iter()` 依次产生每个元素的只读引用；`collect()` 把这些引用收集成新列表。`view` 拥有的是引用，真正的整数仍属于 `numbers`。

这里借助左侧 `Vec<&i32>` 告诉编译器要收集成什么类型。练习中的元素类型不同，但数据归属的思路相同。先用循环逐个收集引用，也可以。

### 稳定排序是什么

稳定排序会保留相同排序键元素的原有顺序。独立示例：

```rust
let mut words = vec!["pear", "fig", "plum"];
words.sort_by_key(|word| word.len());
// 结果：["fig", "pear", "plum"]
```

`sort_by_key` 根据你提供的键排序，且是稳定排序。`|word| word.len()` 是一个简单闭包：接收当前元素，计算它的排序键。这里的键是长度；`pear`、`plum` 长度相同，所以保持原顺序。

本题需要排序的是新建的引用列表，不是设备保存的地址列表。先准备列表，再确定排序键；不需要现在掌握迭代器和闭包的所有写法。

完成时通过的四项验收检查优先级升序、同优先级稳定顺序、空列表与单地址，以及借用原记录并保持保存顺序。

## 运行演示

前两课共 10 项验收曾全部通过；自动测试文件现已删除。可以继续运行演示复习：

```sh
# 查看第 1 课 A：模型
cargo run -- lesson01

# 查看第 1 课 B：查询与排序
cargo run -- lesson02
```

第二课演示会查找 `address-2` 和不存在的 `missing`，展示排序后的候选地址，再展示原保存列表。候选顺序应为 `address-2、address-3、address-1`；原保存顺序仍是 `address-1、address-2、address-3`。

`main.rs` 提供课程选择，`demo.rs` 提供 `match` 处理与展示。运行仍只操作内存，不会尝试连接 NAS。第 3 课已把展示改为只读方法，新增的更新/删除 TODO 不影响这两个演示。

参考：[Rust Book：Option](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html#the-option-enum-and-its-advantages-over-null-values)、[迭代器](https://doc.rust-lang.org/book/ch13-02-iterators.html)、[稳定的 sort_by_key](https://doc.rust-lang.org/std/primitive.slice.html#method.sort_by_key)。
