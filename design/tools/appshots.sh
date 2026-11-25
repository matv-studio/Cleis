#!/bin/bash
# Captures the real app (demo data) under a virtual X display, dark theme (user's choice,
# 25/11/2026; pass THEMES="dark light" for both).
# Needs: Xvfb, ImageMagick `import`, mesa-vulkan-drivers (llvmpipe). Output: target/app-captures/.
# ONLY="vault-dark members-dark" takes only those. A capture waits until the window is drawn
# (not a fixed time); an app that stops or never draws fails the run (audit X-5).
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
cargo build -q -p cleis-app --manifest-path "$root/Cargo.toml"
out="$root/target/app-captures"; mkdir -p "$out"
failed=()
shot() { # name, args...
  local name=$1; shift
  if [ -n "$ONLY" ] && [[ " $ONLY " != *" $name "* ]]; then return; fi
  if ! xvfb-run -a -s "-screen 0 1440x900x24" bash -c "
      '$root/target/debug/cleis' $* >/dev/null 2>&1 & pid=\$!
      drawn=0
      for i in \$(seq 1 60); do
        sleep 0.5
        kill -0 \$pid 2>/dev/null || exit 3
        import -window root /tmp/appshot-probe.png 2>/dev/null || continue
        sd=\$(convert /tmp/appshot-probe.png -format '%[fx:standard_deviation>0.02]' info: 2>/dev/null)
        if [ \"\$sd\" = 1 ]; then drawn=\$((drawn+1)); else drawn=0; fi
        [ \$drawn -ge 3 ] && break
      done
      [ \$drawn -ge 3 ] || { kill \$pid; exit 4; }
      sleep 1.5
      import -window root '$out/$name.png'
      kill \$pid"; then
    failed+=("$name")
  fi
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
  shot "create-elsewhere-$theme" --demo --no-org --unlocked --tab settings --page join --menu create-elsewhere --theme $theme
  shot "server-here-$theme" --demo --unlocked --tab settings --page server-here --theme $theme
  shot "org-backups-$theme" --demo --unlocked --tab settings --page org-backups --theme $theme
  shot "activity-$theme" --demo --unlocked --tab activity --theme $theme
  shot "new-space-$theme" --demo --unlocked --menu new-space --theme $theme
  shot "server-code-$theme" --demo --unlocked --menu server-code --theme $theme
  shot "super-admin-$theme" --demo --unlocked --tab settings --page members --menu super-admin --theme $theme
  shot "profile-$theme" --demo --unlocked --tab settings --page profile --theme $theme
  shot "devices-$theme" --demo --unlocked --tab settings --page devices --theme $theme
  shot "spaces-$theme" --demo --unlocked --tab settings --page spaces --theme $theme
  shot "rename-space-$theme" --demo --unlocked --tab settings --page spaces --menu rename-space --theme $theme
  shot "new-folder-$theme" --demo --unlocked --menu new-folder --theme $theme
  shot "folder-menu-$theme" --demo --unlocked --menu folder-menu --theme $theme
  shot "delete-folder-$theme" --demo --unlocked --menu delete-folder --theme $theme
  shot "delete-space-$theme" --demo --unlocked --tab settings --page spaces --menu delete-space --theme $theme
  shot "ctx-entry-$theme" --demo --unlocked --menu ctx-entry --theme $theme
  shot "ctx-space-$theme" --demo --unlocked --menu ctx-space --theme $theme
  shot "ctx-contact-$theme" --demo --unlocked --tab contacts --menu ctx-contact --theme $theme
  shot "ctx-member-$theme" --demo --unlocked --tab settings --page members --menu ctx-member --theme $theme
  shot "confirm-transfer-$theme" --demo --unlocked --tab settings --page members --menu confirm-transfer --theme $theme
  shot "vault-tabs-org-$theme" --demo --unlocked --two-orgs --theme $theme
  shot "vault-tabs-personal-$theme" --demo --unlocked --two-orgs --vault-tab personal --theme $theme
  shot "vault-tabs-settings-$theme" --demo --unlocked --two-orgs --tab settings --theme $theme
  shot "health-vault-$theme" --demo --unlocked --two-orgs --tab health --menu health-vault --theme $theme
  shot "confirm-remove-$theme" --demo --unlocked --tab settings --page members --menu confirm-remove --theme $theme
done
echo "captures in $out"
if [ ${#failed[@]} -gt 0 ]; then
  echo "failed: ${failed[*]}" >&2
  exit 1
fi
