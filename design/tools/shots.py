import sys, subprocess, time
import pathlib
from playwright.sync_api import sync_playwright
srv = subprocess.Popen(['python3','-m','http.server','8180'], cwd=str(pathlib.Path(__file__).resolve().parents[1] / 'maquettes'), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(0.8)
names = sys.argv[1].split(',') if len(sys.argv) > 1 and sys.argv[1] != 'all' else None
themes = sys.argv[2].split(',') if len(sys.argv) > 2 else ['dark','light']
import re, pathlib
src = (pathlib.Path(__file__).resolve().parents[1] / 'maquettes/js/screens.js').read_text()
allnames = re.findall(r"'(\d\d-[a-z0-9-]+)':", src)
names = names or allnames
out = pathlib.Path(__file__).resolve().parents[2] / 'target/captures'; out.mkdir(parents=True, exist_ok=True)
with sync_playwright() as p:
    b = p.chromium.launch()
    pg = b.new_page(viewport={'width':1440,'height':900}, device_scale_factor=1)
    errs = []
    pg.on('console', lambda m: errs.append(m.text) if m.type=='error' else None)
    pg.on('pageerror', lambda e: errs.append(str(e)))
    for n in names:
        for t in themes:
            pg.goto(f'http://127.0.0.1:8180/index.html?s={n}&theme={t}')
            pg.wait_for_selector('body[data-ready="1"]')
            pg.evaluate('document.fonts.ready')
            pg.screenshot(path=str(out/f'{n}-{t}.png'))
    print('errors:', errs[:5])
    b.close()
srv.terminate()
