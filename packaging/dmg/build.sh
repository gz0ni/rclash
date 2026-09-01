#!/usr/bin/env bash
set -euo pipefail
TARGET="${1:-aarch64-apple-darwin}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DIST="$ROOT/dist"
mkdir -p "$DIST"
BIN="$ROOT/target/$TARGET/release/rclash"
CORE="$ROOT/target/$TARGET/release/rclash-core"
APP="$DIST/RClash.app"
mkdir -p "$APP/Contents/MacOS"
cp "$BIN" "$APP/Contents/MacOS/RClash" 2>/dev/null || cp "$ROOT/target/release/rclash" "$APP/Contents/MacOS/RClash" || { echo "binary not found, skip dmg"; exit 0; }
cp "$CORE" "$APP/Contents/MacOS/rclash-core" 2>/dev/null || cp "$ROOT/target/release/rclash-core" "$APP/Contents/MacOS/rclash-core" 2>/dev/null || echo "core not bundled — dmg without core"
cat > "$APP/Contents/Info.plist" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>RClash</string>
  <key>CFBundleIdentifier</key><string>com.rclash.app</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleExecutable</key><string>RClash</string>
  <key>CFBundlePackageType</key><string>APPL</string>
</dict></plist>
EOF
if command -v create-dmg >/dev/null 2>&1; then
  create-dmg --volname RClash "$DIST/RClash-$TARGET.dmg" "$APP" || echo "create-dmg failed"
else
  echo "create-dmg not found — skip"
  hdiutil create -volname RClash -srcfolder "$APP" -ov -format UDZO "$DIST/RClash-$TARGET.dmg" 2>/dev/null || true
fi
