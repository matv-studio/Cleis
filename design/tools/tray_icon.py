"""Draws Cleis's tray icon (the logo: key on a light rounded square) as raw 32x32 RGBA.

Output: crates/cleis-app/assets/tray-32.rgba (+ .png preview). Run: python3 design/tools/tray_icon.py
"""
import pathlib
from PIL import Image, ImageDraw

S = 32
K = 8  # supersampling
root = pathlib.Path(__file__).resolve().parents[2]
img = Image.new("RGBA", (S * K, S * K), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
d.rounded_rectangle([K, K, (S - 1) * K, (S - 1) * K], radius=7 * K, fill=(236, 236, 236, 255))
ink = (24, 24, 24, 255)
u = (S - 2 * 5) / 16  # key.svg is 16x16, drawn in a 22 px box centred
ox = oy = 5
w = int(1.9 * u * K)
def p(x, y):
    return ((ox + x * u) * K, (oy + y * u) * K)
cx, cy = p(5.5, 10.5)
r = 3 * u * K
d.ellipse([cx - r, cy - r, cx + r, cy + r], outline=ink, width=w)
for a, b in [((7.6, 8.4), (13.5, 2.5)), ((11.5, 4.5), (13, 6)), ((10, 6), (11.2, 7.2))]:
    d.line([p(*a), p(*b)], fill=ink, width=w)
    for q in (a, b):
        x, y = p(*q)
        d.ellipse([x - w / 2, y - w / 2, x + w / 2, y + w / 2], fill=ink)
small = img.resize((S, S), Image.LANCZOS)
out = root / "crates/cleis-app/assets"
small.save(out / "tray-32.png")
(out / "tray-32.rgba").write_bytes(small.tobytes())
print("ok", len(small.tobytes()))
