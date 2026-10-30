//! 进程入口；启动顺序与错误处理统一交给 app。

mod app;

fn main() -> std::process::ExitCode {
    app::run(std::env::args_os().skip(1))
}
