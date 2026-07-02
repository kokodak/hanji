#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root_dir"

app_name="${APP_NAME:-Hanji}"
binary_name="${BINARY_NAME:-hanji}"
bundle_identifier="${BUNDLE_IDENTIFIER:-io.github.kokodak.hanji}"
icon_file="${ICON_FILE:-Hanji}"
macos_min_version="${MACOS_MIN_VERSION:-13.0}"
dist_dir="$root_dir/dist"
package_dir="$dist_dir/macos"
app_dir="$package_dir/$app_name.app"
contents_dir="$app_dir/Contents"
macos_dir="$contents_dir/MacOS"
resources_dir="$contents_dir/Resources"
dmg_root="$package_dir/dmg-root"

require_match() {
	local name="$1"
	local value="$2"
	local pattern="$3"

	if [[ ! "$value" =~ $pattern ]]; then
		printf "%s has an unsupported value: %s\n" "$name" "$value" >&2
		exit 1
	fi
}

require_match "APP_NAME" "$app_name" '^[A-Za-z0-9][A-Za-z0-9._ -]*$'
require_match "BINARY_NAME" "$binary_name" '^[A-Za-z0-9_-]+$'
require_match "BUNDLE_IDENTIFIER" "$bundle_identifier" '^[A-Za-z0-9][A-Za-z0-9.-]*$'
require_match "ICON_FILE" "$icon_file" '^[A-Za-z0-9][A-Za-z0-9._ -]*$'
require_match "MACOS_MIN_VERSION" "$macos_min_version" '^[0-9]+(\.[0-9]+){1,2}$'

manifest_version="$(
	awk '
		/^\[workspace\.package\]/ {
			in_workspace_package = 1
			next
		}
		/^\[/ {
			in_workspace_package = 0
		}
		in_workspace_package && $1 == "version" {
			gsub(/"/, "", $3)
			print $3
			exit
		}
	' Cargo.toml
)"

version="${VERSION:-$manifest_version}"
version="${version#v}"

if [ -z "$version" ]; then
	printf "Release version was not provided and could not be read from Cargo.toml.\n" >&2
	exit 1
fi

if [ "$version" != "$manifest_version" ]; then
	printf "VERSION=%s does not match Cargo.toml workspace version %s.\n" "$version" "$manifest_version" >&2
	exit 1
fi
require_match "VERSION" "$version" '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'

build_number="${BUILD_NUMBER:-$(git rev-list --count HEAD 2>/dev/null || printf "1")}"
require_match "BUILD_NUMBER" "$build_number" '^[0-9A-Za-z._-]+$'
host_arch="${ARCH:-$(uname -m)}"

case "$host_arch" in
	arm64|aarch64)
		asset_arch="arm64"
		;;
	x86_64|amd64)
		asset_arch="x86_64"
		;;
	*)
		printf "Unsupported ARCH value: %s\n" "$host_arch" >&2
		exit 1
		;;
esac

if ! command -v hdiutil >/dev/null 2>&1; then
	printf "hdiutil is required to create a macOS DMG.\n" >&2
	exit 1
fi

if ! command -v plutil >/dev/null 2>&1; then
	printf "plutil is required to validate the app bundle Info.plist.\n" >&2
	exit 1
fi

metal_toolchain="${HANJI_METAL_TOOLCHAIN:-${METAL_TOOLCHAIN:-}}"
if [ -z "$metal_toolchain" ]; then
	metal_toolchain="$(xcodebuild -showComponent MetalToolchain 2>/dev/null | sed -n 's/^Toolchain Identifier: //p' | head -n 1 || true)"
fi

if [ -n "$metal_toolchain" ]; then
	printf "Using Metal Toolchain: %s\n" "$metal_toolchain"
	TOOLCHAINS="$metal_toolchain,com.apple.dt.toolchain.XcodeDefault" cargo build -p "$binary_name" --release
elif metal_path="$(xcrun --find metal 2>/dev/null)"; then
	printf "Using Metal compiler: %s\n" "$metal_path"
	cargo build -p "$binary_name" --release
else
	printf "Metal compiler was not found. Install full Xcode or run 'make metal' if your Xcode supports Metal Toolchain downloads.\n" >&2
	exit 1
fi

rm -rf "$app_dir" "$dmg_root"
mkdir -p "$macos_dir" "$resources_dir" "$dmg_root"

install -m 755 "target/release/$binary_name" "$macos_dir/$binary_name"
icon_source="packaging/macos/$icon_file.icns"
if [ ! -f "$icon_source" ]; then
	printf "App icon not found: %s\n" "$icon_source" >&2
	exit 1
fi
install -m 644 "$icon_source" "$resources_dir/$icon_file.icns"

sed \
	-e "s/@APP_NAME@/$app_name/g" \
	-e "s/@BINARY_NAME@/$binary_name/g" \
	-e "s/@BUNDLE_IDENTIFIER@/$bundle_identifier/g" \
	-e "s/@ICON_FILE@/$icon_file/g" \
	-e "s/@VERSION@/$version/g" \
	-e "s/@BUILD_NUMBER@/$build_number/g" \
	-e "s/@MACOS_MIN_VERSION@/$macos_min_version/g" \
	packaging/macos/Info.plist.in > "$contents_dir/Info.plist"

printf "APPL????" > "$contents_dir/PkgInfo"
plutil -lint "$contents_dir/Info.plist"

if [ -n "${CODESIGN_IDENTITY:-}" ]; then
	codesign --force --deep --options runtime --timestamp --sign "$CODESIGN_IDENTITY" "$app_dir"
else
	printf "Ad-hoc signing app bundle because CODESIGN_IDENTITY is not set.\n"
	codesign --force --deep --sign - "$app_dir"
fi

ditto "$app_dir" "$dmg_root/$app_name.app"
ln -s /Applications "$dmg_root/Applications"

dmg_path="$dist_dir/$app_name-$version-macos-$asset_arch.dmg"
rm -f "$dmg_path" "$dmg_path.sha256"

hdiutil create \
	-volname "$app_name $version" \
	-srcfolder "$dmg_root" \
	-ov \
	-format UDZO \
	"$dmg_path"

(
	cd "$dist_dir"
	shasum -a 256 "$(basename "$dmg_path")" > "$(basename "$dmg_path").sha256"
)

printf "Created %s\n" "$dmg_path"
printf "Created %s.sha256\n" "$dmg_path"
