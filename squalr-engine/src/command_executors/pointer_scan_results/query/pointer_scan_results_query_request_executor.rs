use crate::command_executors::privileged_request_executor::PrivilegedCommandRequestExecutor;
use crate::engine_privileged_state::EnginePrivilegedState;
use squalr_engine_api::commands::pointer_scan_results::query::pointer_scan_results_query_request::PointerScanResultsQueryRequest;
use squalr_engine_api::commands::pointer_scan_results::query::pointer_scan_results_query_response::PointerScanResultsQueryResponse;
use std::sync::Arc;

impl PrivilegedCommandRequestExecutor for PointerScanResultsQueryRequest {
    type ResponseType = PointerScanResultsQueryResponse;

    fn execute(
        &self,
        engine_privileged_state: &Arc<EnginePrivilegedState>,
    ) -> <Self as PrivilegedCommandRequestExecutor>::ResponseType {
        let mut results = Vec::new();
        let mut last_page_index = 0;
        let mut result_count = 0;
        let mut page_size = 512u64;

        if let Ok(pointer_scan_results) = engine_privileged_state.get_pointer_scan_results().read() {
            result_count = pointer_scan_results.get_result_count();
            last_page_index = pointer_scan_results.get_last_page_index();
            page_size = pointer_scan_results.get_page_size();
            results = pointer_scan_results.query_page(self.page_index.clamp(0, last_page_index));
        }

        PointerScanResultsQueryResponse {
            results,
            page_index: self.page_index,
            last_page_index,
            page_size,
            result_count,
        }
    }
}
