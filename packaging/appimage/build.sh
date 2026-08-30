#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DIST="$ROOT/dist"
APPDIR="$ROOT/packaging/appimage/RClash.AppDir"
BIN="$ROOT/target/x86_64-unknown-linux-gnu/release/rclash"
mkdir -p "$DIST" "$APPDIR/usr/bin"
cp "$BIN" "$APPDIR/usr/bin/rclash" 2>/dev/null || cp "$ROOT/target/release/rclash" "$APPDIR/usr/bin/rclash" || { echo "binary not found"; exit 0; }
cat > "$APPDIR/RClash.desktop" <<'EOF'
[Desktop Entry]
Name=RClash
Exec=rclash
Icon=rclash
Type=Application
Categories=Network;
EOF
cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
exec "$(dirname "$0")/usr/bin/rclash" "$@"
EOF
chmod +x "$APPDIR/AppRun"
if command -v appimagetool >/dev/null 2>&1; then
  appimagetool "$APPDIR" "$DIST/RClash-x86_64.AppImage"
else
  echo "appimagetool not found — skip"
fi
