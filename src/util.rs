//! Utility functions.

use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a simple unique ID for response objects.
pub fn generate_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{ts:x}")
}
