# 第 1 课 A：设备与地址模型

本小节的 4 个方法已经完成并通过验收，以下保留说明供复习。课程已合并为 12 课，本文件是第 1 课 A；演示命令仍为 `lesson01`。验收测试已移除。当前练习见[第 3 课 A：封装与生命周期](05-encapsulation-and-lifetimes.md)，本课后半见[B：查询与排序](02-address-lookup-and-priority.md)。

本课练习结构体 `struct`、关联函数和方法 `impl`、列表 `Vec`，同时巩固所有权和借用。

## 从哪里开始

打开 `src/model/device.rs`，对照 `new`、`try_add_address`、`address_count`、`rename` 的签名和实现复习。地址模型位于 `src/model/address.rs`。第 3 课已移除旧的 `add_address`，添加统一经过校验；下文保留早期练习要求作为历史说明。第 3 课 A 的五个方法也已完成，位于 `device.rs`。

`todo!()` 是合法的 Rust 占位宏：代码能够编译，但执行到这里会停止并显示提示。因此，`cargo check` 成功只表示类型和语法通过，不代表练习已经完成。

先在项目根目录检查骨架能否编译：

```sh
cargo check
```

## 已经提供的部分

- `Address`：保存地址 ID、Host 和优先级。
- `Device`：保存设备 ID、名称和地址列表。
- `demo` 模块：创建示例、调用你的函数、只读展示设备和地址。
- 演示可以用 `cargo run -- lesson01` 运行，对照下文观察输出。

设备拥有地址列表，列表拥有每个 `Address`，地址又拥有各自的字符串。`Vec<Address>` 可以先理解为只保存 `Address` 的动态列表。

本课初版字段公开；第 3 课已把设备字段私有化，改用 `id()`、`name()`、`addresses()` 等只读方法观察，`Address` 仍可公开构造为候选值。

## 最初完成的 4 个任务（历史签名）

| 顺序 | 待实现的函数 | 应该完成的行为 | 重点 |
|---|---|---|---|
| 1 | `Device::new(id, name)` | 返回设备，保存传入的 ID 和名称，初始地址列表为空 | 结构体构造、所有权移动 |
| 2 | `add_address(&mut self, address)` | 把地址追加到列表末尾，保留已有地址及顺序 | 可变借用设备，接收地址所有权 |
| 3 | `address_count(&self)` | 返回当前地址数量，允许反复调用 | 只读借用、返回值 |
| 4 | `rename(&mut self, name)` | 替换名称，保留设备 ID 和所有地址 | 修改字段、替换拥有的数据 |

### 练习 1 的提示

`impl Device` 为类型定义行为。`new` 没有 `self` 参数，通过 `Device::new(...)` 调用；它是常用命名，不是特殊关键字。返回类型 `Self` 在这里就是 `Device`。

想一想：构造结构体时要填写哪三个字段？两个 `String` 已经作为参数传进来了，能否直接交给字段拥有？空列表可以查看 `Vec::new` 的用法。

Rust 函数可以用最后一个表达式作为返回值；这个表达式末尾不要加分号。

### 练习 2 的提示

`&mut self` 允许修改设备，但不拿走设备的所有权；`address: Address` 接收一个完整地址的所有权。

查一下 `Vec` 在列表末尾添加一个元素的方法。列表的元素类型是 `Address`，所以应该存入地址本身，还是地址的引用？先保留插入顺序，暂时不要排序。

### 练习 3 的提示

列表提供查询元素个数的方法，返回类型与签名中的 `usize` 对应。空列表和非空列表都要正确，不能写死一个数量。

只读操作为什么用 `&self` 就够了？如果拿走整个 `self`，连续两次查询会遇到什么问题？

### 练习 4 的提示

通过 `self` 修改设备的某个字段，新名称来自参数。只替换需要修改的字段，不必重新构造整个设备。

新旧名称都是拥有字符串的 `String`。替换后旧字符串会怎样处理？这里是否真的需要 `clone`？

## 复习已经完成的演示

本课完成时曾通过 4 项自动验收，测试文件现已移除。现在通过演示复习：

```sh
cargo run -- lesson01
```

演示程序创建“我的 NAS”，添加两个地址，展示设备，再将名称改为“家里的 NAS”并再次展示。两次都应保留相同的两条地址。

示例地址只是内存中的字符串，运行不会访问它们。优先级 `2` 的地址先添加，所以先显示；按优先级排序是下一课的内容。

## 再回头理解调用方

填写完成后，阅读 `src/demo.rs`，试着解释：

1. `Device::new(..., name)` 为什么可以让设备长期保存名称？原来的 `name` 还能继续使用吗？
2. `device.try_add_address(first_address)` 为什么既有借用，也有所有权移动？
3. `show_device(&device)` 中，为什么通过 `device.addresses()` 借用地址切片？
4. `name_ref` 最后一次使用后，为什么可以再调用 `rename`？

最后一个问题可以做编译实验：在 `rename(...)` 后再次打印 `name_ref`，运行 `cargo check`，观察借用冲突。理解后撤销这次实验，恢复可运行状态。

## 加一道自主练习

在演示入口追加第三个地址，运行后地址数量应为 `3`，改名后仍为 `3`。完成后，再进入地址查找与优先级排序。

当前项目只有一个包、一个二进制 crate。`src/main.rs` 是 crate 的根文件，通过 `mod model;` 和 `mod demo;` 声明两个模块，再调用 `demo::run()`。

`demo.rs` 从 `crate::model` 引入模型，`crate::` 表示从当前 crate 的根模块开始查找。同一个 crate 内使用模块，不需要添加包依赖。

这里采用 Rust 2018 风格的 non-mod-rs 布局：`src/model.rs` 是模块入口，只声明子模块并统一导出；设备和地址分别位于 `src/model/device.rs`、`src/model/address.rs`，连接及其协议配置位于 `src/model/connection.rs` 和 `src/model/connection/`，错误类型位于 `src/model/error.rs`。入口文件与同名子目录并列，不创建 `src/model/mod.rs`；模块路径仍是 `crate::model`。参见 [Rust 2018 模块路径说明](https://doc.rust-lang.org/edition-guide/rust-2018/path-changes.html)。

其他模块仍通过 `crate::model` 导出的公开类型和方法使用模型。`Device` 定义在 `model::device` 子模块中，字段保持私有，列表修改通过它提供的方法完成；子模块的公开接口由父模块通过 `pub use` 导出。

参考：[Rust Book：结构体与方法](https://doc.rust-lang.org/book/ch05-00-structs.html)、[Vec 与借用遍历](https://doc.rust-lang.org/book/ch08-01-vectors.html)、[模块文件布局](https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html)、[测试组织与私有函数](https://doc.rust-lang.org/book/ch11-03-test-organization.html)。
