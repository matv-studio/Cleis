#!/usr/bin/env python3
"""Screenshots of the mockups (Playwright Chromium) into target/captures/.

Usage: python3 design/tools/shots.py [all|name1,name2] [dark|dark,light]
Dark theme only by default (user's choice, 25/11/2026). A free port is used for the local page
server; any page error makes the script end with status 1 (audit X-6).
"""
import pathlib
import re
import socket
import subprocess
import sys
import time

from playwright.sync_api import sync_playwright

root = pathlib.Path(__file__).resolve().parents[1]
with socket.socket() as s:
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
srv = subprocess.Popen(
    [sys.executable, "-m", "http.server", str(port), "--bind", "127.0.0.1"],
    cwd=str(root / "maquettes"),
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
)
time.sleep(0.8)
names = sys.argv[1].split(",") if len(sys.argv) > 1 and sys.argv[1] != "all" else None
themes = sys.argv[2].split(",") if len(sys.argv) > 2 else ["dark"]
src = (root / "maquettes/js/screens.js").read_text()
allnames = re.findall(r"'(\d\d[a-z]?-[a-z0-9-]+)':", src)
names = names or allnames
unknown = [n for n in names if n not in allnames]
names = [n for n in names if n in allnames]
out = root.parent / "target/captures"
out.mkdir(parents=True, exist_ok=True)
errs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        pg = b.new_page(viewport={"width": 1440, "height": 900}, device_scale_factor=1)
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" else None)
        pg.on("pageerror", lambda e: errs.append(str(e)))
        for n in names:
            for t in themes:
                pg.goto(f"http://127.0.0.1:{port}/index.html?s={n}&theme={t}")
                pg.wait_for_selector('body[data-ready="1"]')
                pg.evaluate("document.fonts.ready")
                pg.screenshot(path=str(out / f"{n}-{t}.png"))
        b.close()
finally:
    srv.terminate()
if unknown:
    errs.append(f"unknown screens: {', '.join(unknown)}")
print("errors:", errs[:5])
sys.exit(1 if errs else 0)
