use std::path::PathBuf;
use std::io::Write;
use clap::Parser;
use media_transcriber::audio::{extract_audio, parse_wav_file};
use media_transcriber::transcription::transcribe_audio;
use media_transcriber::model::download_model;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The media files to transcribe
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

fn main() {
    let args = Args::parse();
    
    let model_path = "models/ggml-small.bin";
    let model_file_path = std::path::Path::new(model_path);
    
    // 0. Download model if missing
    if !model_file_path.exists() {
        println!("Whisper model not found locally.");
        println!("Starting download of ggml-small.bin (approx 140MB)...");
        
        let mut last_percent = -1;
        let progress_callback = move |progress: f32| {
            let percent = (progress * 100.0) as i32;
            if percent > last_percent {
                print!("\rDownloading: {}%", percent);
                std::io::stdout().flush().unwrap();
                last_percent = percent;
            }
        };
        
        if let Err(e) = download_model(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
            model_file_path,
            progress_callback
        ) {
            eprintln!("\nError downloading model: {}", e);
            std::process::exit(1);
        }
        println!("\nDownload complete.");
    }
    
    for input_file in args.files {
        let input_path_str = input_file.to_string_lossy().to_string();
        println!("\nProcessing: {}", input_path_str);
        
        if !input_file.exists() {
            eprintln!("File not found: {}", input_path_str);
            continue;
        }
        
        let mut output_file = input_file.clone();
        output_file.set_extension("txt");
        
        let wav_path = "/tmp/media_transcriber_cli_audio.wav";
        
        println!("  -> Extracting audio...");
        if let Err(e) = extract_audio(&input_path_str, wav_path) {
            eprintln!("  -> Error extracting audio: {}", e);
            continue;
        }
        
        println!("  -> Loading audio...");
        let audio_data = match parse_wav_file(wav_path) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("  -> Error parsing audio: {}", e);
                continue;
            }
        };
        
        println!("  -> Transcribing...");
        let mut last_percent = -1;
        let progress_callback = move |progress: i32| {
            if progress > last_percent {
                print!("\r  -> Progress: {}%", progress);
                std::io::stdout().flush().unwrap();
                last_percent = progress;
            }
        };
        
        match transcribe_audio(model_path, &audio_data, progress_callback) {
            Ok((full_text_stamped, _full_text_raw)) => {
                println!("\n  -> Transcription complete.");
                if let Err(e) = std::fs::write(&output_file, full_text_stamped) {
                    eprintln!("  -> Error writing output file: {}", e);
                } else {
                    println!("  -> Saved to: {}", output_file.display());
                }
            },
            Err(e) => {
                eprintln!("\n  -> Error transcribing: {}", e);
            }
        }
        
        let _ = std::fs::remove_file(wav_path);
    }
}
