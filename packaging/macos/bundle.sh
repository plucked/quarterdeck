#!/bin/sh
# Builds Quarterdeck.app: the binary, Info.plist with the workspace's
# version, and the icon made from AppIcon.png.
#
#   packaging/macos/bundle.sh              # release build, this Mac's architecture
#   packaging/macos/bundle.sh --universal  # release build, Apple Silicon and Intel in one binary
#   packaging/macos/bundle.sh --dev        # debug build for development (bacon's app job)
#
# Release output: target/bundle/Quarterdeck.app, signed ad hoc (enough to
# run on this Mac; distribution signing and notarization come later).
# Dev output: target/debug/Quarterdeck.app, updated in place on every run:
# the debug binary is hard-linked in (no copy) and the icon is only remade
# when AppIcon.png changed, so a rebuild stays quick. Running its
# executable directly gives the app its icon and name while it keeps the
# terminal's environment (GITHUB_TOKEN, gh on PATH).
set -eu

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HERE="$ROOT/packaging/macos"
ICON="$ROOT/target/bundle-icon/AppIcon.icns"
cd "$ROOT"

# The version every crate inherits from the workspace.
VERSION="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)"

# Makes AppIcon.icns (every size macOS asks for, scaled from the 1024 px
# source) unless it is newer than AppIcon.png already.
make_icon() {
	if [ -f "$ICON" ] && [ "$ICON" -nt "$HERE/AppIcon.png" ]; then
		return
	fi
	iconset="$ROOT/target/bundle-icon/AppIcon.iconset"
	rm -rf "$iconset"
	mkdir -p "$iconset"
	for size in 16 32 128 256 512; do
		sips -z "$size" "$size" "$HERE/AppIcon.png" --out "$iconset/icon_${size}x${size}.png" >/dev/null
		double=$((size * 2))
		sips -z "$double" "$double" "$HERE/AppIcon.png" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
	done
	iconutil -c icns "$iconset" -o "$ICON"
}

# Lays out bundle $1 around binary $2: the executable under the app's name
# (hard-linked when $3 is "link", else copied), Info.plist with the
# version, the icon, and the startup movie (Git LFS; a clone without it has
# only a pointer file, and the app then starts without the splash).
lay_out_bundle() {
	app="$1"
	mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
	case "$3" in
		link) ln -f "$2" "$app/Contents/MacOS/Quarterdeck" ;;
		*) cp "$2" "$app/Contents/MacOS/Quarterdeck" ;;
	esac
	cp "$HERE/Info.plist" "$app/Contents/Info.plist"
	plutil -replace CFBundleShortVersionString -string "$VERSION" "$app/Contents/Info.plist"
	plutil -replace CFBundleVersion -string "$VERSION" "$app/Contents/Info.plist"
	make_icon
	cp "$ICON" "$app/Contents/Resources/AppIcon.icns"
	cp "$ROOT/crates/app/assets/startup-intro.mp4" "$app/Contents/Resources/startup-intro.mp4"
}

case "${1:-}" in
	--dev)
		cargo build -p quarterdeck
		lay_out_bundle "$ROOT/target/debug/Quarterdeck.app" "$ROOT/target/debug/quarterdeck" link
		exit 0
		;;
	--universal)
		rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
		cargo build --release -p quarterdeck --target aarch64-apple-darwin
		cargo build --release -p quarterdeck --target x86_64-apple-darwin
		BINARY="$ROOT/target/bundle/quarterdeck-universal"
		mkdir -p "$ROOT/target/bundle"
		lipo -create -output "$BINARY" \
			"$ROOT/target/aarch64-apple-darwin/release/quarterdeck" \
			"$ROOT/target/x86_64-apple-darwin/release/quarterdeck"
		;;
	"")
		cargo build --release -p quarterdeck
		BINARY="$ROOT/target/release/quarterdeck"
		;;
	*)
		echo "usage: $0 [--universal | --dev]" >&2
		exit 2
		;;
esac

# A fresh release bundle, signed ad hoc so the edited bundle still runs here.
APP="$ROOT/target/bundle/Quarterdeck.app"
rm -rf "$APP"
lay_out_bundle "$APP" "$BINARY" copy
codesign --force --deep --sign - "$APP"
echo "Built $APP ($VERSION)"
