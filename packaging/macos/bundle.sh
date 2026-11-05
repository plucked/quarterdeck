#!/bin/sh
# Builds Quarterdeck.app: the release binary, Info.plist with the
# workspace's version, and the icon made from AppIcon.png. The bundle is
# signed ad hoc, enough to run on this Mac; distribution signing and
# notarization come later.
#
#   packaging/macos/bundle.sh              # this Mac's architecture
#   packaging/macos/bundle.sh --universal  # Apple Silicon and Intel in one binary
#
# Output: target/bundle/Quarterdeck.app
set -eu

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HERE="$ROOT/packaging/macos"
APP="$ROOT/target/bundle/Quarterdeck.app"
cd "$ROOT"

# The version every crate inherits from the workspace.
VERSION="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)"

# Build: one architecture, or both joined into a universal binary.
case "${1:-}" in
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
		echo "usage: $0 [--universal]" >&2
		exit 2
		;;
esac

# A fresh bundle: the executable under the app's name, and Info.plist with the version.
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BINARY" "$APP/Contents/MacOS/Quarterdeck"
cp "$HERE/Info.plist" "$APP/Contents/Info.plist"
plutil -replace CFBundleShortVersionString -string "$VERSION" "$APP/Contents/Info.plist"
plutil -replace CFBundleVersion -string "$VERSION" "$APP/Contents/Info.plist"

# The icon: every size macOS asks for, scaled from the 1024 px source.
ICONSET="$ROOT/target/bundle/AppIcon.iconset"
rm -rf "$ICONSET"
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
	sips -z "$size" "$size" "$HERE/AppIcon.png" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
	double=$((size * 2))
	sips -z "$double" "$double" "$HERE/AppIcon.png" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"

# Sign ad hoc, so the edited bundle still runs here.
codesign --force --deep --sign - "$APP"

echo "Built $APP ($VERSION)"
