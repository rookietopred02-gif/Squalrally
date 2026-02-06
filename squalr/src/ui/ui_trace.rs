use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

static TRACE_PATH: OnceLock<PathBuf> = OnceLock::new();

fn trace_path() -> &'static Path {
    TRACE_PATH
        .get_or_init(|| std::env::temp_dir().join("squalr_ui_trace.log"))
        .as_path()
}

pub fn is_enabled() -> bool {
    matches!(std::env::var("SQUALR_UI_TRACE").as_deref(), Ok("1") | Ok("true") | Ok("TRUE"))
}

pub fn trace(message: impl AsRef<str>) {
    if !is_enabled() {
        return;
    }

    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(trace_path())
    {
        let _ = writeln!(file, "[{}] {}", timestamp_ms, message.as_ref());
        let _ = file.flush();
    }
}

