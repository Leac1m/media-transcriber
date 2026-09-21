use std::path::PathBuf;

pub fn get_last_dir_path() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|home| PathBuf::from(home).join(".media_transcriber_last_dir"))
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
