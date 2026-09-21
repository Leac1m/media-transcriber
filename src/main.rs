slint::include_modules!();

use std::path::PathBuf;
use std::thread;
use std::sync::Arc;
use std::sync::Mutex;
use notify_rust::Notification;
use media_transcriber::audio::{extract_audio, parse_wav_file};
use media_transcriber::config::{load_last_dir, save_last_dir};
use media_transcriber::transcription::transcribe_audio;
use media_transcriber::model::download_model;

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let model_path = "models/ggml-small.bin";
    let model_file_path = std::path::Path::new(model_path);
    if !model_file_path.exists() {
        ui.set_show_download_popup(true);
    }

    let ui_handle_download = ui.as_weak();
    ui.on_start_download(move || {
        let ui_bg = ui_handle_download.clone();
        thread::spawn(move || {
            let model_path = "models/ggml-small.bin";
            let model_file_path = std::path::Path::new(model_path);
            
            let download_progress_callback = {
                let ui_bg = ui_bg.clone();
                move |progress: f32| {
                    let _ = slint::invoke_from_event_loop({
                        let ui_bg = ui_bg.clone();
                        move || {
                            if let Some(ui) = ui_bg.upgrade() {
                                ui.set_download_progress(progress);
                            }
                        }
                    });
                }
            };
            
            if let Err(e) = download_model(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
                model_file_path,
                download_progress_callback
            ) {
                println!("Error downloading model: {}", e);
                // optionally handle error in UI
            }
            
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_bg.upgrade() {
                    ui.set_show_download_popup(false);
                }
            });
        });
    });

    // Shared state
    let video_path = Arc::new(Mutex::new(None::<PathBuf>));
    
    // Select file logic
    let ui_handle = ui.as_weak();
    let video_path_clone = Arc::clone(&video_path);
    ui.on_select_file(move || {
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Media Files", &["mp4", "mkv", "avi", "mov", "webm", "mp3", "wav", "m4a", "flac"]);
            
        if let Some(last_dir) = load_last_dir() {
            dialog = dialog.set_directory(last_dir);
        }

        if let Some(path) = dialog.pick_file() 
        {
            save_last_dir(&path);
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
            ui.set_text_with_timestamps("Starting transcription process...\nChecking requirements...".into()); ui.set_text_without_timestamps("Starting transcription process...\nChecking requirements...".into());
            ui.set_progress(0.0);
            ui.set_is_processing(true);
        }
        
        let ui_bg = ui_handle_transcribe.clone();
        
        thread::spawn(move || {
            let wav_path = "/tmp/media_transcriber_audio.wav";

            let _ = slint::invoke_from_event_loop({
                let ui_bg = ui_bg.clone();
                move || {
                    if let Some(ui) = ui_bg.upgrade() {
                        ui.set_progress(0.05);
                        ui.set_text_with_timestamps("Extracting audio with ffmpeg...".into()); ui.set_text_without_timestamps("Extracting audio with ffmpeg...".into());
                    }
                }
            });

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
            
            // 3. Initialize Whisper and transcribe
            let model_path = "models/ggml-small.bin"; 
            
            let progress_callback = {
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
            };
            
            match transcribe_audio(model_path, &audio_data, progress_callback) {
                Ok((full_text_stamped, full_text_raw)) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_bg.upgrade() {
                            ui.set_progress(1.0);
                            ui.set_is_processing(false);
                            ui.set_text_with_timestamps(full_text_stamped.into()); 
                            ui.set_text_without_timestamps(full_text_raw.into());
                        }
                    });
                    
                    let _ = Notification::new()
                        .summary("Media Transcriber")
                        .body("Transcription complete!")
                        .icon("media-transcriber")
                        .show();
                },
                Err(e) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_bg.upgrade() {
                            ui.set_text_with_timestamps(format!("Failed to transcribe: {}", e).into()); ui.set_text_without_timestamps(format!("Failed to transcribe: {}", e).into());
                            ui.set_is_processing(false);
                        }
                    });
                }
            }
        });
    });

    ui.on_copy_to_clipboard(move |text| {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text.to_string());
        }
    });
    
    ui.on_save_to_file(move |text| {
        let mut dialog = rfd::FileDialog::new().add_filter("Text", &["txt"]);
        if let Some(last_dir) = load_last_dir() {
            dialog = dialog.set_directory(last_dir);
        }
        
        if let Some(path) = dialog.save_file() {
            save_last_dir(&path);
            let _ = std::fs::write(path, text.to_string());
        }
    });

    ui.run()
}
