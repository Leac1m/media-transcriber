<div align="center">
  <h1>🎙️ Media Transcriber</h1>
  <p><strong>A blazingly fast, privacy-first desktop application for extracting and transcribing audio from video files using Whisper AI.</strong></p>

  ![Rust](https://img.shields.io/badge/Made%20with-Rust-black?style=for-the-badge&logo=rust)
  ![Slint](https://img.shields.io/badge/UI-Slint-blue?style=for-the-badge)
  ![Whisper](https://img.shields.io/badge/AI-Whisper.cpp-purple?style=for-the-badge)
</div>

<br />

## 🌟 Overview

Media Transcriber is a standalone desktop application that allows you to easily select a video file, automatically rip the audio in the background, and transcribe the speech to text using OpenAI's state-of-the-art Whisper models running **100% locally on your machine**. 

No cloud APIs, no internet connection required, and complete privacy for your sensitive video and audio data.

![Media Transcriber Screenshot](./media_transcirber_screenshot.png)

---

## ✨ Key Features

* 🔒 **100% Local & Private:** Your files never leave your machine. All AI inference is done locally.
* ⚡ **Seamless Pipeline:** Automatically invokes `ffmpeg` to extract and convert audio to the exact format required by the AI, all behind the scenes.
* ⏱️ **Timestamping:** Extracts precise AI timestamps. Toggle them on or off instantly in the UI without re-processing!
* 📊 **Live Progress Tracking:** Real-time visual progress bar tracking the extraction, model loading, and live AI transcription phases.
* 📋 **Frictionless Export:** 1-click export to your system clipboard or save directly as a `.txt` file.

---

## 🛠️ Tech Stack

* **[Rust](https://www.rust-lang.org/):** The core engine, providing safety, concurrency, and blazing-fast performance.
* **[Slint](https://slint.dev/):** A modern, lightweight, and responsive native GUI toolkit.
* **[whisper-rs](https://github.com/tazz4843/whisper-rs):** Rust bindings for `whisper.cpp` to run the AI model efficiently on CPU/GPU.
* **[Hound](https://github.com/ruigc/hound):** For fast, native parsing of WAV audio files.
* **[rfd](https://github.com/PolyMeilex/rfd):** Native system file dialogs for cross-platform compatibility.

---

## 🚀 Getting Started

### Easy Installation (Linux / macOS)

We have provided a convenient `install.sh` script that will:
1. Verify Rust and FFmpeg are installed (and prompt to install if missing).
2. Download the Whisper `ggml-small.bin` AI model for you.
3. Build the application in `--release` mode.
4. Setup a `.desktop` shortcut so it appears in your app launcher with the correct icon!

Simply run:
```bash
git clone https://github.com/your-username/media-transcriber.git
cd media-transcriber
./install.sh
```

### Manual Installation

If you prefer to install manually or are on Windows:

1. **Install Prerequisites**: Ensure Rust and FFmpeg are installed.
2. **Download Model**: Download `ggml-small.bin` from [whisper.cpp HuggingFace](https://huggingface.co/ggerganov/whisper.cpp/tree/main) into a `models/` directory.
3. **Build & Run**:
```bash
cargo run --release
```

---

## 💻 Usage Instructions

1. **Select File:** Click the `Select Video File` button to open your native file explorer and pick any standard video or audio file (`.mp4`, `.mkv`, `.mp3`, etc.).
2. **Start Transcription:** Click `Start Transcription`. The app will begin extracting the audio, loading the model, and transcribing. You can track the progress in the UI.
3. **Format Output:** Once complete, use the `Show Timestamps` checkbox to toggle the timestamp markers (`[MM:SS.ms]`) on or off.
4. **Export:** Click `📋 Copy to Clipboard` to instantly copy the text, or `💾 Save to File` to export it as a `.txt` document.

---

## 🤝 Contributing

Contributions, issues, and feature requests are welcome! Feel free to check the [issues page](#) if you want to contribute.

## 📝 License

This project is open-source and available under the [MIT License](LICENSE).
