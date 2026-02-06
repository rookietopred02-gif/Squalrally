use crate::commands::pointer_scan_results::query::pointer_scan_results_query_request::PointerScanResultsQueryRequest;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(Clone, StructOpt, Debug, Serialize, Deserialize)]
pub enum PointerScanResultsCommand {
    /// Query pointer scan results.
    Query {
        #[structopt(flatten)]
        results_query_request: PointerScanResultsQueryRequest,
    },
}
