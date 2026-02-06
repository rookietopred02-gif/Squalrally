use crate::events::pointer_scan_results::updated::pointer_scan_results_updated_event::PointerScanResultsUpdatedEvent;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PointerScanResultsEvent {
    PointerScanResultsUpdated { pointer_scan_results_updated_event: PointerScanResultsUpdatedEvent },
}
