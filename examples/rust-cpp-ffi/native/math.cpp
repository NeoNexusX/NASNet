#include "math.hpp"

#include <stdexcept>
#include <string>

extern "C" std::int64_t nasnet_add(std::int32_t left, std::int32_t right) noexcept {
    // 先扩展为 64 位再相加，保证任意两个 32 位整数的和都能被表示。
    return static_cast<std::int64_t>(left) + static_cast<std::int64_t>(right);
}

extern "C" std::int32_t nasnet_parse_i64(
    const char* text, std::size_t len, std::int64_t* out_value) noexcept {
    if (text == nullptr || out_value == nullptr) {
        return 1; // 防御：调用方违反约定时，归入“不是整数”。
    }
    try {
        // std::string 用 (指针, 长度) 构造，不依赖 \0 结尾。
        const std::string input(text, len);
        std::size_t parsed = 0;
        const std::int64_t value = std::stoll(input, &parsed);
        if (parsed != input.size()) {
            return 1; // "42abc"、"3.14" 这类只解析了一部分的情况。
        }
        *out_value = value;
        return 0;
    } catch (const std::invalid_argument&) {
        return 1; // 完全不是数字，例如 "abc"。
    } catch (const std::out_of_range&) {
        return 2; // 是数字，但超出 64 位范围。
    } catch (...) {
        // 兜底网：任何未预料到的异常都必须在这里停下，绝不允许
        // 穿过 extern "C" 边界进入 Rust。
        return 1;
    }
}
