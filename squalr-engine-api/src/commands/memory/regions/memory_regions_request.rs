use crate::commands::memory::memory_command::MemoryCommand;
use crate::commands::memory::memory_response::MemoryResponse;
use crate::commands::privileged_command::PrivilegedCommand;
use crate::commands::privileged_command_request::PrivilegedCommandRequest;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(Clone, StructOpt, Debug, Serialize, Deserialize)]
pub struct MemoryRegionsRequest {}

impl PrivilegedCommandRequest for MemoryRegionsRequest {
    type ResponseType = MemoryRegionsResponse;

    fn to_engine_command(&self) -> PrivilegedCommand {
        PrivilegedCommand::Memory(MemoryCommand::Regions { memory_regions_request: self.clone() })
    }
}

impl From<MemoryRegionsResponse> for MemoryResponse {
    fn from(memory_regions_response: MemoryRegionsResponse) -> Self {
        MemoryResponse::Regions { memory_regions_response }
    }
}

use crate::commands::memory::regions::memory_regions_response::MemoryRegionsResponse;
