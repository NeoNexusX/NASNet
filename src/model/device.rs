//! 设备及其地址、协议连接列表的管理。

use super::address::Address;
use super::connection::ProtocolConnection;
use crate::model::ModelError;

/// 一台设备拥有自己的名称和地址列表。
pub struct Device {
    id: String,
    name: String,
    addresses: Vec<Address>,
    connections: Vec<ProtocolConnection>,
}

impl Device {
    /// 创建设备，初始地址和协议连接列表为空。
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            addresses: Vec::new(),
            connections: Vec::new(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// 修改设备名称，保留设备 ID 和地址。
    pub fn rename(&mut self, name: String) {
        self.name = name;
    }

    /// 按保存顺序借用地址，不提供可变切片。
    pub fn addresses(&self) -> &[Address] {
        &self.addresses
    }

    /// 返回当前保存的地址数量。
    pub fn address_count(&self) -> usize {
        self.addresses.len()
    }

    /// 按 ID 查找保存的地址。找到时借出引用，找不到时返回 None。
    pub fn find_address(&self, id: &str) -> Option<&Address> {
        self.addresses.iter().find(|saved| saved.id == id)
    }

    /// 生成按优先级升序排列的地址引用列表，不改变原来的保存顺序。
    pub fn addresses_by_priority(&self) -> Vec<&Address> {
        let mut sorted_data: Vec<&Address> = self.addresses.iter().collect();
        sorted_data.sort_by_key(|addr| addr.priority);
        sorted_data
    }

    /// 校验后追加地址；失败时返回原因，设备里的地址列表保持原样。
    pub fn try_add_address(&mut self, address: Address) -> Result<(), ModelError> {
        address.validate()?;
        if self.find_address(&address.id).is_some() {
            return Err(ModelError::DuplicateAddressId(address.id));
        }
        self.addresses.push(address);
        Ok(())
    }

    /// 更新同 ID 的地址；成功保留原位置，失败保持原值和列表顺序。
    pub fn try_update_address(&mut self, address: Address) -> Result<(), ModelError> {
        address.validate()?;
        if let Some(existing) = self
            .addresses
            .iter_mut()
            .find(|saved| saved.id == address.id)
        {
            *existing = address;
            return Ok(());
        }
        Err(ModelError::AddressNotFound(address.id))
    }

    /// 删除并交还地址的所有权；其余地址的保存顺序不变。
    pub fn remove_address(&mut self, id: &str) -> Result<Address, ModelError> {
        self.addresses
            .iter()
            .position(|saved| saved.id == id)
            .map(|index| self.addresses.remove(index))
            .ok_or_else(|| ModelError::AddressNotFound(id.to_owned()))
    }

    /// 借用已保存的协议连接，只允许读取。
    pub fn connections(&self) -> &[ProtocolConnection] {
        &self.connections
    }

    /// 返回的连接引用借自当前设备，与查询参数 id 的存活时间无关。
    pub fn find_connection(&self, id: &str) -> Option<&ProtocolConnection> {
        self.connections.iter().find(|saved| saved.id == id)
    }

    /// 校验后追加协议连接；同一设备内连接 ID 不可重复。
    pub fn try_add_connection(&mut self, connection: ProtocolConnection) -> Result<(), ModelError> {
        connection.validate()?;
        if self
            .connections
            .iter()
            .any(|saved| saved.id == connection.id)
        {
            return Err(ModelError::DuplicateConnectionId(connection.id));
        }
        self.connections.push(connection);
        Ok(())
    }

    /// 按同 ID 替换整条连接；显式传入新配置，成功保留原位置。
    pub fn try_update_connection(
        &mut self,
        connection: ProtocolConnection,
    ) -> Result<(), ModelError> {
        connection.validate()?;
        if let Some(existing) = self
            .connections
            .iter_mut()
            .find(|saved| saved.id == connection.id)
        {
            *existing = connection;
            return Ok(());
        }
        Err(ModelError::ConnectionNotFound(connection.id))
    }

    /// 删除并交还连接的所有权；其余连接的保存顺序不变。
    pub fn remove_connection(&mut self, id: &str) -> Result<ProtocolConnection, ModelError> {
        self.connections
            .iter()
            .position(|saved| saved.id == id)
            .map(|index| self.connections.remove(index))
            .ok_or_else(|| ModelError::ConnectionNotFound(id.to_owned()))
    }
}
