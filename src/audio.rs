use std::process::Command;

pub fn extract_audio(video_path: &str, output_path: &str) -> Result<(), String> {
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-i", video_path,
            "-ar", "16000",
            "-ac", "1",
            "-c:a", "pcm_s16le",
            output_path
        ])
        .status()
        .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;
        
    if status.success() {
        Ok(())
    } else {
        Err("ffmpeg failed to extract audio".into())
    }
}

pub fn parse_wav_file(path: &str) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| format!("Failed to open WAV file: {}", e))?;
    let spec = reader.spec();
    
    if spec.channels != 1 || spec.sample_rate != 16000 || spec.sample_format != hound::SampleFormat::Int {
        return Err("Unsupported WAV format (expected 16kHz mono 16-bit PCM)".into());
    }
    
    let mut audio_data = Vec::new();
    for sample in reader.samples::<i16>() {
        let s = sample.map_err(|e| format!("Error reading sample: {}", e))?;
        audio_data.push(s as f32 / 32768.0);
    }
    
    Ok(audio_data)
}
