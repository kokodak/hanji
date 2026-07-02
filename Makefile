.PHONY: help app check-app build-app package-macos test metal

METAL_TOOLCHAIN ?=
FILE ?=
VERSION ?=
export HANJI_FILE = $(value FILE)
export HANJI_METAL_TOOLCHAIN = $(value METAL_TOOLCHAIN)

help:
	@printf "Hanji development commands:\n"
	@printf "  make app               Run the Rust GPUI app\n"
	@printf "  make app FILE=note.md  Run the Rust GPUI app with a Markdown file\n"
	@printf "  make check-app         Check the Rust GPUI app\n"
	@printf "  make build-app         Build the Rust GPUI app\n"
	@printf "  make package-macos     Build a macOS app bundle and DMG\n"
	@printf "  make package-macos VERSION=0.1.0\n"
	@printf "  make test              Test Rust core crates\n"
	@printf "  make metal             Install/show the Metal Toolchain\n"

app:
	@metal_toolchain="$$HANJI_METAL_TOOLCHAIN"; \
	if [ -z "$$metal_toolchain" ]; then \
		metal_toolchain="$$(xcodebuild -showComponent MetalToolchain 2>/dev/null | sed -n 's/^Toolchain Identifier: //p')"; \
	fi; \
	if [ -n "$$metal_toolchain" ]; then \
		printf "Using Metal Toolchain: %s\n" "$$metal_toolchain"; \
		if [ -n "$$HANJI_FILE" ]; then \
			TOOLCHAINS="$$metal_toolchain,com.apple.dt.toolchain.XcodeDefault" cargo run -p hanji -- "$$HANJI_FILE"; \
		else \
			TOOLCHAINS="$$metal_toolchain,com.apple.dt.toolchain.XcodeDefault" cargo run -p hanji; \
		fi; \
	elif metal_path="$$(xcrun --find metal 2>/dev/null)"; then \
		printf "Using Metal compiler: %s\n" "$$metal_path"; \
		if [ -n "$$HANJI_FILE" ]; then \
			cargo run -p hanji -- "$$HANJI_FILE"; \
		else \
			cargo run -p hanji; \
		fi; \
	else \
		printf "Metal compiler was not found. Install full Xcode or run 'make metal' if your Xcode supports Metal Toolchain downloads.\n"; \
		exit 1; \
	fi

check-app:
	@metal_toolchain="$$HANJI_METAL_TOOLCHAIN"; \
	if [ -z "$$metal_toolchain" ]; then \
		metal_toolchain="$$(xcodebuild -showComponent MetalToolchain 2>/dev/null | sed -n 's/^Toolchain Identifier: //p')"; \
	fi; \
	if [ -n "$$metal_toolchain" ]; then \
		printf "Using Metal Toolchain: %s\n" "$$metal_toolchain"; \
		TOOLCHAINS="$$metal_toolchain,com.apple.dt.toolchain.XcodeDefault" cargo check -p hanji; \
	elif metal_path="$$(xcrun --find metal 2>/dev/null)"; then \
		printf "Using Metal compiler: %s\n" "$$metal_path"; \
		cargo check -p hanji; \
	else \
		printf "Metal compiler was not found. Install full Xcode or run 'make metal' if your Xcode supports Metal Toolchain downloads.\n"; \
		exit 1; \
	fi

build-app:
	@metal_toolchain="$$HANJI_METAL_TOOLCHAIN"; \
	if [ -z "$$metal_toolchain" ]; then \
		metal_toolchain="$$(xcodebuild -showComponent MetalToolchain 2>/dev/null | sed -n 's/^Toolchain Identifier: //p')"; \
	fi; \
	if [ -n "$$metal_toolchain" ]; then \
		printf "Using Metal Toolchain: %s\n" "$$metal_toolchain"; \
		TOOLCHAINS="$$metal_toolchain,com.apple.dt.toolchain.XcodeDefault" cargo build -p hanji; \
	elif metal_path="$$(xcrun --find metal 2>/dev/null)"; then \
		printf "Using Metal compiler: %s\n" "$$metal_path"; \
		cargo build -p hanji; \
	else \
		printf "Metal compiler was not found. Install full Xcode or run 'make metal' if your Xcode supports Metal Toolchain downloads.\n"; \
		exit 1; \
	fi

package-macos:
	@VERSION="$(VERSION)" scripts/package-macos.sh

test:
	cargo test --workspace --exclude hanji

metal:
	xcodebuild -downloadComponent MetalToolchain -exportPath /private/tmp/HanjiMetalToolchain
	xcodebuild -showComponent MetalToolchain
