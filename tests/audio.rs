//! These tests run FFmpeg, so it must be installed and on PATH.

use std::path::Path;

use media_transcriber::audio::{WHISPER_SAMPLE_RATE, load_audio};

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn decodes_and_resamples_to_16khz_mono() {
    // 0.5 s of a 440 Hz tone, 22.05 kHz stereo.
    let samples = load_audio(&fixture("tone-22khz-stereo.wav")).unwrap();

    let expected = WHISPER_SAMPLE_RATE as usize / 2;
    assert!(
        samples.len().abs_diff(expected) <= 16,
        "got {} samples, expected about {expected}",
        samples.len()
    );

    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    assert!(rms > 0.05, "decoded audio is silent (rms {rms})");
}

#[test]
fn reports_files_without_audio() {
    let err = load_audio(&fixture("video-only.mp4")).unwrap_err();
    assert_eq!(err, "This file has no audio track.");
}

#[test]
fn reports_undecodable_files() {
    // Deterministic noise that no demuxer recognizes. (Plain text won't do:
    // FFmpeg reads it as an ANSI-art video.)
    let mut state: u32 = 0x1234_5678;
    let noise: Vec<u8> = (0..4096)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 24) as u8
        })
        .collect();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("garbage.mp4");
    std::fs::write(&path, noise).unwrap();

    let err = load_audio(&path).unwrap_err();
    assert!(err.starts_with("Could not decode audio"), "{err}");
}
