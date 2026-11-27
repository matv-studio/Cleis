#!/bin/bash
# Captures the real app (demo data) under a virtual X display, dark theme (user's choice,
# 25/11/2026; pass THEMES="dark light" for both).
# Needs: Xvfb, ImageMagick `import`, mesa-vulkan-drivers (llvmpipe). Output: target/app-captures/.
# ONLY="vault-dark members-dark" takes only those. UI_LANG=fr: French interface, into
# target/app-captures/fr/. A capture waits until the window is drawn
# (not a fixed time); an app that stops or never draws fails the run (audit X-5).
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
cargo build -q -p cleis-app --manifest-path "$root/Cargo.toml"
out="$root/target/app-captures${UI_LANG:+/$UI_LANG}"; mkdir -p "$out"
lang=${UI_LANG:+--lang $UI_LANG}
failed=()
shot() { # name, args...
  local name=$1; shift
  if [ -n "$ONLY" ] && [[ " $ONLY " != *" $name "* ]]; then return; fi
  if ! xvfb-run -a -s "-screen 0 1440x900x24" bash -c "
      '$root/target/debug/cleis' $* $lang >/dev/null 2>&1 & pid=\$!
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
  shot "unlock-hello-$theme" --demo --unlock-with hello --theme $theme
  # 27/11/2026 (mockups 37, 38): the new screens.
  shot "unlock-pin-$theme" --demo --unlock-with pin --theme $theme
  shot "folder-new-$theme" --demo --unlocked --menu new-folder --theme $theme
  shot "folder-rename-$theme" --demo --unlocked --menu rename-folder --theme $theme
  shot "pin-setup-$theme" --demo --unlocked --tab settings --page security --menu pin-setup --theme $theme
  shot "erase-account-$theme" --demo --unlocked --tab settings --page sync --menu erase-account --theme $theme
  shot "github-$theme" --demo --unlocked --tab settings --page account --menu github --theme $theme
  shot "github-create-$theme" --demo --unlocked --tab settings --page account --menu github-create --theme $theme
  shot "github-install-$theme" --demo --unlocked --tab settings --page account --menu github-install --theme $theme
  shot "account-menu-$theme" --demo --unlocked --menu account --theme $theme
  shot "profile-menu-$theme" --demo --unlocked --menu profile-menu --theme $theme
  shot "trash-menu-$theme" --demo --unlocked --menu ctx-trash --theme $theme
  shot "recovery-keys-$theme" --demo --unlocked --menu recovery --theme $theme
  shot "profile-edit-$theme" --demo --unlocked --menu profile-edit --theme $theme
  shot "website-column-$theme" --demo --unlocked --menu website --theme $theme
  shot "space-members-$theme" --demo --unlocked --tab settings --page spaces --menu space-members --theme $theme
  shot "account-page-$theme" --demo --unlocked --tab settings --page account --theme $theme
  # 27/11/2026 (D-104): Microsoft and Google sync.
  shot "account-pick-microsoft-$theme" --demo --unlocked --tab settings --page account --menu pick-microsoft --theme $theme
  shot "cloud-microsoft-$theme" --demo --unlocked --tab settings --page account --menu cloud-microsoft --theme $theme
  shot "cloud-google-$theme" --demo --unlocked --tab settings --page account --menu cloud-google --theme $theme
  shot "account-onedrive-$theme" --demo --unlocked --tab settings --page account --menu onedrive --theme $theme
  shot "account-gdrive-$theme" --demo --unlocked --tab settings --page account --menu gdrive --theme $theme
  shot "welcome-signin-$theme" --vault "/tmp/cleis-none-$RANDOM/vault.cleis" --welcome-signin --theme $theme
  shot "profile-edit-$theme" --demo --unlocked --menu profile-edit --theme $theme
  # 28/11/2026 (0.2.4, mockups 48): folder lines, search clear, type menu, birthday calendar,
  # folder icons, billing period, folder lines in Preferences, the window narrower than its columns.
  shot "vault-lines-$theme" --demo --unlocked --theme $theme
  shot "search-clear-$theme" --demo --unlocked --menu search-text --theme $theme
  shot "contacts-lines-$theme" --demo --unlocked --tab contacts --theme $theme
  shot "contact-kind-menu-$theme" --demo --unlocked --menu kind-menu --theme $theme
  shot "contact-birthday-$theme" --demo --unlocked --menu birthday-cal --theme $theme
  shot "folder-icon-$theme" --demo --unlocked --menu ctx-space-icon --theme $theme
  shot "price-period-$theme" --demo --unlocked --menu price-period --theme $theme
  shot "prefs-lines-$theme" --demo --unlocked --tab settings --page preferences --theme $theme
  shot "welcome-local-$theme" --vault "/tmp/cleis-none-$RANDOM/vault.cleis" --welcome-local --theme $theme
  shot "org-pin-setup-$theme" --demo --unlocked --tab settings --page policy --menu org-pin-setup --theme $theme
  # 27/11/2026 (mockups 42, D-98): organization access, logo, deleting the server.
  shot "policy-access-$theme" --demo --unlocked --tab settings --page policy --menu policy-access --theme $theme
  shot "org-password-$theme" --demo --unlocked --menu org-password --theme $theme
  shot "server-logo-$theme" --demo --unlocked --tab settings --page server --theme $theme
  shot "delete-org-$theme" --demo --unlocked --tab settings --page server --menu delete-org --theme $theme
  # 27/11/2026 (mockups 43a, 44a, 45a): Change asks first, Contacts as the Vault, shortcuts' grid.
  shot "health-change-$theme" --demo --unlocked --menu health-change --theme $theme
  shot "contacts-org-$theme" --demo --unlocked --menu contacts-org --theme $theme
  shot "ctx-password-$theme" --demo --unlocked --menu ctx-password --theme $theme
  shot "shortcuts-$theme" --demo --unlocked --tab settings --page shortcuts --theme $theme
  shot "account-$theme" --demo --unlocked --tab settings --page account --theme $theme
  shot "vault-open-padlock-$theme" --demo --unlocked --unlock-with nothing --theme $theme
  shot "vault-$theme" --demo --unlocked --theme $theme
  shot "update-$theme" --demo --unlocked --update 0.2.1 --theme $theme
  shot "update-confirm-$theme" --demo --unlocked --update 0.2.1 --menu update --theme $theme
  shot "updates-settings-$theme" --demo --unlocked --update 0.2.1 --tab settings --theme $theme
  shot "filter-$theme" --demo --unlocked --menu filter --theme $theme
  shot "columns-$theme" --demo --unlocked --menu new-column --theme $theme
  shot "import-$theme" --demo --unlocked --tab settings --page import --import "$root/design/tools/notion-export.csv" --theme $theme
  shot "import-review-$theme" --demo --unlocked --tab settings --page import --import "$root/design/tools/notion-export.csv" --review --theme $theme
  shot "contacts-$theme" --demo --unlocked --tab contacts --theme $theme
  shot "contact-photo-$theme" --demo --unlocked --tab contacts --menu contact-photo --theme $theme
  shot "merge-$theme" --demo --unlocked --tab contacts --page merge --theme $theme
  shot "health-$theme" --demo --unlocked --tab health --theme $theme
  shot "account-$theme" --demo --unlocked --tab settings --page account --theme $theme
  shot "preferences-$theme" --demo --unlocked --tab settings --theme $theme
  shot "wheel-$theme" --demo --unlocked --tab settings --menu wheel --theme $theme
  shot "security-$theme" --demo --unlocked --tab settings --page security --theme $theme
  shot "recovery-key-$theme" --demo --unlocked --tab settings --page security --menu recovery-key --theme $theme
  shot "open-code-$theme" --demo --unlocked --tab settings --page security --menu open-code --theme $theme
  shot "autofill-$theme" --demo --unlocked --tab settings --page autofill --theme $theme
  shot "quick-access-$theme" --demo --unlocked --quick-access --theme $theme
  shot "sync-$theme" --demo --unlocked --tab settings --page sync --theme $theme
  shot "restore-$theme" --vault "/tmp/cleis-none-$RANDOM/vault.cleis" --restore --theme $theme
  shot "welcome-$theme" --vault "/tmp/cleis-none-$RANDOM/vault.cleis" --theme $theme
  shot "welcome-org-$theme" --vault "/tmp/cleis-none-$RANDOM/vault.cleis" --org-welcome --theme $theme
  shot "org-pin-$theme" --demo --unlocked --menu org-pin --theme $theme
  shot "policy-pin-$theme" --demo --unlocked --tab settings --page policy --menu policy-pin --theme $theme
  shot "create-org-$theme" --demo --unlocked --no-org --tab settings --page join --theme $theme
  shot "create-folder-$theme" --demo --unlocked --no-org --tab settings --page join --menu create-folder --theme $theme
  shot "create-server-$theme" --demo --unlocked --no-org --tab settings --page join --menu create-server --theme $theme
  shot "create-server-here-$theme" --demo --unlocked --no-org --tab settings --page join --menu create-server-here --theme $theme
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
  shot "ctx-list-$theme" --demo --unlocked --menu ctx-list --theme $theme
  shot "ctx-sidebar-$theme" --demo --unlocked --menu ctx-sidebar --theme $theme
  shot "calendar-$theme" --demo --unlocked --menu calendar --theme $theme
  shot "customize-$theme" --demo --unlocked --menu customize --theme $theme
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
