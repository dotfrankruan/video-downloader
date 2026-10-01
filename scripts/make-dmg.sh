#!/usr/bin/env bash
# make-dmg.sh — repackage a built .app into a .dmg that ALSO contains a
# bilingual READ-ME-FIRST notice about the ad-hoc signature, plus an
# /Applications shortcut. The app itself is signed ad-hoc by tauri
# (signingIdentity "-"); macOS Gatekeeper therefore requires one of the
# steps described in the notice.
#
#   Usage: scripts/make-dmg.sh <path/to/Video Downloader.app> <output.dmg>

set -euo pipefail

APP="${1:?path to .app required}"
OUT="${2:?output .dmg path required}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VOLNAME="Video Downloader"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# Normalise the app name inside the dmg regardless of the source folder name.
cp -R "$APP" "$TMP/Video Downloader.app"
ln -s /Applications "$TMP/Applications"
cp "$ROOT/packaging/READ-ME-FIRST.txt" "$TMP/请先阅读 READ-ME-FIRST.txt"

rm -f "$OUT"
hdiutil create \
  -volname "$VOLNAME" \
  -srcfolder "$TMP" \
  -ov -format UDZO \
  "$OUT"

echo ">> created $OUT"
