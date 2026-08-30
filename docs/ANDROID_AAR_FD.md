# Android .aar via gomobile — fd wiring (prep, waits TUN desktop API)

## Current (exec sidecar, MVP)

- `rclash-android/app/src/main/java/com/rclash/CoreBridge.kt:12` `startWithFd(tunFd, context)` → `ProcessBuilder(rclash-core -d ... -f ...)` with `env TUN_FD=fd`
- `RClashVpnService.kt:28` `Builder().establish()` → `ParcelFileDescriptor.fd` → `CoreBridge.startWithFd(fd)`
- Works without `.aar`, fallback for all.

## Future (.aar via gomobile, after TUN desktop stable)

- `mihomo/Makefile` target `gomobile`:
  ```make
  aar:
  	gomobile bind -target android -javapkg com.rclash.core -o rclash.aar ./...
  ```
- `mihomo/.github/workflows/build-core.yml` job `gomobile` (runs after `matrix`):
  ```yaml
  gomobile:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: golang/mobile@...
      - run: gomobile bind -target android -o rclash.aar ./...
      - uses: actions/upload-artifact@v4
        with: { name: rclash-aar, path: rclash.aar }
  ```
- `rclash-android/app/build.gradle.kts`:
  ```kotlin
  dependencies { implementation(files("libs/rclash.aar")) }
  ```
  Copy `rclash.aar` → `rclash-android/app/libs/rclash.aar` via CI `download-artifact`.

- `CoreBridge.kt` `init { System.loadLibrary("rclash") }` + `external fun nativeStartWithFd(fd: Int, configDir: String, configFile: String): Int` (gomobile `bind` generates `Go` `StartWithFd`). Fallback to `exec` if `UnsatisfiedLinkError` or `useAar=false`.

- `RClashVpnService.kt` already passes `fd` → no change, but `protectSocket` must be set **before** `startWithFd` for Go to call `protect(fd)` via `VpnService.protect`.

- Test: `./gradlew :app:assembleDebug` with `rclash.aar` in `libs/`, `adb logcat | grep rclash`, tun `fd` valid (non- -1), traffic via `VpnService`.

## Why wait TUN desktop?

TUN design (`pkexec`/`wintun`/`utun`) fixes `fd` type and `protect` contract. Changing after `.aar` requires `CoreBridge` rewrite. Decision `.agents/project-decisions.md:39` → TUN first.

## CI stub today

- `rclash-android/.github/workflows/ci.yml` keeps exec sidecar, no `gomobile` yet.
- `sync-core.yml` in `rclash` mirrors nightly `rclash-core-*` for updater; `.aar` mirror added after `gomobile` job lands in `mihomo`.
