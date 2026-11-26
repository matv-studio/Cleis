#!/bin/bash
# Regenerates the app's logo files from design/logo/*.svg (needs rsvg-convert, ImageMagick, Pillow).
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
tmp=$(mktemp -d)
for v in light dark; do
  rsvg-convert -w 128 -h 128 "$root/design/logo/cleis-$v.svg" -o "$root/crates/cleis-app/assets/logo-$v.png"
done
sizes="16 24 32 48 64 128 256"
for s in $sizes; do rsvg-convert -w $s -h $s "$root/design/logo/cleis-light.svg" -o "$tmp/$s.png"; done
convert $(for s in $sizes; do echo "$tmp/$s.png"; done) "$root/packaging/windows/cleis.ico"
cp "$tmp/32.png" "$root/crates/cleis-app/assets/tray-32.png"
python3 -c "
from PIL import Image
im = Image.open('$tmp/32.png').convert('RGBA')
open('$root/crates/cleis-app/assets/tray-32.rgba', 'wb').write(im.tobytes())"
# QNAP App Center icons (D-78): 64 px, 80 px and a grey 64 px for "disabled", as GIF.
rsvg-convert -w 64 -h 64 "$root/design/logo/cleis-light.svg" -o "$tmp/q64.png"
rsvg-convert -w 80 -h 80 "$root/design/logo/cleis-light.svg" -o "$tmp/q80.png"
convert "$tmp/q64.png" "$root/packaging/qnap/icons/CleisServer.gif"
convert "$tmp/q80.png" "$root/packaging/qnap/icons/CleisServer_80.gif"
convert "$tmp/q64.png" -colorspace Gray "$root/packaging/qnap/icons/CleisServer_gray.gif"
rm -rf "$tmp"
