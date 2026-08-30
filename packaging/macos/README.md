# macOS TUN via utun + osascript

- Default (self-signed): `osascript -e 'do shell script "rclash-tun-helper up" with administrator privileges'` — no Developer ID needed.
- Signed path (optional): `SMJobBless` helper `com.rclash.helper` in `/Library/PrivilegedHelperTools/` — requires `Developer ID Application` cert + `launchd` plist. Gated behind `feature = "macos-signed"`.

`helper_up` uses `ifconfig utun3 create` + `route add 198.18.0.0/16 -interface utun3`.
