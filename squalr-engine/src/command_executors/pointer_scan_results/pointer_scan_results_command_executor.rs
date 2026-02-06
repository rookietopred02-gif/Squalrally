use crate::command_executors::privileged_command_executor::PrivilegedCommandExecutor;
use crate::command_executors::privileged_request_executor::PrivilegedCommandRequestExecutor;
use crate::engine_privileged_state::EnginePrivilegedState;
use squalr_engine_api::commands::pointer_scan_results::pointer_scan_results_command::PointerScanResultsCommand;
use squalr_engine_api::commands::privileged_command_response::{PrivilegedCommandResponse, TypedPrivilegedCommandResponse};
use std::sync::Arc;

impl PrivilegedCommandExecutor for PointerScanResultsCommand {
    type ResponseType = PrivilegedCommandResponse;

    fn execute(
        &self,
        engine_privileged_state: &Arc<EnginePrivilegedState>,
    ) -> <Self as PrivilegedCommandExecutor>::ResponseType {
        match self {
            PointerScanResultsCommand::Query { results_query_request } => results_query_request
                .execute(engine_privileged_state)
                .to_engine_response(),
        }
    }
}
