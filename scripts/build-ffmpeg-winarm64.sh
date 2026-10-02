#!/usr/bin/env bash
# build-ffmpeg-winarm64.sh — build a static ffmpeg + ffprobe for
# aarch64-pc-windows-msvc, because the only prebuilt winarm64 ffmpeg
# (yt-dlp/FFmpeg-Builds) segfaults at startup (twice in CI).
#
# Runs natively on a windows-11-arm GitHub runner inside MSYS2 CLANGARM64
# (workflow uses `shell: msys2 {0}` + msys2/setup-msys2 with msystem:
# CLANGARM64). No cross-compilation, no MSVC.
#
# The build is tuned to what yt-dlp actually needs: remuxing (mp4/mkv/
# m4a...), audio extraction/encoding (mp3/opus/vorbis/aac/pcm), subtitle
# embedding, thumbnail embedding. No video encoders (yt-dlp never
# re-encodes video), no X11/SDL/hardware accel — keeps it small and fast
# to compile.
#
#   Usage: scripts/build-ffmpeg-winarm64.sh <target-triple>
#
# License: --enable-gpl makes this a GPLv3 build, same as the prebuilt
# binaries we ship for other platforms; see src-tauri/LICENSES/.

set -euo pipefail

TRIPLE="${1:?target triple required}"
FFMPEG_VER="7.1.1"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT/src-tauri/binaries"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

mkdir -p "$OUT_DIR"

echo ">> downloading ffmpeg $FFMPEG_VER source"
curl -fL --retry 3 -o "$WORK/ffmpeg.tar.xz" \
  "https://ffmpeg.org/releases/ffmpeg-$FFMPEG_VER.tar.xz"
tar -xJf "$WORK/ffmpeg.tar.xz" -C "$WORK"
cd "$WORK/ffmpeg-$FFMPEG_VER"

echo ">> configuring (static, yt-dlp-oriented, aarch64 mingw)"
./configure \
  --arch=aarch64 \
  --target-os=mingw32 \
  --cc=clang \
  --enable-gpl \
  --enable-version3 \
  --enable-static \
  --disable-shared \
  --disable-doc \
  --disable-debug \
  --disable-autodetect \
  --disable-sdl2 \
  --disable-network \
  --enable-protocol=file,pipe,crypto,http,https,tcp,tls,fd,md5 \
  --enable-libmp3lame \
  --enable-libopus \
  --enable-libvorbis \
  --enable-ffmpeg \
  --enable-ffprobe \
  --disable-ffplay \
  --disable-encoders \
  --enable-encoder=aac,libmp3lame,libopus,libvorbis,flac,pcm_s16le,pcm_s24le,pcm_f32le,webvtt,srt,mov_text,ass,subrip,png,mjpeg,copy \
  --disable-decoders \
  --enable-decoder=aac,mp3,mp3float,opus,vorbis,flac,pcm_s16le,pcm_s24le,pcm_f32le,h264,hevc,vp9,av1,webp,png,mjpeg,subrip,mov_text,webvtt,ass,ssa,dvd_subtitle \
  --extra-ldflags="-static" \
  --pkg-config=pkg-config

echo ">> building with $(nproc) jobs (this takes a while)"
make -j"$(nproc)" ffmpeg.exe ffprobe.exe

install_bin() {
  cp "$1" "$OUT_DIR/$2"
  chmod 755 "$OUT_DIR/$2"
  echo "   installed $OUT_DIR/$2 ($(du -h "$OUT_DIR/$2" | cut -f1))"
}

install_bin ffmpeg.exe  "ffmpeg-$TRIPLE.exe"
install_bin ffprobe.exe "ffprobe-$TRIPLE.exe"

echo ">> smoke test: running the freshly built ffmpeg"
"./ffmpeg.exe" -version | head -1
echo ">> done"
