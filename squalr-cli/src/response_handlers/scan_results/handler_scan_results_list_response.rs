use squalr_engine_api::commands::scan_results::list::scan_results_list_response::ScanResultsListResponse;
use squalr_engine_api::structures::data_values::anonymous_value_string_format::AnonymousValueStringFormat;

pub fn handle_scan_results_list_response(results_list_response: ScanResultsListResponse) {
    for scan_result in results_list_response.scan_results {
        let address = scan_result.get_address();
        let value = scan_result
            .get_current_display_value(AnonymousValueStringFormat::String)
            .map(|value| value.get_anonymous_value_string())
            .unwrap_or("??");

        log::info!("0x{:X}\t{}", address, value);
    }
}
