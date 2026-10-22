//! 按协议保存连接配置；地址候选列表由所属设备提供。

use super::ConfigError;

/// 请求使用的 NFS 版本，实际支持情况由协议后端检查。
#[derive(Debug)]
pub enum NfsVersion {
    V3,
    V4,
}

/// NFS AUTH_SYS 请求身份，与远程文件属主及服务器授权分开管理。
pub struct NfsAuthSys {
    pub uid: u32,
    pub gid: u32,
    pub supplementary_groups: Vec<u32>,
}

pub struct NfsConfig {
    pub export_path: String,
    pub version: NfsVersion,
    pub port: Option<u16>,
    pub authentication: NfsAuthSys,
}

impl NfsConfig {
    /// 检查本地配置格式，不探测服务器导出或后端能力。
    pub fn validate(&self) -> Result<(), ConfigError> {
        // TODO 10：导出路径须以 / 开头且不能含 '\0'，否则返回 InvalidNfsExportPath。
        // 再检查 port：None 表示交给后端决定；Some(0) 返回 InvalidPort。
        // UID/GID 为 0、附加组为空、导出路径为 / 都合法，不在这里推断远程权限。
        // 所有检查通过后返回 Ok(())。
        todo!("练习 10：校验 NFS 配置，导出路径={:?}", self.export_path)
    }
}

/// 自动协商，或限制在对应的 SMB 版本系列；具体方言由后端协商。
#[derive(Debug)]
pub enum SmbDialect {
    Auto,
    Smb2,
    Smb3,
}

/// 当前提供账号认证，密码保存在系统凭据存储中，此处只持有引用标识。
pub struct SmbAuth {
    pub username: String,
    pub domain: Option<String>,
    pub password_ref: String,
}

pub struct SmbConfig {
    pub share_name: String,
    /// None 或空字符串表示共享根目录；其他值按共享内相对路径解释。
    pub base_path: Option<String>,
    pub port: Option<u16>,
    pub authentication: SmbAuth,
    pub dialect: SmbDialect,
}

impl SmbConfig {
    /// 检查共享、账号和端口格式，不读取密码或检查远程权限。
    pub fn validate(&self) -> Result<(), ConfigError> {
        // TODO 11：按下面的顺序检查，遇到第一项错误就返回。
        // 1. share_name 去掉首尾空白后为空，或含 /、反斜杠、'\0'：InvalidSmbShareName。
        // 2. base_path 为 Some 时，不能以 / 或反斜杠开头，也不能含 '\0'：InvalidSmbBasePath。
        //    None、Some("") 和 Some("photos/2026") 均合法；本轮不做路径规范化。
        // 3. username.trim() 为空：EmptySmbUsername。
        // 4. password_ref.trim() 为空：EmptyPasswordRef。
        // 5. port 为 Some(0)：InvalidPort；否则返回 Ok(())。
        // 使用 &self.base_path 借用 Option<String>，不要从 &self 中移出字符串。
        todo!("练习 11：校验 SMB 配置，共享名={:?}", self.share_name)
    }
}

/// 每条连接恰好拥有一种协议的配置。
pub enum ConnectionConfig {
    Nfs(NfsConfig),
    Smb(SmbConfig),
}

/// 连接归所属设备管理，不重复保存设备的 Host 或地址优先级。
pub struct ProtocolConnection {
    pub id: String,
    pub config: ConnectionConfig,
}

impl ProtocolConnection {
    /// 检查连接 ID，并按实际协议分派配置校验。
    pub fn validate(&self) -> Result<(), ConfigError> {
        // TODO 12：先检查 id.trim() 是否为空，失败返回 EmptyConnectionId。
        // 然后 match &self.config，取出 NFS 或 SMB 配置，调用对应的 validate。
        // 只借用内部配置；保留原错误，不自动切换协议。
        todo!("练习 12：按协议校验连接，id={:?}", self.id)
    }
}
