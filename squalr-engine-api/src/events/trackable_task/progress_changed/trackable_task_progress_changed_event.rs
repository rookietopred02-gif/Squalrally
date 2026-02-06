use crate::events::{
    engine_event::{EngineEvent, EngineEventRequest},
    trackable_task::trackable_task_event::TrackableTaskEvent,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackableTaskProgressChangedEvent {
    pub task_id: String,
    pub progress: f32,
}

impl EngineEventRequest for TrackableTaskProgressChangedEvent {
    fn to_engine_event(&self) -> EngineEvent {
        EngineEvent::TrackableTask(TrackableTaskEvent::ProgressChanged {
            progress_changed_event: self.clone(),
        })
    }
}
