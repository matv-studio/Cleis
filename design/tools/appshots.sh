#!/bin/bash
# Captures the real app (demo data) under a virtual X display, both themes.
# Needs: Xvfb, ImageMagick `import`, mesa-vulkan-drivers (llvmpipe). Output: target/app-captures/.
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
cargo build -q -p cleis-app --manifest-path "$root/Cargo.toml"
out="$root/target/app-captures"; mkdir -p "$out"
shot() { # name, args...
  local name=$1; shift
  xvfb-run -a -s "-screen 0 1440x900x24" bash -c "'$root/target/debug/cleis' $* >/dev/null 2>&1 & pid=\$!; sleep 9; import -window root '$out/$name.png'; kill \$pid"
}
for theme in dark light; do
  shot "unlock-$theme" --demo --theme $theme
  shot "vault-$theme" --demo --unlocked --theme $theme
  shot "columns-$theme" --demo --unlocked --menu new-column --theme $theme
  shot "import-$theme" --demo --unlocked --tab settings --page import --import "$root/design/tools/notion-export.csv" --theme $theme
  shot "import-review-$theme" --demo --unlocked --tab settings --page import --import "$root/design/tools/notion-export.csv" --review --theme $theme
  shot "contacts-$theme" --demo --unlocked --tab contacts --theme $theme
  shot "health-$theme" --demo --unlocked --tab health --theme $theme
  shot "preferences-$theme" --demo --unlocked --tab settings --theme $theme
  shot "autofill-$theme" --demo --unlocked --tab settings --page autofill --theme $theme
  shot "quick-access-$theme" --demo --unlocked --quick-access --theme $theme
  shot "welcome-$theme" --vault "/tmp/cleis-none-$RANDOM/vault.cleis" --theme $theme
done
echo "captures in $out"
