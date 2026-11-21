#!/usr/bin/env bash
# Gera o "Block Screen Saver.app" universal (Intel + Apple Silicon) e o .zip
# publicado no release. Roda no macOS, com rustup e as Command Line Tools.
# Uso: ./packaging/macos/build-app.sh
# Saída: target/macos/Block Screen Saver.app e
#        target/macos/block-screen-saver-macos-universal.zip

set -euo pipefail

cd "$(dirname "$0")/../.."

APP_NAME="Block Screen Saver"
BINARY="block-screen-saver"
ASSET="block-screen-saver-macos-universal.zip"
MIN_MACOS="11.0"
TARGETS=(x86_64-apple-darwin aarch64-apple-darwin)
VERSION=$(sed -n -E 's/^version = "([0-9]+\.[0-9]+\.[0-9]+)"$/\1/p' Cargo.toml | head -n 1)
OUT="target/macos"
APP="$OUT/$APP_NAME.app"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Este script só roda no macOS." >&2
  exit 1
fi

export MACOSX_DEPLOYMENT_TARGET="$MIN_MACOS"
rustup target add "${TARGETS[@]}"
binaries=()
for target in "${TARGETS[@]}"; do
  cargo build --locked --release --target "$target"
  binaries+=("target/$target/release/$BINARY")
done

rm -rf "$OUT"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
lipo -create -output "$APP/Contents/MacOS/$BINARY" "${binaries[@]}"

# O ícone legado só existe em 16 px; o iconset é gerado a partir dele.
iconset="$OUT/AppIcon.iconset"
mkdir -p "$iconset"
sips -s format png assets/preferences-desktop-screensaver.ico --out "$OUT/icon.png" >/dev/null
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" "$OUT/icon.png" --out "$iconset/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z "$double" "$double" "$OUT/icon.png" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$APP/Contents/Resources/AppIcon.icns"
rm -rf "$iconset" "$OUT/icon.png"

sed -e "s/@VERSION@/$VERSION/g" -e "s/@MIN_MACOS@/$MIN_MACOS/g" \
  packaging/macos/Info.plist > "$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist" >/dev/null

# Assinatura ad hoc: o Apple Silicon não executa binários sem assinatura. Não há
# notarização, então um download pelo navegador passa pelo Gatekeeper (ver README).
codesign --force --sign - --timestamp=none "$APP"
codesign --verify --strict "$APP"

ditto -c -k --keepParent "$APP" "$OUT/$ASSET"

echo "Aplicativo: $APP ($(lipo -archs "$APP/Contents/MacOS/$BINARY"), macOS $MIN_MACOS+)"
echo "Pacote:     $OUT/$ASSET"
