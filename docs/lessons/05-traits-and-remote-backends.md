# 第 05 课：用 trait 统一远程访问

NASNet 要同时支持 NFS 和 SMB。两种协议的连接参数、认证方式和底层 C 库都不一样，上层代码却只关心三件事：连上设备的某个地址，读一个目录，失败时知道该不该换地址。本课用 trait 把这三件事写成统一的接口，再用内存里的模拟后端离线验证地址回退、协议分派和目录整理。第 07 课接入 libnfs、libsmb2 时，只需让真实后端实现同样的 trait，本课写好的调用链和测试都不用改。

**状态**：进行中。接口、数据类型和错误类型已提供，P5.1–P5.5 由你实现。

[课程目录](../Course-Syllabus.md) · [上一课](01-04-foundations.md) · [下一课](06-unsafe-and-c-ffi.md) · [源码地图](../Code-Reading-Guide.md)

## 一、新知识

本课的代码都在 [src/remote.rs](../../src/remote.rs)，一共不到 200 行。先通读一遍，再按下表的顺序学习。每一节先看官方教程里的例子，再回到这个文件看项目怎样用它。

| 知识 | 解决的问题 | 在项目中 |
|---|---|---|
| 1. trait | 不同协议怎样提供"同一种能力" | `RemoteBackend`、`RemoteSession`，`impl Error for RemoteError {}` |
| 2. 泛型与 trait bound | 一个函数怎样接收"任何一种后端" | `connect_by_priority<B: ...>`、`Backends<N, S>` |
| 3. trait 对象 | 要到运行时才知道类型时怎么办 | `Box<dyn RemoteSession>`、`&dyn RemoteBackend` |
| 4. Sized、`?Sized` 与 dyn 兼容 | 怎样让同一个函数既能用于泛型，也能用于 trait 对象 | `?Sized`、`Box<dyn …>` 默认的 `'static` |
| 5. RefCell 与模拟对象 | 测试怎样替换真实网络 | 测试文件里的 `MemoryBackend` |
| 6. 按模式判断与断言 | 怎样按枚举变体分类、断言 | `allows_next_address`、测试中的错误断言 |
| 7. 过滤与排序 | 怎样把目录整理成稳定的顺序 | `list_directory_sorted` |

### 1. trait：定义共同行为

trait 把一组方法签名放在一起，表示"实现了它的类型都能做这些事"，类似其他语言里的接口（interface）。

官方示例（[The Rust Book 10.2](https://doc.rust-lang.org/book/ch10-02-traits.html)）：一个新闻聚合库要给新闻和社交帖子生成摘要。

```rust
pub trait Summary {
    fn summarize(&self) -> String;       // 只有签名，以分号结尾：实现者必须提供方法体
}

pub struct NewsArticle {
    pub headline: String,
    pub location: String,
    pub author: String,
    pub content: String,
}

impl Summary for NewsArticle {           // impl trait 名 for 类型名
    fn summarize(&self) -> String {
        format!("{}, by {} ({})", self.headline, self.author, self.location)
    }
}

pub struct SocialPost {
    pub username: String,
    pub content: String,
    pub reply: bool,
    pub repost: bool,
}

impl Summary for SocialPost {
    fn summarize(&self) -> String {
        format!("{}: {}", self.username, self.content)
    }
}
```

使用方要把 trait 和类型一起引入作用域，`use aggregator::{SocialPost, Summary};`，然后像普通方法一样调用 `post.summarize()`，得到 `horse_ebooks: of course, as you probably already know, people`。

trait 还可以给方法写默认实现（[10.2：默认实现](https://doc.rust-lang.org/book/ch10-02-traits.html#using-default-implementations)）。默认实现可以调用同一个 trait 里还没有实现的方法：

```rust
pub trait Summary {
    fn summarize_author(&self) -> String;          // 必须实现

    fn summarize(&self) -> String {                // 有默认实现，实现者可以覆盖
        format!("(Read more from {}...)", self.summarize_author())
    }
}

impl Summary for SocialPost {
    fn summarize_author(&self) -> String {         // 只实现必须的那一个
        format!("@{}", self.username)
    }
}

// post.summarize() → "(Read more from @horse_ebooks...)"
```

- 方法签名就是调用约定。编译器检查每个 `impl` 是否提供了签名完全一致的方法，漏掉一个没有默认实现的方法就编译不通过。
- 第一个参数叫接收者，决定调用时怎样借用实现者：`&self` 是共享借用，多处可以同时使用；`&mut self` 是独占借用，调用期间别处不能使用；`self` 会拿走所有权。
- 空的 `impl Summary for NewsArticle {}` 表示所有方法都用默认实现。
- **孤儿规则**：trait 和类型至少有一个定义在当前 crate，才能写这个 `impl`。可以给自己的类型实现标准库的 `Display`，也可以给 `Vec<T>` 实现自己的 `Summary`，但不能给 `Vec<T>` 实现 `Display`。这条规则保证两个 crate 不会为同一个类型实现同一个 trait。

**在项目中**：remote.rs 定义了两个 trait，把"建立连接"和"读目录"交给两种对象：

```rust
/// 可复用的协议后端。
pub trait RemoteBackend {
    /// 对单个地址尝试建立会话，地址回退由路由层负责。
    fn connect(
        &self,
        host: &str,
        connection: &ProtocolConnection,
    ) -> Result<Box<dyn RemoteSession>, RemoteError>;
}

/// 已建立的会话。每次操作独占会话状态。
pub trait RemoteSession {
    fn list_directory(&mut self, path: &str) -> Result<Vec<DirectoryEntry>, RemoteError>;
}
```

| | `RemoteBackend` | `RemoteSession` |
|---|---|---|
| 代表 | 一种协议的客户端（NFS 或 SMB） | 连上某个地址后得到的一条连接 |
| 数量 | 每种协议一个，反复使用 | 每次连接成功产生一个 |
| 接收者 | `&self`：尝试连接不改变后端本身，同一个后端可以连很多次 | `&mut self`：真实会话里有原生连接句柄和读取状态，同一时刻只能做一个操作 |
| 谁来实现 | 第 07 课的真实后端；本课测试里的 `MemoryBackend` | 第 07 课的真实会话；本课测试里的 `MemorySession` |

拆成两个 trait 后，负责地址回退的 `connect_by_priority` 只用到 `RemoteBackend`，负责整理目录的 `list_directory_sorted` 只用到 `RemoteSession`，两者都不需要知道具体是哪种协议。

默认实现在项目里也已经用到了。标准库 `std::error::Error` 的方法都有默认实现（`source()` 默认返回 `None`），只是它要求类型先实现 `Debug` 和 `Display`：

```rust
impl std::error::Error for RemoteError {}        // 全部用默认实现：RemoteError 没有下层错误

impl std::error::Error for ConnectError {        // 覆盖 source：Rejected 里带着下层的 RemoteError
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Rejected { source, .. } => Some(source),
            _ => None,
        }
    }
}
```

孤儿规则决定了模拟后端写在哪里。`RemoteBackend` 属于 nasnet 库，`MemoryBackend` 定义在测试文件里，类型属于测试 crate，所以测试文件里可以写 `impl RemoteBackend for MemoryBackend`，库本身不需要知道有这个类型。

### 2. 泛型与 trait bound：接收任何一种实现

官方示例一（[The Rust Book 10.2：trait 作为参数](https://doc.rust-lang.org/book/ch10-02-traits.html#using-traits-as-parameters)）：

```rust
pub fn notify(item: &impl Summary) {               // 接收任何实现了 Summary 的类型
    println!("Breaking news! {}", item.summarize());
}
```

`impl Trait` 是下面这种完整写法（[trait bound](https://doc.rust-lang.org/book/ch10-02-traits.html#trait-bound-syntax)）的简写，两种写法二选一：

```rust
pub fn notify<T: Summary>(item: &T) {              // T 是类型参数，": Summary" 是约束
    println!("Breaking news! {}", item.summarize());
}
```

完整写法能表达更多约束（下面几个签名是各自独立的例子）：

```rust
pub fn notify(item1: &impl Summary, item2: &impl Summary) {}  // 两个参数可以是不同类型
pub fn notify<T: Summary>(item1: &T, item2: &T) {}            // 两个参数必须是同一类型
pub fn notify(item: &(impl Summary + Display)) {}             // + 要求同时实现多个 trait

fn some_function<T, U>(t: &T, u: &U) -> i32                   // 约束多时移到 where 子句
where
    T: Display + Clone,
    U: Clone + Debug,
{
    unimplemented!()
}
```

官方示例二（[10.1：结构体中的泛型](https://doc.rust-lang.org/book/ch10-01-syntax.html#in-struct-definitions)、[10.2：用 trait bound 有条件地实现方法](https://doc.rust-lang.org/book/ch10-02-traits.html#using-trait-bounds-to-conditionally-implement-methods)）：

```rust
struct Point<T, U> {                     // 两个类型参数，x 和 y 可以是不同类型
    x: T,
    y: U,
}

fn main() {
    let both_integer = Point { x: 5, y: 10 };
    let both_float = Point { x: 1.0, y: 4.0 };
    let integer_and_float = Point { x: 5, y: 4.0 };
}
```

```rust
use std::fmt::Display;

struct Pair<T> {
    x: T,
    y: T,
}

impl<T> Pair<T> {                        // 任何 T 都有 new
    fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T: Display + PartialOrd> Pair<T> {  // 只有 T 能比较、能打印时，才有 cmp_display
    fn cmp_display(&self) {
        if self.x >= self.y {
            println!("The largest member is x = {}", self.x);
        } else {
            println!("The largest member is y = {}", self.y);
        }
    }
}
```

**单态化**（[10.1：泛型代码的性能](https://doc.rust-lang.org/book/ch10-01-syntax.html#performance-of-code-using-generics)）：编译器找出泛型代码被哪些具体类型使用，为每个类型各生成一份代码。例如 `Some(5)` 和 `Some(5.0)` 会展开成类似 `Option_i32`、`Option_f64` 的两份定义。所以泛型没有运行时开销，调用哪个方法在编译时就确定了，这叫**静态分派**。

- 类型参数不是"任意类型"，而是"调用时确定下来的某一个类型"。`notify<T: Summary>(item1: &T, item2: &T)` 传入 `&NewsArticle` 和 `&SocialPost` 会报 E0308（类型不匹配）。
- trait bound 既是承诺也是限制：函数体里只能调用 bound 中 trait 的方法，传入没实现这个 trait 的类型（如 `String`）就编译不通过。
- 简单参数用 `impl Trait`。需要多个参数是同一类型，或者要在别处提到这个类型时，用具名的类型参数 `<T: Trait>`。

**在项目中**：

`connect_by_priority` 不关心后端是 NFS、SMB 还是测试用的内存后端，只要求它实现了 `RemoteBackend`：

```rust
pub fn connect_by_priority<B: RemoteBackend + ?Sized>(   // ?Sized 见知识 4
    backend: &B,
    device: &Device,
    connection: &ProtocolConnection,
) -> Result<Connected, ConnectError>
```

因为有 `B: RemoteBackend` 这个约束，函数体里可以调用 `backend.connect(host, connection)`。测试传入 `&MemoryBackend`，编译器生成一份 `B = MemoryBackend` 的代码；第 07 课传入真实后端时，再为它生成一份。

`Backends` 和 `Point<T, U>` 一样有两个类型参数，NFS 后端和 SMB 后端可以是不同的类型。它的 `impl` 和 `Pair` 一样带约束：两个字段都是后端时，才有 `connect` 方法。

```rust
pub struct Backends<N, S> {
    pub nfs: N,
    pub smb: S,
}

impl<N: RemoteBackend, S: RemoteBackend> Backends<N, S> {
    /// 只使用连接所属协议的后端。
    pub fn connect(&self, device: &Device, connection_id: &str) -> Result<Connected, ConnectError> {
        // P5.4
    }
}
```

测试中两个字段都放模拟后端，`N` 和 `S` 都是 `MemoryBackend`。第 07 课两个字段分别放真实的 NFS 和 SMB 后端，`Backends` 的代码不用改。

### 3. trait 对象：运行时才确定类型

泛型的类型参数在编译时就固定成一个类型。有时要到运行时才知道是哪种类型，或者要把不同类型的值放在一起，这时用 trait 对象：`&dyn Trait`、`&mut dyn Trait`、`Box<dyn Trait>`。

官方示例（[The Rust Book 18.2](https://doc.rust-lang.org/book/ch18-02-trait-objects.html)）：GUI 库的 `Screen` 要绘制各种组件，写库时不知道用户还会加入哪些组件。

```rust
pub trait Draw {
    fn draw(&self);
}

pub struct Screen {
    pub components: Vec<Box<dyn Draw>>,  // 每个元素可以是不同的类型，只要实现了 Draw
}

impl Screen {
    pub fn run(&self) {
        for component in self.components.iter() {
            component.draw();            // 运行时查表，调用这个元素自己的 draw
        }
    }
}

pub struct Button {
    pub width: u32,
    pub height: u32,
    pub label: String,
}

impl Draw for Button {
    fn draw(&self) {
        // code to actually draw a button
    }
}
```

库的使用者在自己的 crate 里定义新组件，和 `Button` 放进同一个 `Vec`：

```rust
use gui::Draw;

struct SelectBox {
    width: u32,
    height: u32,
    options: Vec<String>,
}

impl Draw for SelectBox {
    fn draw(&self) {
        // code to actually draw a select box
    }
}

use gui::{Button, Screen};

fn main() {
    let screen = Screen {
        components: vec![
            Box::new(SelectBox {
                width: 75,
                height: 10,
                options: vec![
                    String::from("Yes"),
                    String::from("Maybe"),
                    String::from("No"),
                ],
            }),
            Box::new(Button {
                width: 50,
                height: 10,
                label: String::from("OK"),
            }),
        ],
    };

    screen.run();
}
```

放进一个没有实现 `Draw` 的类型，比如 `Box::new(String::from("Hi"))`，编译时就报错，不用等到运行：

```text
error[E0277]: the trait bound `String: Draw` is not satisfied
   = note: required for the cast from `Box<String>` to `Box<dyn Draw>`
```

对比用泛型写的 `Screen`，它的所有组件只能是同一种类型：

```rust
pub struct Screen<T: Draw> {
    pub components: Vec<T>,              // 全是 Button，或者全是 SelectBox
}
```

第 10.2 节还有一个反例：返回类型写成 `-> impl Summary` 时，函数只能返回一种类型，在 `if` 分支里返回 `NewsArticle`、在 `else` 分支里返回 `SocialPost` 编译不通过（[10.2：返回实现了 trait 的类型](https://doc.rust-lang.org/book/ch10-02-traits.html#returning-types-that-implement-traits)）。改成 trait 对象就可以：

```rust
fn returns_summarizable(switch: bool) -> Box<dyn Summary> {
    if switch {
        Box::new(NewsArticle { /* … */ })
    } else {
        Box::new(SocialPost { /* … */ })
    }
}
```

- `dyn Draw` 读作"某个实现了 Draw 的类型"。指向 trait 对象的指针由两部分组成：数据的地址，以及该类型的方法表（vtable）的地址（[Rust Reference：trait 对象](https://doc.rust-lang.org/reference/types/trait-object.html)）。`component.draw()` 在运行时从方法表里找到函数再调用，这叫**动态分派**（[18.2：动态分派](https://doc.rust-lang.org/book/ch18-02-trait-objects.html#performing-dynamic-dispatch)）。
- 动态分派多一次间接调用，编译器也无法内联。换来的是一个变量、一个容器可以装不同的类型，而且写库时不必知道所有类型。
- 具体类型的引用或 `Box` 会自动转换成 trait 对象（这叫 coercion）：`Box::new(Button { .. })` 放进 `Vec<Box<dyn Draw>>`、`&button` 赋给 `&dyn Draw` 类型的变量，都不用手写转换。
- `if`/`match` 的各个分支必须是同一类型。两个分支分别得到 `&Button` 和 `&SelectBox` 时，直接写会报 E0308 `` `if` and `else` have incompatible types ``。给变量标上 trait 对象类型，两个分支就都会自动转换：

  ```rust
  let component: &dyn Draw = if use_button { &button } else { &select_box };
  component.draw();
  ```

- 怎样选：类型在编译时就确定、而且只有一种时用泛型；要混合多种类型，或者运行时才决定用哪个时，用 trait 对象。

**在项目中**：remote.rs 有三处用到 trait 对象，理由各不相同。

1. **返回值 `Box<dyn RemoteSession>`**。NFS 后端返回 NFS 会话，SMB 后端返回 SMB 会话，测试后端返回内存会话，具体类型各不相同，和上面的 `returns_summarizable` 一样。trait 里的方法签名只能写一个返回类型，`Box<dyn RemoteSession>` 能装下任何一种会话。`Box` 让会话拥有自己的数据，调用者可以把它存起来长期使用。

2. **字段 `Connected::session`**。连接结果要同时记下用了哪个地址和得到的会话。它和 `Screen::components` 一样，字段里放的是 trait 对象：

   ```rust
   pub struct Connected {
       pub address_id: String,
       pub session: Box<dyn RemoteSession>,
   }
   ```

   `dyn RemoteSession` 没有实现 `Debug`，所以 `Connected` 不能 `#[derive(Debug)]`。项目手写了 `Debug`，只打印 `address_id`，[`finish_non_exhaustive()`](https://doc.rust-lang.org/std/fmt/struct.DebugStruct.html#method.finish_non_exhaustive) 在末尾加上 `..`，表示还有字段没显示：`Connected { address_id: "lan", .. }`。有了 `Debug`，测试才能对 `Result<Connected, ConnectError>` 调用 `expect_err`、`assert_matches!`，因为这两个工具失败时要打印值。

3. **参数 `&mut dyn RemoteSession` 和局部变量 `&dyn RemoteBackend`**。`list_directory_sorted(session: &mut dyn RemoteSession, path)` 只编译一份，任何会话都能传进来。P5.4 要到运行时读了连接配置才知道用哪个后端，`match` 的两个分支分别得到 `&N` 和 `&S`，两者类型不同。用上面 `component: &dyn Draw` 的写法，把结果统一成 `&dyn RemoteBackend`，再交给 P5.3 的 `connect_by_priority`。

`Connected` 里存的是 `Box<dyn RemoteSession>`，而 `list_directory_sorted` 要的是 `&mut dyn RemoteSession`。这里有个容易踩的坑：

```rust
list_directory_sorted(&mut connected.session, "/photos")         // 编译失败
list_directory_sorted(&mut *connected.session, "/photos")        // 先解引用 Box，再可变借用里面的会话
list_directory_sorted(connected.session.as_mut(), "/photos")     // 同上
```

第一种写法传的是 `&mut Box<dyn RemoteSession>`。编译器会尝试把整个 `Box` 当成一个会话来转换，于是报 E0277 ``the trait bound `Box<(dyn RemoteSession + 'static)>: RemoteSession` is not satisfied``。

### 4. Sized、?Sized 与 dyn 兼容性

官方示例一（[The Rust Book 20.3：动态大小类型与 Sized](https://doc.rust-lang.org/book/ch20-03-advanced-types.html#dynamically-sized-types-and-the-sized-trait)）：

```rust
fn generic<T>(t: T) {          // 编译器把它当作 fn generic<T: Sized>(t: T)
    // --snip--
}

fn generic<T: ?Sized>(t: &T) { // ?Sized："T 可能是 Sized，也可能不是"，参数因此改成 &T
    // --snip--
}
```

- 编译器要知道每个值占多少内存。`str`、`[T]` 和 `dyn Trait` 的大小要到运行时才知道，它们叫动态大小类型（DST）。Book 称之为"黄金规则"：DST 的值必须放在某种指针后面，如 `&str`、`Box<str>`、`&dyn Trait`、`Box<dyn Trait>`。
- 这种指针比普通引用多带一份信息：`&str` 带着长度，`&dyn Trait` 带着方法表的地址。可以这样验证：

  ```rust
  use std::mem::size_of;

  assert_eq!(size_of::<&u8>(), size_of::<usize>());
  assert_eq!(size_of::<&str>(), 2 * size_of::<usize>());        // 地址 + 长度
  assert_eq!(size_of::<&dyn Draw>(), 2 * size_of::<usize>());   // 地址 + 方法表
  ```

- 类型参数默认带有 `Sized` 约束，`impl Trait` 参数也一样。`?Sized` 去掉这个默认约束，这种 `?` 写法只能用于 `Sized`。

标准库里就有 `?Sized` 的例子，[HashMap::get](https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.get) 的签名是：

```rust
pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
where
    K: Borrow<Q>,
    Q: Hash + Eq,
```

键的类型是 `String` 时，`Q` 可以是 `str`（一个 DST），所以能直接写 `map.get("lan")`，不用先造一个 `String`。P5.1 的模拟后端用 `host: &str` 查 `HashMap<String, _>`，靠的就是这一点。文档里的例子：

```rust
use std::collections::HashMap;

let mut map = HashMap::new();
map.insert(1, "a");
assert_eq!(map.get(&1), Some(&"a"));
assert_eq!(map.get(&2), None);
```

官方示例二（[Rust Reference：dyn 兼容性](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility)）：不是每个 trait 都能做成 trait 对象，方法表里的每个方法都必须能通过 trait 对象调用。下面每一行都会让 trait 失去 dyn 兼容性：

```rust
trait DynIncompatible {
    const CONST: i32 = 1;  // ERROR: cannot have associated const

    fn foo() {}  // ERROR: associated function without Sized
    fn returns(&self) -> Self; // ERROR: Self in return type
    fn typed<T>(&self, x: T) {} // ERROR: has generic type parameters
    fn nested(self: Rc<Box<Self>>) {} // ERROR: nested receiver cannot be dispatched on
}
```

写 `Box<dyn DynIncompatible>` 时，编译器报 E0038 ``the trait `DynIncompatible` is not dyn compatible``，并逐条列出原因，例如 ``because method `typed` has generic type parameters``。

给这样的方法加上 `where Self: Sized`，它就不进方法表，只能在具体类型上调用，trait 本身仍然可以用作 `dyn`：

```rust
trait NonDispatchable {
    // Non-methods cannot be dispatched.
    fn foo() where Self: Sized {}
    // Self type isn't known until runtime.
    fn returns(&self) -> Self where Self: Sized;
    // `other` may be a different concrete type of the receiver.
    fn param(&self, other: Self) where Self: Sized {}
    // Generics are not compatible with vtables.
    fn typed<T>(&self, x: T) where Self: Sized {}
}
```

- 泛型方法放不进方法表：方法表在编译时生成，而 `typed::<T>` 每换一个 `T` 就是一个新函数，没法预先列全。
- 返回 `Self`、返回 `impl Trait` 或写成 `async fn` 也不行：通过 trait 对象调用时，调用者不知道返回值的具体类型和大小。
- 对 dyn 兼容的 trait，编译器自动让 `dyn Trait` 实现这个 trait，调用时转发到方法表。因此 `dyn RemoteBackend` 本身就满足 `B: RemoteBackend` 约束。
- 这个概念以前叫 object safety（对象安全），旧资料里常见这个名字。

官方示例三（[Rust Reference：trait 对象的默认生命周期](https://doc.rust-lang.org/reference/lifetime-elision.html#default-trait-object-lifetimes)）：

```rust
trait Foo { }

// These two are the same because Box<T> has no lifetime bound on T
type T1 = Box<dyn Foo>;
type T2 = Box<dyn Foo + 'static>;

// ...so are these, because &'a T requires T: 'a
type T3<'a> = &'a dyn Foo;
type T4<'a> = &'a (dyn Foo + 'a);
```

trait 对象隐藏了具体类型，也就隐藏了这个类型里可能存着的借用，所以需要一个生命周期说明"里面的借用至少有效多久"。省略不写时，`Box<dyn Foo>` 按 `'static` 处理，里面的值不能借用任何会提前失效的数据；`&'a dyn Foo` 只要求在 `'a` 期间有效。

**在项目中**：

- `connect_by_priority<B: RemoteBackend + ?Sized>(backend: &B, …)` 有了 `?Sized`，`B` 可以是 `dyn RemoteBackend`。同一个函数既能接收 `&MemoryBackend`（单态化，静态分派），也能接收 `&dyn RemoteBackend`（动态分派）。P5.4 把后端统一成 `&dyn RemoteBackend` 后能复用 P5.3，靠的就是这一点。去掉 `?Sized`，那次调用会报 E0277 ``the size for values of type `dyn RemoteBackend` cannot be known at compilation time``，编译器还会指出 ``required by an implicit `Sized` bound in `connect_by_priority` ``。
- `RemoteBackend` 和 `RemoteSession` 都是 dyn 兼容的：方法没有类型参数，接收者是 `&self` 或 `&mut self`，返回值是确定的类型。如果把 `connect` 改成返回 `impl RemoteSession`，`&dyn RemoteBackend` 处会报 E0038 ``because method `connect` references an `impl Trait` type in its return type``，P5.4 也就没法把两个后端统一起来。返回 `Box<dyn RemoteSession>` 既保留了 dyn 兼容，又允许每个后端返回自己的会话类型。
- `Box<dyn RemoteSession>` 就是 `Box<dyn RemoteSession + 'static>`，所以会话不能借用 `connect` 的参数（`host`、`connection`），也不能借用后端自己的字段。真实会话持有自己的原生连接。P5.1 的 `MemorySession` 要持有一份自己的目录数据（从后端 `clone`），不能只存 `&self.directories`。存借用的话，编译器会报 ``returning this value requires that `'1` must outlive `'static` ``，并建议改成 `+ '_`，但那等于修改 trait 的约定，不要照做。
- `list_directory_sorted(session: &mut dyn RemoteSession, …)` 的参数相当于 `&'a mut (dyn RemoteSession + 'a)`，只在调用期间借用，没有 `'static` 的要求。

### 5. RefCell 与模拟对象

官方示例（[The Rust Book 15.5：用模拟对象测试](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html#testing-with-mock-objects)）：`LimitTracker` 在额度快用完时通过 `Messenger` 发警告。测试不想真的发邮件，就写一个只记下消息的模拟对象（mock object）。这种在测试中顶替真实对象的类型叫测试替身（test double）。

```rust
pub trait Messenger {
    fn send(&self, msg: &str);           // 注意：接收者是 &self
}

pub struct LimitTracker<'a, T: Messenger> {
    messenger: &'a T,
    value: usize,
    max: usize,
}

impl<'a, T> LimitTracker<'a, T>
where
    T: Messenger,
{
    pub fn new(messenger: &'a T, max: usize) -> LimitTracker<'a, T> {
        LimitTracker {
            messenger,
            value: 0,
            max,
        }
    }

    pub fn set_value(&mut self, value: usize) {
        self.value = value;

        let percentage_of_max = self.value as f64 / self.max as f64;

        if percentage_of_max >= 1.0 {
            self.messenger.send("Error: You are over your quota!");
        } else if percentage_of_max >= 0.9 {
            self.messenger
                .send("Urgent warning: You've used up over 90% of your quota!");
        } else if percentage_of_max >= 0.75 {
            self.messenger
                .send("Warning: You've used up over 75% of your quota!");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct MockMessenger {
        sent_messages: RefCell<Vec<String>>,
    }

    impl MockMessenger {
        fn new() -> MockMessenger {
            MockMessenger {
                sent_messages: RefCell::new(vec![]),
            }
        }
    }

    impl Messenger for MockMessenger {
        fn send(&self, message: &str) {
            self.sent_messages.borrow_mut().push(String::from(message));
        }
    }

    #[test]
    fn it_sends_an_over_75_percent_warning_message() {
        let mock_messenger = MockMessenger::new();
        let mut limit_tracker = LimitTracker::new(&mock_messenger, 100);

        limit_tracker.set_value(80);

        assert_eq!(mock_messenger.sent_messages.borrow().len(), 1);
    }
}
```

如果把字段写成 `sent_messages: Vec<String>`，在 `send` 里直接 `push`，编译器会拒绝：E0596 ``cannot borrow `self.sent_messages` as mutable, as it is behind a `&` reference``。编译器建议把 trait 改成 `&mut self`，但不应该为了测试去改正式接口。`RefCell` 解决了这个问题：

- `RefCell<T>` 把借用规则的检查从编译时移到运行时。`borrow()` 返回 `Ref<T>`（只读），`borrow_mut()` 返回 `RefMut<T>`（可写），它们离开作用域时归还借用。
- 规则和普通引用一样：同一时刻可以有多个 `borrow()`，或者只有一个 `borrow_mut()`。违反规则不会编译失败，而是在运行时 panic（[RefCell::borrow](https://doc.rust-lang.org/std/cell/struct.RefCell.html#method.borrow)）：

  ```rust
  use std::cell::RefCell;

  let c = RefCell::new(5);

  let m = c.borrow_mut();
  let b = c.borrow(); // this causes a panic
  ```

  panic 信息是 `RefCell already mutably borrowed`；反过来，先 `borrow()` 再 `borrow_mut()`，信息是 `RefCell already borrowed`。

- 写完马上读的常见写法（[RefCell::borrow_mut](https://doc.rust-lang.org/std/cell/struct.RefCell.html#method.borrow_mut)）。`borrow_mut()` 返回的临时值在语句结束时就释放了，所以下一行的 `borrow()` 不会冲突：

  ```rust
  use std::cell::RefCell;

  let c = RefCell::new("hello".to_owned());

  *c.borrow_mut() = "bonjour".to_owned();

  assert_eq!(&*c.borrow(), "bonjour");
  ```

- `RefCell` 只能在单线程中使用。多线程下要换成 `Mutex` 这类同步类型，第 08 课讲 `Send`、`Sync` 时再展开。
- `HashMap::get` 返回 `Option<&V>`，结果借用着 map。要把数据交出去（比如放进会话），用 [Option::cloned](https://doc.rust-lang.org/std/option/enum.Option.html#method.cloned) 得到拥有所有权的 `Option<V>`：

  ```rust
  let x = 12;
  let opt_x = Some(&x);
  assert_eq!(opt_x, Some(&12));
  let cloned = opt_x.cloned();
  assert_eq!(cloned, Some(12));
  ```

**在项目中**：P5.1 的 `MemoryBackend` 就是照着 `MockMessenger` 写的：

| Book 中 | 本课 |
|---|---|
| `Messenger` trait，`send(&self, …)` | `RemoteBackend` trait，`connect(&self, …)` |
| 被测代码 `LimitTracker` 只依赖 trait | 被测代码 `connect_by_priority`、`Backends::connect` 只依赖 trait |
| `MockMessenger` 记下每条消息 | `MemoryBackend` 记下每次收到的 Host |
| 断言 `sent_messages` 的长度 | 断言尝试过哪些 Host、顺序如何 |

`connect` 只拿到 `&self`，所以尝试记录放在 `RefCell<Vec<String>>` 里。预设的连接错误和目录数据只会被读取，用普通的 `HashMap` 就够了：用 `get(host)` 按 Host 查预设错误（知识 4：用 `&str` 直接查 `String` 键）；连接成功时把目录 `clone` 一份交给会话（知识 4：`'static`）。`RemoteError` 派生了 `Clone`，从 `&self` 里返回预设的错误时要 `clone()`，因为不能把值从借用的 map 里移出来。

`MemorySession::list_directory` 的接收者本来就是 `&mut self`，可以直接修改自己的字段，不需要 `RefCell`。

### 6. 按模式判断与断言：matches! 与 assert_matches!

官方示例（[std::matches!](https://doc.rust-lang.org/std/macro.matches.html)）：

```rust
let foo = 'f';
assert!(matches!(foo, 'A'..='Z' | 'a'..='z'));

let bar = Some(4);
assert!(matches!(bar, Some(x) if x > 2));
```

官方示例（[std::assert_matches!](https://doc.rust-lang.org/std/macro.assert_matches.html)，Rust 1.96 起稳定，需要 `use std::assert_matches;`）：

```rust
use std::assert_matches;

let a = Some(345);
let b = Some(56);
assert_matches!(a, Some(_));
assert_matches!(b, Some(_));

assert_matches!(a, Some(345));
assert_matches!(a, Some(345) | None);

// assert_matches!(a, None); // panics
// assert_matches!(b, Some(345)); // panics
// assert_matches!(b, Some(345) | None); // panics

assert_matches!(a, Some(x) if x > 100);
// assert_matches!(a, Some(x) if x < 100); // panics
```

- `matches!(值, 模式)` 等价于 `match 值 { 模式 => true, _ => false }`。模式的写法和 `match` 分支完全一样，可以用 `|` 列出多个模式，用 `if` 加守卫条件。
- 测试里用 `assert_matches!` 比 `assert!(matches!(..))` 好：失败时它会打印实际的值（值要实现 `Debug`），`assert!` 只说条件不成立。
- 值里有 `String` 时，在模式中绑定变量，再用守卫比较内容，例如 `Some(name) if name == "lan"`。对引用（`&result`）匹配，值不会被移进宏里，之后还能继续使用。
- 用 `matches!` 列出"是"的几种变体很简洁，但以后新增变体时，编译器不会提醒你更新它，新变体都被 `_` 归进了"否"。想让编译器帮你检查有没有遗漏，就写完整的 `match`，不写 `_`。

需要取出错误再细看时，用 [Result::expect_err](https://doc.rust-lang.org/std/result/enum.Result.html#method.expect_err) 取出 `Err` 里的值（结果是 `Ok` 时 panic 并打印说明），再用 [let…else](https://doc.rust-lang.org/book/ch06-03-if-let.html#staying-on-the-happy-path-with-letelse) 解构出想要的变体。

**在项目中**：

- P5.2 的 `allows_next_address` 要把八种 `RemoteError` 分成两类。可以用 `matches!` 列出允许换地址的四种，也可以写完整的 `match`，以后新增变体时让编译器提醒你归类。动手前想好用哪种。
- `ConnectError` 没有派生 `PartialEq`，不能直接 `assert_eq!`。P5.3、P5.4 的测试用模式检查变体和里面的值，例如"在 `domain` 上认证失败"：

  ```rust
  assert_matches!(
      &result,
      Err(ConnectError::Rejected { address_id, source: RemoteError::AuthenticationFailed })
          if address_id == "domain"
  );
  ```

- 要逐项比较 `AllUnreachable` 里的列表时，先取出错误再解构：

  ```rust
  let error = connect_by_priority(&backend, &device, connection).expect_err("三个地址都应失败");
  let ConnectError::AllUnreachable(failures) = &error else {
      panic!("unexpected error: {error:?}");
  };
  // failures 是 &Vec<(String, RemoteError)>，可以和预期的列表 assert_eq!
  ```

### 7. 过滤与排序：retain、sort_by、then_with

官方示例（[Vec::retain](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.retain)、[slice::sort_by](https://doc.rust-lang.org/std/primitive.slice.html#method.sort_by)、[Ordering::then_with](https://doc.rust-lang.org/std/cmp/enum.Ordering.html#method.then_with)）：

```rust
let mut vec = vec![1, 2, 3, 4];
vec.retain(|&x| x % 2 == 0);             // 只保留闭包返回 true 的元素，原地修改
assert_eq!(vec, [2, 4]);

let mut v = [4, -5, 1, -3, 2];
v.sort_by(|a, b| a.cmp(b));              // 比较函数返回 Ordering
assert_eq!(v, [-5, -3, 1, 2, 4]);

// reverse sorting
v.sort_by(|a, b| b.cmp(a));              // 交换 a、b 就是降序
assert_eq!(v, [4, 2, 1, -3, -5]);
```

```rust
use std::cmp::Ordering;

let x: (i64, i64, i64) = (1, 2, 7);
let y: (i64, i64, i64) = (1, 5, 3);
let result = x.0.cmp(&y.0).then_with(|| x.1.cmp(&y.1)).then_with(|| x.2.cmp(&y.2));

assert_eq!(result, Ordering::Less);      // 第一项相等，由第二项 2 < 5 决定
```

先比一项、相等时再比另一项的写法，用在元组数组上是这样：

```rust
let mut tasks = [("backup", 2), ("check", 1), ("archive", 2)];
tasks.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(b.0)));
assert_eq!(tasks, [("check", 1), ("archive", 2), ("backup", 2)]);
```

- `retain` 按原顺序访问每个元素一次，留下的元素保持原来的顺序。
- `sort_by` 是稳定排序，比较结果为 `Equal` 的元素保持原来的先后。第 01 课 `addresses_by_priority` 用的 `sort_by_key` 也是稳定的，所以同优先级的地址按保存顺序排列。
- `then_with` 只在前一次比较结果为 `Equal` 时才调用后面的闭包。
- `bool` 也可以比较大小，`false < true`。
- 字符串按字节比较（[Ord for str](https://doc.rust-lang.org/std/primitive.str.html#impl-Ord-for-str)）："Strings are ordered lexicographically by their byte values"，不是字典或拼音顺序。ASCII 里数字排在大写字母前，大写字母排在小写字母前，所以 `"2026" < "Album" < "a.txt"`。

**在项目中**：`list_directory_sorted` 先从会话读回原始条目。条目的顺序由服务器决定，还可能包含 `.` 和 `..`。函数把它们整理成界面上要显示的稳定顺序：用 `retain` 去掉 `.`、`..`，再用 `sort_by` 按"目录在前"排序，同组内用 `then_with` 按名称排。`DirectoryEntry` 派生了 `PartialEq`，测试可以直接用 `assert_eq!` 比较整个结果。

## 二、可选示例 E5.1：看懂编译器对 dyn 的两条限制

```sh
cargo run --locked --example course05
```

[course05.rs](../../examples/course05.rs) 中的 `Speaker` trait 有一个默认方法 `announce` 和一个泛型方法 `announce_all`。函数 `describe` 用 `?Sized` 同时接收 `&Headphones` 和 `&dyn Speaker`。正常运行时输出：

```text
设备：耳机
["耳机 正在播放 晴天", "耳机 正在播放 夜曲"]
```

做下面三个实验，每做完一个就恢复原样，用 `cargo check --locked --example course05` 查看报错：

1. 把 `describe` 的参数改成 `&impl Speaker`（去掉 `+ ?Sized`）。预期报 E0277 ``the size for values of type `dyn Speaker` cannot be known at compilation time``，编译器指出这是 `describe` 隐含的 `Sized` 约束造成的，并建议加回 `?Sized`。这说明 `impl Trait` 参数也是类型参数，同样默认 `Sized`。
2. 删掉 `announce_all` 上的 `where Self: Sized`。预期报 E0038 ``the trait `Speaker` is not dyn compatible``，原因是 ``method `announce_all` has generic type parameters``。注意报错位置在 `let speaker: &dyn Speaker`：trait 本身仍然可以定义和实现，只是不能再做成 trait 对象。
3. 保留 `Self: Sized`，在 `main` 里加一行 `speaker.announce_all(&["晴天"]);`。预期报错 ``the `announce_all` method cannot be invoked on a trait object``。再加一行 `speaker.announce("晴天")`，它可以正常调用，因为它没有类型参数，在方法表里。

想一想：`RemoteBackend::connect` 为什么不需要 `where Self: Sized`？如果给它加上，P5.4 会出什么问题？

## 三、项目练习

### 先看全局

```text
Backends::connect(device, "smb-data")          P5.4  按连接协议选后端
  → connect_by_priority(backend, device, conn)  P5.3  按优先级逐个尝试地址
    → RemoteBackend::connect(host, conn)        P5.1  对一个 Host 建立会话（测试中由 MemoryBackend 实现）
    → RemoteError::allows_next_address          P5.2  失败后判断能否换下一个地址
  ← Connected { address_id, session }

list_directory_sorted(&mut *session, "/photos") P5.5  读目录并整理
  → RemoteSession::list_directory(path)         P5.1  返回会话中的原始条目（测试中由 MemorySession 实现）
```

| 文件 | 你要写的 |
|---|---|
| [src/remote.rs](../../src/remote.rs) | P5.2–P5.5 的四个 `todo!()` |
| [tests/course05_remote.rs](../../tests/course05_remote.rs) | P5.1 的 `MemoryBackend`、`MemorySession`，以及五个测试体 |

测试夹具 `sample_device()` 有两条连接：SMB 的 `smb-data` 和 NFS 的 `nfs-media`。设备有三个地址，故意按 `domain`、`lan`、`vpn` 的顺序保存，和优先级顺序不同：

| 尝试顺序 | 地址 ID | Host | 优先级 | 场景 |
|---|---|---|---|---|
| 1 | `lan` | `192.168.50.10` | 1 | 在家里的局域网 |
| 2 | `domain` | `nas.example.com` | 2 | 在外面，通过域名访问 |
| 3 | `vpn` | `10.8.0.10` | 3 | 通过 VPN |

后端的 `connect` 收到的是 Host，返回结果 `Connected` 里记的是地址 ID。测试断言"后端收到了哪些 Host"时写 Host，断言"用了哪个地址"时写地址 ID。

### 项目约定

- remote.rs 只依赖 model，不访问网络，不打印，也不记日志。真实的网络访问在第 07 课接入。
- 远程路径从连接的根目录算起，以 `/` 开头，例如 `/photos`。各平台统一用 `/`。
- 每道题先补测试并移除 `#[ignore]`，看到它失败（调用 `todo!()` 会 panic），再实现到通过。不要用 `#[should_panic]` 让占位代码"通过"。

### P5.1 模拟后端 `MemoryBackend` 与 `MemorySession`

**作用**：P5.3–P5.5 要测的函数都需要一个后端。真实后端第 07 课才有，而且依赖网络环境。`MemoryBackend` 是测试替身：按预设返回成功或失败，并记下每次被要求连接的 Host，测试由此断言"按什么顺序试了哪些地址"。

**例子**：预设 `192.168.50.10` 超时，`/` 目录里有一个名为 `photos` 的目录：

| 操作 | 结果 |
|---|---|
| `connect("192.168.50.10", 连接)` | `Err(RemoteError::TimedOut)` |
| `connect("nas.example.com", 连接)` | `Ok(会话)` |
| 读取尝试记录 | `["192.168.50.10", "nas.example.com"]`，失败的那次也在里面 |
| `会话.list_directory("/")` | `Ok`，一个条目：`photos`，类型 `Directory` |
| `会话.list_directory("/missing")` | `Err(RemoteError::NotFound)` |

**为什么这样要求**：

- 失败的尝试也要记录：P5.3 要断言回退顺序是 lan → domain → vpn，只记成功的看不出前面试过谁。
- 按 Host 预设错误，而不是按地址 ID：`connect` 只收到 Host 字符串，和真实后端一样不知道地址 ID。
- 会话持有自己的目录副本：`Box<dyn RemoteSession>` 要求 `'static`（知识 4）。

**先读**：知识 1 的两个 trait，知识 5 的 `MockMessenger`。

**实现**：在测试文件的 `// TODO P5.1` 处定义两个类型并实现对应的 trait。可以参考下面的结构，方法名自己定：

```rust
struct MemoryBackend {
    connect_errors: HashMap<String, RemoteError>,      // Host → 连接它时返回的错误
    directories: HashMap<String, Vec<DirectoryEntry>>, // 路径 → 条目，连接成功时复制一份给会话
    attempts: RefCell<Vec<String>>,                    // 按调用顺序记下收到的 Host
}

struct MemorySession {
    directories: HashMap<String, Vec<DirectoryEntry>>, // 自己拥有数据，满足 'static
}
```

还要给测试留出入口：设置某个 Host 的连接错误、准备目录、读出尝试记录。P5.5 还需要让某个路径读取时返回 `AccessDenied`，这个入口可以现在留，也可以到 P5.5 再加。测试文件顶部要引入 `std::cell::RefCell`、`std::collections::HashMap` 和 `nasnet::remote` 里用到的类型。

**测试** `p5_1_mock_records_attempts_and_serves_directories`：覆盖上表五行。连接参数用 `sample_device().find_connection("smb-data")` 取得。

### P5.2 判断能否换地址 `RemoteError::allows_next_address`

**作用**：连接某个地址失败后，`connect_by_priority` 用它判断"换一个地址有没有可能成功"。

**例子**：

| 错误 | 典型场景 | 允许换地址 |
|---|---|---|
| `NameResolution` | 在外网时，只在家里 DNS 中登记的域名解析不到 | 是 |
| `TimedOut` | 人在公司，连家里的 `192.168.50.10` 没有回应 | 是 |
| `NoRoute` | VPN 没连上，本机没有到 `10.8.0.10` 的路由 | 是 |
| `Refused` | 这个地址上的主机在线，但端口没有服务在监听 | 是 |
| `AuthenticationFailed` | 用户名或密码错误 | 否 |
| `AccessDenied` | 身份有效，但没有访问这个共享或目录的权限 | 否 |
| `NotFound` | 共享、导出或目录不存在 | 否 |
| `NotADirectory` | 路径指向的是文件 | 否 |

**为什么这样要求**：前四种是"这条网络路径走不通"，换一条路径可能就通了。后四种说明已经到达了 NAS，但 NAS 拒绝了请求。所有地址都指向同一台 NAS，换个地址答案也不会变。对每个地址都重复认证失败，还可能触发 NAS 的登录失败保护。完整规则见[技术设计](../NAS-Client-Architecture.md)第八节"连接路由规则"。

**先读**：知识 6，以及 [remote.rs](../../src/remote.rs) 中 `RemoteError` 各变体的注释。

**测试** `p5_2_only_network_errors_allow_address_fallback`：遍历八组（错误, 预期），写成 `assert_eq!(error.allows_next_address(), expected, "{error:?}")`。最后的格式化参数会出现在失败信息里，直接指出是哪个变体分错了类。

### P5.3 按优先级尝试地址 `connect_by_priority`

**作用**：一台设备可能有多个地址（局域网 IP、域名、VPN 地址）。给定一条协议连接，按优先级逐个尝试，返回第一个连上的会话和所用的地址 ID。

**例子**：

| 模拟情况 | 后端收到的 Host 顺序 | 预期结果 |
|---|---|---|
| 所有地址可用 | lan | `Connected`，地址为 `lan` |
| lan 超时 | lan → domain | `Connected`，地址为 `domain` |
| lan 超时，domain 解析失败 | lan → domain → vpn | `Connected`，地址为 `vpn` |
| lan 拒绝连接，domain 认证失败 | lan → domain | `Rejected { address_id: "domain", source: AuthenticationFailed }` |
| 三个地址分别超时、解析失败、无路由 | lan → domain → vpn | `AllUnreachable`，按顺序包含三组（地址 ID, 错误） |
| 没有地址 | 空 | `NoAddresses` |

全部失败时，`ConnectError` 的 `Display` 会打印每个地址各自的原因：

```text
All addresses are unreachable: lan (Connection timed out), domain (Host name could not be resolved), vpn (No route to host)
```

**为什么这样要求**：

- 用 `addresses_by_priority()` 而不是 `addresses()`：保存顺序（domain, lan, vpn）不是尝试顺序。
- 可回退的错误按尝试顺序全部保留：全部失败时，用户要看到每个地址分别为什么连不上，才知道该检查网线、DNS 还是 VPN。
- 遇到不可回退的错误立即停止，并带上出错的地址：后面的地址不再尝试，理由见 P5.2。
- 没有地址时返回 `NoAddresses`，而不是空的 `AllUnreachable`：一个是"没配置地址"，一个是"配置了但都连不上"，用户要做的事不同。

**先读**：知识 2、知识 4，以及 `ConnectError` 的四个变体和它的 `Display`。

**实现**：遍历 `device.addresses_by_priority()`，对每个地址调用 `backend.connect(&address.host, connection)`。成功时返回 `Connected`，其中 `address_id` 是该地址的 ID。失败时用 P5.2 判断：可以回退就记下（地址 ID, 错误）并继续，否则返回 `Rejected`。可以用带守卫的分支 `Err(error) if error.allows_next_address() => …` 区分这两种失败。循环结束后，没有尝试过任何地址就返回 `NoAddresses`，否则返回 `AllUnreachable`。

**测试** `p5_3_fallback_uses_priority_and_stops_on_rejection`：每行都比较返回结果和后端实际收到的 Host 顺序，每个场景用一个新的后端。至少有一例先把后端绑定成 trait 对象再调用，以验证 `?Sized`：

```rust
let backend: &dyn RemoteBackend = &memory_backend;
let connected = connect_by_priority(backend, &device, connection).expect("domain 应该能连上");
```

无地址的场景可以对夹具调用三次 `remove_address`，删掉全部地址。

### P5.4 按协议选后端 `Backends::connect`

**作用**：调用方只说"用 nas-home 的 `smb-data` 连接"，不想关心该用哪个后端。`Backends::connect` 根据这条连接的配置是 NFS 还是 SMB，选出对应的后端，再交给 P5.3 去尝试地址。

**例子**：

| 调用 | 预期行为 |
|---|---|
| `smb-data` 可连 | 只有 SMB 后端记录到 `192.168.50.10` |
| `nfs-media` 可连 | 只有 NFS 后端记录到 `192.168.50.10` |
| `smb-data` 的三个地址均超时 | 返回 `AllUnreachable`，NFS 后端的尝试记录为空 |
| 连接 ID 为 `ftp` | 返回 `ConnectionNotFound("ftp")`，两个后端的尝试记录都为空 |

**为什么这样要求**：

- 只用连接所属协议的后端：SMB 的地址全部失败时，也不能改用 NFS 后端。两个协议的身份和权限配置是独立的，NFS 能连上不代表 SMB 的那个共享可用（技术设计第八节"连接路由规则"第 2 条）。
- 连接 ID 不存在时，不调用任何后端，直接返回错误。

**先读**：知识 3 的 `component: &dyn Draw` 和项目中的第 3 点，知识 4 的 `?Sized`。

**实现**：用 `device.find_connection(connection_id)` 查连接，找不到就返回 `ConnectionNotFound(connection_id.to_owned())`，可以用 `ok_or_else` 加 `?`。然后对 `&connection.config` 做 `match`，`ConnectionConfig::Nfs(_)` 选 `&self.nfs`，`ConnectionConfig::Smb(_)` 选 `&self.smb`，把结果标注为 `&dyn RemoteBackend`，再调用 `connect_by_priority`。remote.rs 需要引入 `crate::model::ConnectionConfig`。

写完后想一想另一种写法：在两个分支里各自调用一次 `connect_by_priority(&self.nfs, …)` 和 `connect_by_priority(&self.smb, …)`，这样不需要 trait 对象。两种写法分别是静态分派还是动态分派？编译器各生成几份 `connect_by_priority`？

**测试** `p5_4_dispatch_calls_only_the_selected_protocol`：每个场景用一组新的 `Backends`，分别断言返回值和两个后端各自的尝试记录。`Backends` 的字段是公开的，可以通过 `backends.smb`、`backends.nfs` 读取记录。

### P5.5 读取并整理目录 `list_directory_sorted`

**作用**：界面和终端显示目录时需要稳定的顺序。服务器返回的顺序不固定，还会带上 `.` 和 `..`。这个函数读取一层目录，并整理成显示用的顺序。

**例子**：会话读回下面的乱序结果，`/` 表示目录，`@` 表示 `Other`（如符号链接）：

```text
读回   .  ..  b.txt  Album/  a.txt  2026/  latest@
显示   2026/  Album/  a.txt  b.txt  latest@
```

| 读取 `/missing`（不存在） | 读取 `/private`（无权限） |
|---|---|
| `Err(RemoteError::NotFound)` | `Err(RemoteError::AccessDenied)` |

**为什么这样要求**：

- 读取错误原样返回：`NotFound` 和 `AccessDenied` 对用户意义不同，也不能当成空目录显示。
- 只滤掉名称恰好是 `.` 和 `..` 的条目：`.config` 这类隐藏文件是真实的条目，要保留。
- 目录排在其他条目前面，两组内部各自按名称升序。`File` 和 `Other` 同属非目录组。名称按字节比较，所以 `2026` 排在 `Album` 前面（知识 7）。

**先读**：知识 7；[remote.rs](../../src/remote.rs) 中 `RemoteSession::list_directory` 的注释。

**实现**：`session.list_directory(path)?` 读出条目，用 `retain` 过滤，再用 `sort_by` 加 `then_with` 排序。

**测试** `p5_5_listing_sorts_entries_and_preserves_errors`：让模拟会话返回上面的乱序结果，断言整理后的名称和类型；再检查 `NotFound` 和 `AccessDenied` 原样传回。可以先用 `connect_by_priority` 得到 `Connected`，再按知识 3 的写法 `list_directory_sorted(&mut *connected.session, "/photos")` 调用。也可以直接构造 `MemorySession`，传入 `&mut session`。

## 四、验收

```sh
cargo test --locked --test course05_remote -- --list   # 列出全部测试
cargo test --locked --test course05_remote p5_1_        # 只运行 P5.1
cargo test --locked --test course05_remote              # 第 05 课全部测试
cargo clippy --all-targets --locked -- -D warnings
```

- 第 05 课的测试全部通过，没有 `ignored`；每个表格中的场景都有断言。
- 能解释：
  - 以一次 SMB 目录读取为例：选后端、换地址、读目录分别发生在哪个函数、通过哪个 trait；哪一步是静态分派，哪一步是动态分派。
  - 为什么超时后继续尝试下一个地址，认证失败后就停止。
  - `connect` 为什么返回 `Box<dyn RemoteSession>`，而不是 `impl RemoteSession`；去掉 `connect_by_priority` 的 `?Sized` 后，哪一次调用会编译失败。
  - 为什么 `MemoryBackend` 的尝试记录要放在 `RefCell` 里，`MemorySession` 却不需要。
