use crate::commands::pointer_scan_results::pointer_scan_results_response::PointerScanResultsResponse;
use crate::commands::privileged_command_response::PrivilegedCommandResponse;
use crate::commands::privileged_command_response::TypedPrivilegedCommandResponse;
use crate::structures::pointer_scan::pointer_scan_result::PointerScanResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PointerScanResultsQueryResponse {
    pub results: Vec<PointerScanResult>,
    pub page_index: u64,
    pub last_page_index: u64,
    pub page_size: u64,
    pub result_count: u64,
}

impl TypedPrivilegedCommandResponse for PointerScanResultsQueryResponse {
    fn to_engine_response(&self) -> PrivilegedCommandResponse {
        PrivilegedCommandResponse::PointerScanResults(PointerScanResultsResponse::Query {
            pointer_scan_results_query_response: self.clone(),
        })
    }

    fn from_engine_response(response: PrivilegedCommandResponse) -> Result<Self, PrivilegedCommandResponse> {
        if let PrivilegedCommandResponse::PointerScanResults(PointerScanResultsResponse::Query {
            pointer_scan_results_query_response,
        }) = response
        {
            Ok(pointer_scan_results_query_response)
        } else {
            Err(response)
        }
    }
}
