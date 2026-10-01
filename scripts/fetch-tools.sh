#!/usr/bin/env bash
# fetch-tools.sh — download PREBUILT yt-dlp + ffmpeg binaries for one target
# triple into src-tauri/binaries/, named as Tauri externalBin sidecars
# (<name>-<target-triple>[.exe]). Nothing is compiled; this only downloads
# official builds, keeping CI fast.
#
#   Usage: scripts/fetch-tools.sh <target-triple>
#
# Supported triples:
#   aarch64-apple-darwin      x86_64-apple-darwin
#   x86_64-pc-windows-msvc    aarch64-pc-windows-msvc
#   x86_64-unknown-linux-gnu  aarch64-unknown-linux-gnu
#
# Sources (all official prebuilt binaries):
#   yt-dlp : https://github.com/yt-dlp/yt-dlp/releases (Unlicense)
#   ffmpeg : https://github.com/yt-dlp/FFmpeg-Builds   (GPLv3, Windows/Linux)
#            https://ffmpeg.martin-riedl.de           (GPLv3, macOS)
# License texts bundled in src-tauri/LICENSES/ satisfy GPL redistribution
# terms for the aggregated ffmpeg binaries.

set -euo pipefail

TRIPLE="${1:-}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT/src-tauri/binaries"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

case "$TRIPLE" in
  aarch64-apple-darwin|x86_64-apple-darwin)
    YTDLP_ASSET="yt-dlp_macos"
    EXE=""
    if [ "$TRIPLE" = "aarch64-apple-darwin" ]; then MACARCH="arm64"; else MACARCH="amd64"; fi
    FFMPEG_KIND="macos-$MACARCH"
    ;;
  x86_64-pc-windows-msvc)
    YTDLP_ASSET="yt-dlp.exe"; EXE=".exe"; FFMPEG_KIND="win64" ;;
  aarch64-pc-windows-msvc)
    YTDLP_ASSET="yt-dlp_arm64.exe"; EXE=".exe"; FFMPEG_KIND="winarm64" ;;
  x86_64-unknown-linux-gnu)
    YTDLP_ASSET="yt-dlp_linux"; EXE=""; FFMPEG_KIND="linux64" ;;
  aarch64-unknown-linux-gnu)
    YTDLP_ASSET="yt-dlp_linux_aarch64"; EXE=""; FFMPEG_KIND="linuxarm64" ;;
  *)
    echo "unsupported target triple: '$TRIPLE'" >&2
    exit 1
    ;;
esac

mkdir -p "$OUT_DIR"

echo ">> downloading yt-dlp ($YTDLP_ASSET)"
curl -fL --retry 3 -o "$TMP/yt-dlp$EXE" \
  "https://github.com/yt-dlp/yt-dlp/releases/latest/download/$YTDLP_ASSET"

case "$FFMPEG_KIND" in
  macos-*)
    echo ">> downloading ffmpeg + ffprobe (martin-riedl.de, $FFMPEG_KIND)"
    curl -fL --retry 3 -o "$TMP/ffmpeg.zip" \
      "https://ffmpeg.martin-riedl.de/redirect/latest/macos/${FFMPEG_KIND#macos-}/release/ffmpeg.zip"
    curl -fL --retry 3 -o "$TMP/ffprobe.zip" \
      "https://ffmpeg.martin-riedl.de/redirect/latest/macos/${FFMPEG_KIND#macos-}/release/ffprobe.zip"
    unzip -q -o "$TMP/ffmpeg.zip" -d "$TMP/ff"
    unzip -q -o "$TMP/ffprobe.zip" -d "$TMP/ffp"
    FFMPEG_BIN="$TMP/ff/ffmpeg"
    FFPROBE_BIN="$TMP/ffp/ffprobe"
    ;;
  win*)
    echo ">> downloading ffmpeg (yt-dlp/FFmpeg-Builds, $FFMPEG_KIND)"
    curl -fL --retry 3 -o "$TMP/ffmpeg.zip" \
      "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-$FFMPEG_KIND-gpl.zip"
    unzip -q -o "$TMP/ffmpeg.zip" -d "$TMP/ffx"
    FFMPEG_BIN="$(find "$TMP/ffx" -name ffmpeg.exe | head -1)"
    FFPROBE_BIN="$(find "$TMP/ffx" -name ffprobe.exe | head -1)"
    ;;
  linux*)
    echo ">> downloading ffmpeg (yt-dlp/FFmpeg-Builds, $FFMPEG_KIND)"
    curl -fL --retry 3 -o "$TMP/ffmpeg.tar.xz" \
      "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-$FFMPEG_KIND-gpl.tar.xz"
    mkdir -p "$TMP/ffx"
    tar -xJf "$TMP/ffmpeg.tar.xz" -C "$TMP/ffx"
    FFMPEG_BIN="$(find "$TMP/ffx" -name ffmpeg -type f | head -1)"
    FFPROBE_BIN="$(find "$TMP/ffx" -name ffprobe -type f | head -1)"
    ;;
esac

[ -n "${FFMPEG_BIN:-}" ] || { echo "ffmpeg binary not found in archive" >&2; exit 1; }
[ -n "${FFPROBE_BIN:-}" ] || { echo "ffprobe binary not found in archive" >&2; exit 1; }

install_bin() {
  cp "$1" "$OUT_DIR/$2"
  chmod 755 "$OUT_DIR/$2"
  echo "   installed $OUT_DIR/$2 ($(du -h "$OUT_DIR/$2" | cut -f1))"
}

install_bin "$TMP/yt-dlp$EXE" "yt-dlp-$TRIPLE$EXE"
install_bin "$FFMPEG_BIN"       "ffmpeg-$TRIPLE$EXE"
install_bin "$FFPROBE_BIN"      "ffprobe-$TRIPLE$EXE"

echo ">> done. sidecars ready for tauri build --target $TRIPLE"
