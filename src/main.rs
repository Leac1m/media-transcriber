slint::include_modules!();

use std::process::Command;
use std::path::PathBuf;
use std::thread;
use std::sync::Arc;
use std::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

fn extract_audio(video_path: &str, output_path: &str) -> Result<(), String> {
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

fn parse_wav_file(path: &str) -> Result<Vec<f32>, String> {
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

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    // Shared state
    let video_path = Arc::new(Mutex::new(None::<PathBuf>));
    
    // Select file logic
    let ui_handle = ui.as_weak();
    let video_path_clone = Arc::clone(&video_path);
    ui.on_select_file(move || {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Media Files", &["mp4", "mkv", "avi", "mov", "webm", "mp3", "wav", "m4a", "flac"])
            .pick_file() 
        {
            *video_path_clone.lock().unwrap() = Some(path.clone());
            println!("Video path: {video_path_clone:#?}");
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_transcription_text(format!("Selected: {}\nClick 'Start Transcription' to begin.", path.display()).into());
            }
        }
    });
    
    let ui_handle_transcribe = ui.as_weak();
    let video_path_transcribe = Arc::clone(&video_path);
    
    ui.on_start_transcription(move || {
        let path_opt = video_path_transcribe.lock().unwrap().clone();
        if path_opt.is_none() {
            if let Some(ui) = ui_handle_transcribe.upgrade() {
                ui.set_transcription_text("Please select a file first.".into());
            }
            return;
        }
        let input_path = path_opt.unwrap().to_string_lossy().to_string();
        
        if let Some(ui) = ui_handle_transcribe.upgrade() {
            ui.set_transcription_text("Starting transcription process...\nExtracting audio with ffmpeg...".into());
            ui.set_progress(0.0);
            ui.set_is_processing(true);
        }
        
        let ui_bg = ui_handle_transcribe.clone();
        
        thread::spawn(move || {
            let wav_path = "/tmp/media_transcriber_audio.wav";
            
            // 1. Extract audio
            if let Err(e) = extract_audio(&input_path, wav_path) {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_bg.upgrade() {
                        ui.set_transcription_text(format!("Error extracting audio: {}", e).into());
                        ui.set_is_processing(false);
                    }
                });
                return;
            }
            
            let _ = slint::invoke_from_event_loop({
                let ui_bg = ui_bg.clone();
                move || {
                    if let Some(ui) = ui_bg.upgrade() {
                        ui.set_transcription_text("Audio extracted. Loading Whisper model...".into());
                    }
                }
            });
            
            // 2. Parse WAV
            let audio_data = match parse_wav_file(wav_path) {
                Ok(data) => data,
                Err(e) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_bg.upgrade() {
                            ui.set_transcription_text(format!("Error parsing audio: {}", e).into());
                            ui.set_is_processing(false);
                        }
                    });
                    return;
                }
            };
            
            let _ = slint::invoke_from_event_loop({
                let ui_bg = ui_bg.clone();
                move || {
                    if let Some(ui) = ui_bg.upgrade() {
                        ui.set_transcription_text("Model loaded. Transcribing...".into());
                    }
                }
            });
            
            // 3. Initialize Whisper
            let model_path = "models/ggml-small.bin"; 
            let ctx = match WhisperContext::new_with_params(model_path, WhisperContextParameters::default()) {
                Ok(c) => c,
                Err(e) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_bg.upgrade() {
                            ui.set_transcription_text(format!("Failed to load model: {}", e).into());
                            ui.set_is_processing(false);
                        }
                    });
                    return;
                }
            };
            
            let mut state = ctx.create_state().expect("failed to create state");
            
            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            params.set_print_progress(false);
            params.set_print_special(false);
            params.set_print_realtime(false);
            params.set_print_timestamps(false);
            
            // Just run it synchronously in this thread
            if let Err(e) = state.full(params, &audio_data[..]) {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_bg.upgrade() {
                        ui.set_transcription_text(format!("Failed to transcribe: {}", e).into());
                        ui.set_is_processing(false);
                    }
                });
                return;
            }
            
            let num_segments = state.full_n_segments();
            let mut full_text = String::new();
            
            for i in 0..num_segments {
                if let Some(segment) = state.get_segment(i) {
                    full_text.push_str(&segment.to_str().unwrap_or_else(|_| "".into()));
                    full_text.push('\n');
                }
            }
            
            if full_text.trim().is_empty() {
                full_text = "No speech detected.".to_string();
            }
            
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_bg.upgrade() {
                    ui.set_progress(1.0);
                    ui.set_is_processing(false);
                    ui.set_transcription_text(full_text.into());
                }
            });
            
        });
    });

    ui.run()
}
