use serde::{Deserialize, Serialize};

/// Settings for the agent tracker
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArcadiaSettingsForTracker {
    pub global_upload_factor: i16,
    pub global_download_factor: i16,
}

impl Default for ArcadiaSettingsForTracker {
    fn default() -> Self {
        Self {
            global_upload_factor: 1,
            global_download_factor: 1,
        }
    }
}
