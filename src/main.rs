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
                ui.set_text_with_timestamps(format!("Selected: {}\nClick 'Start Transcription' to begin.", path.display()).into()); ui.set_text_without_timestamps(format!("Selected: {}\nClick 'Start Transcription' to begin.", path.display()).into());
            }
        }
    });
    
    let ui_handle_transcribe = ui.as_weak();
    let video_path_transcribe = Arc::clone(&video_path);
    
    ui.on_start_transcription(move || {
        let path_opt = video_path_transcribe.lock().unwrap().clone();
        if path_opt.is_none() {
            if let Some(ui) = ui_handle_transcribe.upgrade() {
                ui.set_text_with_timestamps("Please select a file first.".into()); ui.set_text_without_timestamps("Please select a file first.".into());
            }
            return;
        }
        let input_path = path_opt.unwrap().to_string_lossy().to_string();
        
        if let Some(ui) = ui_handle_transcribe.upgrade() {
            ui.set_text_with_timestamps("Starting transcription process...\nExtracting audio with ffmpeg...".into()); ui.set_text_without_timestamps("Starting transcription process...\nExtracting audio with ffmpeg...".into());
            ui.set_progress(0.05);
            ui.set_is_processing(true);
        }
        
        let ui_bg = ui_handle_transcribe.clone();
        
        thread::spawn(move || {
            let wav_path = "/tmp/media_transcriber_audio.wav";
            
            // 1. Extract audio
            if let Err(e) = extract_audio(&input_path, wav_path) {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_bg.upgrade() {
                        ui.set_text_with_timestamps(format!("Error extracting audio: {}", e).into()); ui.set_text_without_timestamps(format!("Error extracting audio: {}", e).into());
                        ui.set_is_processing(false);
                    }
                });
                return;
            }
            
            let _ = slint::invoke_from_event_loop({
                let ui_bg = ui_bg.clone();
                move || {
                    if let Some(ui) = ui_bg.upgrade() {
                        ui.set_progress(0.10);
                        ui.set_text_with_timestamps("Audio extracted. Loading Whisper model...".into()); ui.set_text_without_timestamps("Audio extracted. Loading Whisper model...".into());
                    }
                }
            });
            
            // 2. Parse WAV
            let audio_data = match parse_wav_file(wav_path) {
                Ok(data) => data,
                Err(e) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_bg.upgrade() {
                            ui.set_text_with_timestamps(format!("Error parsing audio: {}", e).into()); ui.set_text_without_timestamps(format!("Error parsing audio: {}", e).into());
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
                        ui.set_progress(0.15);
                        ui.set_text_with_timestamps("Model loaded. Transcribing...".into()); ui.set_text_without_timestamps("Model loaded. Transcribing...".into());
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
                            ui.set_text_with_timestamps(format!("Failed to load model: {}", e).into()); ui.set_text_without_timestamps(format!("Failed to load model: {}", e).into());
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
            
            params.set_progress_callback_safe({
                let ui_bg = ui_bg.clone();
                move |progress| {
                    let _ = slint::invoke_from_event_loop({
                        let ui_bg = ui_bg.clone();
                        move || {
                            if let Some(ui) = ui_bg.upgrade() {
                                let p = 0.20 + (progress as f32 / 100.0) * 0.80;
                                ui.set_progress(p);
                                let msg = format!("Model loaded. Transcribing... {}%", progress);
                                ui.set_text_with_timestamps(msg.clone().into());
                                ui.set_text_without_timestamps(msg.into());
                            }
                        }
                    });
                }
            });
            
            // Just run it synchronously in this thread
            if let Err(e) = state.full(params, &audio_data[..]) {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_bg.upgrade() {
                        ui.set_text_with_timestamps(format!("Failed to transcribe: {}", e).into()); ui.set_text_without_timestamps(format!("Failed to transcribe: {}", e).into());
                        ui.set_is_processing(false);
                    }
                });
                return;
            }
            
            let num_segments = state.full_n_segments();
            let mut full_text_stamped = String::new();
            let mut full_text_raw = String::new();
            
            for i in 0..num_segments {
                if let Some(segment) = state.get_segment(i) {
                    let text = segment.to_str().unwrap_or_else(|_| "".into());
                    
                    let start = segment.start_timestamp();
                    let start_sec = start / 100;
                    let m = start_sec / 60;
                    let s = start_sec % 60;
                    let ms = (start % 100) * 10;
                    
                    let stamped_line = format!("[{:02}:{:02}.{:03}] {}\n", m, s, ms, text.trim());
                    
                    full_text_stamped.push_str(&stamped_line);
                    full_text_raw.push_str(&text);
                    full_text_raw.push('\n');
                }
            }
            
            if full_text_raw.trim().is_empty() {
                full_text_raw = "No speech detected.".to_string();
                full_text_stamped = full_text_raw.clone();
            }
            
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_bg.upgrade() {
                    ui.set_progress(1.0);
                    ui.set_is_processing(false);
                    ui.set_text_with_timestamps(full_text_stamped.into()); 
                    ui.set_text_without_timestamps(full_text_raw.into());
                }
            });
            
        });
    });

    ui.on_copy_to_clipboard(move |text| {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text.to_string());
        }
    });
    
    ui.on_save_to_file(move |text| {
        if let Some(path) = rfd::FileDialog::new().add_filter("Text", &["txt"]).save_file() {
            let _ = std::fs::write(path, text.to_string());
        }
    });

    ui.run()
}
