use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use std::fmt;

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct MemorySettings {
    #[serde(default)]
    pub memory_type_none: bool,
    #[serde(default)]
    pub memory_type_private: bool,
    #[serde(default)]
    pub memory_type_image: bool,
    #[serde(default)]
    pub memory_type_mapped: bool,
    #[serde(default)]
    pub required_write: bool,
    #[serde(default)]
    pub required_execute: bool,
    #[serde(default)]
    pub required_copy_on_write: bool,
    #[serde(default)]
    pub excluded_write: bool,
    #[serde(default)]
    pub excluded_execute: bool,
    #[serde(default)]
    pub excluded_copy_on_write: bool,
    #[serde(default)]
    pub excluded_no_cache: bool,
    #[serde(default)]
    pub excluded_write_combine: bool,
    #[serde(default)]
    pub only_main_module_image: bool,
    #[serde(default)]
    pub start_address: u64,
    #[serde(default)]
    pub end_address: u64,
    #[serde(default)]
    pub only_query_usermode: bool,
}

impl fmt::Debug for MemorySettings {
    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match to_string_pretty(&self) {
            Ok(json) => write!(formatter, "Settings for memory: {}", json),
            Err(_) => write!(formatter, "Memory config {{ could not serialize to JSON }}"),
        }
    }
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self {
            memory_type_none: false,
            memory_type_private: true,
            memory_type_image: true,
            memory_type_mapped: false,

            required_write: true,
            required_execute: false,
            required_copy_on_write: false,

            excluded_write: false,
            excluded_execute: false,
            excluded_copy_on_write: false,
            excluded_no_cache: false,
            excluded_write_combine: false,

            only_main_module_image: true,

            start_address: 0,
            end_address: u64::MAX,
            only_query_usermode: true,
        }
    }
}
