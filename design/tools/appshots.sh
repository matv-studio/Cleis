#!/bin/bash
# Captures the real app (demo data) under a virtual X display, dark theme (user's choice,
# 25/11/2026; pass THEMES="dark light" for both).
# Needs: Xvfb, ImageMagick `import`, mesa-vulkan-drivers (llvmpipe). Output: target/app-captures/.
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
cargo build -q -p cleis-app --manifest-path "$root/Cargo.toml"
out="$root/target/app-captures"; mkdir -p "$out"
shot() { # name, args...
  local name=$1; shift
  xvfb-run -a -s "-screen 0 1440x900x24" bash -c "'$root/target/debug/cleis' $* >/dev/null 2>&1 & pid=\$!; sleep 9; import -window root '$out/$name.png'; kill \$pid"
}
for theme in ${THEMES:-dark}; do
  shot "unlock-$theme" --demo --theme $theme
  shot "vault-$theme" --demo --unlocked --theme $theme
  shot "filter-$theme" --demo --unlocked --menu filter --theme $theme
  shot "columns-$theme" --demo --unlocked --menu new-column --theme $theme
  shot "import-$theme" --demo --unlocked --tab settings --page import --import "$root/design/tools/notion-export.csv" --theme $theme
  shot "import-review-$theme" --demo --unlocked --tab settings --page import --import "$root/design/tools/notion-export.csv" --review --theme $theme
  shot "contacts-$theme" --demo --unlocked --tab contacts --theme $theme
  shot "merge-$theme" --demo --unlocked --tab contacts --page merge --theme $theme
  shot "health-$theme" --demo --unlocked --tab health --theme $theme
  shot "preferences-$theme" --demo --unlocked --tab settings --theme $theme
  shot "security-$theme" --demo --unlocked --tab settings --page security --theme $theme
  shot "open-code-$theme" --demo --unlocked --tab settings --page security --menu open-code --theme $theme
  shot "autofill-$theme" --demo --unlocked --tab settings --page autofill --theme $theme
  shot "quick-access-$theme" --demo --unlocked --quick-access --theme $theme
  shot "sync-$theme" --demo --unlocked --tab settings --page sync --theme $theme
  shot "restore-$theme" --vault "/tmp/cleis-none-$RANDOM/vault.cleis" --restore --theme $theme
  shot "welcome-$theme" --vault "/tmp/cleis-none-$RANDOM/vault.cleis" --theme $theme
  shot "welcome-org-$theme" --vault "/tmp/cleis-none-$RANDOM/vault.cleis" --org-welcome --theme $theme
  shot "members-$theme" --demo --unlocked --tab settings --page members --theme $theme
  shot "invite-$theme" --demo --unlocked --tab settings --page members --menu invite --theme $theme
  shot "invite-code-$theme" --demo --unlocked --tab settings --page members --menu invite-code --theme $theme
  shot "new-role-$theme" --demo --unlocked --tab settings --page members --menu new-role --theme $theme
  shot "policy-$theme" --demo --unlocked --tab settings --page policy --theme $theme
  shot "encryption-$theme" --demo --unlocked --tab settings --page encryption --theme $theme
  shot "rekey-$theme" --demo --unlocked --tab settings --page encryption --menu rekey --theme $theme
  shot "server-$theme" --demo --unlocked --tab settings --page server --theme $theme
  shot "join-$theme" --demo --no-org --unlocked --tab settings --page join --theme $theme
  shot "org-backups-$theme" --demo --unlocked --tab settings --page org-backups --theme $theme
  shot "activity-$theme" --demo --unlocked --tab activity --theme $theme
  shot "new-space-$theme" --demo --unlocked --menu new-space --theme $theme
  shot "server-code-$theme" --demo --unlocked --menu server-code --theme $theme
  shot "super-admin-$theme" --demo --unlocked --tab settings --page members --menu super-admin --theme $theme
  shot "profile-$theme" --demo --unlocked --tab settings --page profile --theme $theme
  shot "devices-$theme" --demo --unlocked --tab settings --page devices --theme $theme
  shot "spaces-$theme" --demo --unlocked --tab settings --page spaces --theme $theme
  shot "rename-space-$theme" --demo --unlocked --tab settings --page spaces --menu rename-space --theme $theme
  shot "delete-space-$theme" --demo --unlocked --tab settings --page spaces --menu delete-space --theme $theme
done
echo "captures in $out"
