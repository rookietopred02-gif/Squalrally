use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PointerScanResult {
    base_address: u64,
    module_name: String,
    module_offset: u64,
    offsets: Vec<u64>,
    is_module: bool,
}

impl PointerScanResult {
    pub fn new(
        base_address: u64,
        module_name: String,
        module_offset: u64,
        offsets: Vec<u64>,
        is_module: bool,
    ) -> Self {
        Self {
            base_address,
            module_name,
            module_offset,
            offsets,
            is_module,
        }
    }

    pub fn get_base_address(&self) -> u64 {
        self.base_address
    }

    pub fn get_module_name(&self) -> &str {
        &self.module_name
    }

    pub fn get_module_offset(&self) -> u64 {
        self.module_offset
    }

    pub fn get_offsets(&self) -> &Vec<u64> {
        &self.offsets
    }

    pub fn is_module(&self) -> bool {
        self.is_module
    }
}
