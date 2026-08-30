# Windows TUN via wintun

- `wintun.dll` bundled via `rclash/packaging/windows/wintun.dll` (download from https://www.wintun.net).
- Service `RClashTun` installed by `rclash-tun-helper service` (requires admin, manifest `requireAdministrator`).
- Self-signed CI: ephemeral cert generated on runner if `WIN_PFX_B64` missing.

Copy `wintun.dll` (amd64) to this dir before release. CI will copy from `packaging/windows/wintun.dll` to `dist/`.
