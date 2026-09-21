use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

pub fn download_model<F>(model_url: &str, output_path: &Path, mut progress_callback: F) -> Result<(), String>
where
    F: FnMut(f32) + Send + 'static,
{
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create models directory: {}", e))?;
        }
    }

    let response = ureq::get(model_url)
        .call()
        .map_err(|e| format!("Failed to download model: {}", e))?;

    let total_size = response
        .headers()
        .get("Content-Length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let mut reader = response.into_body().into_reader();
    let mut file = File::create(output_path).map_err(|e| format!("Failed to create output file: {}", e))?;

    let mut buffer = [0; 8192];
    let mut downloaded = 0;

    loop {
        let bytes_read = reader.read(&mut buffer).map_err(|e| format!("Error reading body: {}", e))?;
        if bytes_read == 0 {
            break;
        }
        
        file.write_all(&buffer[..bytes_read]).map_err(|e| format!("Error writing to file: {}", e))?;
        downloaded += bytes_read as u64;
        
        if total_size > 0 {
            let progress = downloaded as f32 / total_size as f32;
            progress_callback(progress);
        }
    }

    Ok(())
}
