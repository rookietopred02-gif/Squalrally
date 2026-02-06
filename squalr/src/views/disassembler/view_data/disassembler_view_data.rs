use crate::app_context::AppContext;
use iced_x86::{Decoder, DecoderOptions, Formatter, IntelFormatter};
use squalr_engine_api::commands::privileged_command_request::PrivilegedCommandRequest;
use squalr_engine_api::commands::memory::read::memory_read_request::MemoryReadRequest;
use squalr_engine_api::commands::memory::regions::memory_regions_request::MemoryRegionsRequest;
use squalr_engine_api::conversions::conversions_from_primitives::Conversions;
use squalr_engine_api::dependency_injection::dependency::Dependency;
use squalr_engine_api::engine::engine_unprivileged_state::EngineUnprivilegedState;
use squalr_engine_api::structures::data_types::built_in_types::u8::data_type_u8::DataTypeU8;
use squalr_engine_api::structures::data_types::data_type_ref::DataTypeRef;
use squalr_engine_api::structures::data_values::container_type::ContainerType;
use squalr_engine_api::structures::structs::symbolic_field_definition::SymbolicFieldDefinition;
use squalr_engine_api::structures::structs::symbolic_struct_definition::SymbolicStructDefinition;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct DisassemblerLine {
    pub address: u64,
    pub display_address: String,
    pub bytes: String,
    pub instruction: String,
}

#[derive(Clone)]
pub struct DisassemblerViewData {
    pub address_input: String,
    pub base_address: u64,
    pub module_name: Option<String>,
    pub module_base: Option<u64>,
    pub highlight_address: Option<u64>,
    pub highlight_pending: bool,
    pub lines: Vec<DisassemblerLine>,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub read_size: usize,
}

impl DisassemblerViewData {
    pub fn new() -> Self {
        Self {
            address_input: String::new(),
            base_address: 0,
            module_name: None,
            module_base: None,
            highlight_address: None,
            highlight_pending: false,
            lines: Vec::new(),
            is_loading: false,
            error_message: None,
            read_size: 0x200,
        }
    }

    pub fn register(app_context: &Arc<AppContext>) -> Dependency<Self> {
        app_context
            .dependency_container
            .register(Self::new())
    }

    pub fn set_target_address(
        disassembler_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
        address: u64,
    ) {
        if let Some(mut disassembler_view_data) = disassembler_view_data.write("Disassembler view data set target address") {
            disassembler_view_data.address_input = format!("{:X}", address);
            disassembler_view_data.base_address = address;
            disassembler_view_data.highlight_address = Some(address);
            disassembler_view_data.highlight_pending = true;
        }

        Self::refresh(disassembler_view_data, engine_unprivileged_state);
    }

    pub fn refresh(
        disassembler_view_data: Dependency<Self>,
        engine_unprivileged_state: Arc<EngineUnprivilegedState>,
    ) {
        let (address_input, read_size) = {
            let mut guard = match disassembler_view_data.write("Disassembler view data refresh") {
                Some(guard) => guard,
                None => return,
            };

            guard.is_loading = true;
            guard.error_message = None;
            guard.module_name = None;
            guard.module_base = None;
            guard.lines.clear();

            (guard.address_input.trim().to_string(), guard.read_size)
        };

        let module_parse = address_input.split_once('+').map(|(module, offset)| (module.trim().to_string(), offset.trim().to_string()));
        let parsed_address = if module_parse.is_some() {
            None
        } else {
            match Conversions::parse_hex_address(address_input.trim()) {
                Ok(address) => Some(address),
                Err(_) => {
                    if let Some(mut disassembler_view_data) = disassembler_view_data.write("Disassembler view data invalid address") {
                        disassembler_view_data.error_message = Some("Invalid address".to_string());
                        disassembler_view_data.is_loading = false;
                    }
                    return;
                }
            }
        };

        let memory_regions_request = MemoryRegionsRequest {};
        let disassembler_view_data_clone = disassembler_view_data.clone();
        let engine_unprivileged_state_clone = engine_unprivileged_state.clone();

        memory_regions_request.send(&engine_unprivileged_state, move |memory_regions_response| {
            let mut resolved_address = parsed_address.unwrap_or(0);
            let mut module_name: Option<String> = None;
            let mut module_base: Option<u64> = None;

            if let Some((module, offset_string)) = module_parse.clone() {
                let offset = match Conversions::parse_hex_address(&offset_string) {
                    Ok(offset) => offset,
                    Err(_) => {
                        if let Some(mut disassembler_view_data) = disassembler_view_data_clone.write("Disassembler invalid module offset") {
                            disassembler_view_data.is_loading = false;
                            disassembler_view_data.error_message = Some("Invalid module offset".to_string());
                        }
                        return;
                    }
                };

                if let Some(region) = memory_regions_response
                    .regions
                    .iter()
                    .find(|region| region.module_name.eq_ignore_ascii_case(&module))
                {
                    let base = region.base_address.saturating_sub(region.module_offset);
                    resolved_address = base.saturating_add(offset);
                    module_name = Some(region.module_name.clone());
                    module_base = Some(base);
                } else {
                    if let Some(mut disassembler_view_data) = disassembler_view_data_clone.write("Disassembler module not found") {
                        disassembler_view_data.is_loading = false;
                        disassembler_view_data.error_message = Some("Module not found".to_string());
                    }
                    return;
                }
            } else if let Some(address) = parsed_address {
                if let Some(region) = memory_regions_response
                    .regions
                    .iter()
                    .find(|region| address >= region.base_address && address < region.base_address.saturating_add(region.region_size))
                {
                    if !region.module_name.is_empty() {
                        let base = region.base_address.saturating_sub(region.module_offset);
                        module_name = Some(region.module_name.clone());
                        module_base = Some(base);
                    }
                }
            }

            let symbolic_struct_definition = SymbolicStructDefinition::new_anonymous(vec![SymbolicFieldDefinition::new(
                DataTypeRef::new(DataTypeU8::get_data_type_id()),
                ContainerType::ArrayFixed(read_size as u64),
            )]);

            let memory_read_request = MemoryReadRequest {
                address: resolved_address,
                module_name: String::new(),
                symbolic_struct_definition,
            };

            memory_read_request.send(&engine_unprivileged_state_clone, move |memory_read_response| {
                let bytes = memory_read_response.valued_struct.get_bytes();
                let base_address = memory_read_response.address;

                if let Some(mut disassembler_view_data) = disassembler_view_data_clone.write("Disassembler view data refresh response") {
                    disassembler_view_data.is_loading = false;
                    disassembler_view_data.base_address = base_address;
                    disassembler_view_data.module_name = module_name.clone();
                    disassembler_view_data.module_base = module_base;
                    disassembler_view_data.highlight_address = Some(base_address);
                    disassembler_view_data.highlight_pending = true;

                    if !memory_read_response.success || bytes.is_empty() {
                        disassembler_view_data.error_message = None;
                        disassembler_view_data.lines = vec![DisassemblerLine {
                            address: base_address,
                            display_address: format!("{:016X}", base_address),
                            bytes: "??".to_string(),
                            instruction: "db ??".to_string(),
                        }];
                        return;
                    }

                    disassembler_view_data.error_message = None;
                    let decoded = Self::decode_instructions(&bytes, base_address, module_name.as_deref(), module_base);
                    if decoded.is_empty() {
                        disassembler_view_data.lines = vec![DisassemblerLine {
                            address: base_address,
                            display_address: format!("{:016X}", base_address),
                            bytes: "??".to_string(),
                            instruction: "db ??".to_string(),
                        }];
                    } else {
                        disassembler_view_data.lines = decoded;
                    }
                }
            });
        });
    }

    fn decode_instructions(
        bytes: &[u8],
        base_address: u64,
        module_name: Option<&str>,
        module_base: Option<u64>,
    ) -> Vec<DisassemblerLine> {
        let mut decoder = Decoder::with_ip(64, bytes, base_address, DecoderOptions::NONE);
        let mut formatter = IntelFormatter::new();
        let options = formatter.options_mut();
        options.set_uppercase_hex(true);
        options.set_hex_prefix("0x");
        options.set_rip_relative_addresses(true);
        let mut lines = Vec::new();

        while decoder.can_decode() {
            let instruction = decoder.decode();
            let offset = instruction
                .ip()
                .saturating_sub(base_address) as usize;
            let length = instruction.len() as usize;

            if offset + length > bytes.len() {
                break;
            }

            let mut instr_string = String::new();
            formatter.format(&instruction, &mut instr_string);

            let instr_bytes = &bytes[offset..offset + length];
            let bytes_string = instr_bytes
                .iter()
                .map(|byte| format!("{:02X}", byte))
                .collect::<Vec<_>>()
                .join(" ");
            let bytes_string = format!("{:<47}", bytes_string);
            let display_address = if let (Some(module_name), Some(module_base)) = (module_name, module_base) {
                format!("{}+{:X}", module_name, instruction.ip().saturating_sub(module_base))
            } else {
                format!("{:016X}", instruction.ip())
            };

            lines.push(DisassemblerLine {
                address: instruction.ip(),
                display_address,
                bytes: bytes_string,
                instruction: instr_string,
            });
        }

        lines
    }
}
