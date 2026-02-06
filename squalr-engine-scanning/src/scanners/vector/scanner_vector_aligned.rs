use crate::scanners::snapshot_scanner::Scanner;
use crate::scanners::structures::snapshot_region_filter_run_length_encoder::SnapshotRegionFilterRunLengthEncoder;
use squalr_engine_api::structures::data_types::generics::vector_comparer::VectorComparer;
use squalr_engine_api::structures::data_types::generics::vector_function::GetVectorFunction;
use squalr_engine_api::structures::data_types::generics::vector_generics::VectorGenerics;
use squalr_engine_api::structures::scanning::comparisons::scan_function_scalar::ScanFunctionScalar;
use squalr_engine_api::structures::scanning::comparisons::scan_function_vector::ScanFunctionVector;
use squalr_engine_api::structures::scanning::filters::snapshot_region_filter::SnapshotRegionFilter;
use squalr_engine_api::structures::scanning::plans::element_scan::snapshot_filter_element_scan_plan::SnapshotFilterElementScanPlan;
use squalr_engine_api::structures::snapshots::snapshot_region::SnapshotRegion;
use std::simd::cmp::SimdPartialEq;
use std::simd::{LaneCount, Simd, SupportedLaneCount};

pub struct ScannerVectorAligned<const N: usize>
where
    LaneCount<N>: SupportedLaneCount + VectorComparer<N> + GetVectorFunction<N>, {}

impl<const N: usize> ScannerVectorAligned<N>
where
    LaneCount<N>: SupportedLaneCount + VectorComparer<N> + GetVectorFunction<N>,
{
    fn encode_results(
        compare_result: &Simd<u8, N>,
        run_length_encoder: &mut SnapshotRegionFilterRunLengthEncoder,
        memory_alignment: u64,
        true_mask: Simd<u8, N>,
        false_mask: Simd<u8, N>,
    ) {
        // Optimization: Check if all scan results are true. This helps substantially when scanning for common values like 0.
        if compare_result.simd_eq(true_mask).all() {
            run_length_encoder.encode_range(N as u64);
        // Optimization: Check if all scan results are false. This is also a very common result, and speeds up scans.
        } else if compare_result.simd_eq(false_mask).all() {
            run_length_encoder.finalize_current_encode(N as u64);
        // Otherwise, there is a mix of true/false results that need to be processed manually.
        } else {
            Self::encode_remainder_results(compare_result, run_length_encoder, memory_alignment, N as u64);
        }
    }

    fn encode_remainder_results(
        compare_result: &Simd<u8, N>,
        run_length_encoder: &mut SnapshotRegionFilterRunLengthEncoder,
        memory_alignment: u64,
        remainder_bytes: u64,
    ) {
        let start_byte_index = N.saturating_sub(remainder_bytes as usize);

        for byte_index in (start_byte_index..N).step_by(memory_alignment as usize) {
            if compare_result[byte_index] != 0 {
                run_length_encoder.encode_range(memory_alignment);
            } else {
                run_length_encoder.finalize_current_encode(memory_alignment);
            }
        }
    }
}

/// Implements a CPU-bound SIMD memory region scanner that is optmized for scanning for a sequence of N bytes.
/// In other words, this scan efficiently handles searching for values where the data type size is exactly equal to the memory alignment.
impl<const N: usize> Scanner for ScannerVectorAligned<N>
where
    LaneCount<N>: SupportedLaneCount + VectorComparer<N> + GetVectorFunction<N>,
{
    fn get_scanner_name(&self) -> &'static str {
        &"Vector (Aligned)"
    }

    /// Performs a sequential iteration over a region of memory, performing the scan comparison.
    /// A run-length encoding algorithm is used to generate new sub-regions as the scan progresses.
    fn scan_region(
        &self,
        snapshot_region: &SnapshotRegion,
        snapshot_region_filter: &SnapshotRegionFilter,
        snapshot_filter_element_scan_plan: &SnapshotFilterElementScanPlan,
    ) -> Vec<SnapshotRegionFilter> {
        let current_values_pointer = snapshot_region.get_current_values_filter_pointer(&snapshot_region_filter);
        let previous_value_pointer = snapshot_region.get_previous_values_filter_pointer(&snapshot_region_filter);
        let base_address = snapshot_region_filter.get_base_address();
        let region_size = snapshot_region_filter.get_region_size();

        let mut run_length_encoder = SnapshotRegionFilterRunLengthEncoder::new(base_address);
        let data_type_size = snapshot_filter_element_scan_plan.get_unit_size_in_bytes();
        let memory_alignment_size = snapshot_filter_element_scan_plan.get_memory_alignment() as u64;

        let vectorization_plan = VectorGenerics::plan_vector_scan::<N>(region_size, data_type_size, memory_alignment_size);
        let vectorizable_iterations = vectorization_plan.get_vectorizable_iterations();
        let remainder_ptr_offset = vectorization_plan.get_remainder_ptr_offset();
        let remainder_bytes = vectorization_plan.get_remainder_bytes();

        let false_mask = Simd::<u8, N>::splat(0x00);
        let true_mask = Simd::<u8, N>::splat(0xFF);

        debug_assert!(data_type_size == memory_alignment_size);
        debug_assert!(memory_alignment_size == 1 || memory_alignment_size == 2 || memory_alignment_size == 4 || memory_alignment_size == 8);

        let mut did_vector_scan = false;

        if vectorizable_iterations > 0 {
            if let Some(vector_compare_func) = snapshot_filter_element_scan_plan.get_scan_function_vector() {
                did_vector_scan = true;

                match vector_compare_func {
                    ScanFunctionVector::Immediate(compare_func) => {
                        // Compare as many full vectors as we can.
                        for index in 0..vectorizable_iterations {
                            let current_values_pointer =
                                unsafe { current_values_pointer.add((index * vectorization_plan.vector_size_in_bytes) as usize) };
                            let compare_result = compare_func(current_values_pointer);

                            Self::encode_results(&compare_result, &mut run_length_encoder, memory_alignment_size, true_mask, false_mask);
                        }

                        // Handle remainder elements.
                        if remainder_bytes > 0 {
                            let current_values_pointer = unsafe { current_values_pointer.add(remainder_ptr_offset as usize) };
                            let compare_result = compare_func(current_values_pointer);

                            Self::encode_remainder_results(&compare_result, &mut run_length_encoder, memory_alignment_size, remainder_bytes);
                        }
                    }
                    ScanFunctionVector::RelativeOrDelta(compare_func) => {
                        // Compare as many full vectors as we can.
                        for index in 0..vectorizable_iterations {
                            let current_values_pointer =
                                unsafe { current_values_pointer.add((index * vectorization_plan.vector_size_in_bytes) as usize) };
                            let previous_value_pointer =
                                unsafe { previous_value_pointer.add((index * vectorization_plan.vector_size_in_bytes) as usize) };
                            let compare_result = compare_func(current_values_pointer, previous_value_pointer);

                            Self::encode_results(&compare_result, &mut run_length_encoder, memory_alignment_size, true_mask, false_mask);
                        }

                        // Handle remainder elements.
                        if remainder_bytes > 0 {
                            let current_values_pointer = unsafe { current_values_pointer.add(remainder_ptr_offset as usize) };
                            let previous_value_pointer = unsafe { previous_value_pointer.add(remainder_ptr_offset as usize) };
                            let compare_result = compare_func(current_values_pointer, previous_value_pointer);

                            Self::encode_remainder_results(&compare_result, &mut run_length_encoder, memory_alignment_size, remainder_bytes);
                        }
                    }
                }
            }
        }

        // For filters smaller than a single vector (or when no vector compare is available), fall back to scalar.
        if !did_vector_scan {
            let element_count = vectorization_plan.element_count;

            if let Some(scalar_compare_func) = snapshot_filter_element_scan_plan.get_scan_function_scalar() {
                match scalar_compare_func {
                    ScanFunctionScalar::Immediate(compare_func) => {
                        for index in 0..element_count {
                            let current_value_pointer = unsafe { current_values_pointer.add((index * memory_alignment_size) as usize) };
                            let compare_result = compare_func(current_value_pointer);

                            if compare_result {
                                run_length_encoder.encode_range(memory_alignment_size);
                            } else {
                                run_length_encoder.finalize_current_encode(memory_alignment_size);
                            }
                        }
                    }
                    ScanFunctionScalar::RelativeOrDelta(compare_func) => {
                        for index in 0..element_count {
                            let current_value_pointer = unsafe { current_values_pointer.add((index * memory_alignment_size) as usize) };
                            let previous_value_pointer = unsafe { previous_value_pointer.add((index * memory_alignment_size) as usize) };
                            let compare_result = compare_func(current_value_pointer, previous_value_pointer);

                            if compare_result {
                                run_length_encoder.encode_range(memory_alignment_size);
                            } else {
                                run_length_encoder.finalize_current_encode(memory_alignment_size);
                            }
                        }
                    }
                }
            } else {
                log::error!("No scalar scan function available for aligned scan fallback.");
            }
        }

        run_length_encoder.finalize_current_encode(0);
        run_length_encoder.take_result_regions()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use squalr_engine_api::structures::data_types::built_in_types::i32::data_type_i32::DataTypeI32;
    use squalr_engine_api::structures::data_types::built_in_types::u64::data_type_u64::DataTypeU64;
    use squalr_engine_api::structures::data_types::data_type_ref::DataTypeRef;
    use squalr_engine_api::structures::data_types::floating_point_tolerance::FloatingPointTolerance;
    use squalr_engine_api::structures::data_values::data_value::DataValue;
    use squalr_engine_api::structures::memory::memory_alignment::MemoryAlignment;
    use squalr_engine_api::structures::memory::normalized_region::NormalizedRegion;
    use squalr_engine_api::structures::scanning::comparisons::scan_compare_type::ScanCompareType;
    use squalr_engine_api::structures::scanning::comparisons::scan_compare_type_immediate::ScanCompareTypeImmediate;
    use squalr_engine_api::structures::scanning::constraints::scan_constraint::ScanConstraint;
    use squalr_engine_api::structures::scanning::constraints::scan_constraint_finalized::ScanConstraintFinalized;

    fn make_snapshot_region(
        base_address: u64,
        bytes: Vec<u8>,
    ) -> SnapshotRegion {
        let region_size = bytes.len() as u64;
        let normalized_region = NormalizedRegion::new(base_address, region_size);
        let mut snapshot_region = SnapshotRegion::new(normalized_region, vec![]);
        snapshot_region.current_values = bytes.clone();
        snapshot_region.previous_values = bytes;
        snapshot_region
    }

    #[test]
    fn aligned_vector_scan_small_region_does_not_overread() {
        let base_address = 0u64;
        let region_size = 12u64;

        let snapshot_region = make_snapshot_region(base_address, vec![0u8; region_size as usize]);
        let snapshot_region_filter = SnapshotRegionFilter::new(base_address, region_size);

        let data_value = DataValue::new(DataTypeRef::new(DataTypeI32::DATA_TYPE_ID), 0i32.to_le_bytes().to_vec());
        let scan_constraint = ScanConstraint::new(
            ScanCompareType::Immediate(ScanCompareTypeImmediate::Equal),
            data_value,
            FloatingPointTolerance::default(),
        );
        let scan_constraint_finalized = ScanConstraintFinalized::new(scan_constraint);
        let snapshot_filter_element_scan_plan = SnapshotFilterElementScanPlan::new(
            &scan_constraint_finalized,
            MemoryAlignment::Alignment4,
            FloatingPointTolerance::default(),
        );

        let scanner = ScannerVectorAligned::<16> {};
        let results = scanner.scan_region(&snapshot_region, &snapshot_region_filter, &snapshot_filter_element_scan_plan);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].get_base_address(), base_address);
        assert_eq!(results[0].get_region_size(), region_size);
    }

    #[test]
    fn aligned_vector_scan_ignores_trailing_bytes_without_full_element() {
        let base_address = 0u64;
        let region_size = 20u64; // 2x u64 (16 bytes) + 4 trailing bytes

        let snapshot_region = make_snapshot_region(base_address, vec![0u8; region_size as usize]);
        let snapshot_region_filter = SnapshotRegionFilter::new(base_address, region_size);

        let data_value = DataValue::new(DataTypeRef::new(DataTypeU64::DATA_TYPE_ID), 0u64.to_le_bytes().to_vec());
        let scan_constraint = ScanConstraint::new(
            ScanCompareType::Immediate(ScanCompareTypeImmediate::Equal),
            data_value,
            FloatingPointTolerance::default(),
        );
        let scan_constraint_finalized = ScanConstraintFinalized::new(scan_constraint);
        let snapshot_filter_element_scan_plan = SnapshotFilterElementScanPlan::new(
            &scan_constraint_finalized,
            MemoryAlignment::Alignment8,
            FloatingPointTolerance::default(),
        );

        let scanner = ScannerVectorAligned::<16> {};
        let results = scanner.scan_region(&snapshot_region, &snapshot_region_filter, &snapshot_filter_element_scan_plan);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].get_base_address(), base_address);
        assert_eq!(results[0].get_region_size(), 16);
    }
}
