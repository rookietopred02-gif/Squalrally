use crate::command_executors::privileged_request_executor::PrivilegedCommandRequestExecutor;
use crate::engine_privileged_state::EnginePrivilegedState;
use squalr_engine_api::commands::memory::regions::memory_regions_request::MemoryRegionsRequest;
use squalr_engine_api::commands::memory::regions::memory_regions_response::{MemoryRegionInfo, MemoryRegionsResponse};
use squalr_engine_memory::memory_queryer::memory_queryer::MemoryQueryer;
use squalr_engine_memory::memory_queryer::memory_queryer_trait::IMemoryQueryer;
use squalr_engine_memory::memory_queryer::page_retrieval_mode::PageRetrievalMode;
use std::sync::Arc;

impl PrivilegedCommandRequestExecutor for MemoryRegionsRequest {
    type ResponseType = MemoryRegionsResponse;

    fn execute(
        &self,
        engine_privileged_state: &Arc<EnginePrivilegedState>,
    ) -> <Self as PrivilegedCommandRequestExecutor>::ResponseType {
        let mut regions = Vec::new();

        if let Some(opened_process_info) = engine_privileged_state.get_process_manager().get_opened_process() {
            // Memory Viewer wants a broad region list (CE-style). Using the scan settings can hide the
            // region containing the requested address, making "View Memory Region" appear broken.
            let pages = MemoryQueryer::get_memory_page_bounds(&opened_process_info, PageRetrievalMode::FromUserMode);
            let modules = MemoryQueryer::get_instance().get_modules(&opened_process_info);
            regions = pages
                .into_iter()
                .map(|region| {
                    let base_address = region.get_base_address();
                    let mut module_name = String::new();
                    let mut module_offset = 0u64;

                    for module in modules.iter() {
                        if module.contains_address(base_address) {
                            module_name = module.get_module_name().to_string();
                            module_offset = base_address.saturating_sub(module.get_base_address());
                            break;
                        }
                    }

                    MemoryRegionInfo {
                        base_address,
                        region_size: region.get_region_size(),
                        module_name,
                        module_offset,
                    }
                })
                .collect();
        }

        MemoryRegionsResponse { regions }
    }
}
