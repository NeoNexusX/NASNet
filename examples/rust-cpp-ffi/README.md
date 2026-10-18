# 第一课：Rust 调用 C++ 静态库

这一课只完成一个功能：用 C++ 实现加法，在 Rust 中调用并打印结果。先手写一条 FFI 声明，理解调用关系；bindgen 自动生成的也是这类底层声明。

## 先运行起来

你的 Windows 机器已经安装了 Rust 和 Visual Studio C++ 工具。在 PowerShell 中运行：

```powershell
Set-Location 'C:\Users\13427\Desktop\NASNet'
cargo run --manifest-path examples/rust-cpp-ffi/Cargo.toml
```

第一次运行时，Cargo 会下载 `cc` 及其依赖，编译 C++ 库，再编译和运行 Rust 程序。最后会输出：

```text
C++ returned: 3 + 5 = 8
```

后续修改代码后，仍然执行同一条命令。如果终端提示找不到 `cargo`，重新打开终端，让安装 Rust 时设置的 PATH 生效。

这个例子手写 FFI 声明，不调用 bindgen，因此运行它不需要 LLVM。C++ 代码仍由 MSVC 编译。

## 文件分别负责什么

```text
rust-cpp-ffi/
├── Cargo.toml       Rust 项目配置、构建依赖
├── Cargo.lock       Cargo 生成的依赖版本锁定文件
├── build.rs         编译 C++ 静态库的 Rust 构建脚本
├── native/
│   ├── math.hpp     C++ 函数声明
│   └── math.cpp     C++ 函数实现
└── src/
    └── main.rs      Rust FFI 声明、安全封装和程序入口
```

这是一个独立的学习项目，还没有创建 NAS 客户端的业务代码。

## 按这个顺序阅读代码

### 1. C++ 实现：native/math.cpp

`nasnet_add` 接收两个 32 位有符号整数，返回 64 位有符号整数。先转成 64 位再相加，可以避免 C++ 的 32 位有符号整数溢出。

| C++ 类型 | Rust 类型 |
|---|---|
| `std::int32_t` | `i32` |
| `std::int64_t` | `i64` |

跨语言时，两边必须对参数和返回值的类型达成一致。这里使用固定宽度整数，避免把不同平台的 `long` 长度当成一样。

`noexcept` 表示函数不允许异常传播出去。本例只执行整数转换和加法，不会抛出异常。

### 2. C ABI：native/math.hpp

```cpp
extern "C" std::int64_t nasnet_add(std::int32_t left, std::int32_t right) noexcept;
```

`extern "C"` 指定 C 语言的链接约定，让函数使用 Rust 能对应声明的接口，避免 C++ 的函数名修饰。函数体仍然是 C++ 代码。

遇到已有 C++ 类库时，通常可以在 C++ 侧写一层这样的 C 接口，再让 Rust 调用它。类、模板和 `std::string` 不能简单地当成 C 函数参数直接传给 Rust。

### 3. 构建脚本：build.rs

`build.rs` 本身也是 Rust 程序，由 Cargo 在构建过程中执行。它与 `src/main.rs` 的 `main` 属于不同程序。

```rust
cc::Build::new()
    .cpp(true)
    .std("c++17")
    .flag_if_supported("/utf-8")
    .include("native")
    .file("native/math.cpp")
    .compile("nasnet_math");
```

- `.cpp(true)`：告诉 `cc` 要编译 C++。
- `.std("c++17")`：使用 C++17 标准。
- `.flag_if_supported("/utf-8")`：在支持此选项的编译器上使用 UTF-8，避免 MSVC 按本地编码误读中文注释。
- `.include("native")`：添加头文件搜索目录。
- `.file(...)`：指定要编译的源码。
- `.compile("nasnet_math")`：调用编译器生成静态库，并向 Cargo 输出链接配置。

`cc` 是 Cargo 的构建依赖，它负责查找和调用编译器。在你的 Windows MSVC 环境下，实际编译工具是 Visual Studio 的 `cl.exe`。

产物位于 Cargo 的 `target` 构建目录中，Windows 下的静态库名是 `nasnet_math.lib`。Rust 链接器会把需要的 C++ 代码链接进最终程序，所以运行时不需要另外寻找我们这个示例的 DLL。

`Cargo.toml` 中的 `[build-dependencies]` 表示这项依赖供构建脚本使用。`Cargo.lock` 记录本次实际选用的依赖版本，应随这个应用示例提交。

### 4. Rust FFI 声明：src/main.rs

```rust
unsafe extern "C" {
    fn nasnet_add(left: i32, right: i32) -> i64;
}
```

FFI 是 Foreign Function Interface，即外部函数接口。

这里告诉 Rust：链接时会找到一个叫 `nasnet_add` 的外部函数，它的参数和返回值分别是什么。分号说明这里是声明，函数实现在 C++ 库中。

Rust 2024 要求外部声明块显式使用 `unsafe extern`。Rust 编译器无法替我们核对 C++ 实现与声明是否完全一致，这部分正确性需要封装作者保证。

### 5. Rust 安全封装与调用

```rust
fn add(left: i32, right: i32) -> i64 {
    // 外部函数的安全前提已由封装作者确认。
    unsafe { nasnet_add(left, right) }
}
```

普通函数 `add` 把 `unsafe` 限制在很小的边界内。它能安全地接收任意 `i32`：两边签名匹配，没有指针、共享状态，也没有异常跨越边界。

在 `main` 中，`let` 声明变量，`add(left, right)` 调用安全封装，`println!` 把结果打印到终端。业务代码不需要直接接触外部函数。

## 动手练习

1. 把 `src/main.rs` 中的两个数改成 `10` 和 `32`，重新运行，观察输出是否为 `42`。
2. 再改成 `-7` 和 `5`，观察输出是否为 `-2`。
3. 阅读 `build.rs`，尝试说清“谁调用了 MSVC、谁调用了 C++ 函数”。

完成前两项，只需要修改 Rust 的两个变量；不用同时修改 C++ 代码或 FFI 声明。

## 与 NAS 项目的关系

后续调用 libnfs、libsmb2 时，会沿用“构建 C 库 → Rust 外部声明 → Rust 安全封装 → 业务调用”的结构。这些库的构建会使用 CMake；当前例子只有一个 C++ 源文件，用 `cc` 即可完成。

当前示例选择静态链接，先学习最短的完整调用过程。示例构建脚本没有写死 Windows 编译器路径；其他平台仍需要对应的 C++ 编译器，本次运行结果以 Windows 为准。

参考：[cc 官方说明](https://docs.rs/cc/latest/cc/)、[Cargo 构建脚本](https://doc.rust-lang.org/cargo/reference/build-scripts.html)、[Rust 2024 外部声明](https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-extern.html)。

---

# 第二课：字符串与错误处理

第一课只在边界两侧传整数。这一课解决真实集成中立刻会遇到的两个问题：字符串怎么过边界；C++ 内部解析失败时，怎么把失败安全地交给 Rust。本课不新增文件，只扩展 `native/math.hpp`、`native/math.cpp` 和 `src/main.rs`。

由于 `build.rs` 已经用 `cargo:rerun-if-changed` 声明过这两个文件，运行上一课的命令即可，Cargo 会自动重新编译改动的部分：

```powershell
cargo run --manifest-path examples/rust-cpp-ffi/Cargo.toml
```

现在的输出：

```text
C++ returned: 3 + 5 = 8
parse_i64("42") = 42
parse_i64("-7") = -7
parse_i64("3.14") 失败: 不是整数: "3.14"
parse_i64("abc") 失败: 不是整数: "abc"
parse_i64("999999999999999999999") 失败: 超出 64 位范围: "999999999999999999999"
```

## 1. C++ 侧：解析函数与异常处理

新增函数把十进制整数字符串解析成 64 位整数：

```cpp
extern "C" std::int32_t nasnet_parse_i64(
    const char* text, std::size_t len, std::int64_t* out_value) noexcept;
```

- `text`/`len`：只读字节串，C++ 只读，不负责释放。
- `out_value`：调用方提供的输出位置，解析成功时写入，失败时不改动。
- 返回值是状态码：`0` 成功，`1` 不是整数，`2` 超出 64 位范围。

实现里真正干活的是 `std::stoll`，它在解析失败时会**抛出 C++ 异常**（`std::invalid_argument`、`std::out_of_range`）。第一课说过：`noexcept` 表示异常不允许传播出去。本课的做法是把异常挡在边界内、转成状态码：

```cpp
try {
    ...
    *out_value = value;
    return 0;
} catch (const std::invalid_argument&) {
    return 1;
} catch (const std::out_of_range&) {
    return 2;
} catch (...) {
    return 1; // 兜底：任何未预料到的异常也绝不出边界。
}
```

最后的 `catch (...)` 是“边界兜底网”：即使将来实现被改动、出现了没预料到的异常，最坏结果也只是返回一个错误码，而不是程序被 `std::terminate` 终止。注意两层的分工——`noexcept` 是“异常真的逃出去就终止程序”的最后防线；`catch (...)` 是让这条防线永远不被触发的第一道工事。两层都在，才安全。

另外，`try` 块里的 `std::string input` 无论走哪条退出路径（正常返回或抛异常），C++ 都会自动释放。这是 C++ 的 RAII 机制，也是 C++ 敢在边界内用异常的原因：异常路径不会漏内存。

## 2. Rust 侧声明与类型对照

```rust
fn nasnet_parse_i64(text: *const c_char, len: usize, out_value: *mut i64) -> i32;
```

| C++ 类型 | Rust 类型 | 含义 |
|---|---|---|
| `const char*` | `*const c_char` | 只读字节串 |
| `std::size_t` | `usize` | 字节长度（无符号） |
| `std::int64_t*` | `*mut i64` | 输出参数 |
| `std::int32_t` | `i32` | 状态码 |

C 字符串本质上只是“一个指针 + 约定的 \0 结尾”，本身没有长度信息，所以必须把 `len` 一起传过去。

## 3. CString：把 &str 变成 C++ 能读的格式

Rust 的 `&str` 是 UTF-8 字节序列，但不保证以 \0 结尾；`CString` 补上这个保证：

```rust
let c_text = CString::new(text).map_err(|_| ParseError::NotANumber(text.to_string()))?;
```

- 若 `text` 中间本身含 `\0`，`CString::new` 返回错误——C 字符串根本无法表示它，这里直接按“不是整数”处理。
- 调用时同时传 `c_text.as_ptr()` 和 `text.len()`：长度是主要依据，`\0` 结尾是保险。

## 4. 内存归属：谁分配，谁释放

跨边界传数据时必须明确每块内存归谁。本课的安排让这个问题最简单——**C++ 从不跨边界分配**：

- 输入字节串：Rust 分配（`CString`），Rust 释放（`c_text` 在函数结束时自动析构）；C++ 只读。
- 输出位置：调用方的栈变量 `value`，C++ 往里写一个 `int64_t`。
- C++ 内部的 `std::string`：在 C++ 内部分配、内部释放，外界无感知。

后续课程会学习反向情况——C++ 分配的内存需要 Rust 调用专门的释放函数。

## 5. 状态码翻译成 Result

C/C++ 的习惯是用返回值状态码报错；Rust 的习惯是用 `Result`。安全封装 `parse_i64` 做的就是这两种习惯之间的翻译：

- 状态码 `0` → `Ok(value)`
- `1` → `Err(ParseError::NotANumber(..))`
- `2` → `Err(ParseError::OutOfRange(..))`
- 其他 → `Err(ParseError::Unknown(code))`

`ParseError` 是描述“失败原因”的枚举，业务代码用 `match` 处理，永远不接触裸状态码、指针或 `unsafe`。它已实现 `Debug` 和 `Display`，失败信息通过 `Display` 打印成可读文本。

## 动手练习

1. 在输入列表中加入 `"0012"` 和 `" +42"`，先预测输出，再运行验证（`std::stoll` 接受前导空白和正负号）。
2. 把 `native/math.cpp` 里 `catch (...)` 那段删掉，并在 `try` 块开头加一句 `throw std::runtime_error("boom");`，重新运行。`runtime_error` 不属于前两个 catch，异常会试图逃出 `noexcept` 函数——观察程序直接崩溃终止，这就是第一课说的 `std::terminate`。改完记得恢复原样。
3. Rust 错误类型的惯例三件套是 `Debug` + `Display` + `Error`。`ParseError` 已有前两个，补上最后一行代码 `impl std::error::Error for ParseError {}`，让它能接入 `?` 错误传播链。

## 与 NAS 项目的关系

libnfs 等网络库是 C 语言写的，用错误码报错、错误信息放在别处。到时 Rust 侧用的正是今天这套模式：调用 C 函数 → 检查状态码 → 翻译成 `Result`。本课练的就是这个模式。
