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

# MSYS2's clangarm64 repo lacks libmp3lame/libopus packages, so build the
# two small audio codec libs from source first (each ~1-2 min).
# lame/opus install into the toolchain prefix itself, so clang and
# pkg-config find them via their built-in defaults — no PKG_CONFIG_PATH
# overrides needed (those risk breaking pkgconf's default search path).
PREFIX="/clangarm64"
PKGCONF="/clangarm64/bin/pkg-config"

echo ">> building lame (mp3 encoder) from source"
curl -fL --retry 3 -o "$WORK/lame.tar.gz" \
  "https://downloads.sourceforge.net/project/lame/lame/3.100/lame-3.100.tar.gz"
tar -xzf "$WORK/lame.tar.gz" -C "$WORK"
cd "$WORK/lame-3.100"
# lame 3.100's bundled config.sub/guess predate aarch64-w64-mingw32.
for f in config.sub config.guess; do
  curl -fsSL --retry 3 -o "$f" "https://git.savannah.gnu.org/cgit/config.git/plain/$f"
done
./configure --prefix="$PREFIX" --enable-static --disable-shared --disable-frontend
make -j"$(nproc)"
make install

echo ">> building opus from source"
curl -fL --retry 3 -o "$WORK/opus.tar.gz" \
  "https://github.com/xiph/opus/releases/download/v1.5.2/opus-1.5.2.tar.gz"
tar -xzf "$WORK/opus.tar.gz" -C "$WORK"
cd "$WORK/opus-1.5.2"
# opus's ARM asm has no CPU-detection path for Windows-on-ARM
# ("Configured to use ARM asm but no CPU detection method available"),
# so use the plain C implementation.
./configure --prefix="$PREFIX" --enable-static --disable-shared --disable-doc \
  --disable-rtcd --disable-asm
make -j"$(nproc)"
make install

echo ">> downloading ffmpeg $FFMPEG_VER source"
curl -fL --retry 3 -o "$WORK/ffmpeg.tar.xz" \
  "https://ffmpeg.org/releases/ffmpeg-$FFMPEG_VER.tar.xz"
tar -xJf "$WORK/ffmpeg.tar.xz" -C "$WORK"
cd "$WORK/ffmpeg-$FFMPEG_VER"

echo ">> pkg-config sanity check"
"$PKGCONF" --version
"$PKGCONF" --exists vorbis vorbisenc vorbisfile && echo "vorbis: ok" || { echo "vorbis MISSING:"; "$PKGCONF" --print-errors --exists vorbis vorbisenc vorbisfile || true; }
"$PKGCONF" --exists opus && echo "opus: ok"
"$PKGCONF" --cflags --libs vorbis vorbisenc vorbisfile opus
ls "$PREFIX/include/lame/lame.h" && echo "lame header: ok"

echo ">> configuring (static, yt-dlp-oriented, aarch64 mingw)"
set +e
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
  --extra-cflags="-I$PREFIX/include" \
  --extra-ldflags="-L$PREFIX/lib -static" \
  --pkg-config="$PKGCONF" \
  --pkg-config-flags="--static"
rc=$?
if [ $rc -ne 0 ]; then
  echo ">> configure failed ($rc); tail of ffbuild/config.log:"
  tail -60 ffbuild/config.log || true
  exit $rc
fi
set -e

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
