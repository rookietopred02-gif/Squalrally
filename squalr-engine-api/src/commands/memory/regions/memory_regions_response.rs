use crate::commands::memory::memory_response::MemoryResponse;
use crate::commands::privileged_command_response::PrivilegedCommandResponse;
use crate::commands::privileged_command_response::TypedPrivilegedCommandResponse;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MemoryRegionInfo {
    pub base_address: u64,
    pub region_size: u64,
    /// Optional module name if this region base falls within a loaded module.
    pub module_name: String,
    /// Offset from the module base when `module_name` is set.
    pub module_offset: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MemoryRegionsResponse {
    pub regions: Vec<MemoryRegionInfo>,
}

impl TypedPrivilegedCommandResponse for MemoryRegionsResponse {
    fn to_engine_response(&self) -> PrivilegedCommandResponse {
        PrivilegedCommandResponse::Memory(MemoryResponse::Regions {
            memory_regions_response: self.clone(),
        })
    }

    fn from_engine_response(response: PrivilegedCommandResponse) -> Result<Self, PrivilegedCommandResponse> {
        if let PrivilegedCommandResponse::Memory(MemoryResponse::Regions { memory_regions_response }) = response {
            Ok(memory_regions_response)
        } else {
            Err(response)
        }
    }
}
