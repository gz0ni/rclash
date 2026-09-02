# Design Spec — mockup/index.html → Rust egui

Source of truth: `mockup/index.html` (NekoBox 840×560 fixed). Window uses native OS titlebar (no custom 28px bar — intentional gap, see § Window).

Tokens: `docs/design-tokens.json`. This doc maps each mockup selector to Rust location and notes accepted diffs.

## 1. Global

| Mockup | Value | Rust | Status |
|--------|-------|------|--------|
| `:root --accent` | `#3B82F6` | `src/app.rs:150 NEKO_ACCENT` | ✓ |
| `--accent-hover` | `#4A90FF` | `src/app.rs:151 NEKO_ACCENT_HOVER` | ✓ |
| `--green / --red` | `#27AE60 / #E74C3C` | `src/app.rs:152-153` | ✓ |
| `--r` | `6px` | `src/app.rs:157 R6` | ✓ |
| `* font-weight 500` | `500 !important` | `src/app.rs:160-196 Inter/JetBrainsMono` — egui renders ~400 (static TTF) | accepted diff |
| `body bg #0e0e10` | page bg | native window, `panel_fill_for(Dark) #1E1E1E` | accepted |

## 2. Window & layout (fixed 840×560)

```
mockup .window 840×560 bg #1E1E1E border #3C3F41 R8 shadow 0 20px 60px
code    src/main.rs:18 with_inner_size/min/max 840×560 resizable false — panel_fill/border same
gap     .main gap 12 pad 12 → src/app.rs:1125 Margin::same(12), gap 12.0
columns .left 486 / .right 318 → src/app.rs:1130-1131 left_w 486 (=840-2*12-12-318, egui no flex-shrink) / right_w 318 ✓
       mockup .left 498 shrinks to ~490 via flex; egui 486 fits without overflow (was 498+12+318=828>816)
```

Accepted: `R8` on native window not set (OS decoration), shadow is OS native.

## 3. Colors by theme

| Theme | panel | card | border | text | weak | hover |
|-------|-------|------|--------|------|------|-------|
| Dark (default) | `#1E1E1E` | `#2D2F33` | `#3C3F41` | `#FFF` | `#9AA0A6` | `#3A3D45` |
| Light | `#F0F2F5` | `#FFF` | `#D0D3D8` | `#000` | `#6B7280` | `#E8EAED` |

Code: `src/app.rs:198-284 card_fill_for/panel_fill_for/border_color_for/theme_visuals`.
OLED removed (user decision 2026-09-02) — only Light/Dark remain. Mockup only defines dark + `.theme-light` overrides.

Delay colors theme-aware: `src/app.rs:3459 delay_color(theme)` — dark `#50B478/#C8B45A/#E06060`, light `#0a7a3a/#8a6d00/#c0392b`, na `#E74C3C`.

## 4. Components — mockup → Rust

| Component | Mockup | Rust | Note |
|-----------|--------|------|------|
| `.card` | `#2D2F33 / #3C3F41 R6` | `card_fill_for + border_color_for, R6` | ✓ |
| `.btn h22 R6` | `22px 0 10px R6 11px gap6` | `src/app.rs:1180/1192 Button [*,22] R6` | ✓ |
| `.btn:hover` | `#3A3D45 / #E8EAED(light)` | `v.widgets.hovered.bg_fill` — fixed `#E8EAED` light | ✓ |
| `.btn-accent` | `accent/white` | `v.widgets.active = NEKO_ACCENT` | ✓ |
| `.btn-big/master` | `32px 12px green/red` | `src/app.rs:1465 [right_w-12,32] R6` | ✓, text `ВЫКЛ/ВКЛ` |
| `.combo/input h22 R6` | `22px R6 0 8px 11px` | `ComboBox/TextEdit + widget visuals R6` | ✓ (height via visuals) |
| `.proxy-row h28 R4` | `28px R4 0 4px gap6` | `src/app.rs:1261 Frame R4 28.0` | ✓ |
| `.proxy-row.sel` | `rgba(59,130,246,.14) + accent` | `from_rgba(59,130,246,36) + NEKO_ACCENT` | ✓ alpha 36≈0.14 |
| `.radio 12 dot6` | `12px #6B6F76 / accent 6px` | `src/app.rs:1277-1282` | ✓ |
| `.pill 10 mono` | forced `#fff (dark)/#000 (light)` | removed `.weak()` → default text | ✓ fixed |
| `.delay mono 11` | theme variants | `delay_color(theme)` + `NEKO_RED` | ✓ fixed |
| `.graph 114 R6 pad6` | `114px R6 6px grid #3C3F41 0.8 / lines 1.6 up #3B82F6 down #10B981` | `src/app.rs:1338-1369 graph_h 114, border grid, painter 1.6` | ✓ fixed grid gray70→border |
| `.stat 34` | `34px 0 8px labels white/black values mono 11 700` | `src/app.rs:1390-1408 allocate 34, label size10 (no weak), value mono 11` | ✓ fixed weak→default; label `приём` (ё) |
| `.mode-btn 26 R6` | `26px R6 #3C3F41/#2D2F33 / sel accent` | `src/app.rs:1414-1430 [bw,26] fill accent` | ✓ |
| `.sheet` full-window | `840×560 no padding, all overlays: config/rawKeys/editor/logs/settings` | `src/app.rs:1501-2730 all 840×560, top 828, content 824, max 510` | ✓ fixed (was 760/560/800/640); без полей per user |
| `.sheet-head 36` | `36px 0 8px #2D2F33/#3C3F41` | `top_frame fill card, separator` | ✓ (height implicit) |
| `.tabs gap6 h22 R6` | `22px 0 10px R6 gap6` | `src/app.rs settings/logs tabs` | ✓ |
| `.add-menu 160 R6` | `min160 R6 #2D2F33/#3C3F41 btn 28 R6` | `src/app.rs:3253-3282` | ✓ |
| `#updateBanner` | `#E74C3C rgba(.08)` | `src/app.rs:1486-1513 rgba 20` | ✓ |

## 5. Typography specifics

* Mockup forces all text white (dark) / black (light) via `.window .muted/.weak/.pill/.stat span` `!important`, but inline styles for `График #9AA0A6`, delays, flags override.
* Rust: stat labels/pills/count/legend use default text (white/black) to match forced mockup; `График`/`Ожидание данных` keep `.weak()` (`#9AA0A6/#6B7280`).
* Texts: `приём` (ё) in mockup, `Отключить/Включить` via master button, `Все группы`, `◎ Пинг`.

## 6. Accepted diffs (not fixed)

* Font weight 500 — Inter static TTF renders ~400; need variable font for exact weight.
* Window `R8` + `shadow 0 20px 60px` — native OS only.
* `.toast` + `.add-menu` shadow — no egui shadow equivalent.
* `titlebar 28px` — native OS titlebar by design (intentional, no custom header).
* Sheets full-window 840×560 without padding (user: вообще без полей) — overlays fill window, no outer margin.
* Main overflow 498+12+318=828>816 → left 486 fix.
* proxy/tun independent of master (master OFF = desire only; ON applies current desires, no forced both-on, no at-least-one guard).

## 7. Diff workflow (tokens+diff)

1. `cargo run` vs `mockup/index.html` opened at `840×560` (or headless `chromium --window-size=840,560`).
2. Screenshot both → `python scripts/pixel_diff.py mockup.png app.png` → `diff.png` + `% mismatched` (threshold 0.1%).
3. Tokens validated via `python scripts/verify_tokens.py` (checks `design-tokens.json` vs `src/app.rs` constants).

## 8. Files

* `mockup/index.html` — эталон
* `docs/design-tokens.json` — токены
* `docs/design-spec.md` — этот файл
* `scripts/verify_tokens.py` — токен-валидация
* `scripts/pixel_diff.py` — pixel diff
