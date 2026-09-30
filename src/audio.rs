use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

/// Whisper expects 16 kHz mono audio.
pub const WHISPER_SAMPLE_RATE: u32 = 16_000;

const FFMPEG_BIN: &str = if cfg!(windows) {
    "ffmpeg.exe"
} else {
    "ffmpeg"
};

/// Finds FFmpeg: the copy bundled next to our executable first, then PATH.
///
/// On macOS, apps launched from Finder don't inherit the shell's PATH, so
/// the Homebrew locations are checked explicitly.
fn find_ffmpeg() -> Option<PathBuf> {
    let bundled = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(FFMPEG_BIN)))
        .filter(|path| path.is_file());
    if bundled.is_some() {
        return bundled;
    }

    let path_dirs = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default();
    let extra_dirs: &[&str] = if cfg!(target_os = "macos") {
        &["/opt/homebrew/bin", "/usr/local/bin"]
    } else {
        &[]
    };

    path_dirs
        .into_iter()
        .chain(extra_dirs.iter().map(PathBuf::from))
        .map(|dir| dir.join(FFMPEG_BIN))
        .find(|path| path.is_file())
}

fn ffmpeg_missing_message() -> String {
    let install = if cfg!(target_os = "windows") {
        "winget install Gyan.FFmpeg"
    } else if cfg!(target_os = "macos") {
        "brew install ffmpeg"
    } else {
        "sudo apt install ffmpeg (or your distribution's equivalent)"
    };
    format!(
        "FFmpeg was not found. Reinstall Media Transcriber, or install FFmpeg with: {}",
        install
    )
}

/// Decodes the audio track of any media file FFmpeg understands to 16 kHz
/// mono samples. FFmpeg's raw PCM output is read straight from its stdout,
/// so no temporary file is written.
pub fn load_audio(path: &Path) -> Result<Vec<f32>, String> {
    if !path.is_file() {
        return Err(format!("File not found: {}", path.display()));
    }
    let ffmpeg = find_ffmpeg().ok_or_else(ffmpeg_missing_message)?;

    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-nostdin", "-loglevel", "error", "-i"])
        .arg(path)
        .args([
            "-vn",
            "-ac",
            "1",
            "-ar",
            &WHISPER_SAMPLE_RATE.to_string(),
            "-f",
            "s16le",
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        // Don't flash a console window when launched from the GUI.
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to run FFmpeg: {}", e))?;

    // Drain stderr on its own thread so a chatty FFmpeg can't fill the pipe
    // and deadlock while we're blocked reading stdout.
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let stderr_reader = thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });

    let mut stdout = child.stdout.take().expect("stdout is piped");
    let samples = read_pcm_s16le(&mut stdout)
        .map_err(|e| format!("Failed to read audio from FFmpeg: {}", e))?;

    let status = child
        .wait()
        .map_err(|e| format!("Failed to run FFmpeg: {}", e))?;
    let stderr = stderr_reader.join().unwrap_or_default();

    if !status.success() {
        if stderr.contains("does not contain any stream") || stderr.contains("matches no streams") {
            return Err("This file has no audio track.".into());
        }
        let reason = stderr
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("unknown error");
        return Err(format!("Could not decode audio: {}", reason.trim()));
    }
    if samples.is_empty() {
        return Err("This file has no audio track.".into());
    }

    Ok(samples)
}

/// Converts a stream of little-endian 16-bit PCM to `f32` samples as it
/// arrives, so the raw bytes are never held in memory all at once.
fn read_pcm_s16le(reader: &mut impl Read) -> io::Result<Vec<f32>> {
    let mut samples = Vec::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut filled = 0;

    loop {
        let n = reader.read(&mut buf[filled..])?;
        if n == 0 {
            break;
        }
        filled += n;

        // A read can end mid-sample; carry the odd byte over to the next one.
        let usable = filled - filled % 2;
        samples.extend(
            buf[..usable]
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0),
        );
        buf.copy_within(usable..filled, 0);
        filled -= usable;
    }

    Ok(samples)
}
