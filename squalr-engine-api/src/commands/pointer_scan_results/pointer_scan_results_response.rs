use crate::commands::pointer_scan_results::query::pointer_scan_results_query_response::PointerScanResultsQueryResponse;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PointerScanResultsResponse {
    Query {
        pointer_scan_results_query_response: PointerScanResultsQueryResponse,
    },
}
