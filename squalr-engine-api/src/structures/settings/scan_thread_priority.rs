use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ScanThreadPriority {
    Normal,
    AboveNormal,
    Highest,
}

impl Default for ScanThreadPriority {
    fn default() -> Self {
        ScanThreadPriority::Normal
    }
}

impl FromStr for ScanThreadPriority {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.trim().to_lowercase().as_str() {
            "normal" => Ok(ScanThreadPriority::Normal),
            "abovenormal" | "above_normal" | "above-normal" | "above" => Ok(ScanThreadPriority::AboveNormal),
            "highest" | "high" => Ok(ScanThreadPriority::Highest),
            _ => Err(format!("Unknown thread priority: {}", input)),
        }
    }
}
