#!/usr/bin/env bash
# Builds Media Transcriber from source and installs it for the current user.
# Prebuilt installers: https://github.com/Leac1m/media-transcriber/releases
set -euo pipefail

cd "$(dirname "$0")"

BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"

echo "🎙️  Installing Media Transcriber from source..."

for tool in cargo cmake; do
    if ! command -v "$tool" &> /dev/null; then
        echo "❌ '$tool' not found. See 'Building from source' in README.md." >&2
        exit 1
    fi
done
if ! command -v ffmpeg &> /dev/null; then
    echo "⚠️  FFmpeg not found. Install it before transcribing (see README.md)."
fi

echo "⚙️  Building (this may take a few minutes)..."
cargo build --release --locked

mkdir -p "$BIN_DIR"
install -m 755 target/release/media-transcriber target/release/media-transcriber-cli "$BIN_DIR/"
echo "✅ Installed media-transcriber and media-transcriber-cli to $BIN_DIR"

if [[ "$(uname -s)" == "Linux" ]]; then
    # The icon name is also used for desktop notifications.
    mkdir -p "$DATA_DIR/icons" "$DATA_DIR/applications"
    install -m 644 ui/icon.png "$DATA_DIR/icons/media-transcriber.png"
    cat > "$DATA_DIR/applications/media-transcriber.desktop" << EOF
[Desktop Entry]
Name=Media Transcriber
Comment=Transcribe speech in video and audio files
Exec=$BIN_DIR/media-transcriber
Icon=media-transcriber
Type=Application
Terminal=false
StartupWMClass=media-transcriber
Categories=AudioVideo;Utility;
EOF
    if command -v update-desktop-database &> /dev/null; then
        update-desktop-database "$DATA_DIR/applications" || true
    fi
    echo "✅ Added Media Transcriber to your application menu"
fi

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "ℹ️  Add $BIN_DIR to your PATH to run the command-line tool from anywhere." ;;
esac

echo "🎉 Done! The Whisper model (~465 MB) is downloaded on first launch."
