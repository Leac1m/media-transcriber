use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};

use crate::config::{MODEL_SHA256, MODEL_SIZE, MODEL_URL};

/// A model counts as installed only if it has the expected size, so a file
/// truncated by an older, non-atomic download is fetched again.
pub fn is_model_installed(path: &Path) -> bool {
    fs::metadata(path).map(|m| m.len() == MODEL_SIZE).unwrap_or(false)
}

/// Downloads the Whisper model to `output_path`.
///
/// The data is streamed to a `.part` file and only renamed into place once
/// its SHA-256 matches, so an interrupted or corrupted download never leaves
/// a file at `output_path`.
pub fn download_model<F>(output_path: &Path, progress_callback: F) -> Result<(), String>
where
    F: FnMut(f32) + Send + 'static,
{
    let part_path = part_path(output_path);
    let result = download_to(&part_path, progress_callback)
        .and_then(|_| fs::rename(&part_path, output_path).map_err(|e| format!("Failed to move model into place: {}", e)));

    if result.is_err() {
        let _ = fs::remove_file(&part_path);
    }
    result
}

fn part_path(output_path: &Path) -> PathBuf {
    let mut name = output_path.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    output_path.with_file_name(name)
}

fn download_to<F>(part_path: &Path, mut progress_callback: F) -> Result<(), String>
where
    F: FnMut(f32),
{
    if let Some(parent) = part_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create models directory: {}", e))?;
    }

    let response = ureq::get(MODEL_URL)
        .call()
        .map_err(|e| format!("Failed to download model: {}", e))?;

    let mut reader = response.into_body().into_reader();
    let mut file = File::create(part_path).map_err(|e| format!("Failed to create output file: {}", e))?;
    let mut hasher = Sha256::new();

    let mut buffer = [0; 64 * 1024];
    let mut downloaded: u64 = 0;
    let mut last_permille = 0;

    loop {
        let bytes_read = reader.read(&mut buffer).map_err(|e| format!("Error reading body: {}", e))?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read]).map_err(|e| format!("Error writing to file: {}", e))?;
        hasher.update(&buffer[..bytes_read]);
        downloaded += bytes_read as u64;

        // Report at most once per 0.1% so the UI event loop isn't flooded.
        let permille = downloaded * 1000 / MODEL_SIZE;
        if permille > last_permille {
            last_permille = permille;
            progress_callback((downloaded as f32 / MODEL_SIZE as f32).min(1.0));
        }
    }

    file.sync_all().map_err(|e| format!("Error writing to file: {}", e))?;

    if downloaded != MODEL_SIZE {
        return Err(format!(
            "Download incomplete: received {} of {} bytes",
            downloaded, MODEL_SIZE
        ));
    }

    let digest: String = hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect();
    if digest != MODEL_SHA256 {
        return Err("Downloaded model is corrupted (checksum mismatch)".into());
    }

    Ok(())
}
