# 第 01 课：设备模型、所有权、查询与排序

**状态：功能 ✅ 已完成并验收；完整调用链理解待补验。** 本课包含 A/B 两部分，保留原有讲解与练习要求供复习。

[返回课程目录与 Rust 知识清单](../Course-Syllabus.md) · [第 04 课补课](04-errors-logging-and-toml-storage.md#part-a)

先按[源码地图与前三课补充理解验收](../Code-Reading-Guide.md)阅读本课入口、演示、辅助函数和错误实现，再结合下文复习。已通过的实现不要求重做；理解状态须依据你的解释与小改动单独确认。

## 本课目录

- [A 部分](#part-a)
- [B 部分](#part-b)
- 完整演示：`cargo run --locked -- course01`

<a id="part-a"></a>

## A：设备与地址模型

本课练习结构体 `struct`、关联函数和方法 `impl`、列表 `Vec`，同时巩固所有权和借用。

### 从哪里开始

打开 `src/model/device.rs`，对照 `new`、`try_add_address`、`address_count`、`rename` 的签名和实现复习。地址模型位于 `src/model/address.rs`。第 3 课已移除旧的 `add_address`，添加统一经过校验；下文保留早期练习要求作为历史说明。第 3 课 A 的五个方法也已完成，位于 `device.rs`。

`todo!()` 是合法的 Rust 占位宏：代码能够编译，但执行到这里会停止并显示提示。因此，`cargo check` 成功只表示类型和语法通过，不代表练习已经完成。

先在项目根目录检查骨架能否编译：

```sh
cargo check
```

### 已经提供的部分

- `Address`：保存地址 ID、Host 和优先级。
- `Device`：保存设备 ID、名称和地址列表。
- `demo` 模块：创建示例、调用你的函数、只读展示设备和地址。
- 演示可以用 `cargo run -- lesson01` 运行，对照下文观察输出。

设备拥有地址列表，列表拥有每个 `Address`，地址又拥有各自的字符串。`Vec<Address>` 可以先理解为只保存 `Address` 的动态列表。

本课初版字段公开；第 3 课已把设备字段私有化，改用 `id()`、`name()`、`addresses()` 等只读方法观察，`Address` 仍可公开构造为候选值。

### 最初完成的 4 个任务（历史签名）

| 顺序 | 待实现的函数 | 应该完成的行为 | 重点 |
|---|---|---|---|
| 1 | `Device::new(id, name)` | 返回设备，保存传入的 ID 和名称，初始地址列表为空 | 结构体构造、所有权移动 |
| 2 | `add_address(&mut self, address)` | 把地址追加到列表末尾，保留已有地址及顺序 | 可变借用设备，接收地址所有权 |
| 3 | `address_count(&self)` | 返回当前地址数量，允许反复调用 | 只读借用、返回值 |
| 4 | `rename(&mut self, name)` | 替换名称，保留设备 ID 和所有地址 | 修改字段、替换拥有的数据 |

#### 练习 1 的提示

`impl Device` 为类型定义行为。`new` 没有 `self` 参数，通过 `Device::new(...)` 调用；它是常用命名，不是特殊关键字。返回类型 `Self` 在这里就是 `Device`。

想一想：构造结构体时要填写哪三个字段？两个 `String` 已经作为参数传进来了，能否直接交给字段拥有？空列表可以查看 `Vec::new` 的用法。

Rust 函数可以用最后一个表达式作为返回值；这个表达式末尾不要加分号。

#### 练习 2 的提示

`&mut self` 允许修改设备，但不拿走设备的所有权；`address: Address` 接收一个完整地址的所有权。

查一下 `Vec` 在列表末尾添加一个元素的方法。列表的元素类型是 `Address`，所以应该存入地址本身，还是地址的引用？先保留插入顺序，暂时不要排序。

#### 练习 3 的提示

列表提供查询元素个数的方法，返回类型与签名中的 `usize` 对应。空列表和非空列表都要正确，不能写死一个数量。

只读操作为什么用 `&self` 就够了？如果拿走整个 `self`，连续两次查询会遇到什么问题？

#### 练习 4 的提示

通过 `self` 修改设备的某个字段，新名称来自参数。只替换需要修改的字段，不必重新构造整个设备。

新旧名称都是拥有字符串的 `String`。替换后旧字符串会怎样处理？这里是否真的需要 `clone`？

### 复习已经完成的演示

本课完成时曾通过 4 项自动验收，测试文件现已移除。现在通过演示复习：

```sh
cargo run -- lesson01
```

演示程序创建“我的 NAS”，添加两个地址，展示设备，再将名称改为“家里的 NAS”并再次展示。两次都应保留相同的两条地址。

示例地址只是内存中的字符串，运行不会访问它们。优先级 `2` 的地址先添加，所以先显示；按优先级排序是本课 B 的内容。

### 再回头理解调用方

填写完成后，阅读 `src/app/demo.rs`，试着解释：

1. `Device::new(..., name)` 为什么可以让设备长期保存名称？原来的 `name` 还能继续使用吗？
2. `device.try_add_address(first_address)` 为什么既有借用，也有所有权移动？
3. `show_device(&device)` 中，为什么通过 `device.addresses()` 借用地址切片？
4. `name_ref` 最后一次使用后，为什么可以再调用 `rename`？

最后一个问题可以做编译实验：在 `rename(...)` 后再次打印 `name_ref`，运行 `cargo check`，观察借用冲突。理解后撤销这次实验，恢复可运行状态。

### 加一道自主练习

在演示入口追加第三个地址，运行后地址数量应为 `3`，改名后仍为 `3`。完成后，再进入地址查找与优先级排序。

当前项目一个 Cargo package 同时提供 library crate 与 binary crate。`src/lib.rs` 声明并公开模型、配置、错误和存储；`src/main.rs` 进入应用编排，`src/app.rs` 选择课程并调用 `src/app/demo.rs` 的演示。

库内 `crate::model` 从 library 根查找；应用通过 `nasnet::model` 使用库。这里不需要增加另一个 package 依赖。模块采用同名文件和同名目录并列：`src/model.rs` 声明并导出子模块，设备和地址在其子目录；独立错误体系已移到 `src/error.rs`。不创建 `src/model/mod.rs`。

这部分比初次完成第 01 课时的结构更完整。迁移的原因与每段入口代码的解释见[第 04 课补课](04-errors-logging-and-toml-storage.md#part-a)；已通过的模型方法不要求重写。

## B：地址查找与优先级排序

这一课分两段做：先用 `Option` 表达“查得到或查不到”，完成练习 5；再学习引用列表和简单排序，完成练习 6。两个实现和本课 B 演示均已保留。

查找和排序方法位于 `src/model/device.rs`，地址模型位于 `src/model/address.rs`；调用方仍从 `crate::model` 引入 `Device` 和 `Address`。

### 练习 5：按 ID 查找地址

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

### 练习 6：生成候选地址列表

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

#### 引用列表怎么理解

用整数举例：

```rust
let numbers = vec![30, 10, 20];
let view: Vec<&i32> = numbers.iter().collect();
```

`iter()` 依次产生每个元素的只读引用；`collect()` 把这些引用收集成新列表。`view` 拥有的是引用，真正的整数仍属于 `numbers`。

这里借助左侧 `Vec<&i32>` 告诉编译器要收集成什么类型。练习中的元素类型不同，但数据归属的思路相同。先用循环逐个收集引用，也可以。

#### 稳定排序是什么

稳定排序会保留相同排序键元素的原有顺序。独立示例：

```rust
let mut words = vec!["pear", "fig", "plum"];
words.sort_by_key(|word| word.len());
// 结果：["fig", "pear", "plum"]
```

`sort_by_key` 根据你提供的键排序，且是稳定排序。`|word| word.len()` 是一个简单闭包：接收当前元素，计算它的排序键。这里的键是长度；`pear`、`plum` 长度相同，所以保持原顺序。

本题需要排序的是新建的引用列表，不是设备保存的地址列表。先准备列表，再确定排序键；不需要现在掌握迭代器和闭包的所有写法。

完成时通过的四项验收检查优先级升序、同优先级稳定顺序、空列表与单地址，以及借用原记录并保持保存顺序。

### 运行演示

前两课共 10 项验收曾全部通过；自动测试文件现已删除。可以继续运行演示复习：

```sh
# 查看第 1 课 A：模型
cargo run -- lesson01

# 查看第 1 课 B：查询与排序
cargo run -- lesson02
```

本课 B 演示会查找 `address-2` 和不存在的 `missing`，展示排序后的候选地址，再展示原保存列表。候选顺序应为 `address-2、address-3、address-1`；原保存顺序仍是 `address-1、address-2、address-3`。

`app.rs` 提供课程选择，`app/demo.rs` 提供 `match` 处理与展示。运行仍只操作内存，不会尝试连接 NAS。第 3 课已把展示改为只读方法，更新/删除已验收，不影响这两个演示。

参考：[Rust Book：Option](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html#the-option-enum-and-its-advantages-over-null-values)、[迭代器](https://doc.rust-lang.org/book/ch13-02-iterators.html)、[稳定的 sort_by_key](https://doc.rust-lang.org/std/primitive.slice.html#method.sort_by_key)。
