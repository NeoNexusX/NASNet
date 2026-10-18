#pragma once

#include <cstddef>
#include <cstdint>

// 使用 C ABI 导出函数，让 Rust 能以稳定的接口调用 C++ 实现。
extern "C" std::int64_t nasnet_add(std::int32_t left, std::int32_t right) noexcept;

// 解析十进制整数字符串。text/len 描述只读字节串，out_value 接收解析结果。
// 返回状态码：0 成功；1 不是整数；2 超出 64 位范围。
extern "C" std::int32_t nasnet_parse_i64(
    const char* text, std::size_t len, std::int64_t* out_value) noexcept;
