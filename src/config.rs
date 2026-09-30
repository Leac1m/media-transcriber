use std::path::PathBuf;

pub const MODEL_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin";
pub const MODEL_SIZE: u64 = 487_601_967;
pub const MODEL_SHA256: &str = "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b";

pub fn get_last_dir_path() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|home| PathBuf::from(home).join(".media_transcriber_last_dir"))
}

pub fn get_model_path() -> PathBuf {
    std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".media_transcriber").join("models").join("ggml-small.bin"))
        .unwrap_or_else(|| PathBuf::from("models/ggml-small.bin"))
}

pub fn save_last_dir(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        if let Some(config_path) = get_last_dir_path() {
            let _ = std::fs::write(config_path, parent.to_string_lossy().as_ref());
        }
    }
}

pub fn load_last_dir() -> Option<PathBuf> {
    if let Some(config_path) = get_last_dir_path() {
        if let Ok(content) = std::fs::read_to_string(config_path) {
            let path = PathBuf::from(content.trim());
            if path.exists() && path.is_dir() {
                return Some(path);
            }
        }
    }
    None
}
