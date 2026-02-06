use crate::commands::pointer_scan_results::pointer_scan_results_command::PointerScanResultsCommand;
use crate::commands::pointer_scan_results::pointer_scan_results_response::PointerScanResultsResponse;
use crate::commands::privileged_command::PrivilegedCommand;
use crate::commands::privileged_command_request::PrivilegedCommandRequest;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(Clone, StructOpt, Debug, Serialize, Deserialize)]
pub struct PointerScanResultsQueryRequest {
    #[structopt(short = "p", long)]
    pub page_index: u64,
}

impl PrivilegedCommandRequest for PointerScanResultsQueryRequest {
    type ResponseType = PointerScanResultsQueryResponse;

    fn to_engine_command(&self) -> PrivilegedCommand {
        PrivilegedCommand::PointerScanResults(PointerScanResultsCommand::Query {
            results_query_request: self.clone(),
        })
    }
}

impl From<PointerScanResultsQueryResponse> for PointerScanResultsResponse {
    fn from(pointer_scan_results_query_response: PointerScanResultsQueryResponse) -> Self {
        PointerScanResultsResponse::Query {
            pointer_scan_results_query_response,
        }
    }
}

use crate::commands::pointer_scan_results::query::pointer_scan_results_query_response::PointerScanResultsQueryResponse;
