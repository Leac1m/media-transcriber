#!/usr/bin/env python3
"""Fetches the pinned FFmpeg build that is bundled with the release packages.

Downloads the build for the given Rust target triple, verifies its SHA-256
and writes the executable to dist/ffmpeg-<target-triple>[.exe], which is where
cargo-packager looks for external binaries.

Usage: fetch_ffmpeg.py <target-triple> [output-dir]
"""

import hashlib
import io
import sys
import time
import urllib.request
import zipfile
from pathlib import Path

# FFmpeg 9.0.2. Update all three together, and FFMPEG-NOTICE.txt with them.
BUILDS = {
    # https://www.gyan.dev/ffmpeg/builds/ (mirrored on GitHub)
    "x86_64-pc-windows-msvc": (
        "https://github.com/GyanD/codexffmpeg/releases/download/9.0.2/ffmpeg-9.0.2-essentials_build.zip",
        "60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba",
        "ffmpeg-9.0.2-essentials_build/bin/ffmpeg.exe",
    ),
    # https://ffmpeg.martin-riedl.de/
    "aarch64-apple-darwin": (
        "https://ffmpeg.martin-riedl.de/download/macos/arm64/1789931890_9.0.2/ffmpeg.zip",
        "c8ed4c4e6978a03c485edbfe4e0a5dc2380f8a30bba5150531b31b094492d924",
        "ffmpeg",
    ),
    "x86_64-apple-darwin": (
        "https://ffmpeg.martin-riedl.de/download/macos/amd64/1789931006_9.0.2/ffmpeg.zip",
        "7c6b4125b191cbf773832dc51f424cf2b6bb7da43007d1e066f95909e47cacd4",
        "ffmpeg",
    ),
}


def download(url: str, attempts: int = 3) -> bytes:
    # Some hosts reject urllib's default User-Agent.
    request = urllib.request.Request(url, headers={"User-Agent": "media-transcriber-release-build"})
    for attempt in range(1, attempts + 1):
        print(f"Downloading {url} (attempt {attempt}/{attempts})")
        try:
            with urllib.request.urlopen(request, timeout=120) as response:
                return response.read()
        except OSError as e:  # URLError and connection resets are OSErrors
            if attempt == attempts:
                sys.exit(f"Download failed: {e}")
            time.sleep(5 * attempt)
    raise AssertionError("unreachable")


def main() -> None:
    if len(sys.argv) not in (2, 3) or sys.argv[1] not in BUILDS:
        sys.exit(f"usage: {sys.argv[0]} <{'|'.join(BUILDS)}> [output-dir]")

    target = sys.argv[1]
    out_dir = Path(sys.argv[2] if len(sys.argv) == 3 else "dist")
    url, expected_sha256, member = BUILDS[target]

    archive = download(url)

    actual_sha256 = hashlib.sha256(archive).hexdigest()
    if actual_sha256 != expected_sha256:
        sys.exit(f"Checksum mismatch for {url}\n  expected {expected_sha256}\n  actual   {actual_sha256}")

    with zipfile.ZipFile(io.BytesIO(archive)) as zf:
        binary = zf.read(member)

    suffix = ".exe" if "windows" in target else ""
    dest = out_dir / f"ffmpeg-{target}{suffix}"
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_bytes(binary)
    dest.chmod(0o755)  # zipfile doesn't preserve the executable bit
    print(f"Verified and wrote {dest} ({len(binary) // (1024 * 1024)} MB)")


if __name__ == "__main__":
    main()
