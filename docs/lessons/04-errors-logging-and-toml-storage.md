# 第 04 课：测试、TOML 配置、文件存储与日志

把内存中的设备保存到磁盘，并在启动时读回：主配置 `config.toml` 只登记设备，每台设备单独一个 TOML 文件；程序启动时读取主配置、安装日志，选中设备时才读取对应的文件。

**状态**：进行中，从第二部分的 E4.1 开始。项目中主配置的校验、解码、读取以及启动和日志已经实现，作为阅读材料；设备文件的编解码、读写、登记与移除由你实现。

[课程目录](../Course-Syllabus.md) · [上一课](03-encapsulation-and-serialization.md) · [源码地图](../Code-Reading-Guide.md)

## 一、新知识

### 1. Serde 与 TOML

Serde 定义“Rust 值 ↔ 通用数据模型”的转换规则，toml 负责“数据模型 ↔ TOML 文本”。两者分工，所以同一个 `#[derive(Deserialize)]` 的类型既能读 TOML，也能读 JSON。

官方示例（[toml 文档](https://docs.rs/toml/latest/toml/)）：

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    ip: String,
    port: Option<u16>,
    keys: Keys,
}

#[derive(Deserialize)]
struct Keys {
    github: String,
    travis: Option<String>,
}

let config: Config = toml::from_str(r#"
    ip = '127.0.0.1'

    [keys]
    github = 'xxxxxxxxxxxxxxxxx'
    travis = 'yyyyyyyyyyyyyyyyy'
"#).unwrap();

assert_eq!(config.ip, "127.0.0.1");
assert_eq!(config.port, None);                     // 文本里没有 port，Option 字段得到 None
assert_eq!(config.keys.github, "xxxxxxxxxxxxxxxxx");
```

反过来，给同样的结构体派生 `Serialize`，用 `toml::to_string_pretty(&config)` 编码，得到：

```toml
ip = "127.0.0.1"

[keys]
github = "xxxxxxxxxxxxxxxxx"
travis = "yyyyyyyyyyyyyyyyy"
```

值为 `None` 的 `port` 被省略了。Rust 与 TOML 的对应关系：

| Rust | TOML |
|---|---|
| 字段 `ip: String` | `ip = "127.0.0.1"` |
| 嵌套结构体字段 `keys: Keys` | `[keys]` 表 |
| `Vec<结构体>` 字段 `devices` | `[[devices]]` 表数组，每个元素一段 |
| 值为 `None` 的 `Option` | 省略这个键（TOML 没有 null） |

- `toml::from_str` 的目标类型由变量标注（`let config: Config`）或函数返回类型决定。解析错误是 `toml::de::Error`，编码错误是 `toml::ser::Error`。
- 默认会忽略多余的键。加上容器属性 [`#[serde(deny_unknown_fields)]`](https://serde.rs/container-attrs.html#deny_unknown_fields) 后，多余的键会报错：``unknown field `colour`, expected `language` ``。缺少非 `Option` 字段同样报错：``missing field `language` ``。
- 枚举的表示方式由属性决定（[serde 枚举表示](https://serde.rs/enum-representations.html)）。项目用“相邻标签”`#[serde(tag = "protocol", content = "settings")]`：标签和内容各占一个键。
- 解析成功只说明结构对得上；版本号、ID 不重复、端口不为 0 这些业务规则还要另外检查。编码会重新生成排版，手写的注释不会保留。

**在项目中**：主配置 `AppConfig`、设备文件 `ConfigDocument` 都派生了这两个 trait，并加了 `deny_unknown_fields`。读取顺序固定为“解析 → 业务校验”：

```rust
pub fn decode_app_config(text: &str) -> Result<AppConfig, ConfigIoError> {
    let config: AppConfig = toml::from_str(text)?;   // 结构对不上 → TomlDecode
    config.validate()?;                               // 版本、设置、引用规则
    Ok(config)
}
```

协议枚举在设备文件里写成（摘自 [home.toml](../../examples/configuration/devices/home.toml)）：

```toml
[device.connections.config]
protocol = "smb"

[device.connections.config.settings]
share_name = "data"
```

依赖写在 Cargo.toml（`serde = "1.0"`、`toml = "1.1"`），实际版本由 Cargo.lock 锁定，所以命令统一带 `--locked`。

### 2. 错误链：Display、source、From 与 ?

第 02 课的错误类型只有一层。读写配置时会同时遇到 I/O 错误、TOML 错误和业务规则错误，需要把它们包装成自己的错误类型。

官方示例一（[std::convert::From](https://doc.rust-lang.org/std/convert/trait.From.html)）：为每种下层错误实现 `From`，`?` 就会自动转换。

```rust
use std::fs;
use std::io;
use std::num;

enum CliError {
    IoError(io::Error),
    ParseError(num::ParseIntError),
}

impl From<io::Error> for CliError {
    fn from(error: io::Error) -> Self {
        CliError::IoError(error)
    }
}

impl From<num::ParseIntError> for CliError {
    fn from(error: num::ParseIntError) -> Self {
        CliError::ParseError(error)
    }
}

fn open_and_parse_file(file_name: &str) -> Result<i32, CliError> {
    let mut contents = fs::read_to_string(&file_name)?;   // io::Error → CliError
    let num: i32 = contents.trim().parse()?;               // ParseIntError → CliError
    Ok(num)
}
```

官方示例二（[std::error::Error::source](https://doc.rust-lang.org/std/error/trait.Error.html#method.source)）：`source` 借出下一层错误，形成错误链。

```rust
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct SuperError {
    source: SuperErrorSideKick,
}

impl fmt::Display for SuperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SuperError is here!")
    }
}

impl Error for SuperError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

#[derive(Debug)]
struct SuperErrorSideKick;

impl fmt::Display for SuperErrorSideKick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SuperErrorSideKick is here!")
    }
}

impl Error for SuperErrorSideKick {}

// 使用：
// println!("Error: {e}");                          → Error: SuperError is here!
// println!("Caused by: {}", e.source().unwrap());  → Caused by: SuperErrorSideKick is here!
```

- `?` 在返回 `Err` 前调用一次 `From::from`，只转换一步：A 要变成 C，需要 `From<A> for C`，不会自动经过中间类型 B。
- 需要补充上下文（哪个操作、哪个文件）时，用 `map_err` 包装：`fs::read_to_string(&path).map_err(|error| StoreError::io("read device", &path, error))?`。
- `dyn Error + 'static` 里的 `'static` 表示错误类型本身不含借用的数据，不表示返回的引用永远有效。
- `Display` 写给人看的摘要，细节留给 `source()`。toml 的错误信息会引用出错的原文：

  ```text
  TOML parse error at line 1, column 11
    |
  1 | SECRET = [
    |           ^
  unclosed array, expected `]`
  ```

  配置里有密码，所以项目里每一层的 `Display` 都只写摘要，不拼接下层错误的文本；终端报告也不递归打印 source。

**在项目中**：错误按模块分层，每层只包装下一层。错误类型和它的 `From` 实现写在接收方的模块里，例如 `From<ModelError> for ConfigIoError` 在 `config.rs`。

| 类型 | 位置 | 表达的失败 | 向上一层 |
|---|---|---|---|
| `ModelError` | `src/model/error.rs` | 地址、协议和模型操作规则 | 恢复快照时变成 `ConfigIoError::ModelValidation` |
| `ConfigError` | `src/config.rs` | 软件/日志设置、设备引用规则 | `ConfigIoError::Validation`，或直接变成 `AppError::Config` |
| `ConfigIoError` | `src/config.rs` | TOML 编解码、版本不支持、上面两类规则 | `StoreError::Config`，补上文件路径 |
| `StoreError` | `src/store.rs` | 文件操作失败（带操作名和路径）、配置内容错误（带路径） | `AppError::Store` |
| `AppError` | `src/app.rs`（只在 bin 中） | 参数、配置、存储、I/O、日志初始化 | `app::run` 打印摘要，参数错误退出码 2，其他为 1 |

```rust
// store.rs：读取失败补上操作和路径；内容错误补上路径
let text = std::fs::read_to_string(&self.config_path)
    .map_err(|error| StoreError::io("read configuration", &self.config_path, error))?;
decode_app_config(&text).map_err(|error| StoreError::config(&self.config_path, error))
```

### 3. 路径：Path 与 PathBuf

`Path` 与 `PathBuf` 的关系如同 `str` 与 `String`：参数用 `&Path`，需要保存时用 `PathBuf`。路径不保证是 UTF-8，打印时用 `path.display()`。

官方示例（[std::path](https://doc.rust-lang.org/std/path/index.html)、[Path::join](https://doc.rust-lang.org/std/path/struct.Path.html#method.join)、[Path::parent](https://doc.rust-lang.org/std/path/struct.Path.html#method.parent)、[Path::components](https://doc.rust-lang.org/std/path/struct.Path.html#method.components)）：

```rust
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

let path = Path::new("/tmp/foo/bar.txt");
assert_eq!(path.parent(), Some(Path::new("/tmp/foo")));
assert_eq!(path.file_stem(), Some(OsStr::new("bar")));
assert_eq!(path.extension(), Some(OsStr::new("txt")));

assert_eq!(Path::new("/etc").join("passwd"), PathBuf::from("/etc/passwd"));
assert_eq!(Path::new("/etc").join("/bin/sh"), PathBuf::from("/bin/sh"));   // 绝对路径整个替换

let mut components = Path::new("/tmp/foo.txt").components();
assert_eq!(components.next(), Some(Component::RootDir));
assert_eq!(components.next(), Some(Component::Normal(OsStr::new("tmp"))));
assert_eq!(components.next(), Some(Component::Normal(OsStr::new("foo.txt"))));
assert_eq!(components.next(), None);

let relative_path = Path::new("foo/bar");
let parent = relative_path.parent();
assert_eq!(parent, Some(Path::new("foo")));
let grand_parent = parent.and_then(Path::parent);
assert_eq!(grand_parent, Some(Path::new("")));   // 只剩一段时，parent 是空路径
```

- `join` 一个相对路径是追加；`join` 一个绝对路径会**整个替换**原路径。
- `components()` 把路径拆成 `Normal`、`ParentDir`（`..`）、`CurDir`（开头的 `.`）、`RootDir` 和 Windows 盘符 `Prefix`。它不会帮你消去 `..`：`./a/../b.toml` 拆成 `[CurDir, Normal("a"), ParentDir, Normal("b.toml")]`。
- 相对路径默认相对进程的当前工作目录（cwd）。
- 这些方法只处理文字，不访问磁盘；文件是否存在，要实际读取才知道。

**在项目中**：

- 设备路径必须“每一段都是 `Normal`”，这样就排除了 `..` 越出目录、绝对路径替换基准和盘符：

  ```rust
  pub(crate) fn is_managed_device_path(path: &Path) -> bool {
      !path.as_os_str().is_empty()
          && path.extension().is_some_and(|extension| extension == "toml")
          && path.components().all(|component| matches!(component, Component::Normal(_)))
  }
  ```

- 命令行 `--config` 的相对路径相对启动时的 cwd，`startup` 先用 `std::path::absolute` 转成绝对路径；配置文件里的设备路径和日志路径则相对主配置所在的目录。日志路径直接 `config_directory.join(&file_path)`：相对路径接在后面，绝对路径整个替换。

### 4. 文件读写与原子替换

官方示例（[fs::read_to_string](https://doc.rust-lang.org/std/fs/fn.read_to_string.html)、[The Rust Book 9.2](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)）：

```rust
use std::error::Error;
use std::fs;

fn main() -> Result<(), Box<dyn Error>> {
    let message: String = fs::read_to_string("message.txt")?;
    println!("{}", message);
    Ok(())
}
```

按错误种类分别处理：

```rust
use std::fs::File;
use std::io::ErrorKind;

let greeting_file = match File::open("hello.txt") {
    Ok(file) => file,
    Err(error) => match error.kind() {
        ErrorKind::NotFound => match File::create("hello.txt") {
            Ok(fc) => fc,
            Err(e) => panic!("Problem creating the file: {e:?}"),
        },
        _ => panic!("Problem opening the file: {error:?}"),
    },
};
```

写入相关（[OpenOptions::create_new](https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.create_new)、[File::sync_all](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all)、[fs::rename](https://doc.rust-lang.org/std/fs/fn.rename.html)）：

```rust
use std::fs::{self, File, OpenOptions};
use std::io::prelude::*;                 // 引入 Write trait，才能调用 write_all

let file = OpenOptions::new()
    .write(true)
    .create_new(true)                    // 目标已存在（包括悬空的符号链接）时失败，检查和创建一次完成
    .open("foo.txt");

let mut f = File::create("foo.txt")?;    // 不存在就创建，存在就截断
f.write_all(b"Hello, world!")?;
f.sync_all()?;                           // 要求系统把数据和元数据写到磁盘后再返回

fs::rename("a.txt", "b.txt")?;           // 改名；b.txt 已存在时被替换
```

- 只有 `ErrorKind::NotFound` 才表示“不存在”。权限不足、路径是目录、内容损坏都是真正的错误，不能当成缺失后悄悄换成默认值。
- `fs::write` 和 `File::create` 会先截断目标，写到一半失败就留下残缺文件。
- **原子替换**把这些组合起来：在同一目录 `create_new` 一个临时文件 → `write_all` → `sync_all` → 关闭 → `rename` 覆盖目标。同一文件系统内的 rename 由操作系统一次完成，读者要么看到完整的旧文件，要么看到完整的新文件。任何一步失败都删掉自己创建的临时文件，返回最初的错误；不能先删旧文件，也不能删别人的文件。
- `rename` 不能跨挂载点，所以临时文件要和目标放在同一目录。

**在项目中**：

- `load_app` 只把 `NotFound` 换成内置默认值，其他错误照常返回；`load_app_required` 用于 `--config`，文件必须存在。
- `write_atomic`（P4.6）实现上面的原子替换，`temporary_path` 已经写好。
- 多个文件之间没有事务。新增设备时先写设备文件、再写主配置：主配置写失败只会留下一个没被引用的设备文件，而不会出现“引用了不存在的文件”。
- 内存里的 `AppConfig` 只在磁盘写入成功后更新：先修改 `clone()` 出来的候选副本，成功后再整体替换。
- 当前不提供：目录 fsync（所以不承诺掉电后新文件一定保留）、多文件事务、多个写入者合并、配置文件监听。

### 5. tracing 日志

tracing 只负责**产生**诊断信息；tracing-subscriber 决定**过滤和输出**。没有安装 subscriber 时，事件直接丢弃。

官方示例（[tracing 文档](https://docs.rs/tracing/latest/tracing/)）：

```rust
use tracing::{event, info, span, Level};

// span：一段有开始和结束的操作
let span = span!(Level::TRACE, "my_span");
// enter 返回一个 RAII guard，guard 被 drop 时退出 span
let _enter = span.enter();

// event：某一时刻发生的事
event!(Level::INFO, "something happened");

// info! 是 event!(Level::INFO, ...) 的简写；等号左边是字段名
let question = "the answer to the ultimate question of life, the universe, and everything";
let answer = 42;
info!(question.answer = answer, question.tricky = true, "the answer to {} is {}.", question, answer);

#[derive(Debug)]
struct MyStruct {
    field: &'static str,
}
let my_struct = MyStruct { field: "Hello world!" };
event!(Level::TRACE, greeting = ?my_struct);         // ? 用 Debug 记录
event!(Level::TRACE, greeting = %my_struct.field);   // % 用 Display 记录
```

安装 subscriber（[tracing-subscriber 文档](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html)）：

```rust
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)           // DEBUG 及以上输出，TRACE 被过滤
    .init();                                         // 全局只能安装一次
```

多个输出、各自过滤（改编自[分层过滤](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/index.html#per-layer-filtering)）：

```rust
use tracing_subscriber::{filter::LevelFilter, prelude::*};

let stdout_log = tracing_subscriber::fmt::layer()
    .with_filter(LevelFilter::INFO);                 // 终端只要 INFO 及以上
let file_log = tracing_subscriber::fmt::layer()
    .with_writer(std::sync::Mutex::new(file))        // 写进一个已打开的 File
    .with_ansi(false)
    .with_filter(LevelFilter::TRACE);                // 文件要全部
tracing_subscriber::registry().with(stdout_log).with(file_log).init();
```

- 级别从高到低：error、warn、info、debug、trace。
- `enter()` 返回的 guard 要用具名变量保存（如 `_enter`）；写成 `let _ = span.enter();` 会立即 drop，等于没进入。
- `EnvFilter` 解析过滤指令，常见写法：`info`、`nasnet=debug`（只针对某个 target）、`off,nasnet::app_error=trace`。约定从环境变量 `RUST_LOG` 读取。

**在项目中**（[app/logging.rs](../../src/app/logging.rs)）：

- 一个 registry 装两个 `fmt::layer`，**每层各自过滤**：文件默认 TRACE，终端默认 INFO。如果把 INFO 过滤器放在整个 registry 上，TRACE 事件在到达文件层之前就被丢掉了。
- `RUST_LOG` 只覆盖终端层的过滤，文件层仍按配置。
- 文件以 `OpenOptions::append` 打开，用 `Mutex<File>` 同步写入，不输出颜色控制符；终端日志写 stderr，命令结果写 stdout。
- 命令失败的事件使用专用 target `nasnet::app_error`，只进文件层；终端由 `app::run` 直接打印一次摘要，所以即使终端日志设为 off，也能看到失败原因。
- 日志不记录密码、配置正文或可能复述输入的解析错误。当前没有日志轮转。

Cargo feature 是编译期的选择：`tracing = { version = "0.1", default-features = false, features = ["std"] }` 关掉了默认的 `attributes`（提供 `#[instrument]`，本项目不用）；tracing-subscriber 的 `env-filter` 提供 `EnvFilter`，`fmt` 提供格式化输出。日志级别则是运行时的设置。

### 6. 测试

官方示例（[The Rust Book 11.1](https://doc.rust-lang.org/book/ch11-01-writing-tests.html)）：

```rust
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]                  // 只在 cargo test 时编译这个模块
mod tests {
    use super::*;             // 引入外层的条目，包括私有的

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn it_works_with_result() -> Result<(), String> {
        if add(2, 2) == 4 {
            Ok(())
        } else {
            Err(String::from("two plus two does not equal four"))
        }
    }
}
```

集成测试（[The Rust Book 11.3](https://doc.rust-lang.org/book/ch11-03-test-organization.html)）放在包根目录的 `tests/` 下，每个文件单独编译，像外部用户一样只能使用库的公开 API：

```rust
// tests/integration_test.rs
use adder::add_two;

#[test]
fn it_adds_two() {
    let result = add_two(2);
    assert_eq!(result, 4);
}
```

共用代码放在 `tests/common/mod.rs`（不是 `tests/common.rs`），Cargo 就不会把它当成一个测试文件。

临时目录（[tempfile 文档](https://docs.rs/tempfile/latest/tempfile/)）：

```rust
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

let tmp_dir = tempdir()?;                            // 在系统临时目录下新建一个目录
let file_path = tmp_dir.path().join("my-temporary-note.txt");
let mut tmp_file = File::create(file_path)?;
writeln!(tmp_file, "Brian was here. Briefly.")?;
drop(tmp_file);
tmp_dir.close()?;                                    // 不调用的话，tmp_dir 离开作用域时也会删除目录
```

运行外部程序（[std::process::Command](https://doc.rust-lang.org/std/process/struct.Command.html)）：

```rust
use std::process::Command;

let output = Command::new("echo")
    .arg("Hello world")
    .output()                                        // 等待退出，收集 status、stdout、stderr
    .expect("Failed to execute command");
assert_eq!(b"Hello world\n", output.stdout.as_slice());
```

- 断言：`assert!(条件)`、`assert_eq!(实际, 预期)`（失败时打印两边），都可以在最后加一句失败说明；检查具体错误变体用 `assert!(matches!(值, 模式))`。
- 测试返回 `Result` 时，可以用 `?` 处理意外错误，返回 `Err` 即失败。预期中的错误要用断言检查，不要用 `?`。
- 测试默认并行运行：不要共享文件名，也不要修改测试进程的 cwd 或环境变量；需要时用 `Command` 的 `current_dir`、`env` 只设置子进程。
- 先写测试并看到它失败（`todo!()` 会让测试 panic），再实现到通过。`0 tests`、`ignored`、`filtered out` 都不算通过；不要用 `#[should_panic]` 让占位代码“通过”。

```sh
cargo test --locked                                   # 全部测试
cargo test --locked --test course04_store             # 只运行 tests/course04_store.rs
cargo test --locked --test course04_store load_device # 再按测试名过滤
cargo test --locked --example course04                # 运行 example 里的测试
cargo test --locked -- --nocapture                    # 显示测试中的打印
```

**在项目中**：

- `tests/common/mod.rs` 定义 `TestResult = Result<(), Box<dyn Error>>`，并用 `include_str!` 在编译期读入示例主配置 `APP_CONFIG`；`tests/common/model.rs` 提供测试设备，用 `#[path = "common/model.rs"] mod fixtures;` 引入。
- 文件测试每例一个 `tempfile::tempdir()`，从不改动 `examples/configuration/`。
- `tests/course04_cli.rs` 的 `Cli` 夹具用 `Command::new(env!("CARGO_BIN_EXE_nasnet"))` 启动真正的程序：`CARGO_BIN_EXE_<name>` 是 Cargo 在编译集成测试时提供的二进制路径。

### 7. 模块、lib 与 bin、启动顺序

一个包（package）最多一个库 crate，可以有多个二进制 crate；同时有 `src/main.rs` 和 `src/lib.rs` 时，就有两个与包同名的 crate（[The Rust Book 7.1](https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html)）。

把模块拆到文件（[The Rust Book 7.5](https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html)）：

```rust
// src/lib.rs
mod front_of_house;                       // 编译器去找 src/front_of_house.rs

pub use crate::front_of_house::hosting;   // 重新导出：外部可以直接用 crate::hosting

pub fn eat_at_restaurant() {
    hosting::add_to_waitlist();
}

// src/front_of_house.rs
pub mod hosting;                          // 对应 src/front_of_house/hosting.rs

// src/front_of_house/hosting.rs
pub fn add_to_waitlist() {}
```

进程退出码（[std::process::ExitCode](https://doc.rust-lang.org/std/process/struct.ExitCode.html)）：

```rust
use std::process::ExitCode;

fn main() -> ExitCode {
    if !check_foo() {
        return ExitCode::from(42);
    }
    ExitCode::SUCCESS
}
```

命令行参数：`std::env::args()` 遇到非 UTF-8 参数会 panic；`std::env::args_os()` 返回 `OsString`，不会 panic。

**在项目中**：`src/lib.rs` 公开 `nasnet::model`、`nasnet::config`、`nasnet::store`；`src/main.rs` 带一个私有模块 `app`，通过 `nasnet::...` 使用库，库不知道命令行、日志和退出码。`model.rs` 里的 `mod device;` 对应 `model/device.rs`，`pub use self::device::Device;` 让外部写 `nasnet::model::Device`，子模块本身保持私有。bin 这边，`app.rs` 里的 `mod startup;`、`mod logging;` 对应 `app/startup.rs`、`app/logging.rs`：只有能独立成段、逻辑足够多的部分才拆成文件，启动流程和 `AppError` 留在 `app.rs`。

启动只有一条路径：

```text
main → app::run（唯一打印失败摘要、决定退出码的地方）
         → execute
             1. startup::prepare   先解析参数（不访问文件），再定位并读取主配置、确定日志路径并检查冲突
             2. logging::init      创建日志目录、打开日志文件、安装 subscriber
             3. show               打印主配置；有 --device 时只读取这一个设备
```

- 默认主配置是可执行文件旁的 `config.toml`，不存在时使用内置默认值，不创建文件；`--config` 指定的文件必须存在。
- 任何一步失败都经 `?` 回到 `app::run`：参数错误退出码为 2，其他失败为 1。前一步失败，后面的步骤不会执行。
- 参数用 `args_os` 接收，因为路径不一定是 UTF-8；设备 ID 则要求是合法的 Unicode。

## 二、示例练习：examples/course04.rs

```sh
cargo run --locked --example course04
```

用一个与项目无关的“歌单”练习上面的知识点。每道题的具体要求写在对应函数的 TODO 注释里；main 依次检查 E4.1–E4.5，遇到未完成的题会停在那里并显示题号。卡住时可以暂时注释掉 main 里对应的调用。

- **E4.1 TOML 往返**（知识 1）：实现 `encode_playlist`、`decode_playlist`。通过后对照打印出的 TOML 回答：`[[songs]]` 从哪来？第二首歌的 artist 去哪了？
- **E4.2 错误链**（知识 2）：补全 `PlaylistError` 的 `Display`、`source`、`From`，再实现 `load_playlist`。通过后回答：程序打印的 toml 原始错误里有什么，为什么 Display 不能拼接它？
- **E4.3 路径**（知识 3）：先运行，看打印出的 `parent()` 和 `components()`，再实现 `resolve_beside`。
- **E4.4 文件**（知识 4）：实现 `read_optional`、`write_new`。检查函数最后用 `rename` 替换文件，P4.6 会用到这一步。
- **E4.5 tracing**（知识 5）：按注释的四步记录事件。运行后回答：trace 事件为什么没有出现？最后一条 info 属于哪个 span？
- **E4.6 为示例写测试**（知识 6）：按文件末尾注释写三个测试，并说明第 2 个测试为什么要求启动时先把主配置路径变成绝对路径。

  ```sh
  cargo test --locked --example course04 -- --list   # 应列出 3 个测试
  cargo test --locked --example course04             # 3 passed
  ```

## 三、项目练习

### 项目约定

格式以三个示例配置为准：[主配置](../../examples/configuration/config.toml)、[完整设备](../../examples/configuration/devices/home.toml)、[空设备](../../examples/configuration/devices/backup.toml)。主配置 `AppConfig` 只有 `schema_version`、`[software]`、`[logging]` 和有序的 `[[devices]]` 引用（id 与 path）；每个设备文件是一个 `ConfigDocument`：`schema_version` 加 `[device]`。

`StoreConnection` 只保存主配置路径：不缓存设备，不扫描目录自动登记；运行时状态（连接结果、目录内容、传输进度）不写入配置。测试需要示例文件时，用 `include_str!` 读入内容再写进临时目录。

### 阶段 1：配置转换（src/config.rs，测试写在 tests/course04_config.rs）

只处理内存中的值和文本，不碰文件。

#### P4.1 编码主配置 `encode_app_config`

先读：`AppConfig::validate`、`LoggingSettings::validate`、`decode_app_config`。

实现：先 `config.validate()?`，再 `toml::to_string_pretty`。只编码主配置本身，不嵌入设备内容。

测试：`common::APP_CONFIG` 解码 → 编码 → 再解码，逐项比较语言、日志设置以及设备引用的 id、path 和顺序（不比较注释和排版）；非法候选（如 console_level 为 `"verbose"`）返回 `ConfigIoError::Validation(ConfigError::InvalidLoggingSettings)`。

#### P4.2 编码单设备 `encode_device_toml`

先读：第 03 课的 `DeviceData::from`，以及 `ConfigDocument`、`SCHEMA_VERSION`。

实现：构造 `ConfigDocument { schema_version: SCHEMA_VERSION, device: DeviceData::from(device) }` 后编码。

测试：用 `tests/common/model.rs` 的测试设备，断言编码结果包含 `schema_version = 1`、两个地址 ID，以及保留首尾空格的密码 `" demo-only-password "`。

#### P4.3 解码并恢复单设备 `decode_device_toml`

实现：`toml::from_str` 解析成 `ConfigDocument` → 版本不是 `SCHEMA_VERSION` 时返回 `UnsupportedVersion` → `Device::try_from(document.device)?`（`ModelError` 经 `From` 变成 `ConfigIoError::ModelValidation`）。不要让 `Device` 直接派生 `Deserialize`。

测试：

- 解码 `home.toml`（ID、地址顺序、SMB 密码的空格）和 `backup.toml`（空列表），在测试文件中用 `include_str!("../examples/configuration/devices/home.toml")` 读入；
- P4.2 编码的结果能解码回同一台设备；
- 多余字段、未知协议 `protocol = "ftp"` → `TomlDecode`；`schema_version = 99` → `UnsupportedVersion(99)`；host 为 `https://nas` → `ModelValidation(ModelError::HostContainsSchemeOrPath)`。

### 阶段 2：读取设备（src/store.rs，测试写在 tests/course04_store.rs）

#### P4.4 解析设备路径 `resolve_device_path`

先读：`config::is_managed_device_path`。

实现：路径形式不合法时返回 `StoreError::config(主配置路径, ConfigError::InvalidDeviceReference.into())`；主配置路径没有父目录时返回 `StoreError::InvalidConfigPath`；否则返回主配置所在目录 join 引用路径。不检查文件是否存在。

测试：`StoreConnection::new(临时目录)` 把 `devices/home.toml` 解析为 `临时目录/devices/home.toml`；`../a.toml`、`/a.toml`、`a.txt` 返回 `StoreError::Config { source: ConfigIoError::Validation(ConfigError::InvalidDeviceReference), .. }`。

#### P4.5 加载单个设备 `load_device`

先读：`load_app_required` 怎样给错误补上操作名和路径；`load_app` 为什么只把 NotFound 换成默认值，`app/startup.rs` 为什么对 `--config` 用前者、对默认路径用后者。

实现：解析路径 → `read_to_string`，失败时返回 `StoreError::io("read device", &path, error)` → `decode_device_toml`，失败时返回 `StoreError::config(&path, error)` → 设备 ID 与引用不一致时返回 `StoreError::config(&path, ConfigError::DeviceIdMismatch.into())`。

测试：把示例主配置和两个设备文件写进临时目录，加载 nas-home 成功；backup.toml 损坏不影响加载 nas-home；home.toml 缺失时返回 `StoreError::Io`，`source.kind()` 为 NotFound，路径就是该文件；文件内 ID 与引用不一致时返回带该路径的 `StoreError::Config`。

完成后可以运行完整链路：

```sh
cargo run --locked -- --config examples/configuration/config.toml --device nas-home
```

### 阶段 3：写入文件（src/store.rs，测试写在 tests/course04_store.rs）

#### P4.6 原子替换 `write_atomic`

先读：知识 4 的原子替换，以及 `temporary_path`。

实现：父目录不存在时创建 → 用 `temporary_path` 得到同目录的临时路径，`create_new` 打开 → `write_all` → `sync_all` → 关闭 → `rename`。写入或 rename 失败时尽力删除临时文件（删除失败可以忽略），返回最初的错误。每一步的 I/O 错误都用 `StoreError::io(操作, 路径, error)` 写明是哪一步、哪个文件。

理解：`temporary_path` 用进程号和 `static AtomicU64` 序号生成唯一名称；`Ordering::Relaxed` 足够，因为这里只要求每次取到不同的数；`create_new` 防止打开已有文件；临时名称不是文件锁。

测试：写入带有不存在父目录的新文件；再次写入替换内容；成功后目录里没有残留的临时文件；目标路径是一个非空目录时返回错误，目录里原有的文件不变，也没有残留的临时文件。

#### P4.7 保存主配置 `save_app`

实现：`encode_app_config`（失败时用 `StoreError::config` 带上主配置路径）→ `write_atomic`。编码失败时不写文件。

测试：保存后 `load_app_required` 读回相同的设置；保存前后设备文件的字节不变；非法配置返回错误，旧主配置的字节不变。

#### P4.8 保存单设备 `save_device`

实现：解析路径 → 设备 ID 与引用不一致时返回 `StoreError::config(&path, ConfigError::DeviceIdMismatch.into())`，不写文件 → `encode_device_toml` → `write_atomic`。

测试：保存后 `load_device` 能读回；主配置和另一个设备文件的字节不变；ID 不一致时目标文件不存在。

### 阶段 4：登记与移除（src/store.rs，测试写在 tests/course04_store.rs）

#### P4.9 登记设备 `register_device`

实现顺序：

1. `config.clone()` 得到候选副本，追加引用后 `validate()`，失败时用 `StoreError::config(主配置路径, error)`：重复 ID、重复路径、非法路径都在这里被拒绝；
2. 新路径上已有文件时拒绝（用 `try_exists` 检查），返回 `StoreError::Io`，其 source 的 kind 为 `AlreadyExists`，不覆盖未登记的文件；
3. `save_device`（它会核对 ID）→ `save_app(&候选)`；
4. 都成功后 `*config = 候选`。

测试：成功时设备文件和主配置都更新，内存中多一条引用；重复 ID 时不写任何文件、内存不变；新路径已有文件时那个文件不变；主配置无法写入时（把主配置路径建成一个目录即可模拟）返回错误、内存不变，并断言设备文件已经写入但没有被引用——这是“先写设备、后发布引用”的预期结果。

#### P4.10 移除设备引用 `remove_device_reference`

实现：找不到 ID 返回 `Ok(false)`，不写文件；找到时从候选副本移除并 `save_app`，成功后替换内存，返回 `Ok(true)`。设备文件保留，不删除。

测试：移除后主配置不再引用该设备、设备文件仍在；不存在的 ID 返回 false 且主配置字节不变；保存失败时内存不变。

### 阶段 5：启动与日志（src/main.rs、src/app/，测试写在 tests/course04_cli.rs）

#### P4.11 跟踪启动链路

先读 `main.rs` 和 `app.rs`（从上到下依次是 `run`、`execute`、`show` 和 `AppError`），再读 `app/startup.rs`、`app/logging.rs`，对照知识 7 的调用链写下：

- 三个步骤各自的输入、输出和文件副作用；
- 一个损坏的主配置怎样经过 `ConfigIoError` → `StoreError` → `AppError` 到达终端，为什么终端看不到原始输入；
- 为什么参数错误时不读配置、配置错误时不打开日志、日志初始化失败时不执行命令。

用 `argument_validation_precedes_configuration_and_logging`、`malformed_configuration_is_reported_without_input_or_file_changes`、`logging_initialization_failure_prevents_command_execution` 三个测试验证你的结论。

#### P4.12 为 --device 补端到端测试

P4.5 完成后，`--device` 第一次能真正加载设备，但还没有测试从真实程序的角度覆盖它。先读 `Cli` 夹具（`command`、`write_config`、`log`），然后写一个测试：

1. 在夹具的配置目录下创建 `devices/home.toml`（内容用 `include_str!` 读入示例文件），运行 `--device nas-home`：成功退出，stdout 有 `Selected device: Home NAS (nas-home)`，日志文件里有 `selected device loaded`；
2. 把 home.toml 改成 `TEST_SECRET_SENTINEL = [` 再运行：退出码为 1，stderr 包含 `devices/home.toml` 和 `Invalid TOML configuration`，但不包含 `TEST_SECRET_SENTINEL`；日志里只有一条 `application failed`。

## 四、验收

```sh
cargo run --locked --example course04
cargo test --locked --example course04
cargo test --locked
```

- E4.1–E4.6、P4.1–P4.12 全部完成，`src/` 里没有剩余的 `todo!()`；
- 每道 P 题都有自己的行为测试，覆盖成功、失败和失败后的状态；
- 能解释：单设备保存为什么不碰其他文件；损坏的配置为什么不能被默认值掩盖；登记设备时主配置写失败会留下什么；日志的两层为什么分别过滤。
