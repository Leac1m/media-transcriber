use clap::Parser;
use media_transcriber::audio::load_audio;
use media_transcriber::config::{MODEL_SIZE, get_model_path};
use media_transcriber::model::{download_model, is_model_installed};
use media_transcriber::transcription::Transcriber;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The media files to transcribe
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

fn main() {
    let args = Args::parse();

    let model_path = get_model_path();

    // 0. Download model if missing
    if !is_model_installed(&model_path) {
        println!("Whisper model not found locally.");
        println!(
            "Starting download of ggml-small.bin (approx {} MB)...",
            MODEL_SIZE / (1024 * 1024)
        );

        let mut last_percent = -1;
        let progress_callback = move |progress: f32| {
            let percent = (progress * 100.0) as i32;
            if percent > last_percent {
                print!("\rDownloading: {}%", percent);
                std::io::stdout().flush().unwrap();
                last_percent = percent;
            }
        };

        if let Err(e) = download_model(&model_path, progress_callback) {
            eprintln!("\nError downloading model: {}", e);
            std::process::exit(1);
        }
        println!("\nDownload complete.");
    }

    println!("Loading Whisper model...");
    let transcriber = match Transcriber::load(&model_path.to_string_lossy()) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error loading model: {}", e);
            std::process::exit(1);
        }
    };

    for input_file in args.files {
        let input_path_str = input_file.to_string_lossy().to_string();
        println!("\nProcessing: {}", input_path_str);

        if !input_file.exists() {
            eprintln!("File not found: {}", input_path_str);
            continue;
        }

        let mut output_file = input_file.clone();
        output_file.set_extension("txt");

        println!("  -> Decoding audio...");
        let audio_data = match load_audio(&input_file) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("  -> Error: {}", e);
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

        match transcriber.transcribe(&audio_data, progress_callback) {
            Ok((full_text_stamped, _full_text_raw)) => {
                println!("\n  -> Transcription complete.");
                if let Err(e) = std::fs::write(&output_file, full_text_stamped) {
                    eprintln!("  -> Error writing output file: {}", e);
                } else {
                    println!("  -> Saved to: {}", output_file.display());
                }
            }
            Err(e) => {
                eprintln!("\n  -> Error transcribing: {}", e);
            }
        }
    }
}
