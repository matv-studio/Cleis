"""Regenerates crates/cleis-app/assets/icons/*.svg and src/icons_data.rs from the mockup icon set."""
import pathlib, re

root = pathlib.Path(__file__).resolve().parents[2]
src = (root / 'design/maquettes/js/icons.js').read_text()
body = src[src.index('const P = {') + 11:src.index('};')]
items = re.findall(r"^\s*([a-zA-Z0-9_]+):\s*'(.*)',\s*$", body, re.M)
out = root / 'crates/cleis-app/assets/icons'
out.mkdir(parents=True, exist_ok=True)
rs = '//! Icons drawn for Cleis (16 × 16, stroke 1.5), generated from design/maquettes/js/icons.js\n//! by design/tools/icons.py. Do not edit by hand.\n\npub const ICONS: &[(&str, &[u8])] = &[\n'
for n, p in items:
    note = '<!-- Path from Lucide "settings" (ISC licence, https://lucide.dev), see THIRD_PARTY.md -->\n' if n == 'gear' else ''
    (out / f'{n}.svg').write_text(f'{note}<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="#000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">{p}</svg>\n')
    rs += f'    ("icons/{n}.svg", include_bytes!("../assets/icons/{n}.svg")),\n'
rs += '];\n'
(root / 'crates/cleis-app/src/icons_data.rs').write_text(rs)
print(len(items), 'icons')
