//! NASNet 入口：12 课安排见 docs/Course-Syllabus.md。
//! 第 01 课：设备模型、所有权、查询与排序。
//! 第 02 课：错误处理与独立 NFS/SMB 配置。
//! 第 03 课：封装、生命周期与配置序列化。
//! 第 04 课：完整源码、统一错误、tracing 与 config.toml / 独立设备 TOML 持久化。
//! 第 05 课：C FFI、指针与资源安全封装。
//! 第 06 课：真实 NFS 与 SMB 协议接入。
//! 第 07 课：统一远程能力与异步调用。
//! 第 08 课：会话管理、取消与地址故障切换。
//! 第 09 课：终端应用、独立后台与本地 IPC。
//! 第 10 课：Tauri/Vue 桌面与配置管理。
//! 第 11 课：远程浏览与系统挂载。
//! 第 12 课：自动恢复、跨平台打包与交付。
//! 每课包含全部相关源码阅读和理解验收；模型和存储不输出日志。

mod app;

fn main() -> std::process::ExitCode {
    if let Err(error) = app::logging::init() {
        // 订阅器尚未建立时，入口负责最后的英文错误展示。
        eprintln!("{error}");
        return std::process::ExitCode::FAILURE;
    }
    match app::run(std::env::args_os().skip(1).collect()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            app::logging::report_error(&error);
            std::process::ExitCode::from(error.exit_code() as u8)
        }
    }
}
