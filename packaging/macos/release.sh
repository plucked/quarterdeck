#!/bin/sh
# Builds, signs, notarizes and publishes a Quarterdeck release from this Mac.
#
#   packaging/macos/release.sh                  # release the version in Cargo.toml
#   packaging/macos/release.sh --notes FILE     # with these release notes (Markdown)
#   packaging/macos/release.sh --dry-run        # build and sign only: no notarization, no upload
#
# Output in target/release-artifacts/<version>/:
#   Quarterdeck-<version>.zip   the app, what the in-app updater downloads
#   Quarterdeck-<version>.dmg   the app and an Applications link, for first installs
#   latest.json                 version, notes, zip URL, SHA-256 and size: the updater's feed
#   notes.md                    the release notes
#
# Publishing creates the GitHub release v<version> on plucked/quarterdeck at
# HEAD as a draft, uploads the three files, then publishes it, so
# releases/latest/download/latest.json never points at a release without them.
#
# Needs, once:
#   - the "Developer ID Application" certificate in the login keychain;
#   - notary credentials stored under the profile name below:
#       xcrun notarytool store-credentials quarterdeck-notary \
#         --apple-id <apple id> --team-id 97ZZEC8R43 --password <app-specific password>
#   - gh logged in (gh auth login) with push access to plucked/quarterdeck.
#     The script uses gh's own login, never GITHUB_TOKEN or GH_TOKEN: those
#     often hold a token for Quarterdeck itself that cannot create releases.
# Before each release: bump [workspace.package] version in Cargo.toml,
# commit, and push.
set -eu

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HERE="$ROOT/packaging/macos"
cd "$ROOT"

REPOSITORY="plucked/quarterdeck"
SIGNING_IDENTITY="${QUARTERDECK_SIGNING_IDENTITY:-Developer ID Application: Rainer Heynke (97ZZEC8R43)}"
NOTARY_PROFILE="${QUARTERDECK_NOTARY_PROFILE:-quarterdeck-notary}"
VERSION="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)"
TAG="v$VERSION"
OUT="$ROOT/target/release-artifacts/$VERSION"
APP="$ROOT/target/bundle/Quarterdeck.app"
ZIP="$OUT/Quarterdeck-$VERSION.zip"
DMG="$OUT/Quarterdeck-$VERSION.dmg"
MANIFEST="$OUT/latest.json"
NOTES="$OUT/notes.md"

fail() {
	echo "release: $*" >&2
	exit 1
}

# gh with its own stored login: a token in the environment would take precedence.
github() {
	env -u GITHUB_TOKEN -u GH_TOKEN gh "$@"
}

# Options.
NOTES_SOURCE=""
DRY_RUN=0
while [ $# -gt 0 ]; do
	case "$1" in
		--notes) NOTES_SOURCE="$2"; shift 2 ;;
		--dry-run) DRY_RUN=1; shift ;;
		*) echo "usage: $0 [--notes FILE] [--dry-run]" >&2; exit 2 ;;
	esac
done

# Checks before anything is built: a published release must be exactly a
# pushed commit, a new version, with the real startup movie.
if [ "$DRY_RUN" = 0 ]; then
	git diff --quiet && git diff --cached --quiet || fail "the working tree has uncommitted changes"
	[ -n "$(git branch -r --contains HEAD)" ] || fail "HEAD is not pushed"
	[ "$(github api "repos/$REPOSITORY" --jq .permissions.push 2>/dev/null)" = "true" ] \
		|| fail "gh's login cannot push to $REPOSITORY (run gh auth login)"
	! github release view "$TAG" --repo "$REPOSITORY" >/dev/null 2>&1 || fail "$TAG is already released; bump the version in Cargo.toml"
	xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" >/dev/null 2>&1 \
		|| fail "no notary credentials under the profile $NOTARY_PROFILE (see the header of this script)"
fi
head -c 64 crates/app/assets/startup-intro.mp4 | grep -q "git-lfs" \
	&& fail "the startup movie is a Git LFS pointer; run git lfs pull"
security find-identity -v -p codesigning | grep -q "$SIGNING_IDENTITY" \
	|| fail "the signing identity \"$SIGNING_IDENTITY\" is not in the keychain"
rm -rf "$OUT"
mkdir -p "$OUT"

# Release notes: the file given, else the feat: and fix: commits since the last release.
if [ -n "$NOTES_SOURCE" ]; then
	cp "$NOTES_SOURCE" "$NOTES"
else
	previous="$(git describe --tags --abbrev=0 --match 'v*' HEAD 2>/dev/null || true)"
	range="${previous:+$previous..}HEAD"
	features="$(git log --no-merges --format='%s' "$range" | sed -n 's/^feat: \(.*\)/- \1/p')"
	fixes="$(git log --no-merges --format='%s' "$range" | sed -n 's/^fix: \(.*\)/- \1/p')"
	{
		[ -n "$features" ] && printf '### New\n\n%s\n\n' "$features"
		[ -n "$fixes" ] && printf '### Fixed\n\n%s\n' "$fixes"
		true
	} >"$NOTES"
fi

# Build the bundle, then sign it for distribution: Developer ID, hardened
# runtime (required for notarization) and a secure timestamp.
"$HERE/bundle.sh"
codesign --force --options runtime --timestamp --sign "$SIGNING_IDENTITY" "$APP"
codesign --verify --strict --deep "$APP"

# Sends $1 to Apple's notary service, waits for the verdict, and staples the ticket to $2.
notarize() {
	xcrun notarytool submit "$1" --keychain-profile "$NOTARY_PROFILE" --wait
	xcrun stapler staple "$2"
}

# The zip the updater downloads: notarize the app, staple the ticket into
# it (so it opens without a network check), then zip the stapled app.
ditto -c -k --keepParent "$APP" "$ZIP"
if [ "$DRY_RUN" = 0 ]; then
	notarize "$ZIP" "$APP"
	spctl --assess --type execute "$APP"
	ditto -c -k --keepParent "$APP" "$ZIP"
fi

# The disk image for first installs: the app beside a link to Applications.
dmg_folder="$OUT/dmg"
mkdir -p "$dmg_folder"
ditto "$APP" "$dmg_folder/Quarterdeck.app"
ln -s /Applications "$dmg_folder/Applications"
hdiutil create -quiet -volname "Quarterdeck" -srcfolder "$dmg_folder" -format UDZO -ov "$DMG"
rm -rf "$dmg_folder"
codesign --force --timestamp --sign "$SIGNING_IDENTITY" "$DMG"
[ "$DRY_RUN" = 0 ] && notarize "$DMG" "$DMG"

# The updater's feed: written as a property list with the system's plutil,
# then converted, so the notes are escaped correctly.
zip_name="$(basename "$ZIP")"
plutil -create xml1 "$MANIFEST"
plutil -insert version -string "$VERSION" "$MANIFEST"
plutil -insert notes -string "$(cat "$NOTES")" "$MANIFEST"
plutil -insert zip_url -string "https://github.com/$REPOSITORY/releases/download/$TAG/$zip_name" "$MANIFEST"
plutil -insert zip_sha256 -string "$(shasum -a 256 "$ZIP" | cut -d ' ' -f 1)" "$MANIFEST"
plutil -insert zip_size -integer "$(stat -f %z "$ZIP")" "$MANIFEST"
plutil -convert json -r "$MANIFEST"

if [ "$DRY_RUN" = 1 ]; then
	echo "Dry run: built and signed $VERSION in $OUT (not notarized, not published)"
	exit 0
fi

# Publish: a draft with every file first, then public in one step.
github release create "$TAG" "$ZIP" "$DMG" "$MANIFEST" \
	--repo "$REPOSITORY" \
	--target "$(git rev-parse HEAD)" \
	--title "Quarterdeck $VERSION" \
	--notes-file "$NOTES" \
	--draft
github release edit "$TAG" --repo "$REPOSITORY" --draft=false --latest
git fetch --tags --quiet
echo "Released $TAG: https://github.com/$REPOSITORY/releases/tag/$TAG"
