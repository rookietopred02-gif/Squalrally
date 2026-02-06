use crate::app_context::AppContext;
use squalr_engine_api::commands::privileged_command_request::PrivilegedCommandRequest;
use squalr_engine_api::commands::memory::read::memory_read_request::MemoryReadRequest;
use squalr_engine_api::commands::memory::regions::memory_regions_request::MemoryRegionsRequest;
use squalr_engine_api::commands::memory::regions::memory_regions_response::MemoryRegionInfo;
use squalr_engine_api::conversions::conversions_from_primitives::Conversions;
use squalr_engine_api::dependency_injection::dependency::Dependency;
use squalr_engine_api::engine::engine_unprivileged_state::EngineUnprivilegedState;
use squalr_engine_api::structures::data_types::built_in_types::u8::data_type_u8::DataTypeU8;
use squalr_engine_api::structures::data_types::data_type_ref::DataTypeRef;
use squalr_engine_api::structures::data_values::container_type::ContainerType;
use squalr_engine_api::structures::structs::symbolic_field_definition::SymbolicFieldDefinition;
use squalr_engine_api::structures::structs::symbolic_struct_definition::SymbolicStructDefinition;
use std::sync::Arc;

#[derive(Clone)]
pub struct MemoryViewerViewData {
    pub address_input: String,
    pub base_address: u64,
    pub target_address: u64,
    pub region_base: u64,
    pub region_size: u64,
    pub display_data_type: DataTypeRef,
    pub regions: Vec<MemoryRegionInfo>,
    pub bytes: Vec<u8>,
    pub bytes_per_row: usize,
    pub row_count: usize,
    pub open_popout: bool,
    pub is_loading: bool,
    pub error_message: Option<String>,
}

impl MemoryViewerViewData {
    pub fn new() -> Self {
        Self {
            address_input: String::new(),
            base_address: 0,
            target_address: 0,
            region_base: 0,
            region_size: 0,
            display_data_type: DataTypeRef::new(DataTypeU8::get_data_type_id()),
            regions: Vec::new(),
            bytes: Vec::new(),
            bytes_per_row: 16,
            row_count: 16,
            open_popout: false,
            is_loading: false,
            error_message: None,
        }
    }

    pub fn register(app_context: &Arc<AppContext>) -> Dependency<Self> {
        app_context
            .dependency_container
            .register(Self::new())
    }

    pub fn set_target_address(
        memory_viewer_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
        address: u64,
    ) {
        if let Some(mut memory_viewer_view_data) = memory_viewer_view_data.write("Memory viewer view data set target address") {
            memory_viewer_view_data.address_input = format!("{:X}", address);
            memory_viewer_view_data.open_popout = true;
        }

        Self::refresh(memory_viewer_view_data, engine_unprivileged_state);
    }

    pub fn set_popout_open(
        memory_viewer_view_data: Dependency<Self>,
        is_open: bool,
    ) {
        if let Some(mut view_data) = memory_viewer_view_data.write("Memory viewer set popout visibility") {
            view_data.open_popout = is_open;
        }
    }

    pub fn refresh(
        memory_viewer_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let (address_input, bytes_to_read, auto_select_region) = {
            let mut guard = match memory_viewer_view_data.write("Memory viewer view data refresh") {
                Some(guard) => guard,
                None => return,
            };

            let address_input = guard.address_input.trim().to_string();
            let mut auto_select_region = false;
            guard.is_loading = true;
            guard.error_message = None;
            let bytes_to_read = guard.bytes_per_row.saturating_mul(guard.row_count).max(1);

            if address_input.is_empty() {
                auto_select_region = true;
            }

            (address_input, bytes_to_read, auto_select_region)
        };

        let memory_regions_request = MemoryRegionsRequest {};
        let memory_viewer_view_data_clone = memory_viewer_view_data.clone();
        let engine_unprivileged_state_clone = engine_unprivileged_state.clone();

        memory_regions_request.send(&engine_unprivileged_state, move |memory_regions_response| {
            let mut auto_select_region = auto_select_region;
            let module_parse = address_input.split_once('+').map(|(module, offset)| (module.trim().to_string(), offset.trim().to_string()));
            let mut invalid_address = false;
            let mut target_address = match Conversions::parse_hex_address(address_input.trim()) {
                Ok(address) => address,
                Err(_) => {
                    invalid_address = true;
                    0
                }
            };

            if invalid_address {
                auto_select_region = true;
            }
            let (read_len, read_base_address) = {
                let mut memory_viewer_view_data = match memory_viewer_view_data_clone.write("Memory viewer regions response") {
                    Some(data) => data,
                    None => return,
                };

                memory_viewer_view_data.regions = memory_regions_response.regions.clone();

                if memory_regions_response.regions.is_empty() {
                    memory_viewer_view_data.region_base = 0;
                    memory_viewer_view_data.region_size = 0;
                    memory_viewer_view_data.is_loading = false;
                    memory_viewer_view_data.bytes.clear();
                    memory_viewer_view_data.error_message = Some("No memory regions available (select a process).".to_string());
                    return;
                }

                if let Some((module, offset_string)) = module_parse.clone() {
                    let offset = match Conversions::parse_hex_address(&offset_string) {
                        Ok(offset) => offset,
                        Err(_) => {
                            memory_viewer_view_data.is_loading = false;
                            memory_viewer_view_data.bytes.clear();
                            memory_viewer_view_data.error_message = Some("Invalid module offset".to_string());
                            return;
                        }
                    };

                    if let Some(region) = memory_regions_response
                        .regions
                        .iter()
                        .find(|region| region.module_name.eq_ignore_ascii_case(&module))
                    {
                        let base = region.base_address.saturating_sub(region.module_offset);
                        target_address = base.saturating_add(offset);
                    } else {
                        memory_viewer_view_data.is_loading = false;
                        memory_viewer_view_data.bytes.clear();
                        memory_viewer_view_data.error_message = Some("Module not found".to_string());
                        return;
                    }
                }

                if auto_select_region {
                    if let Some(region) = memory_regions_response.regions.first() {
                        target_address = region.base_address;
                        memory_viewer_view_data.address_input = format!("{:X}", target_address);
                        if invalid_address {
                            memory_viewer_view_data.error_message = Some("Invalid address. Showing first region.".to_string());
                        }
                    }
                }

                memory_viewer_view_data.target_address = target_address;
                memory_viewer_view_data.base_address = target_address & !0xF;

                let mut max_len = bytes_to_read;
                let mut found_region = false;
                if let Some(region) = memory_regions_response
                    .regions
                    .iter()
                    .find(|region| target_address >= region.base_address && target_address < region.base_address.saturating_add(region.region_size))
                {
                    found_region = true;
                    memory_viewer_view_data.region_base = region.base_address;
                    memory_viewer_view_data.region_size = region.region_size;

                    let read_base = memory_viewer_view_data.base_address;
                    let offset = read_base.saturating_sub(region.base_address) as usize;
                    let region_len = region.region_size.saturating_sub(offset as u64) as usize;
                    if region_len > 0 {
                        max_len = max_len.min(region_len);
                    } else {
                        max_len = 0;
                    }
                } else {
                    memory_viewer_view_data.region_base = 0;
                    memory_viewer_view_data.region_size = 0;
                }

                if !found_region {
                    memory_viewer_view_data.is_loading = false;
                    memory_viewer_view_data.bytes.clear();
                    memory_viewer_view_data.error_message = Some("Address not in any memory region.".to_string());
                    return;
                }

                if max_len == 0 {
                    memory_viewer_view_data.is_loading = false;
                    memory_viewer_view_data.bytes.clear();
                    memory_viewer_view_data.error_message = Some("Unreadable memory".to_string());
                }

                (max_len, memory_viewer_view_data.base_address)
            };

            if read_len == 0 {
                return;
            }

            let symbolic_struct_definition = SymbolicStructDefinition::new_anonymous(vec![SymbolicFieldDefinition::new(
                DataTypeRef::new(DataTypeU8::get_data_type_id()),
                ContainerType::ArrayFixed(read_len as u64),
            )]);

            let memory_read_request = MemoryReadRequest {
                address: read_base_address,
                module_name: String::new(),
                symbolic_struct_definition,
            };

            memory_read_request.send(&engine_unprivileged_state_clone, move |memory_read_response| {
                if let Some(mut memory_viewer_view_data) = memory_viewer_view_data.write("Memory viewer view data refresh response") {
                    memory_viewer_view_data.is_loading = false;

                    if !memory_read_response.success {
                        memory_viewer_view_data.bytes.clear();
                        memory_viewer_view_data.error_message = Some("Unreadable memory".to_string());
                    } else {
                        let bytes = memory_read_response.valued_struct.get_bytes();
                        if bytes.is_empty() {
                            memory_viewer_view_data.bytes.clear();
                            memory_viewer_view_data.error_message = Some("Unreadable memory".to_string());
                        } else {
                            memory_viewer_view_data.bytes = bytes;
                            memory_viewer_view_data.error_message = None;
                        }
                    }
                }
            });
        });
    }
}
