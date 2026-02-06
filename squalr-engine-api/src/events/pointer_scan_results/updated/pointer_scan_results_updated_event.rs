use crate::events::{
    engine_event::{EngineEvent, EngineEventRequest},
    pointer_scan_results::pointer_scan_results_event::PointerScanResultsEvent,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PointerScanResultsUpdatedEvent {}

impl EngineEventRequest for PointerScanResultsUpdatedEvent {
    fn to_engine_event(&self) -> EngineEvent {
        EngineEvent::PointerScanResults(PointerScanResultsEvent::PointerScanResultsUpdated {
            pointer_scan_results_updated_event: self.clone(),
        })
    }
}
