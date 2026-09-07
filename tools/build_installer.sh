#!/usr/bin/env bash
# Build the macOS installer: a universal (Intel + Apple Silicon) app bundle and
# a dmg containing the configuration window with the engine as its sidecar.
#
#   1. cargo build --release -p create-companion for both Apple targets
#   2. copy them to ui/src-tauri/binaries/create-companion-<triple> (Tauri builds
#      the app per architecture and merges the sidecars into the universal bundle)
#   3. npm run tauri build -- --target universal-apple-darwin
#   Output: target/universal-apple-darwin/release/bundle/dmg/Create Companion_<version>_universal.dmg
#
# Usage: tools/build_installer.sh          (from anywhere; needs rustup targets, Node 22, Xcode CLT)
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

for t in aarch64-apple-darwin x86_64-apple-darwin; do
  rustup target add "$t" >/dev/null
done

echo "== engine (both architectures) =="
cargo build --release -p create-companion --target aarch64-apple-darwin
cargo build --release -p create-companion --target x86_64-apple-darwin

# Tauri builds the app once per architecture and wants a sidecar named for
# each; it merges them into the universal bundle itself.
mkdir -p ui/src-tauri/binaries
for t in aarch64-apple-darwin x86_64-apple-darwin; do
  cp "target/$t/release/create-companion" "ui/src-tauri/binaries/create-companion-$t"
done
lipo -create \
  target/aarch64-apple-darwin/release/create-companion \
  target/x86_64-apple-darwin/release/create-companion \
  -output ui/src-tauri/binaries/create-companion-universal-apple-darwin
lipo -info ui/src-tauri/binaries/create-companion-universal-apple-darwin

echo "== app bundle + dmg =="
cd ui
[ -d node_modules ] || npm ci
npm run tauri build -- --target universal-apple-darwin
cd "$root"

out="target/universal-apple-darwin/release/bundle/dmg"
for f in "$out"/*.dmg; do
  shasum -a 256 "$f" | sed "s#$out/##" > "$f.sha256"
  ls -lh "$f" | awk '{print $5, $9}'
done
