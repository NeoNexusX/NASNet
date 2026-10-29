# NASNet 技术设计

本文描述目标产品的技术边界。当前实现模型、主配置读取、启动和日志；设备持久化及真实协议仍未完成。具体状态见[实现计划](Implementation-Plan.md)，代码位置见[源码地图](Code-Reading-Guide.md)。

## 一、功能范围

- 本机安装的跨平台图形界面，使用 Vue 与 Tailwind CSS 构建。
- 管理 NAS 设备，每台设备配置多个 IP 地址或域名，调整连接优先级。
- 按优先级连接，地址不可达时自动尝试后续地址。
- 按 SMB、NFS 分别配置连接、认证与权限信息。
- 在界面中浏览远程目录和受支持的文件内容。
- 将共享挂载为本地盘符或目录，供其他应用使用。
- 开机后的自动连接与挂载。

产品设置只包含设备、地址、协议连接、权限和挂载。地址记录不包含网络类型、地址来源、使用环境、网络依赖或出口网卡。程序只负责连接用户提供的地址。

## 二、技术栈

| 部分 | 选型 | 职责 |
|---|---|---|
| 桌面框架 | Tauri 2 | 桌面窗口、应用集成与前后端通信 |
| 前端 | Vue 3、TypeScript、Vite | 设备配置、地址排序、协议表单、文件和权限界面 |
| 样式与组件 | Tailwind CSS、shadcn-vue | 界面布局、表单、列表、弹窗 |
| 后台 | Rust、Tokio | 连接选择、协议访问、自动连接和挂载 |
| 配置存储 | config.toml + 每台设备一个 TOML | 主文件保存软件设置、日志设置与设备路径，设备文件保存地址和协议配置 |
| 凭据存储 | 各自设备 TOML 中明文保存 | SMB 账号密码原样保留；日志隐藏密码 |
| 日志 | tracing、tracing-subscriber、Mutex<File> | 单个 registry、终端/文件独立过滤、同步追加写入和英文错误摘要 |
| 进程通信 | 本地 IPC | 桌面界面与独立用户后台通信 |
| NFS 访问 | libnfs，经 Rust FFI 封装 | NFS 目录、文件读取及受支持的权限操作 |
| SMB 访问 | libsmb2，经 Rust FFI 封装 | SMB2/3 目录、文件读取及受支持的安全信息操作 |
| 系统挂载 | Windows、macOS、Linux 分别实现 | 调用平台挂载接口 |

Tauri 官方推荐通过 Vite 接入 Vue：[前端配置](https://v2.tauri.app/start/frontend/)。协议后端固定使用 [libnfs](https://github.com/sahlberg/libnfs) 与 [libsmb2](https://github.com/sahlberg/libsmb2)，FFI 和构建方式见第十三节。具体协议版本、认证和权限接口以锁定的库版本与实际后端能力为准。

磁盘主配置保存软件/日志设置与有序设备引用，每台设备独立使用 TOML 文件。启动先读取主文件，再初始化日志；选择设备时才读对应文件。默认路径、命令和日志选项见 [README](../README.md)，保存与失败边界见[第 04 课 B](lessons/04-errors-logging-and-toml-storage.md#part-b)。

## 三、运行结构

~~~mermaid
flowchart TD
    UI[Vue + Tailwind 界面] --> Agent[Rust 用户后台]
    Agent --> Router[连接路由层：按地址优先级尝试]
    Router --> Protocol[协议适配层]
    Protocol --> NFS[Rust NFS 适配器：连接与权限模型]
    Protocol --> SMB[Rust SMB 适配器：连接与权限模型]
    NFS --> NfsLib[libnfs：通过 FFI 调用]
    SMB --> SmbLib[libsmb2：通过 FFI 调用]
    NfsLib --> Files[远程目录与文件内容浏览]
    SmbLib --> Files
    Agent --> Mount[系统挂载层]
    Router --> Mount
    Mount --> OS[Windows / macOS / Linux 挂载后端]
    Agent --> Store[配置与凭据存储]
~~~

用户后台是当前用户身份下独立运行的本机程序。它在界面关闭后仍可负责自动连接与挂载。需要权限的系统挂载操作使用受限的系统辅助程序，普通界面与文件浏览不整体提权。

开机自动连接的默认实现是用户登录后启动后台，按保存的配置连接并挂载。只有需要无人登录时也挂载的要求明确提出后，才设计系统级运行方式。

## 四、公共模型只保存公共信息

以下 TypeScript 结构描述规划中的 UI/IPC 数据；当前磁盘结构以 Rust 配置类型和第九节的 TOML 文件为准。

~~~typescript
interface Address {
  id: string
  host: string          // IPv4、IPv6 或域名
  priority: number      // 数值越小，连接优先级越高
}

interface Device {
  id: string
  name: string
  addresses: Address[]
  connections: ProtocolConnection[]
}

type ProtocolConnection =
  | {
      id: string
      deviceId: string
      protocol: "nfs"
      config: NfsConfig
    }
  | {
      id: string
      deviceId: string
      protocol: "smb"
      config: SmbConfig
    }
~~~

地址的用户配置只有 host 和 priority；id 是存储与引用所需的内部标识。协议端口、认证、远程路径与权限配置属于协议连接，不属于地址，也不统一放在 Device 上。

同一个设备可以包含多个 NFS 导出或 SMB 共享。它们复用设备地址列表，但各自使用协议配置与身份。SMB 连接失败不会自动改用 NFS，反之亦然。

优先级相同时，按保存顺序稳定排序。界面拖拽改序后可以重新生成优先级。地址是域名时，由系统解析；多个 DNS 结果仍属于该地址记录，不被自动保存成新的用户地址条目。

## 五、NFS 连接模型

NFS 不能统一套用用户名与密码模型。以下配置以 AUTH_SYS 身份方式为例：

~~~typescript
interface NfsConfig {
  exportPath: string
  version: string       // 选择值必须由后端能力校验
  port?: number
  authentication: {
    type: "sys"
    uid: number
    gid: number
    supplementaryGroups: number[]
  }
}
~~~

| 字段 | 含义 |
|---|---|
| exportPath | NFS 导出路径，例如 /volume1/data |
| version | NFS 协议版本 |
| port | NFS 服务端口；省略时按协议后端规则处理 |
| uid | AUTH_SYS 请求使用的用户标识 |
| gid | AUTH_SYS 请求使用的主组标识 |
| supplementaryGroups | 请求携带的附加组 |
| authentication | NFS 认证配置，必须使用对应安全方式的独立结构 |

AUTH_SYS 按 UID/GID 等身份信息进行访问检查，libnfs 也提供设置 UID/GID 的入口：[libnfs 配置说明](https://github.com/sahlberg/libnfs)、[NFS 客户端认证说明](https://www.man7.org/linux/man-pages/man5/nfs.5.html)。若实际选择其他 NFS 安全方式，应使用该方式对应的认证结构及后端实现，不能直接复用 AUTH_SYS 的 UID/GID 字段。本示例不声明实现其他认证方式。

NFS 版本与安全方式分别建模，不能根据 NFSv3/v4 直接推断认证模式或 ACL 支持。

## 六、NFS 权限模型

至少区分访问身份、远程文件权限和服务器导出规则三件事。

1. 访问身份：连接配置里的 UID、GID 和附加组。
2. 文件权限：服务器返回的文件属主、属组与权限位；服务器和后端支持时还可以返回 ACL。
3. 导出规则：服务器决定哪些客户端能访问某个导出、导出是否只读、是否进行身份映射等。

基础文件权限采用独立模型：

~~~typescript
interface NfsFilePermissions {
  ownerUid: number
  ownerGid: number
  mode: string          // 例如 "0755"，保留协议返回的特殊权限位
}
~~~

权限界面可以展示属主 UID、属组 GID、权限位及对应的读/写/执行含义。受支持且服务器授权时，通过 NFS 后端执行权限位或属主修改。ACL 如被后端支持，应使用独立、无损的 ACL 结构，不能压缩为三组 rwx 后再覆盖回去。

需要明确：

- 设置连接 UID/GID 是设置请求身份，不是直接修改服务器上的文件属主。
- 设置客户端只读挂载，只能限制该挂载的写入，不能赋予服务器未授权的访问能力。
- 设置 UID 为 0 不保证获得服务器 root 权限，服务器可能进行身份映射。
- root_squash、all_squash、anonuid/anongid、允许访问的客户端地址和导出 ro/rw 都是服务器规则，不能放入客户端连接表单并宣称可直接修改。
- 本项目中的 NFS 权限管理指客户端身份配置，以及服务器授权下的文件权限查看和修改；服务器导出规则不属于普通 NFS 客户端可直接管理的设置。

上述服务器导出规则由服务器管理员维护：[exports 手册](https://www.man7.org/linux/man-pages/man5/exports.5.html)。客户端不假定能完整读取或更改这些规则。

## 七、SMB 使用独立模型

~~~typescript
interface SmbConfig {
  shareName: string
  basePath?: string
  port?: number
  authentication: {
    username: string
    domain?: string
    password: string
  }
  dialect: string       // 由后端协商能力校验
}
~~~

| 字段 | 含义 |
|---|---|
| shareName | SMB 共享名，例如 data |
| basePath | 共享内部的起始目录，可选 |
| port | SMB 服务端口 |
| username / domain | 连接账号及所属域 |
| password | 本地明文保存的密码，原样保留，不 trim |
| dialect | SMB 协议版本配置 |

SMB 权限遵循 SMB 服务器的安全模型。需要表达权限时，采用后端返回的安全描述符、属主和 ACL 信息，不使用 NFS 的 UID/GID 与 mode 字段代替。Windows 风格的 DACL 由 ACE 条目描述访问允许与拒绝等信息：[DACL 与 ACE](https://learn.microsoft.com/en-us/windows/win32/secauthz/dacls-and-aces)。

SMB 的共享级授权与文件/目录级授权不能只凭一个客户端 readOnly 字段推断。权限读取或修改是否可用，由协议后端和服务器共同决定；客户端不能假定能通过文件访问接口管理服务器共享规则。

## 八、连接路由规则

连接路由层只接收某个协议连接及其设备地址列表，按优先级尝试。

~~~text
选择协议连接
    ↓
读取设备地址列表
    ↓
按 priority 升序排序
    ↓
使用该连接的协议配置尝试第一个地址
    ├─ 成功：保存本次实际地址并建立会话
    └─ 网络连接失败：尝试下一个地址
                     ↓
               全部失败：返回连接失败
~~~

规则如下：

1. 只使用用户保存的 IP/域名与优先级，不推断地址用途。
2. 同一个地址分别按目标 NFS 或 SMB 连接配置测试；不能用一个协议的成功替代另一个协议的结果。
3. DNS 失败、连接超时、无路由或目标连接被拒绝时，尝试下一地址。
4. 认证、导出访问、文件权限、协议版本等错误保留协议类型与错误原因，不能全部归类成地址不可达。
5. 默认对认证和授权错误向用户返回具体原因，避免对全部地址循环重复认证。协议后端若能明确判断为某个地址特有的连接故障，再按连接故障处理。
6. 已建立的健康会话保持当前地址；需要新连接或当前连接失效时，重新按优先级选择。
7. 修改优先级影响后续连接，不立即强制卸载正在使用的盘符。
8. 自动连接未成功时由后台定时重试，仍采用同一顺序。超时与重试节奏是内部实现参数，不增加额外的网络策略界面。

运行时结果包含 connectionId、addressId、resolvedAddress、protocol、status、error。连接状态按协议连接保存；同设备的不同共享可能使用不同地址。

## 九、磁盘配置示例

配置格式以[主文件](../examples/configuration/config.toml)、[设备文件](../examples/configuration/devices/home.toml)和[空设备](../examples/configuration/devices/backup.toml)为准。设备文档的 schema_version、device、地址数组与协议标签对应 Rust 配置类型；软件和日志设置只在主文件中。

## 十、远程浏览与系统挂载

两项能力使用同一个协议连接配置和地址选择过程，但各自维护会话。

远程浏览由 Rust 协议适配器通过 FFI 调用 libnfs 或 libsmb2 提供：

~~~text
list_directory(connection_id, path)
stat(connection_id, path)
read_range(connection_id, path, offset, length)
get_permissions(connection_id, path)
update_permissions(connection_id, path, protocol_specific_change)
~~~

权限变更参数是按协议区分的类型。例如 NFS 使用权限位或属主变更；SMB 如支持安全描述符修改，则使用 SMB 对应结构，不把 NFS chmod 语义硬套到 SMB 上。

后端返回具体能力，例如是否能读取权限、修改权限位、修改属主或处理 ACL。支持某种操作不代表当前身份获准执行，实际权限由服务器在执行时判断。失败时保留服务器错误。

文件列表分批加载，内容按需读取，避免一次性将大文件整体传入 UI。这是浏览功能的实现方式，不扩展为下载队列、同步、索引等功能。

系统挂载通过 MountManager 执行：

~~~text
mount(connection_id, target, platform_options)
unmount(mount_id)
list_mounts()
~~~

| 平台 | SMB 挂载 | NFS 挂载 |
|---|---|---|
| Windows | 当前桌面用户会话中的 WNet 网络连接 API | 检测系统 NFS 客户端和协议能力 |
| macOS | 系统 SMB 挂载接口 | 系统 mount_nfs |
| Linux | mount.cifs | mount.nfs |

挂载目标是本机盘符或目录，平台参数与协议参数分开建模。应用内使用的身份与系统挂载身份必须分别校验：某些系统挂载后端无法按每个连接设置相同的 UID/GID，不能默默换用其他身份。

Windows 内置 NFS 客户端按微软公开矩阵支持 v2/v3，不能假定与应用内协议后端的能力完全一致：[NFS 支持矩阵](https://learn.microsoft.com/en-us/windows-server/storage/nfs/nfs-overview)。不支持时明确返回原因，不改变用户选择的协议或身份。

Windows 盘符映射属于用户登录会话，需在对应用户会话中处理：[微软说明](https://learn.microsoft.com/en-us/windows/win32/services/services-and-redirected-drives)。

## 十一、地址切换时的行为

| 使用场景 | 处理 |
|---|---|
| 新连接 | 按优先级尝试所有候选地址 |
| 浏览连接失效 | 重新选地址并建立连接，保留逻辑目录位置 |
| 正在读取文件内容 | 明确当前读取是否失败，重连后重新发起可安全重试的读取 |
| 权限修改结果未知 | 先查询实际权限，不直接重复发送修改 |
| 已挂载共享失效 | 检查系统状态；确需重挂载时使用新选地址 |
| 挂载仍被其他应用占用 | 普通卸载失败后停止，不自动强制卸载 |

系统挂载更换目标地址不能统一保证外部应用打开的文件句柄和锁无感迁移。浏览状态与挂载状态应分别显示，必要时分别显示实际地址。

自动挂载保存在配置中。后台启动后检查实际挂载状态，避免重复挂载或占用已经被其他连接使用的盘符/目录。

## 十二、界面与模块边界

界面仅包含设备及地址配置、协议连接表单、远程文件浏览、协议权限信息和本地挂载设置。NFS 与 SMB 使用不同的连接表单和权限面板。地址表仅显示 IP/域名、优先级、连接结果和当前使用情况。

以下按逻辑职责列出后续模块；当前包含 model、config、store、error、app 及 app 内的 startup/logging，其余随功能逐步增加，优先在同一 library 内组织：

~~~text
desktop-ui       Vue / Tailwind 界面
agent            用户后台调度、自动连接与挂载
model            Device、Address 与协议区分模型
router           按优先级选择地址
nfs              NFS 连接、文件读取与权限操作
smb              SMB 连接、文件读取与安全信息操作
nfs::ffi         libnfs 底层 FFI 绑定
smb::ffi         libsmb2 底层 FFI 绑定
mount            各系统挂载实现
config           配置快照、版本与主配置/单设备 TOML 编解码
store            StoreConnection，config.toml 与设备文件 I/O
error            分层错误、英文 Display、来源与转换
app              项目流程编排与展示
app::startup     参数、主配置定位与读取校验、启动上下文
app::logging     双输出 tracing 订阅、独立过滤、同步文件写入与错误报告
ipc              界面与后台通信
~~~

协议适配器负责各自的连接参数、认证、错误与权限语义。公共层只统一设备、地址顺序、连接状态和必要的文件读取入口。

nfs::ffi 与 smb::ffi 是协议模块内部维护的底层绑定；nfs 与 smb 在其上封装安全的 Rust 接口。职责划分优先通过模块表达。桌面壳和独立用户后台需要不同进程入口时，再确定对应的构建组织，不预先要求每个模块成为独立包。

## 十三、依赖管理与跨平台构建

NFS 后端固定为 libnfs，SMB 后端固定为 libsmb2。协议库选型已经完成，后续工作按这两个 C 库的 FFI 接入方案开展。

前端使用 npm，提交 package-lock.json，构建使用 npm ci。Rust 当前采用 Rust 2024 edition 的单包 lib/bin 结构，设备、路由、协议、存储等职责在 library 内按模块组织，binary 负责启动和应用编排，提交 Cargo.lock，发布构建使用 --locked。Rust 依赖在根 Cargo.toml 中声明，平台专属依赖使用 target 条件配置。出现独立构建或复用的实际需求时，再评估是否需要多个包或 workspace。后续工程化阶段使用 rust-toolchain.toml 固定 Rust 版本；Node 版本同样固定在开发和 CI 配置中。

协议适配模块只向应用暴露本项目的接口，C 库类型不进入公共设备、权限和 IPC 模型。每种协议使用一个固定实现。

| 协议 | C 库 | 底层绑定 | Rust 安全封装 |
|---|---|---|---|
| NFS | libnfs | nfs::ffi | nfs |
| SMB2/3 | libsmb2 | smb::ffi | smb |

上述绑定由项目维护，并与固定的 C 库版本配套。底层绑定封装原生构建、链接配置和 C 函数声明；高层适配器封装连接上下文、目录与文件句柄、错误转换，以及异步回调与 Rust 任务之间的衔接。

原始指针、回调数据、字符串与缓冲区的生命周期限制在 FFI 封装内部，资源通过 Rust 所有权与 Drop 统一释放。每个 C 连接上下文由所属任务或工作线程管理，避免并发误用；UI 和公共业务模块只使用封装后的 Rust 类型。

NFS 适配器通过 libnfs 处理导出连接、AUTH_SYS 身份、目录与文件读取，以及库支持且服务器授权的权限操作。SMB 适配器通过 libsmb2 处理 SMB2/3 协商、账号认证、目录与文件读取，以及库支持且服务器授权的安全信息操作。协议与权限模型沿用前文定义。

接入原生库时，由包根目录的 build.rs 调用 cmake 构建依赖，再由系统 CMake 调用目标平台 C 编译器。Cargo 能编排构建，但不会代替编译器、链接器或系统 SDK：[Cargo 构建脚本](https://doc.rust-lang.org/cargo/reference/build-scripts.html)、[cmake crate](https://docs.rs/cmake/latest/cmake/)。

~~~text
Cargo
  → 包根目录的 build.rs
    → CMake
      → 目标平台 C 编译器
        → libnfs / libsmb2 原生库
  → Rust 链接与应用打包
~~~

原生库源码固定到发行版本或提交，随项目保存或由独立的准备步骤按校验值获取；build.rs 不联网拉取最新源码。Cargo.lock 不替代原生源码版本、CMake 选项和工具链记录。FFI 绑定使用 bindgen 按目标平台 ABI 生成并随源码保存，普通开发构建直接使用保存的绑定；重新生成绑定时使用 libclang。绑定与 C 库源码版本同步更新。静态或动态链接根据依赖许可证和打包要求确定，动态库需随应用一起分发。

推荐在目标操作系统的构建机器或 CI runner 上分别构建：

| 目标平台 | 原生工具链与系统依赖 |
|---|---|
| Windows | Rust MSVC target、MSVC Build Tools、Windows SDK、WebView2、CMake |
| macOS | Rust 对应架构 target、Xcode Command Line Tools 与 Apple SDK、CMake |
| Linux | Rust 对应架构 target、GCC 或 Clang、Tauri 所需 WebKitGTK 等开发依赖、CMake |

Tauri 系统依赖以[官方环境要求](https://v2.tauri.app/start/prerequisites/)为准。Rust 支持多目标不代表仅执行 rustup target add 就能从一台机器构建完整的三平台桌面应用。初期采用各平台本机构建，避免额外维护跨系统交叉编译工具链。最终用户安装已打包程序，无需安装 Rust、Node 或 C/C++ 编译器；系统挂载仍需对应操作系统提供所需客户端能力。

## 验证范围

本地回归命令见 [README](../README.md)。自动测试不能替代真实 NAS、服务器授权、系统挂载与三平台打包验证；这些验证在对应功能接入时执行。
