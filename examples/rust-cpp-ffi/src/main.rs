use std::ffi::{c_char, CString};

// 声明 C++ 库提供的函数。这里没有 Rust 函数体，实现在 native/math.cpp。
unsafe extern "C" {
    fn nasnet_add(left: i32, right: i32) -> i64;
    // text/len 描述一个只读字节串；out_value 是调用方提供的输出位置。
    fn nasnet_parse_i64(text: *const c_char, len: usize, out_value: *mut i64) -> i32;
}

// 解析失败的原因。Rust 错误类型的惯例三件套是 Debug + Display + Error，
// 前两个在这里实现，Error 留作 README 第二课的练习。
#[derive(Debug)]
enum ParseError {
    NotANumber(String),
    OutOfRange(String),
    Unknown(i32),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::NotANumber(text) => write!(f, "不是整数: {text:?}"),
            ParseError::OutOfRange(text) => write!(f, "超出 64 位范围: {text:?}"),
            ParseError::Unknown(code) => write!(f, "C++ 返回了未知状态码: {code}"),
        }
    }
}

// 用普通 Rust 函数包住 FFI 调用，把 C++ 的状态码翻译成 Rust 的 Result。
fn parse_i64(text: &str) -> Result<i64, ParseError> {
    // CString 保证以 \0 结尾；若 text 中间本身含 \0，构造会失败，
    // 因为 C 字符串无法表示它——这里直接按“不是整数”处理。
    let c_text = CString::new(text).map_err(|_| ParseError::NotANumber(text.to_string()))?;

    let mut value: i64 = 0;
    // SAFETY: c_text.as_ptr() 指向合法缓冲区，text.len() 是它的字节数；
    // &mut value 指向调用方拥有的合法变量。C++ 侧 noexcept，异常不会越过边界。
    let status = unsafe { nasnet_parse_i64(c_text.as_ptr(), text.len(), &mut value) };

    match status {
        0 => Ok(value),
        1 => Err(ParseError::NotANumber(text.to_string())),
        2 => Err(ParseError::OutOfRange(text.to_string())),
        code => Err(ParseError::Unknown(code)),
    }
}

// 用普通 Rust 函数包住 FFI 调用，业务代码只需要调用 add。
fn add(left: i32, right: i32) -> i64 {
    // SAFETY: 声明与 C++ 的 C ABI、参数和返回类型一致；函数接受所有 i32 值，
    // 不涉及指针或共享状态，也不会向 Rust 抛出 C++ 异常。
    unsafe { nasnet_add(left, right) }
}

fn main() {
    // 第一课：整数加法。
    let left = 3;
    let right = 5;
    let result = add(left, right);
    println!("C++ returned: {left} + {right} = {result}");

    // 第二课：把字符串交给 C++ 解析，失败情况以 Result 返回。
    for input in ["42", "-7", "3.14", "abc", "999999999999999999999"] {
        match parse_i64(input) {
            Ok(value) => println!("parse_i64({input:?}) = {value}"),
            Err(error) => println!("parse_i64({input:?}) 失败: {error}"),
        }
    }
}
