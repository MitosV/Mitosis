use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use mitosis_bot_runtime::BotAction;

#[derive(Debug, Error)]
pub enum ReplayerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedEvent {
    pub relative_time_ms: u64,
    pub action: BotAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedSession {
    pub name: String,
    pub description: String,
    pub recorded_at: String,
    pub events: Vec<RecordedEvent>,
}

impl RecordedSession {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            recorded_at: chrono_now(),
            events: Vec::new(),
        }
    }

    pub fn add_event(&mut self, relative_time_ms: u64, action: BotAction) {
        self.events.push(RecordedEvent {
            relative_time_ms,
            action,
        });
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), ReplayerError> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &Path) -> Result<Self, ReplayerError> {
        let json = fs::read_to_string(path)?;
        let session = serde_json::from_str(&json)?;
        Ok(session)
    }
}

fn chrono_now() -> String {
    "2026-11-01T23:22:48Z".to_string()
}
