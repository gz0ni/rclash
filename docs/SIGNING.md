# Signing — self-signed gated (MVP) + trusted path

CI `release.yml` is gated: if secrets present → trusted sign, else → ephemeral self-signed / ad-hoc, never fails.

## Current (self-signed MVP)

- **Windows:** no `WIN_PFX_B64` → `New-SelfSignedCertificate CN=RClash Self-Signed` → `signtool sign /f self.pfx /p rclash` on `rclash.exe` + `dist/*.exe`. Trusted UI still shows `Неизвестный издатель`, but binary is signed. For local trust: `certutil -addstore Root RClash.cer` or import `self.pfx`.
- **macOS:** no `APPLE_CERT_B64` → `codesign --force --deep --sign -` (ad-hoc). No notarization. User opens via `ПКМ → Открыть` or `xattr -d com.apple.quarantine /Applications/RClash.app`. TUN helper via `osascript with administrator privileges`, not `SMJobBless`.
- **Linux:** no signing (DEB/RPM `dpkg-sig` optional).
- **Android:** no `ANDROID_KEYSTORE_B64` → `keytool -genkey` ephemeral `debug.jks` → `apksigner sign` or Gradle `assembleRelease` with generated keystore. APK installs with `Allow from unknown sources`.

## Trusted path (when you get certs)

### Windows — Azure Trusted Signing (recommended, public trust)
Follow steps in `docs/superpowers/plans/2026-08-30-tun-selfsigned-updater-android.md` → create `Trusted Signing Account` → set secrets `AZURE_CLIENT_ID/TENANT_ID/SUBSCRIPTION_ID/TRUSTED_SIGNING_ACCOUNT/PROFILE`. Replace `signtool` step with `azure/trusted-signing-action@v0.5.2`.

### Windows — classical PFX
Buy OV/EV (Sectigo/DigiCert) → export `PFX` → `base64 -w0 cert.pfx` → `WIN_PFX_B64`, `WIN_PFX_PASSWORD`. CI `signtool sign /f /tmp/cert.pfx /p ... /fd SHA256 /tr http://timestamp.digicert.com`.

### macOS — Apple Developer Program ($99)
`developer.apple.com` → `Certificates → Developer ID Application` → export `.p12` → `APPLE_CERT_B64` + `APPLE_CERT_PASSWORD` + `APPLE_API_KEY/ISSUER/TEAM_ID` for `notarytool`. CI `codesign --sign "Developer ID Application"` + `notarytool submit --wait && stapler staple`. TUN helper can then use `SMJobBless` (`feature="macos-signed"`).

### Android — Play/App signing
`keytool -genkey -v -keystore rclash.jks -keyalg RSA -keysize 2048 -validity 10000 -alias rclash` → `base64` → `ANDROID_KEYSTORE_B64` etc. For Play, use Upload key or `apksigner`.

## Switching
Add secrets in `GitHub → Settings → Secrets and variables → Actions → New repository secret`. Next `tag v*` run will pick trusted path automatically (`if: secrets.* != ''`).

## Local self-signed generation (manual)

```powershell
# Windows
$cert = New-SelfSignedCertificate -Type CodeSigning -Subject "CN=RClash" -KeyExportPolicy Exportable -KeySpec Signature -KeyLength 2048 -HashAlgorithm SHA256 -CertStoreLocation Cert:\CurrentUser\My
$pwd = ConvertTo-SecureString -String "rclash" -Force -AsPlainText
Export-PfxCertificate -Cert $cert -FilePath RClash.pfx -Password $pwd
```

```bash
# macOS ad-hoc (no cert)
codesign --force --deep --sign - target/release/rclash && codesign --verify --verbose target/release/rclash

# Android debug
keytool -genkey -v -keystore debug.jks -keyalg RSA -keysize 2048 -validity 10000 -alias androiddebugkey -storepass android -keypass android -dname "CN=RClash"
```

## Artifact locations
- Windows: `dist/setup-RClash-*.exe` (Inno), `target/x86_64-pc-windows-msvc/release/rclash.exe`
- macOS: `dist/RClash.dmg`, `target/*/release/rclash`
- Android: `rclash-android/app/build/outputs/apk/release/*.apk`
