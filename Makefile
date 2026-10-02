# Makefile — fast local build & test loop (macOS-focused; CI handles
# Windows/Linux). Run `make help` to list targets.
#
# NOTE: no CI required for daily development — `make dmg-full` gives you a
# locally signed (ad-hoc), fully bundled dmg in under ~2 minutes after the
# first build.

# $(shell ...) expands with make's ORIGINAL environment, so give these
# probes their own PATH explicitly.
TOOLPATH := $(HOME)/.cargo/bin:/usr/local/bin:/opt/homebrew/bin:$(PATH)
TRIPLE  := $(shell PATH="$(TOOLPATH)" rustc -vV | sed -n 's/^host: //p')
ARCH    := $(shell uname -m)
VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' src-tauri/Cargo.toml)
APP_DIR := src-tauri/target/$(TRIPLE)/release/bundle/macos
OUT     := release-out

# Recipes DO get the exported PATH below; also prefer a real pnpm,
# falling back to the repo-local shim (.bin/pnpm).
export PATH := $(CURDIR)/.bin:$(HOME)/.cargo/bin:/usr/local/bin:/opt/homebrew/bin:$(PATH)
PNPM  := $(shell PATH="$(TOOLPATH)" command -v pnpm 2>/dev/null || echo npx --yes pnpm)
TAURI := ./node_modules/.bin/tauri

.PHONY: help dev test tools build build-full dmg dmg-full print-tools clean

help:
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  make %-12s %s\n", $$1, $$2}'

dev: ## run the app in hot-reload dev mode
	$(TAURI) dev

test: ## run ALL checks: cargo test + frontend tests + svelte-check
	cd src-tauri && cargo test
	$(PNPM) test
	$(PNPM) run check

tools: ## download prebuilt yt-dlp+ffmpeg sidecars for this machine
	bash scripts/fetch-tools.sh $(TRIPLE)

build: ## build the lite .app (fast, no bundle tools)
	$(TAURI) build --target $(TRIPLE) --bundles app

build-full: tools ## build the full .app (bundled yt-dlp/ffmpeg/ffprobe)
	$(TAURI) build --target $(TRIPLE) --bundles app --config src-tauri/tauri.full.conf.json

dmg: build ## lite .app + dmg with the unsigned-app notice
	mkdir -p $(OUT)
	bash scripts/make-dmg.sh "$(APP_DIR)/Video Downloader.app" "$(OUT)/Video-Downloader_$(VERSION)_lite_macos_$(ARCH).dmg"

dmg-full: build-full ## full .app + dmg with the unsigned-app notice
	mkdir -p $(OUT)
	bash scripts/make-dmg.sh "$(APP_DIR)/Video Downloader.app" "$(OUT)/Video-Downloader_$(VERSION)_full_macos_$(ARCH).dmg"

print-tools: build-full ## show which yt-dlp/ffmpeg the full app resolves
	"$(APP_DIR)/Video Downloader.app/Contents/MacOS/video-downloader" --print-tools

clean: ## remove build artifacts (keeps node_modules and .cargo-home)
	rm -rf src-tauri/target dist $(OUT) release-out-x64
