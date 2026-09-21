# Goal
Build a desktop application written in Rust with a Slint GUI that takes a video file upload, transcribes the audio using the Whisper "small" model, and provides a polished interface with real-time progress feedback.

## Finalized Scope
1. **Model & Engine**: We will use `whisper.cpp` via the `whisper-rs` crate for native Rust performance. We will download the Whisper "small" model (GGUF/BIN format) during the build/setup phase.
2. **Audio Extraction**: We will use the system's `ffmpeg` command-line tool via a subprocess to extract a 16kHz WAV file from the uploaded media.
3. **Project Location**: `/home/michael/projects/media-transcriber`

## Proposed Architecture

### 1. Slint GUI layer
- A file-picker interface for video selection.
- A progress bar (Slint `ProgressIndicator`) that updates as transcription occurs.
- A scrolling text area for the final transcribed text.

### 2. Rust Core Logic
- **UI Logic**: Slint runs on the main thread.
- **File Picker**: Use `rfd` (Rust File Dialog) for native file picking.
- **Background Thread**: A dedicated worker thread will handle the heavy lifting to prevent UI freezing.
- **Extraction**: `std::process::Command` invokes `ffmpeg` to extract a 16kHz, mono, 16-bit WAV file from the video.
- **Transcription**: `whisper-rs` processes the WAV file.
- **Communication**: We will use Slint's thread-safe features (`slint::invoke_from_event_loop`) to send progress updates (percentage based on audio duration processed) and the final text back to the Slint UI thread.

## Verification Plan
1. **Scaffolding**: Initialize the Rust project in `/home/michael/projects/media-transcriber` and compile a basic Slint "Hello World" to ensure the GUI framework builds.
2. **Download Model**: Download the Whisper small model.
3. **Audio Pipeline**: Test `ffmpeg` extraction logic on a sample video.
4. **Integration**: Run the transcription thread and verify the Slint progress bar updates smoothly without freezing the UI.
