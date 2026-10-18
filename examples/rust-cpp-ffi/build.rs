fn main() {
    // C++ 源码或头文件改变时，让 Cargo 重新编译这个库。
    println!("cargo:rerun-if-changed=native/math.cpp");
    println!("cargo:rerun-if-changed=native/math.hpp");

    // cc 负责调用本机 C++ 编译器；在 Windows MSVC 环境中使用 cl.exe。
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .flag_if_supported("/utf-8")
        .include("native")
        .file("native/math.cpp")
        .compile("nasnet_math");
}
