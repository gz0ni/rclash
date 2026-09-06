# Gotchas

Traps and pitfalls the agent has stepped on in this project. Log a gotcha immediately whenever something does not build, run, or pass — right after the incident, while context is fresh, do not wait for session end. Check this file before repeating similar work. One entry per trap.

## Format

```
### <Short trap title> (YYYY-MM-DD)

- What happened / what was wrong
- Root cause
- How to avoid next time
```

---

<!-- New entries go here, newest first. -->

### Phantom modified files with identical content (2026-09-06)

- `git status` showed 1000+ `M` (all of `core/` plus 14 other files) while `git diff` was empty and `hash-object == ls-files == HEAD` for every sampled file.
- Root cause: stale index stat info on Windows — no content, mode, eol, or attr change; `git update-index --refresh` did not clear it.
- Avoid: never trust bare `git status` here — stage only paths from `git diff HEAD --name-only` plus intended untracked files, never blanket `git add -A`.

### GEOSITE dash-names vs meta-rules-dat @-names + slow -t healing loop (2026-09-04)

- Профиль требовал `GEOSITE,adobe-ads` и ещё ~18 дефисных `*-ads`, а свежий `meta-rules-dat` содержит только `adobe@ads`-стиль — обновление файла не лечит, ядро падает на `-t`.
- Root cause: ядро не обновляет существующий `GeoSite.dat` перед парсингом; `geo-auto-update` помогает только после успешного старта; mihomo падает на первом же отсутствующем списке, поэтому цикл strip+retry идёт по одному списку за `-t` (~0.25–3 с каждый, зависит от прогрева кэша и размера `GeoIP.dat`).
- Avoid: `ensure_geodata` + цикл `precheck → parse_missing_geodata → strip_missing_geodata_rules` (лимит 60) + `geox-url` на jsdelivr; хвост `-t` для парсинга — 12 строк, не 5.

### mihomo PUT /configs требует JSON-body, PUT /dns нет, PATCH молча игнорит ключи (2026-09-04)

- Живой E2E против `rclash-core` показал: `PUT /configs?force=true` без тела отвечает `400 Body invalid` (был сломан и старый `reload_blocking` без body); эндпоинта `PUT /dns` нет вовсе (только `GET /dns/query`); `PATCH /configs` молча игнорирует неизвестные ключи — проверены рабочие `mode/log-level/ipv6/allow-lan/tcp-concurrent/find-process-mode`, НЕ работают `unified-delay/keep-alive-interval`.
- Root cause: контракт смотрел по памяти, а не по `core/hub/route/configs.go` + `dns.go`.
- Avoid: DNS/хосты — только пересборка runtime + reload с телом `{}`; hot-PATCH только проверенными ключами, остальное — рестарт; новые REST-вызовы сверять с `core/hub/route/*.go` и живым E2E.

### ImageMagick Q16 renders gray PNGs as 16-bit grayscale (2026-09-04)

- `magick icon.svg -resize 32x32 out.png` produced 16-bit grayscale PNG (Q16 default), `png` crate decoder returned 2 bytes/px and icon unit tests failed on buffer length.
- Root cause: Q16 build keeps 16-bit depth; gray-only image also collapses to `Grayscale` color type.
- Avoid: always render with `-depth 8 -define png:color-type=6` (forced 8-bit sRGB/RGBA), verify with `magick identify -format "%f alpha=%A channels=%[channels]"`.

### magick -background must precede SVG input, else white flats (2026-09-04)

- Renders came out with opaque white corners (`identify` alpha_min=1 alpha_max=1) instead of transparency.
- Root cause: `-background none` was placed AFTER `icon.svg`, so MSVG rasterized onto the default white canvas; the flag only affects ops after it.
- Avoid: `magick -background none icon.svg -resize ... out.png`; always verify `alpha_min=0`.

### mihomo ignores relative -f, empty proxy group is fatal (2026-09-03)

- E2E `rclash-core -d <dir> -f config.yaml` silently fell back to default config (mixed 7890); empty `proxies: []` in a `PROXY` select group is a fatal parse error.
- Root cause: `-f` must be absolute; mihomo requires non-empty `proxies`/`use` in every group.
- Avoid: supervisor always passes absolute `-d`/`-f` and runs `-t` pre-check; `build_raw_keys_config` never emits empty groups.

### mihomo /traffic never ends (2026-09-03)

- `Invoke-WebRequest /traffic` hung until shell timeout.
- Root cause: `/traffic` is an infinite chunked JSON stream by design.
- Avoid: read it only as a line stream in a dedicated thread (`BufRead::read_line`), never with one-shot request helpers.

### egui Area/Window reuses persisted rect by id (2026-09-03)

- Input dialog rendered wider than its fixed content suggested.
- Root cause: suspect — `Area`/`Window` with id `input_dialog` reused a persisted rect from the earlier `Window` with the same id; egui/eframe remembers rects per id.
- Avoid: give rebuilt overlays a fresh id (e.g. `input_dialog_v2`); cap content with `set_max_width`.

### cargo test fails while debug exe is running (2026-09-03)

- `cargo test --workspace` failed to link: `failed to remove file target\debug\rclash.exe`, os error 5 (access denied).
- Root cause: running `rclash.exe` (PID 26348) locks the binary; cargo cannot replace it.
- Avoid: close the running app before `cargo test --workspace`; lib-only tests (`cargo test -p <crate>`) work while it runs.

## Example

### Riverpod provider was not overridden in tests (2026-01-15)

- Tests crashed with ProviderNotFoundException.
- Root cause: `settingsRepositoryProvider` must be overridden in `main()` and in tests via `ProviderScope`.
- Avoid: always override providers in widget tests; see `.agents/commands.md` for the test setup.