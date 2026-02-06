use crate::memory_queryer::memory_protection_enum::MemoryProtectionEnum;
use crate::memory_queryer::memory_queryer_trait::IMemoryQueryer;
use crate::memory_queryer::memory_type_enum::MemoryTypeEnum;
use crate::memory_queryer::page_retrieval_mode::PageRetrievalMode;
use crate::memory_queryer::region_bounds_handling::RegionBoundsHandling;
use crate::{config::memory_settings_config::MemorySettingsConfig, memory_queryer::MemoryQueryerImpl};
use squalr_engine_api::conversions::storage_size_conversions::StorageSizeConversions;
use squalr_engine_api::structures::memory::normalized_region::NormalizedRegion;
use squalr_engine_api::structures::processes::opened_process_info::OpenedProcessInfo;
use std::{collections::HashSet, sync::Once};

pub struct MemoryQueryer;

impl MemoryQueryer {
    const MAX_SCAN_SNAPSHOT_BYTES: u64 = 2 * 1024 * 1024 * 1024; // Snapshot buffers are duplicated (current+previous).

    pub fn get_instance() -> &'static MemoryQueryerImpl {
        static mut INSTANCE: Option<MemoryQueryerImpl> = None;
        static INIT: Once = Once::new();

        unsafe {
            INIT.call_once(|| {
                let instance = MemoryQueryerImpl::new();
                INSTANCE = Some(instance);
            });

            #[allow(static_mut_refs)]
            INSTANCE.as_ref().unwrap_unchecked()
        }
    }

    pub fn get_memory_page_bounds(
        process_info: &OpenedProcessInfo,
        page_retrieval_mode: PageRetrievalMode,
    ) -> Vec<NormalizedRegion> {
        match page_retrieval_mode {
            PageRetrievalMode::FromSettings => MemoryQueryer::query_pages_from_settings(process_info),
            PageRetrievalMode::FromUserMode => MemoryQueryer::query_pages_from_usermode_memory(process_info),
            PageRetrievalMode::FromModules => MemoryQueryer::query_pages_from_modules(process_info),
            PageRetrievalMode::FromNonModules => MemoryQueryer::query_pages_from_non_modules(process_info),
        }
    }

    pub fn query_pages_by_address_range(
        process_info: &OpenedProcessInfo,
        start_address: u64,
        end_address: u64,
    ) -> Vec<NormalizedRegion> {
        let required_page_flags = MemoryProtectionEnum::empty();
        let excluded_page_flags = MemoryProtectionEnum::empty();
        let allowed_type_flags = MemoryTypeEnum::NONE | MemoryTypeEnum::PRIVATE | MemoryTypeEnum::IMAGE | MemoryTypeEnum::MAPPED;
        let bounds_handling = RegionBoundsHandling::Resize;

        let normalized_regions = MemoryQueryer::get_instance().get_virtual_pages(
            process_info,
            required_page_flags,
            excluded_page_flags,
            allowed_type_flags,
            start_address,
            end_address,
            bounds_handling,
        );

        normalized_regions
    }

    fn query_pages_from_usermode_memory(process_info: &OpenedProcessInfo) -> Vec<NormalizedRegion> {
        let required_page_flags = MemoryProtectionEnum::empty();
        let excluded_page_flags = MemoryProtectionEnum::empty();
        let allowed_type_flags = MemoryTypeEnum::NONE | MemoryTypeEnum::PRIVATE | MemoryTypeEnum::IMAGE | MemoryTypeEnum::MAPPED;
        let start_address = MemoryQueryer::get_instance().get_min_usermode_address(process_info);
        let end_address = MemoryQueryer::get_instance().get_max_usermode_address(process_info);

        let normalized_regions = MemoryQueryer::get_instance().get_virtual_pages(
            process_info,
            required_page_flags,
            excluded_page_flags,
            allowed_type_flags,
            start_address,
            end_address,
            RegionBoundsHandling::Exclude,
        );

        normalized_regions
    }

    fn query_pages_from_usermode_writable(
        process_info: &OpenedProcessInfo,
        allowed_type_flags: MemoryTypeEnum,
    ) -> Vec<NormalizedRegion> {
        let required_page_flags = MemoryProtectionEnum::WRITE;
        let excluded_page_flags = MemoryProtectionEnum::empty();
        let start_address = MemoryQueryer::get_instance().get_min_usermode_address(process_info);
        let end_address = MemoryQueryer::get_instance().get_max_usermode_address(process_info);

        MemoryQueryer::get_instance().get_virtual_pages(
            process_info,
            required_page_flags,
            excluded_page_flags,
            allowed_type_flags,
            start_address,
            end_address,
            RegionBoundsHandling::Exclude,
        )
    }

    fn truncate_regions_to_max(
        regions: Vec<NormalizedRegion>,
        max_bytes: u64,
    ) -> Vec<NormalizedRegion> {
        if max_bytes == 0 {
            return vec![];
        }

        let mut total = 0u64;
        let mut trimmed = Vec::new();

        for region in regions {
            if total >= max_bytes {
                break;
            }

            let size = region.get_region_size();
            let remaining = max_bytes.saturating_sub(total);

            if size <= remaining {
                total = total.saturating_add(size);
                trimmed.push(region);
            } else if remaining > 0 {
                trimmed.push(NormalizedRegion::new(region.get_base_address(), remaining));
                break;
            }
        }

        trimmed
    }

    fn query_pages_from_settings(process_info: &OpenedProcessInfo) -> Vec<NormalizedRegion> {
        let required_page_flags = MemoryQueryer::get_required_protection_settings();
        let excluded_page_flags = MemoryQueryer::get_excluded_protection_settings();
        let allowed_type_flags = MemoryQueryer::get_allowed_type_settings();

        let (start_address, end_address) = if MemorySettingsConfig::get_only_query_usermode() {
            (
                MemoryQueryer::get_instance().get_min_usermode_address(process_info),
                MemoryQueryer::get_instance().get_max_usermode_address(process_info),
            )
        } else {
            (MemorySettingsConfig::get_start_address(), MemorySettingsConfig::get_end_address())
        };

        log::debug!(
            "Querying pages: start=0x{:X} end=0x{:X} required={:?} excluded={:?} types={:?} usermode_only={}",
            start_address,
            end_address,
            required_page_flags,
            excluded_page_flags,
            allowed_type_flags,
            MemorySettingsConfig::get_only_query_usermode()
        );

        let mut normalized_regions = MemoryQueryer::get_instance().get_virtual_pages(
            process_info,
            required_page_flags,
            excluded_page_flags,
            allowed_type_flags,
            start_address,
            end_address,
            RegionBoundsHandling::Exclude,
        );

        if MemorySettingsConfig::get_only_main_module_image() {
            let modules = MemoryQueryer::get_instance().get_modules(process_info);
            let main_module_name = modules
                .iter()
                .find(|module| module.get_module_name().eq_ignore_ascii_case(process_info.get_name()))
                .map(|module| module.get_module_name().to_string());

            if let Some(main_module_name) = main_module_name {
                normalized_regions.retain(|region| {
                    match MemoryQueryer::get_instance().address_to_module(region.get_base_address(), &modules) {
                        Some((module_name, _)) => module_name.eq_ignore_ascii_case(&main_module_name),
                        None => true,
                    }
                });
            } else {
                log::warn!("Main module not found for '{}'; leaving image pages unfiltered.", process_info.get_name());
            }
        }

        let total_size_in_bytes: u64 = normalized_regions.iter().map(|region| region.get_region_size()).sum();

        if total_size_in_bytes == 0 {
            log::warn!("No pages matched the current memory settings. Retrying without required protection flags.");

            let relaxed_regions = MemoryQueryer::get_instance().get_virtual_pages(
                process_info,
                MemoryProtectionEnum::empty(),
                excluded_page_flags,
                allowed_type_flags,
                start_address,
                end_address,
                RegionBoundsHandling::Exclude,
            );

            let relaxed_size_in_bytes: u64 = relaxed_regions.iter().map(|region| region.get_region_size()).sum();
            if relaxed_size_in_bytes > 0 {
                log::warn!(
                    "Recovered {} bytes by relaxing required protection flags.",
                    StorageSizeConversions::value_to_metric_size(relaxed_size_in_bytes as u128)
                );
                return relaxed_regions;
            }

            log::warn!("Relaxed protection flags still yielded no pages. Falling back to usermode + writable pages.");
            let fallback_regions = Self::query_pages_from_usermode_writable(
                process_info,
                MemoryTypeEnum::NONE | MemoryTypeEnum::PRIVATE | MemoryTypeEnum::IMAGE | MemoryTypeEnum::MAPPED,
            );
            if fallback_regions.is_empty() {
                log::error!("Writable fallback returned no pages. Falling back to all usermode pages.");
                return Self::query_pages_from_usermode_memory(process_info);
            }
            return fallback_regions;
        }

        if total_size_in_bytes > Self::MAX_SCAN_SNAPSHOT_BYTES {
            log::warn!(
                "Scan snapshot too large: {} ({}). Falling back to usermode + writable pages.",
                total_size_in_bytes,
                StorageSizeConversions::value_to_metric_size(total_size_in_bytes as u128)
            );

            let fallback_regions =
                Self::query_pages_from_usermode_writable(process_info, MemoryTypeEnum::NONE | MemoryTypeEnum::PRIVATE | MemoryTypeEnum::IMAGE | MemoryTypeEnum::MAPPED);
            let fallback_size_in_bytes: u64 = fallback_regions.iter().map(|region| region.get_region_size()).sum();

            if fallback_size_in_bytes > Self::MAX_SCAN_SNAPSHOT_BYTES {
                log::warn!(
                    "Scan snapshot still too large after fallback: {} ({}). Narrow memory filters in Settings -> Memory (e.g., writable-only + disable mapped).",
                    fallback_size_in_bytes,
                    StorageSizeConversions::value_to_metric_size(fallback_size_in_bytes as u128)
                );

                let private_only_regions = Self::query_pages_from_usermode_writable(process_info, MemoryTypeEnum::PRIVATE);
                let private_only_size_in_bytes: u64 = private_only_regions.iter().map(|region| region.get_region_size()).sum();

                if private_only_size_in_bytes > Self::MAX_SCAN_SNAPSHOT_BYTES {
                    log::error!(
                        "Scan snapshot still too large after private-only fallback: {} ({}). Refusing to snapshot; narrow scan range in Settings -> Memory.",
                        private_only_size_in_bytes,
                        StorageSizeConversions::value_to_metric_size(private_only_size_in_bytes as u128)
                    );
                    let trimmed = Self::truncate_regions_to_max(private_only_regions, Self::MAX_SCAN_SNAPSHOT_BYTES);
                    if trimmed.is_empty() {
                        return vec![];
                    }

                    let trimmed_size_in_bytes: u64 = trimmed.iter().map(|region| region.get_region_size()).sum();
                    log::warn!(
                        "Truncating scan snapshot to {} ({}). Scan results are partial; narrow scan range for full coverage.",
                        trimmed_size_in_bytes,
                        StorageSizeConversions::value_to_metric_size(trimmed_size_in_bytes as u128)
                    );

                    return trimmed;
                }

                return private_only_regions;
            }

            if fallback_regions.is_empty() {
                log::error!("Writable fallback returned no pages. Falling back to all usermode pages.");
                return Self::query_pages_from_usermode_memory(process_info);
            }

            return fallback_regions;
        }

        normalized_regions
    }

    fn query_pages_from_modules(process_info: &OpenedProcessInfo) -> Vec<NormalizedRegion> {
        // Note that we use into_base_region to extract the base region without copying, instead taking ownership
        let module_regions = MemoryQueryer::get_instance()
            .get_modules(process_info)
            .into_iter()
            .map(|module| module.into_base_region())
            .collect();

        module_regions
    }

    fn query_pages_from_non_modules(process_info: &OpenedProcessInfo) -> Vec<NormalizedRegion> {
        let modules: HashSet<u64> = MemoryQueryer::get_instance()
            .get_modules(process_info)
            .into_iter()
            .map(|module| module.get_base_address())
            .collect();

        let required_page_flags = MemoryProtectionEnum::empty();
        let excluded_page_flags = MemoryProtectionEnum::empty();
        let allowed_type_flags = MemoryTypeEnum::NONE | MemoryTypeEnum::PRIVATE | MemoryTypeEnum::IMAGE;
        let start_address = 0;
        let end_address = MemoryQueryer::get_instance().get_max_usermode_address(process_info);

        // Collect all virtual pages
        let virtual_pages = MemoryQueryer::get_instance().get_virtual_pages(
            process_info,
            required_page_flags,
            excluded_page_flags,
            allowed_type_flags,
            start_address,
            end_address,
            RegionBoundsHandling::Exclude,
        );

        // Exclude any virtual pages that are also modules (static)
        let memory_regions = virtual_pages
            .into_iter()
            .filter(|page| !modules.contains(&page.get_base_address()))
            .collect();

        memory_regions
    }

    fn get_allowed_type_settings() -> MemoryTypeEnum {
        let mut result = MemoryTypeEnum::empty();

        if MemorySettingsConfig::get_memory_type_none() {
            result |= MemoryTypeEnum::NONE;
        }

        if MemorySettingsConfig::get_memory_type_private() {
            result |= MemoryTypeEnum::PRIVATE;
        }

        if MemorySettingsConfig::get_memory_type_image() {
            result |= MemoryTypeEnum::IMAGE;
        }

        if MemorySettingsConfig::get_memory_type_mapped() {
            result |= MemoryTypeEnum::MAPPED;
        }

        result
    }

    fn get_required_protection_settings() -> MemoryProtectionEnum {
        let mut result = MemoryProtectionEnum::empty();

        if MemorySettingsConfig::get_required_write() {
            result |= MemoryProtectionEnum::WRITE;
        }

        if MemorySettingsConfig::get_required_execute() {
            result |= MemoryProtectionEnum::EXECUTE;
        }

        if MemorySettingsConfig::get_required_copy_on_write() {
            result |= MemoryProtectionEnum::COPY_ON_WRITE;
        }

        result
    }

    fn get_excluded_protection_settings() -> MemoryProtectionEnum {
        let mut result = MemoryProtectionEnum::empty();

        if MemorySettingsConfig::get_excluded_write() {
            result |= MemoryProtectionEnum::WRITE;
        }

        if MemorySettingsConfig::get_excluded_execute() {
            result |= MemoryProtectionEnum::EXECUTE;
        }

        if MemorySettingsConfig::get_excluded_copy_on_write() {
            result |= MemoryProtectionEnum::COPY_ON_WRITE;
        }

        if MemorySettingsConfig::get_excluded_no_cache() {
            result |= MemoryProtectionEnum::NO_CACHE;
        }

        if MemorySettingsConfig::get_excluded_write_combine() {
            result |= MemoryProtectionEnum::WRITE_COMBINE;
        }

        result
    }
}
