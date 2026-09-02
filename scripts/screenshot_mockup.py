#!/usr/bin/env python3
"""Render mockup/index.html headless at 840x560 and save PNG.
Prefers Playwright if available, falls back to Chrome --headless.

Usage:
  python scripts/screenshot_mockup.py --out mockup_dark.png [--theme light]
Requires: pip install playwright && playwright install chromium  (or Chrome in PATH)
"""
import argparse, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
HTML = ROOT / "mockup/index.html"

def via_playwright(out, theme):
    from playwright.sync_api import sync_playwright
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 900, "height": 700})
        page.goto(f"file://{HTML}")
        if theme == "light":
            page.evaluate("()=> setTheme('light')")
        page.wait_for_timeout(400)
        # Clip to window element 840x560 centered
        loc = page.locator(".window")
        loc.screenshot(path=str(out))
        browser.close()
        print(f"Saved {out} via Playwright")

def via_chrome(out, theme):
    # Use Chrome headless --screenshot (requires chrome/chromium in PATH)
    # We render full page and rely on mockup being centered; crop optional.
    chrome = None
    for cand in ["chrome", "google-chrome", "chromium", "chromium-browser", r"C:\Program Files\Google\Chrome\Application\chrome.exe"]:
        try:
            subprocess.run([cand, "--version"], capture_output=True, timeout=5)
            chrome = cand; break
        except: continue
    if not chrome:
        print("Chrome not found and Playwright not installed. Install Playwright: pip install playwright && playwright install chromium")
        sys.exit(2)
    import tempfile
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td) / "shot.png"
        cmd = [chrome, "--headless", "--disable-gpu", f"--window-size=900,700", f"--screenshot={tmp}", f"file://{HTML}"]
        print(" ".join(cmd))
        subprocess.run(cmd, check=True, timeout=20)
        # Crop center 840x560 if needed — here just move
        tmp.replace(out)
        print(f"Saved {out} via Chrome (full viewport — crop to .window manually if needed)")

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(ROOT / "mockup_dark.png"))
    ap.add_argument("--theme", choices=["dark","light"], default="dark")
    args = ap.parse_args()
    out = Path(args.out)
    try:
        via_playwright(out, args.theme)
    except ImportError:
        via_chrome(out, args.theme)
    except Exception as e:
        print(f"Playwright failed: {e} — trying Chrome")
        try: via_chrome(out, args.theme)
        except Exception as e2:
            print(f"Chrome failed: {e2}"); sys.exit(2)

if __name__ == "__main__":
    main()
