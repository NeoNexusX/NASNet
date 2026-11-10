//! 设备及其地址、协议连接列表的管理。

use super::address::Address;
use super::connection::ProtocolConnection;
use crate::model::ModelError;

pub struct Device {
    id: String,
    name: String,
    addresses: Vec<Address>,
    connections: Vec<ProtocolConnection>,
}

impl Device {
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

    pub fn rename(&mut self, name: String) {
        self.name = name;
    }

    /// 按保存顺序借用地址。
    pub fn addresses(&self) -> &[Address] {
        &self.addresses
    }

    pub fn address_count(&self) -> usize {
        self.addresses.len()
    }

    pub fn find_address(&self, id: &str) -> Option<&Address> {
        self.addresses.iter().find(|saved| saved.id == id)
    }

    /// 返回按优先级稳定排序的地址引用。
    pub fn addresses_by_priority(&self) -> Vec<&Address> {
        let mut sorted_data: Vec<&Address> = self.addresses.iter().collect();
        sorted_data.sort_by_key(|addr| addr.priority);
        sorted_data
    }

    /// 校验并查重后才追加地址。
    pub fn try_add_address(&mut self, address: Address) -> Result<(), ModelError> {
        address.validate()?;
        if self.find_address(&address.id).is_some() {
            return Err(ModelError::DuplicateAddressId(address.id));
        }
        self.addresses.push(address);
        Ok(())
    }

    /// 校验后原位更新同 ID 的地址。
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

    /// 删除并返回地址，保留其余条目的顺序。
    pub fn remove_address(&mut self, id: &str) -> Result<Address, ModelError> {
        self.addresses
            .iter()
            .position(|saved| saved.id == id)
            .map(|index| self.addresses.remove(index))
            .ok_or_else(|| ModelError::AddressNotFound(id.to_owned()))
    }

    pub fn connections(&self) -> &[ProtocolConnection] {
        &self.connections
    }

    /// 返回的引用借自设备。
    pub fn find_connection(&self, id: &str) -> Option<&ProtocolConnection> {
        self.connections.iter().find(|saved| saved.id == id)
    }

    /// 校验并查重后才追加连接。
    pub fn try_add_connection(&mut self, connection: ProtocolConnection) -> Result<(), ModelError> {
        connection.validate()?;
        if self.find_connection(&connection.id).is_some() {
            return Err(ModelError::DuplicateConnectionId(connection.id));
        }
        self.connections.push(connection);
        Ok(())
    }

    /// 校验后原位更新同 ID 的连接。
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

    /// 删除并返回连接，保留其余条目的顺序。
    pub fn remove_connection(&mut self, id: &str) -> Result<ProtocolConnection, ModelError> {
        self.connections
            .iter()
            .position(|saved| saved.id == id)
            .map(|index| self.connections.remove(index))
            .ok_or_else(|| ModelError::ConnectionNotFound(id.to_owned()))
    }
}
