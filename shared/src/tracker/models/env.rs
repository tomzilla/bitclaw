use serde::{Deserialize, Serialize};

/// Settings for the agent tracker
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BitclawSettingsForTracker {
    pub global_upload_factor: i16,
    pub global_download_factor: i16,
}

impl Default for BitclawSettingsForTracker {
    fn default() -> Self {
        Self {
            global_upload_factor: 1,
            global_download_factor: 1,
        }
    }
}
