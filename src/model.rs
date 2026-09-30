use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::config::{MODEL_SHA256, MODEL_SIZE, MODEL_URL};

/// A model counts as installed only if it has the expected size, so a file
/// truncated by an older, non-atomic download is fetched again.
pub fn is_model_installed(path: &Path) -> bool {
    fs::metadata(path)
        .map(|m| m.len() == MODEL_SIZE)
        .unwrap_or(false)
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
    let response = ureq::get(MODEL_URL)
        .call()
        .map_err(|e| format!("Failed to download model: {}", e))?;

    install_verified(
        response.into_body().into_reader(),
        output_path,
        MODEL_SIZE,
        MODEL_SHA256,
        progress_callback,
    )
}

/// Streams `reader` to `output_path`, verifying its size and SHA-256 first.
fn install_verified<F>(
    reader: impl Read,
    output_path: &Path,
    expected_size: u64,
    expected_sha256: &str,
    progress_callback: F,
) -> Result<(), String>
where
    F: FnMut(f32),
{
    let part_path = part_path(output_path);
    let result = write_verified(
        reader,
        &part_path,
        expected_size,
        expected_sha256,
        progress_callback,
    )
    .and_then(|_| {
        fs::rename(&part_path, output_path)
            .map_err(|e| format!("Failed to move model into place: {}", e))
    });

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

fn write_verified<F>(
    mut reader: impl Read,
    part_path: &Path,
    expected_size: u64,
    expected_sha256: &str,
    mut progress_callback: F,
) -> Result<(), String>
where
    F: FnMut(f32),
{
    if let Some(parent) = part_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create models directory: {}", e))?;
    }

    // Scoped so the file is closed before it is renamed (required on Windows).
    let (downloaded, digest) = {
        let mut file =
            File::create(part_path).map_err(|e| format!("Failed to create output file: {}", e))?;
        let mut hasher = Sha256::new();

        let mut buffer = [0; 64 * 1024];
        let mut downloaded: u64 = 0;
        let mut last_permille = 0;

        loop {
            let bytes_read = reader
                .read(&mut buffer)
                .map_err(|e| format!("Error reading body: {}", e))?;
            if bytes_read == 0 {
                break;
            }

            file.write_all(&buffer[..bytes_read])
                .map_err(|e| format!("Error writing to file: {}", e))?;
            hasher.update(&buffer[..bytes_read]);
            downloaded += bytes_read as u64;

            // Report at most once per 0.1% so the UI event loop isn't flooded.
            let permille = downloaded * 1000 / expected_size.max(1);
            if permille > last_permille {
                last_permille = permille;
                progress_callback((downloaded as f32 / expected_size as f32).min(1.0));
            }
        }

        file.sync_all()
            .map_err(|e| format!("Error writing to file: {}", e))?;

        let digest: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        (downloaded, digest)
    };

    if downloaded != expected_size {
        return Err(format!(
            "Download incomplete: received {} of {} bytes",
            downloaded, expected_size
        ));
    }
    if digest != expected_sha256 {
        return Err("Downloaded model is corrupted (checksum mismatch)".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO: &[u8] = b"hello world";
    const HELLO_SHA256: &str = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";

    fn install(data: &[u8], output: &Path, size: u64, sha256: &str) -> Result<Vec<f32>, String> {
        let mut progress = Vec::new();
        install_verified(data, output, size, sha256, |p| progress.push(p))?;
        Ok(progress)
    }

    #[test]
    fn installs_file_when_checksum_matches() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("models").join("model.bin");

        let progress = install(HELLO, &output, HELLO.len() as u64, HELLO_SHA256).unwrap();

        assert_eq!(fs::read(&output).unwrap(), HELLO);
        assert!(!part_path(&output).exists());
        assert_eq!(progress.last(), Some(&1.0));
    }

    #[test]
    fn replaces_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("model.bin");
        fs::write(&output, b"truncated").unwrap();

        install(HELLO, &output, HELLO.len() as u64, HELLO_SHA256).unwrap();

        assert_eq!(fs::read(&output).unwrap(), HELLO);
    }

    #[test]
    fn rejects_incomplete_download() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("model.bin");

        let err = install(HELLO, &output, 100, HELLO_SHA256).unwrap_err();

        assert!(err.contains("incomplete"), "{err}");
        assert!(!output.exists());
        assert!(!part_path(&output).exists());
    }

    #[test]
    fn rejects_corrupted_download() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("model.bin");
        let wrong_sha = "0".repeat(64);

        let err = install(HELLO, &output, HELLO.len() as u64, &wrong_sha).unwrap_err();

        assert!(err.contains("checksum"), "{err}");
        assert!(!output.exists());
        assert!(!part_path(&output).exists());
    }

    #[test]
    fn model_with_wrong_size_is_not_installed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ggml-small.bin");
        assert!(!is_model_installed(&path));

        fs::write(&path, HELLO).unwrap();
        assert!(!is_model_installed(&path));
    }

    #[test]
    fn part_file_sits_next_to_output() {
        let output = Path::new("models").join("ggml-small.bin");
        assert_eq!(
            part_path(&output),
            Path::new("models").join("ggml-small.bin.part")
        );
    }
}
