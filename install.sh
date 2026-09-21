#!/bin/bash
set -e

echo "🎙️  Installing Media Transcriber..."

# 1. Check for ffmpeg
if ! command -v ffmpeg &> /dev/null; then
    echo "📦 FFmpeg not found. Attempting to install..."
    if command -v apt-get &> /dev/null; then
        sudo apt-get update && sudo apt-get install -y ffmpeg
    else
        echo "❌ Please install ffmpeg manually for your OS."
        exit 1
    fi
else
    echo "✅ FFmpeg is already installed."
fi

# 2. Check for cargo/rust
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo not found. Please install Rust from https://rustup.rs/ and try again."
    exit 1
else
    echo "✅ Rust is installed."
fi

# 3. Download Whisper model
echo "🧠 Setting up Whisper AI model..."
mkdir -p models
if [ ! -f "models/ggml-small.bin" ]; then
    echo "Downloading ggml-small.bin..."
    wget -O models/ggml-small.bin https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin
else
    echo "✅ Model already exists. Skipping download."
fi

# 4. Build the release binary
echo "⚙️  Building Media Transcriber (this may take a few minutes)..."
cargo build --release

# 5. Setup Linux Desktop Entry
echo "🖥️  Setting up desktop shortcut..."
DESKTOP_FILE="$HOME/.local/share/applications/media-transcriber.desktop"
APP_DIR="$(pwd)"

# We use icon.png since it's already in the repository
cat << EOF > "$DESKTOP_FILE"
[Desktop Entry]
Name=Media Transcriber
Exec=$APP_DIR/target/release/media-transcriber
Icon=$APP_DIR/ui/icon.png
Type=Application
Terminal=false
Categories=Utility;AudioVideo;
EOF

# Update desktop database if available
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database ~/.local/share/applications/ || true
fi

echo "🎉 Installation complete! You can now launch 'Media Transcriber' from your application menu."
