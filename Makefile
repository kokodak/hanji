.PHONY: help app check-app build-app test metal

METAL_TOOLCHAIN ?=
FILE ?=
export HANJI_FILE = $(value FILE)
export HANJI_METAL_TOOLCHAIN = $(value METAL_TOOLCHAIN)

help:
	@printf "Hanji development commands:\n"
	@printf "  make app               Run the Rust GPUI app\n"
	@printf "  make app FILE=note.md  Run the Rust GPUI app with a Markdown file\n"
	@printf "  make check-app         Check the Rust GPUI app\n"
	@printf "  make build-app         Build the Rust GPUI app\n"
	@printf "  make test              Test Rust core crates\n"
	@printf "  make metal             Install/show the Metal Toolchain\n"

app:
	@metal_toolchain="$$HANJI_METAL_TOOLCHAIN"; \
	if [ -z "$$metal_toolchain" ]; then \
		metal_toolchain="$$(xcodebuild -showComponent MetalToolchain 2>/dev/null | sed -n 's/^Toolchain Identifier: //p')"; \
	fi; \
	if [ -z "$$metal_toolchain" ]; then \
		printf "Metal Toolchain was not found. Run 'make metal' first.\n"; \
		exit 1; \
	fi; \
	printf "Using Metal Toolchain: %s\n" "$$metal_toolchain"; \
	if [ -n "$$HANJI_FILE" ]; then \
		TOOLCHAINS="$$metal_toolchain,com.apple.dt.toolchain.XcodeDefault" cargo run -p hanji-rust -- "$$HANJI_FILE"; \
	else \
		TOOLCHAINS="$$metal_toolchain,com.apple.dt.toolchain.XcodeDefault" cargo run -p hanji-rust; \
	fi

check-app:
	@metal_toolchain="$$HANJI_METAL_TOOLCHAIN"; \
	if [ -z "$$metal_toolchain" ]; then \
		metal_toolchain="$$(xcodebuild -showComponent MetalToolchain 2>/dev/null | sed -n 's/^Toolchain Identifier: //p')"; \
	fi; \
	if [ -z "$$metal_toolchain" ]; then \
		printf "Metal Toolchain was not found. Run 'make metal' first.\n"; \
		exit 1; \
	fi; \
	printf "Using Metal Toolchain: %s\n" "$$metal_toolchain"; \
	TOOLCHAINS="$$metal_toolchain,com.apple.dt.toolchain.XcodeDefault" cargo check -p hanji-rust

build-app:
	@metal_toolchain="$$HANJI_METAL_TOOLCHAIN"; \
	if [ -z "$$metal_toolchain" ]; then \
		metal_toolchain="$$(xcodebuild -showComponent MetalToolchain 2>/dev/null | sed -n 's/^Toolchain Identifier: //p')"; \
	fi; \
	if [ -z "$$metal_toolchain" ]; then \
		printf "Metal Toolchain was not found. Run 'make metal' first.\n"; \
		exit 1; \
	fi; \
	printf "Using Metal Toolchain: %s\n" "$$metal_toolchain"; \
	TOOLCHAINS="$$metal_toolchain,com.apple.dt.toolchain.XcodeDefault" cargo build -p hanji-rust

test:
	cargo test --workspace --exclude hanji-rust

metal:
	xcodebuild -downloadComponent MetalToolchain -exportPath /private/tmp/HanjiMetalToolchain
	xcodebuild -showComponent MetalToolchain
