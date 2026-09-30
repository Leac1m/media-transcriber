use std::path::PathBuf;

pub const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin";
pub const MODEL_SIZE: u64 = 487_601_967;
pub const MODEL_SHA256: &str = "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b";

const APP_DIR: &str = "media-transcriber";

/// The model lives in the per-user local data directory:
/// `~/.local/share` on Linux, `~/Library/Application Support` on macOS and
/// `%LOCALAPPDATA%` on Windows (local, not roaming, since it is ~465 MB).
pub fn get_model_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR)
        .join("models")
        .join("ggml-small.bin")
}

fn get_last_dir_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join(APP_DIR).join("last_dir"))
}

pub fn save_last_dir(path: &std::path::Path) {
    if let (Some(parent), Some(config_path)) = (path.parent(), get_last_dir_path()) {
        if let Some(config_dir) = config_path.parent() {
            let _ = std::fs::create_dir_all(config_dir);
        }
        let _ = std::fs::write(config_path, parent.to_string_lossy().as_ref());
    }
}

pub fn load_last_dir() -> Option<PathBuf> {
    let content = std::fs::read_to_string(get_last_dir_path()?).ok()?;
    let path = PathBuf::from(content.trim());
    path.is_dir().then_some(path)
}
