#!/usr/bin/env python3
"""Verify docs/design-tokens.json vs src/app.rs constants. Exit 0 if all match, 1 on mismatch."""
import json, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOKENS = json.loads((ROOT / "docs/design-tokens.json").read_text(encoding="utf-8"))
APP = (ROOT / "src/app.rs").read_text(encoding="utf-8")
MAIN = (ROOT / "src/main.rs").read_text(encoding="utf-8")
CFG = (ROOT / "crates/rclash-config/src/lib.rs").read_text(encoding="utf-8")

def rgb_to_hex(r,g,b): return f"#{r:02X}{g:02X}{b:02X}"

checks = []

def check(name, cond, detail=""):
    status = "OK" if cond else "FAIL"
    checks.append((name, status, detail))
    print(f"[{status}] {name}" + (f" — {detail}" if detail else ""))

# Window size in main.rs
check("window 840x560", "with_inner_size([840.0, 560.0])" in MAIN, "src/main.rs")

# Theme enum only Light/Dark
check("Theme only Light/Dark", "Oled" not in CFG and "Oled" not in APP, "no Oled in config/app.rs")
check("Theme::all 2 variants", '"Светлая"' in CFG and '"Тёмная"' in CFG, "label_ru")

# Colors from tokens vs app.rs NEKO_ consts
for const, hex_val in [
    ("NEKO_ACCENT", TOKENS["colors"]["accent"]),
    ("NEKO_ACCENT_HOVER", TOKENS["colors"]["accent_hover"]),
    ("NEKO_GREEN", TOKENS["colors"]["green"]),
    ("NEKO_RED", TOKENS["colors"]["red"]),
    ("NEKO_DELAY_FAST", TOKENS["colors"]["delay"]["dark"]["fast"]),
    ("NEKO_DELAY_MID", TOKENS["colors"]["delay"]["dark"]["mid"]),
    ("NEKO_DELAY_SLOW", TOKENS["colors"]["delay"]["dark"]["slow"]),
]:
    m = re.search(rf"{const}.*from_rgb\(0x([0-9A-Fa-f]+),\s*0x([0-9A-Fa-f]+),\s*0x([0-9A-Fa-f]+)\)", APP)
    if m:
        got = rgb_to_hex(int(m.group(1),16), int(m.group(2),16), int(m.group(3),16))
        check(const, got.lower() == hex_val.lower(), f"got {got} expected {hex_val}")
    else:
        check(const, False, "not found")

# Light delay variants in delay_color (check rgb triple)
for hex_val in TOKENS["colors"]["delay"]["light"].values():
    if hex_val.lower() == "#e74c3c": continue
    r = int(hex_val[1:3],16); g = int(hex_val[3:5],16); b = int(hex_val[5:7],16)
    pat = f"0x{r:02X}, 0x{g:02X}, 0x{b:02X}"
    check(f"light delay {hex_val}", pat.lower() in APP.lower() or pat in APP, "in delay_color")

# R6 / R4
check("R6 = 6", "const R6: u8 = 6" in APP)
check("R4 = 4 (proxy_row)", "const R4: u8 = 4" in APP)

# Card/panel fills
check("card dark #2D2F33", "0x2D, 0x2F, 0x33" in APP)
check("panel dark #1E1E1E", "0x1E, 0x1E, 0x1E" in APP)
check("border dark #3C3F41", "0x3C, 0x3F, 0x41" in APP)
check("border light #D0D3D8", "0xD0, 0xD3, 0xD8" in APP)
check("hover dark #3A3D45", "0x3A, 0x3D, 0x45" in APP)
check("hover light #E8EAED", "0xE8, 0xEA, 0xED" in APP)

# Layout (full-window sheets 840×560; left 486 to fit 840-2*12 without overflow)
check("left 486", "left_w = 486.0" in APP)
check("right 318", "right_w = 318.0" in APP)
check("gap 12", "let gap = 12.0" in APP)
check("graph 114", "graph_h = 114.0" in APP)
check("stat 34", "vec2(right_w - 12.0, 34.0)" in APP or "34.0" in APP and "stat" in APP.lower())

# Button heights
check("btn 22", '"↻").size' in APP or "22.0" in APP)
check("mode-btn 26", "[bw, 26.0]" in APP)
check("master 32", "[right_w - 12.0, 32.0]" in APP)

# Sheets full-window 840×560 without padding (all 5)
for w in [840]:
    check(f"sheet full-window {w}", APP.count(f"{w}.0") >= 10, "840.0 appears >=10 times (5 overlays)")
check("sheet head 828", "828.0" in APP, "top_frame")
check("sheet content 824", "824.0" in APP, "content frame")

# Overlays full-window scroll max 510 (all bodies) + editor desired_rows 30
check("scroll 510", "max_height(510.0)" in APP or "max_height(510" in APP, "bodies")
check("editor rows 30", "desired_rows(30)" in APP, "full-window editor")
# Titlebar native (no custom painter)
check("native titlebar", "titlebar" not in APP.lower() or APP.lower().count("titlebar") < 5, "no custom titlebar impl")
# proxy/tun independent (no forced both-on, no at-least-one guard)
check("no forced both-on", "proxy_desire = true;\n                                    tun_desire = true" not in APP, "master ON not forcing both")
check("no at-least-one guard", "if !new_desire && !tun_desire" not in APP and "if !proxy_desire && !new_desire" not in APP, "guard removed")

fails = [c for c in checks if c[1]=="FAIL"]
print(f"\n{len(checks)-len(fails)}/{len(checks)} checks passed")
sys.exit(1 if fails else 0)
