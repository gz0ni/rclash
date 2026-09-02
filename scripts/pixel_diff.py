#!/usr/bin/env python3
"""Pixel diff of two PNGs (mockup vs app screenshot). Requires Pillow: pip install Pillow.

Usage:
  python scripts/pixel_diff.py mockup.png app.png --out diff.png --threshold 0.1

Outputs diff image (red where differs, transparent where same) and % mismatched.
Threshold: % mismatched above which exit 1 (CI fail). Default 0.1%.
Ignores anti-aliasing tolerance: per-channel delta > tol (default 12) counts as diff.
"""
import argparse
from pathlib import Path

try:
    from PIL import Image
except ImportError:
    print("Pillow not installed: pip install Pillow")
    raise SystemExit(2)

def pixel_diff(a_path, b_path, out_path, tol=12):
    a = Image.open(a_path).convert("RGBA")
    b = Image.open(b_path).convert("RGBA")
    if a.size != b.size:
        # Crop to smallest common area for comparison
        w = min(a.size[0], b.size[0]); h = min(a.size[1], b.size[1])
        print(f"Size mismatch {a.size} vs {b.size} — cropping to {w}x{h}")
        a = a.crop((0,0,w,h)); b = b.crop((0,0,w,h))
    w,h = a.size
    diff = Image.new("RGBA", (w,h), (0,0,0,0))
    pa, pb, pd = a.load(), b.load(), diff.load()
    mismatched = 0
    for y in range(h):
        for x in range(w):
            ar,ag,ab,aa = pa[x,y]; br,bg,bb,ba = pb[x,y]
            dr = abs(ar-br); dg = abs(ag-bg); db = abs(ab-bb); da = abs(aa-ba)
            if dr > tol or dg > tol or db > tol or da > tol:
                mismatched += 1
                pd[x,y] = (231, 76, 60, 180)
    pct = mismatched / (w*h) * 100
    diff.save(out_path)
    print(f"Pixels: {w*h}, mismatched: {mismatched} ({pct:.3f}%)")
    print(f"Diff saved: {out_path}")
    return pct

def main():
    p = argparse.ArgumentParser(description="Pixel diff")
    p.add_argument("a", help="mockup screenshot PNG")
    p.add_argument("b", help="app screenshot PNG")
    p.add_argument("--out", default="diff.png", help="output diff PNG")
    p.add_argument("--threshold", type=float, default=0.1, help="fail threshold %% (default 0.1)")
    p.add_argument("--tol", type=int, default=12, help="per-channel tolerance 0-255 (default 12 for AA)")
    args = p.parse_args()
    pct = pixel_diff(args.a, args.b, args.out, tol=args.tol)
    if pct > args.threshold:
        print(f"FAIL: {pct:.3f}% > threshold {args.threshold}%")
        raise SystemExit(1)
    print(f"OK: {pct:.3f}% <= threshold {args.threshold}%")

if __name__ == "__main__":
    main()
