//! NASNet 的设备和地址模型。

mod connection;
mod validation;

pub use connection::{
    ConnectionConfig, NfsAuthSys, NfsConfig, NfsVersion, ProtocolConnection, SmbAuth, SmbConfig,
    SmbDialect,
};
pub use validation::{ConfigError, validate_host};

/// IP/域名与优先级记录，协议参数由独立的连接配置保存。
pub struct Address {
    pub id: String,
    pub host: String,
    /// 数值越小，优先级越高；同优先级按保存顺序排列。
    pub priority: u32,
}

impl Address {
    /// 只读检查地址，不自动修改 ID 或 Host，也不解析域名或连接网络。
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.id.trim().is_empty() {
            return Err(ConfigError::EmptyAddressId);
        }
        validate_host(&self.host)?;
        Ok(())
    }
}

/// 一台设备拥有自己的名称和地址列表。
pub struct Device {
    pub id: String,
    pub name: String,
    pub addresses: Vec<Address>,
    connections: Vec<ProtocolConnection>,
}

impl Device {
    /// 创建设备，初始地址和协议连接列表为空。
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            addresses: Vec::<Address>::new(),
            connections: Vec::new(),
        }
    }

    /// 按保存顺序追加地址，不进行配置校验。
    pub fn add_address(&mut self, address: Address) {
        self.addresses.push(address);
    }

    /// 校验后追加地址；失败时返回原因，设备里的地址列表保持原样。
    pub fn try_add_address(&mut self, address: Address) -> Result<(), ConfigError> {
        address.validate()?;
        if self.find_address(&address.id).is_some() {
            return Err(ConfigError::DuplicateAddressId(address.id));
        }
        self.addresses.push(address);
        Ok(())
    }

    /// 校验后追加协议连接；同一设备内连接 ID 不可重复。
    pub fn try_add_connection(
        &mut self,
        connection: ProtocolConnection,
    ) -> Result<(), ConfigError> {
        connection.validate()?;
        if self
            .connections
            .iter()
            .any(|saved| saved.id == connection.id)
        {
            return Err(ConfigError::DuplicateConnectionId(connection.id));
        }
        self.connections.push(connection);
        Ok(())
    }

    /// 借用已保存的协议连接，只允许读取。
    pub fn connections(&self) -> &[ProtocolConnection] {
        &self.connections
    }

    /// 返回当前保存的地址数量。
    pub fn address_count(&self) -> usize {
        self.addresses.len()
    }

    /// 修改设备名称，保留设备 ID 和地址。
    pub fn rename(&mut self, name: String) {
        self.name = name;
    }

    /// 按 ID 查找保存的地址。找到时借出引用，找不到时返回 None。
    #[allow(clippy::manual_find)]
    pub fn find_address(&self, id: &str) -> Option<&Address> {
        for address in &self.addresses {
            if address.id == id {
                return Some::<&Address>(address);
            }
        }
        None
    }

    /// 生成按优先级升序排列的地址引用列表，不改变原来的保存顺序。
    pub fn addresses_by_priority(&self) -> Vec<&Address> {
        let mut sorted_data: Vec<&Address> = self.addresses.iter().collect();
        sorted_data.sort_by_key(|addr| addr.priority);
        sorted_data
    }
}
