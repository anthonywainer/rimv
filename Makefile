# rimv development and local-capture commands.
#
# Override values as needed, for example:
#   make mic SECONDS=60 DEVICE='input:MacBook Pro Microphone:0'
#   make system SECONDS=30

SHELL := /bin/sh

# Prefer the SDK belonging to the selected Xcode installation. Some macOS
# setups otherwise pair Xcode's compiler with an older Command Line Tools SDK.
# An explicitly supplied SDKROOT always takes precedence.
ifeq ($(shell uname -s),Darwin)
RIMV_XCODE_DEVELOPER_DIR := $(shell xcode-select -p 2>/dev/null)
RIMV_XCODE_SDKROOT := $(RIMV_XCODE_DEVELOPER_DIR)/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk
ifneq ($(wildcard $(RIMV_XCODE_SDKROOT)),)
export SDKROOT ?= $(RIMV_XCODE_SDKROOT)
endif
endif

LOCAL_CARGO_HOME := $(abspath ../.rust-tools/cargo)
LOCAL_RUSTUP_HOME := $(abspath ../.rust-tools/rustup)

ifneq ($(wildcard $(LOCAL_CARGO_HOME)/bin/cargo),)
export CARGO_HOME := $(LOCAL_CARGO_HOME)
export RUSTUP_HOME := $(LOCAL_RUSTUP_HOME)
CARGO ?= $(LOCAL_CARGO_HOME)/bin/cargo
else
CARGO ?= cargo
endif

DOCKER ?= docker
RUST_LINT_IMAGE ?= rust:1.94-bookworm
RUST_LINT_PLATFORM ?= linux/amd64

CLI_PACKAGE := capture-cli
OUT_DIR ?= recordings
RIMV_MODELS_DIR ?= resources/models
PARAKEET_ARCHIVE := $(RIMV_MODELS_DIR)/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2
PARAKEET_MODEL_DIR := $(RIMV_MODELS_DIR)/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8
SILERO_VAD_MODEL := $(RIMV_MODELS_DIR)/silero_vad.onnx
PARAKEET_MODEL_URL := https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2
SILERO_VAD_URL := https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx
MODEL_ENV = RIMV_PARAKEET_MODEL_DIR="$(abspath $(PARAKEET_MODEL_DIR))" RIMV_SILERO_VAD_MODEL="$(abspath $(SILERO_VAD_MODEL))" RIMV_MODELS_DIR="$(abspath $(RIMV_MODELS_DIR))"
LANGUAGE ?= auto
MAX_WER ?= 0.40
SECONDS ?= 10
DEVICE ?=
SAMPLE_RATE ?=
CHANNELS ?=
STAMP ?= $(shell date +%Y%m%d-%H%M%S)
MIC_OUTPUT ?= $(OUT_DIR)/microphone-$(STAMP).wav
SYSTEM_OUTPUT ?= $(OUT_DIR)/system-$(STAMP).wav
BOTH_DIR ?= $(OUT_DIR)/both-$(STAMP)

DEVICE_ARG = $(if $(strip $(DEVICE)),--device '$(DEVICE)')
SAMPLE_RATE_ARG = $(if $(strip $(SAMPLE_RATE)),--sample-rate $(SAMPLE_RATE))
CHANNELS_ARG = $(if $(strip $(CHANNELS)),--channels $(CHANNELS))
CAPTURE_ARGS = --seconds $(SECONDS) $(DEVICE_ARG) $(SAMPLE_RATE_ARG) $(CHANNELS_ARG)

.DEFAULT_GOAL := help
.PHONY: help build debug check test fmt lint lint-linux-docker devices mic system both engine rimv models rimv-models rimv-listen rimv-transcribe rimv-listen-es rimv-e2e-test stage-capture stage-transcribe stage-server stage-full menu menu-build menu-test profile web web-build app-web clean clean-temp

APP_PROFILE ?= release
APP_DIR := $(abspath target/$(APP_PROFILE)/rimv.app)
PROFILE_MODE ?= idle
PROFILE_SECONDS ?= 30
PROFILE_MAX_CPU ?=
PROFILE_MAX_RSS ?=
PROFILE_BUDGET_ARGS = $(if $(PROFILE_MAX_CPU),--max-cpu-percent $(PROFILE_MAX_CPU)) $(if $(PROFILE_MAX_RSS),--max-rss-mib $(PROFILE_MAX_RSS))

help:
	@printf '%s\n' \
	  'rimv targets:' \
	  '  make build       Build the optimized capture CLI.' \
	  '  make debug       Build the development capture CLI.' \
	  '  make check       Format-check, lint, and test the workspace.' \
	  '  make lint        Run strict Rust Clippy for every native workspace target.' \
	  '  make lint-linux-docker  Run the Linux Clippy check in Docker Desktop.' \
	  '  make devices     List audio devices and supported formats.' \
	  '  make mic         Capture microphone audio (SECONDS=10 by default).' \
	  '  make system      Capture system audio (macOS ScreenCaptureKit).' \
	  '  make both        Capture microphone and system audio separately.' \
	  '  make engine      Run the interactive v0.2 controller/listener CLI.' \
	  '  make rimv        Run the unified rimv developer CLI help.' \
	  '  make models      Download the Parakeet ASR and Silero VAD models.' \
	  '  make rimv-models Show the configured rimv model paths and availability.' \
	  '  make rimv-listen Download models if needed, then listen to the microphone.' \
	  '  make rimv-transcribe Capture and transcribe system audio.' \
	  '  make rimv-listen-es Capture microphone + system audio and transcribe Spanish.' \
	  '  make rimv-e2e-test Play, capture, transcribe, and score resources/audio.' \
	  '  make stage-capture    Stage the lightweight capture distribution.' \
	  '  make stage-transcribe Stage rimv with ASR runtime libraries (models excluded).' \
	  '  make stage-server     Stage experimental localhost server distribution.' \
	  '  make menu        Build and open the macOS menu-bar app.' \
	  '  make menu-test   Check native menu rendering and command wiring.' \
	  '  make web         Run the Vite web client at localhost:5173.' \
	  '  make web-build   Build the static web client.' \
	  '  make app-web     Open the menu-bar app, then run the web client.' \
	  '  make profile     Measure menu app CPU/RSS and collect a stack sample.' \
	  '  make clean       Remove Cargo build artifacts only.' \
	  '  make clean-temp  Remove temporary/partial capture files; keeps WAVs.' \
	  '' \
	  'Optional variables: SECONDS, DEVICE, SAMPLE_RATE, CHANNELS, OUT_DIR, LANGUAGE, MAX_WER.' \
	  'Examples:' \
	  "  make mic SECONDS=60 DEVICE='input:MacBook Pro Microphone:0'" \
	  '  make both SECONDS=300 OUT_DIR=recordings' \
	  '  make rimv-listen LANGUAGE=es' \
	  '  make rimv-e2e-test MAX_WER=0.40'

build:
	$(CARGO) build --release --locked

debug:
	$(CARGO) build --locked

fmt:
	$(CARGO) fmt --all

check:
	$(CARGO) fmt --all --check
	$(CARGO) check --workspace --locked
	$(MAKE) lint
	$(CARGO) test --workspace --locked

lint:
	$(CARGO) clippy --workspace --all-targets --locked -- -D warnings

# Linux-only cfg branches need a Linux toolchain and ALSA development files.
# The project is mounted read-only; Docker writes build output only to /tmp.
lint-linux-docker:
	@$(DOCKER) info >/dev/null 2>&1 || { echo 'Start Docker Desktop, then rerun make lint-linux-docker.'; exit 1; }
	$(DOCKER) run --rm \
	  --platform $(RUST_LINT_PLATFORM) \
	  -v "$(CURDIR):/workspace:ro" \
	  -w /workspace \
	  -e CARGO_TARGET_DIR=/tmp/rimv-target \
	  $(RUST_LINT_IMAGE) \
	  bash -c 'apt-get update && apt-get install -y --no-install-recommends build-essential clang cmake libasound2-dev pkg-config && /usr/local/cargo/bin/rustup component add clippy && /usr/local/cargo/bin/cargo clippy --workspace --all-targets --locked -- -D warnings'

test:
	$(CARGO) test --workspace --locked

devices:
	$(CARGO) run --release --locked -p $(CLI_PACKAGE) -- devices

engine:
	$(CARGO) run --release --locked -p engine-cli -- "$(OUT_DIR)"

rimv:
	$(CARGO) run --release --locked -p rimv -- --help

# Downloads occur only when this target is explicitly invoked. Both paths are
# exported above so every Make-launched rimv command sees the same local models.
models:
	@mkdir -p "$(RIMV_MODELS_DIR)"
	@if [ ! -f "$(SILERO_VAD_MODEL)" ]; then \
		echo 'Downloading Silero VAD model...'; \
		curl --fail --location --retry 3 --output "$(SILERO_VAD_MODEL)" "$(SILERO_VAD_URL)"; \
	fi
	@if [ ! -f "$(PARAKEET_MODEL_DIR)/encoder.int8.onnx" ] || \
		[ ! -f "$(PARAKEET_MODEL_DIR)/decoder.int8.onnx" ] || \
		[ ! -f "$(PARAKEET_MODEL_DIR)/joiner.int8.onnx" ] || \
		[ ! -f "$(PARAKEET_MODEL_DIR)/tokens.txt" ]; then \
		echo 'Downloading or repairing Parakeet ASR model (about 640 MB)...'; \
		curl --fail --location --retry 3 --output "$(PARAKEET_ARCHIVE)" "$(PARAKEET_MODEL_URL)"; \
		tar -xjf "$(PARAKEET_ARCHIVE)" -C "$(RIMV_MODELS_DIR)"; \
		rm -f "$(PARAKEET_ARCHIVE)"; \
	fi
	@test -f "$(SILERO_VAD_MODEL)"
	@test -f "$(PARAKEET_MODEL_DIR)/encoder.int8.onnx"
	@test -f "$(PARAKEET_MODEL_DIR)/decoder.int8.onnx"
	@test -f "$(PARAKEET_MODEL_DIR)/joiner.int8.onnx"
	@test -f "$(PARAKEET_MODEL_DIR)/tokens.txt"

rimv-models:
	$(MODEL_ENV) $(CARGO) run --release --locked -p rimv -- models

rimv-listen: models
	@$(MODEL_ENV) $(CARGO) run --release --locked -p rimv -- listen --mic --language "$(LANGUAGE)" --show-partials --show-metrics

rimv-transcribe: models
	@$(MODEL_ENV) $(CARGO) run --release --locked -p rimv -- listen --system --language "$(LANGUAGE)" --show-partials --show-metrics

rimv-listen-es: models
	$(MODEL_ENV) $(CARGO) run --release --locked -p rimv -- listen --both --language es --show-partials --show-metrics

# This is an opt-in macOS hardware integration test. afplay sends the corpus to
# the default output device; rimv captures that output through ScreenCaptureKit,
# transcribes it, then reads the sidecar text only for WER scoring.
rimv-e2e-test: models
	$(MODEL_ENV) $(CARGO) run --release --locked -p rimv -- benchmark --corpus resources/audio --mode realtime-capture --language "$(LANGUAGE)" --skip-silence --max-wer "$(MAX_WER)"

stage-capture:
	CARGO="$(CARGO)" scripts/stage-distribution.sh capture

stage-transcribe:
	CARGO="$(CARGO)" scripts/stage-distribution.sh transcribe

stage-server:
	CARGO="$(CARGO)" scripts/stage-distribution.sh server

stage-full:
	CARGO="$(CARGO)" scripts/stage-distribution.sh full

menu-build:
	@test "$$(uname -s)" = Darwin || { echo 'The menu-bar app requires macOS.'; exit 1; }
	@if pgrep -x rimv-menu-bar >/dev/null; then echo 'Quit rimv before rebuilding its app bundle.'; exit 1; fi
	$(CARGO) build --profile $(APP_PROFILE) --locked -p rimv-menu-bar
	@mkdir -p "$(APP_DIR)/Contents/MacOS"
	cp "target/$(APP_PROFILE)/rimv-menu-bar" "$(APP_DIR)/Contents/MacOS/rimv-menu-bar"
	@for dylib in target/$(APP_PROFILE)/*.dylib; do \
		[ -e "$$dylib" ] || continue; \
		cp "$$dylib" "$(APP_DIR)/Contents/MacOS/"; \
	done
	@if ! otool -l "$(APP_DIR)/Contents/MacOS/rimv-menu-bar" | grep -Fq '@loader_path'; then \
		install_name_tool -add_rpath @loader_path "$(APP_DIR)/Contents/MacOS/rimv-menu-bar"; \
	fi
	cp apps/menu-bar/Info.plist "$(APP_DIR)/Contents/Info.plist"
	codesign --force --deep --sign - --identifier dev.rimv.menu "$(APP_DIR)"

menu: menu-build
	open "$(APP_DIR)" --args --recordings "$(abspath $(OUT_DIR))"

menu-test: menu-build
	"$(APP_DIR)/Contents/MacOS/rimv-menu-bar" --self-test

web:
	@cd apps/web && test -d node_modules || npm install
	@cd apps/web && npm run dev

web-build:
	@cd apps/web && test -d node_modules || npm install
	@cd apps/web && npm run build

app-web:
	@if pgrep -x rimv-menu-bar >/dev/null; then \
		echo 'rimv is already running; starting the web client.'; \
	else \
		$(MAKE) menu; \
	fi
	$(MAKE) web

profile:
	$(MAKE) menu-build APP_PROFILE=profiling
	python3 scripts/profile-menu.py --app target/profiling/rimv.app --mode "$(PROFILE_MODE)" --seconds "$(PROFILE_SECONDS)" $(PROFILE_BUDGET_ARGS)

mic:
	@mkdir -p "$(OUT_DIR)"
	$(CARGO) run --release --locked -p $(CLI_PACKAGE) -- mic $(CAPTURE_ARGS) --output "$(MIC_OUTPUT)"

system:
	@mkdir -p "$(OUT_DIR)"
	$(CARGO) run --release --locked -p $(CLI_PACKAGE) -- system $(CAPTURE_ARGS) --output "$(SYSTEM_OUTPUT)"

both:
	@mkdir -p "$(BOTH_DIR)"
	$(CARGO) run --release --locked -p $(CLI_PACKAGE) -- both $(CAPTURE_ARGS) --output-dir "$(BOTH_DIR)"

clean:
	$(CARGO) clean

# This preserves completed WAV recordings. It only removes conventional
# temporary directories plus partial, temporary, and log files under OUT_DIR.
clean-temp:
	@rm -rf -- .tmp tmp "$(OUT_DIR)/.tmp"
	@if [ -d "$(OUT_DIR)" ]; then \
		find "$(OUT_DIR)" -type f \( -name '*.partial' -o -name '*.tmp' -o -name '*.log' \) -delete; \
		find "$(OUT_DIR)" -type d -empty -delete; \
	fi
