# 第 01 课：设备模型、所有权、查询与排序

用结构体表示一台 NAS 设备和它的地址列表，学习 Rust 的所有权、借用、`Option` 和迭代器。

**状态**：已完成。

[课程目录](../Course-Syllabus.md) · [下一课](02-validation-and-protocols.md)

## 一、新知识

每一节先看官方文档里的示例，再看项目里怎样用。

### 1. 结构体与方法

官方示例（[The Rust Book 5.3](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)）：

```rust
#[derive(Debug)]
struct Rectangle {
    width: u32,
    height: u32,
}

impl Rectangle {
    fn area(&self) -> u32 {
        self.width * self.height
    }

    fn can_hold(&self, other: &Rectangle) -> bool {
        self.width > other.width && self.height > other.height
    }

    fn square(size: u32) -> Self {
        Self {
            width: size,
            height: size,
        }
    }
}

let rect = Rectangle { width: 30, height: 50 };
let square = Rectangle::square(10);
assert_eq!(rect.area(), 1500);
assert!(rect.can_hold(&square));
```

- `struct` 定义数据，`impl` 块定义行为。
- 方法的第一个参数是 `self` 的某种形式：`&self` 只读借用，`&mut self` 可变借用，`self` 取得所有权（调用后原值不能再用）。调用 `rect.area()` 时编译器自动借用 `rect`。
- 没有 `self` 参数的是关联函数，用 `类型::函数()` 调用，常用来构造值，例如 `String::from`、`Rectangle::square`。
- `Self` 指当前类型；字段名和变量名相同时可以简写成 `Self { id, name }`。

**在项目中**：[`Device`](../../src/model/device.rs) 用同样的结构组织。

```rust
impl Device {
    pub fn new(id: String, name: String) -> Self {           // 关联函数
        Self { id, name, addresses: Vec::new(), connections: Vec::new() }
    }

    pub fn name(&self) -> &str {                             // 只读
        &self.name
    }

    pub fn rename(&mut self, name: String) {                 // 修改
        self.name = name;
    }
}
```

### 2. 所有权与移动

官方示例（[The Rust Book 4.1](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)）：

```rust
let s1 = String::from("hello");
let s2 = s1;
println!("{s1}, world!");   // 错误 E0382：s1 的值已经移动到 s2
```

```rust
fn main() {
    let s = String::from("hello");
    takes_ownership(s);         // s 的值移入函数，之后 s 不再有效

    let x = 5;
    makes_copy(x);              // i32 是 Copy 类型，传的是副本，x 之后仍可使用
}

fn takes_ownership(some_string: String) {
    println!("{some_string}");
}                               // some_string 离开作用域，内存被释放

fn makes_copy(some_integer: i32) {
    println!("{some_integer}");
}
```

- 所有权三条规则：每个值都有一个所有者；同一时刻只有一个所有者；所有者离开作用域时，值被丢弃（drop）。
- `String` 把文本放在堆上。赋值或按值传参会**移动**所有权，原变量随即失效；整数、`bool` 这类 `Copy` 类型则是按位复制。
- 需要两份独立的数据时显式调用 `clone()`。从 `&str` 得到 `String`：`String::from`、`to_owned()` 或 `into()`。
- 对照 C++：相当于默认 move，而且编译器禁止再使用被移走的对象；对照 Python：不是多个名字共享同一个对象。

**在项目中**：`Device::new(id: String, name: String)` 按值接收，两个字符串移入设备，从此由设备拥有。调用方之后如果还要使用名称，需要在调用前 `clone()`。

### 3. 引用与借用

官方示例（[The Rust Book 4.2](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)）：

```rust
fn main() {
    let s1 = String::from("hello");
    let len = calculate_length(&s1);    // 借用，不移动
    println!("The length of '{s1}' is {len}.");

    let mut s = String::from("hello");
    change(&mut s);                     // 可变借用
}

fn calculate_length(s: &String) -> usize {
    s.len()
}

fn change(some_string: &mut String) {
    some_string.push_str(", world");
}
```

借用持续到最后一次使用：

```rust
let mut s = String::from("hello");
let r1 = &s;
let r2 = &s;
println!("{r1} and {r2}");
// r1、r2 之后不再使用，只读借用到此结束
let r3 = &mut s;
println!("{r3}");
```

- 同一时刻可以有任意多个 `&T`，**或者**只有一个 `&mut T`，两者不能同时存在。这条规则在编译期排除了数据竞争和悬垂引用。
- 借用从创建持续到最后一次使用，而不是到作用域结束（非词法生命周期）。上面 `r3` 能创建，正是因为 `r1`、`r2` 已经用完。
- 参数写 `&str` 比写 `&String` 更通用：`&String` 会自动转换成 `&str`，字符串字面量本身就是 `&str`。

**在项目中**：`device.name()` 返回借自设备的 `&str`。只要这个引用之后还会被使用，就不能调用需要 `&mut self` 的 `rename`。

### 4. Vec 与切片

官方示例（[The Rust Book 8.1](https://doc.rust-lang.org/book/ch08-01-vectors.html)）：

```rust
let mut v = Vec::new();
v.push(5);
v.push(6);

let v = vec![1, 2, 3, 4, 5];
let third: &i32 = &v[2];             // 越界会 panic
let third: Option<&i32> = v.get(2);  // 越界返回 None
```

持有元素的引用时不能修改 Vec：

```rust
let mut v = vec![1, 2, 3, 4, 5];
let first = &v[0];
v.push(6);                           // 错误 E0502：v 已被不可变借用
println!("The first element is: {first}");
```

- `Vec<T>` 拥有自己的元素，按插入顺序保存。`push` 可能重新分配内存、让旧引用失效，所以借用规则禁止“一边引用元素一边修改 Vec”。
- `&[T]` 是切片：借来的一段连续元素。`&Vec<T>` 会自动转换成 `&[T]`。
- 遍历：`for item in &v`（借用元素）、`for item in &mut v`（可变借用元素）、`for item in v`（消耗整个 Vec）。

**在项目中**：`Device` 用 `Vec<Address>` 保存地址，`addresses(&self) -> &[Address]` 只给出只读视图，外部不能绕过校验直接 push。

### 5. Option

官方示例（[std::option](https://doc.rust-lang.org/std/option/index.html)）：

```rust
fn divide(numerator: f64, denominator: f64) -> Option<f64> {
    if denominator == 0.0 {
        None
    } else {
        Some(numerator / denominator)
    }
}

let result = divide(2.0, 3.0);
match result {
    Some(x) => println!("Result: {x}"),
    None => println!("Cannot divide by 0"),
}
```

- `Option<T>` 只有 `Some(T)` 和 `None` 两种值。Rust 没有 null，“可能没有”必须写进类型，调用方不处理 `None` 就拿不到里面的值。
- 常用写法：`match`、`if let Some(x) = ...`、`is_some()`/`is_none()`、`map`、`unwrap_or`。`unwrap()` 遇到 `None` 会 panic，只在确定有值时使用。
- “没找到”是正常结果，应该返回 `None`，而不是 panic。

**在项目中**：`find_address(&self, id: &str) -> Option<&Address>`，找到时借出列表里的元素，找不到返回 `None`。

### 6. 迭代器与稳定排序

官方示例（[The Rust Book 13.2](https://doc.rust-lang.org/book/ch13-02-iterators.html)、[Iterator::find](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.find)、[slice::sort_by_key](https://doc.rust-lang.org/std/primitive.slice.html#method.sort_by_key)）：

```rust
let v1 = vec![1, 2, 3];
let v2: Vec<_> = v1.iter().map(|x| x + 1).collect();
assert_eq!(v2, vec![2, 3, 4]);

let a = [1, 2, 3];
assert_eq!(a.iter().find(|&&x| x == 2), Some(&2));
assert_eq!(a.iter().find(|&&x| x == 5), None);

let mut v = [4i32, -5, 1, -3, 2];
v.sort_by_key(|k| k.abs());
assert_eq!(v, [1, 2, -3, 4, -5]);
```

稳定排序的含义（自写示例）：

```rust
let mut tasks = [("备份", 2), ("挂载", 1), ("清理", 2)];
tasks.sort_by_key(|task| task.1);
assert_eq!(tasks, [("挂载", 1), ("备份", 2), ("清理", 2)]);   // 同为 2，保持原来的先后
```

- `iter()` 产生 `&T`，`iter_mut()` 产生 `&mut T`，`into_iter()` 取得元素本身。
- 迭代器是惰性的：`map`、`filter` 只描述要做什么，`collect`、`find`、`for` 才真正执行。`collect` 收集成什么类型，由变量标注决定。
- `find` 的闭包参数是“迭代器元素的引用”。`iter()` 的元素已经是 `&T`，所以参数是 `&&T`，官方示例因此写 `|&&x|`。
- `sort_by_key` 是稳定排序：键相等的元素保持原来的相对顺序；`sort_unstable_by_key` 更快，但不保证这一点。

**在项目中**：`addresses_by_priority` 先把 `self.addresses.iter()` 收集成 `Vec<&Address>`，再按 `priority` 稳定排序。排序的只是引用，设备里的保存顺序不变；优先级相同的地址按保存顺序排列。

## 二、示例练习：examples/course01.rs

```sh
cargo run --locked --example course01
```

用一个待办清单练习上面的知识点。按提示改动代码，先预测结果再运行，看完后恢复：

- **E1.1 构造与移动**：取消 main 中 `// println!("{name}")` 的注释，读编译器对 E0382 的说明。
- **E1.2 只读借用与可变借用**：把打印 `current` 的语句移到 `rename` 之后，预测并观察编译错误，解释原来的顺序为什么能通过。
- **E1.3 Vec 拥有元素**：`tasks()` 返回 `&[Task]`，说明调用方为什么无法通过它追加任务。
- **E1.4 查找返回 Option**：对比 `find_by_loop` 和 `find`。删掉 `find_by_loop` 上的 `#[allow(clippy::manual_find)]`，运行 `cargo clippy --example course01` 看它的建议。
- **E1.5 稳定排序**：说明同优先级的 backup 为什么排在 clean 前面，原列表的顺序为什么不变。

## 三、项目练习：src/model/

阅读 [device.rs](../../src/model/device.rs) 和 [address.rs](../../src/model/address.rs)。地址通过 `try_add_address` 加入设备，校验规则在第 02 课。

- **P1.1 构造与只读访问**：`Device::new` 创建空的地址和连接列表；`id()`、`name()` 返回 `&str`，`addresses()` 返回 `&[Address]`，`address_count()` 返回 `usize`。测试：`new_device_owns_its_name_and_starts_empty`。
- **P1.2 改名**：`rename(&mut self, name: String)` 只替换名称，保留 ID 和地址。测试：`renaming_preserves_identity_and_saved_addresses`。
- **P1.3 按 ID 查找地址**：`find_address` 返回列表元素的引用，不复制 `Address`。测试 `address_lookup_borrows_saved_values_and_reports_absence` 用 `std::ptr::eq` 证明返回的引用指向设备内部的那个元素。
- **P1.4 按优先级生成候选顺序**：`addresses_by_priority` 中 priority 越小越靠前，同优先级保持保存顺序，不修改原列表。测试 `priority_sort_is_stable_without_changing_saved_order` 同时断言候选顺序和保存顺序。

## 四、验收

```sh
cargo run --locked --example course01
cargo test --locked --test course01_model
```

能回答：

- `Device::new` 之后，调用方为什么不能再使用传入的 name？
- `find_address` 返回的引用借自谁？在什么情况下不能修改设备？
- `Vec<Address>` 和 `Vec<&Address>` 各自拥有什么？
- 为什么只断言候选顺序，不足以证明原列表没有被改动？
