# Cleis — mémoire de l'app

Gestionnaire de mots de passe et carnet de contacts interne (Windows, macOS), usage personnel et
d'équipe. Rust, interface GPUI (rendu GPU natif). Dépôt public `matv-studio/Cleis`.

## Reprise
1. Lire `docs/ETAT.md` (où on en est, prochaines étapes) puis `docs/DECISIONS.md` (D-01 à D-105,
   « En suspens »). Les décisions récentes priment sur les anciennes qu'elles remplacent.
2. Outils : Rust stable (voir `rust-toolchain.toml`), `cargo test --workspace`,
   `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`.
3. Maquettes : `design/maquettes/` (HTML statique, charte « Apps interne »). Captures :
   `python3 design/tools/shots.py all dark` (Playwright Chromium) → `target/captures/`.
   Canevas partagé : « Cleis — maquettes » (https://claude.ai/artifact/Y9eQTWzQgwtWhrfcXAVsCM).
4. Appli réelle : `cargo run -p cleis-app -- --demo [--unlocked] [--tab contacts|health|settings]
   [--page account|autofill|security|sync|import|org-import|merge|members|policy|encryption|server|org-backups|join|
   profile|devices|spaces|server-here|shortcuts] [--menu filter|columns|new-column|open-code|invite|invite-code|
   new-role|new-space|rekey|server-code|super-admin|rename-space|delete-space|new-folder|folder-menu|
   delete-folder|create-elsewhere|create-folder|server-sms|server-email|server-identity|place-github|place-sftp|
   ctx-entry|ctx-space|ctx-contact|ctx-member|ctx-list|ctx-sidebar|customize|calendar|wheel|contact-photo|org-pin|policy-pin|recovery-key|confirm-transfer|confirm-remove|health-vault|rename-folder|pin-setup|erase-account|github|ctx-password|policy-access|org-password|delete-org|contacts-org|health-change|github-create|github-install|profile-menu|profile-edit|website|space-members|ctx-trash|recovery|create-server|create-server-here|pick-microsoft|pick-google|cloud-microsoft|cloud-google|onedrive|gdrive|kind-menu|birthday-cal|ctx-space-icon|search-text|price-period]
   [--no-org] [--two-orgs [--vault-tab personal]] [--org-welcome] [--import FICHIER [--review]]
   [--quick-access] [--restore] [--welcome-local|--welcome-signin] [--unlock-with hello|nothing|pin]
   [--theme light] [--lang fr] [--update 0.2.1 [--menu update]]` (`--tab activity` aussi). Serveur : `cargo run -p cleis-server -- init --data DIR
   --org NOM` puis `run --data DIR --listen 127.0.0.1:8443` (code du premier admin affiché) ;
   `install [--print]`, `backup`, `restore`, `invite`, `status` ; configuration `cleis-server.toml`,
   Docker et guide dans `deploy/`. Captures de l'appli : `design/tools/appshots.sh` (`UI_LANG=fr` :
   en français), thème sombre seulement
   (demande du 25/11/2026 ; `THEMES="dark light"` pour les deux) (Linux : Xvfb, ImageMagick,
   `mesa-vulkan-drivers` pour le rendu logiciel llvmpipe ; paquets de compilation GPUI listés dans la CI).
5. Toute fonction d'interface nouvelle ou refaite passe d'abord par une maquette validée.
6. Charte commune (`charte-design-apps.md`, canevas) : ne la mettre à jour que pour un gros changement
   de principe ou une nouvelle logique de boutons ou d'animations, pas à chaque lot (demande du 27/11).

## Règles propres à l'app
- **Auteur des commits : `Claude <noreply@anthropic.com>`.** Aucun nom ni adresse de compte personnel
  dans l'historique (D-41). Messages en anglais, à l'impératif, préfixés par la zone :
  `crypto: …`, `core: …`, `store: …`, `app: …`, `server: …`, `docs: …`, `ci: …`. Terminer par les
  lignes `Co-Authored-By` et `Claude-Session`.
- **Une seule branche : `main`** (demande du 28/11/2026, D-106). Tout est commité et poussé sur `main` ;
  ni autre branche, ni PR, ni fusion. `main` reste vert (CI à chaque poussée).
- Version (D-77, D-106) : la session ne pousse pas d'étiquette elle-même (proxy). Build Windows d'essai :
  `gh api -X POST repos/matv-studio/Cleis/actions/workflows/release.yml/dispatches -f ref=main`
  (`-F 'inputs[macos]=true'` pour macOS Apple silicon + Intel, gratuit : dépôt public, D-107), installateur dans les
  artefacts du flux ; **quand l'utilisateur valide** : `gh api -X POST
  repos/matv-studio/Cleis/actions/workflows/tag.yml/dispatches -f ref=main -f 'inputs[version]=vX.Y.Z'
  -f 'inputs[commit]=<sha>'` : `tag.yml` pose l'étiquette (auteur Claude) et lance `release.yml` sur
  elle (brouillon). Suivi : `gh api repos/matv-studio/Cleis/releases`. Notes **en anglais seulement**
  (D-86) dans `docs/notes/vX.Y.Z.md` avant l'étiquette : le brouillon les reprend ; l'utilisateur publie.
  Notes à corriger après coup : `notes.yml` (dispatch, `inputs[version]`). Une étiquette construit tout, macOS compris (D-107). Le proxy refuse l'envoi de
  fichiers à une version (build macOS lancé à part : le donner à l'utilisateur, qui l'ajoute au
  brouillon). Secours : script `etiquette-vX.Y.Z.bat` dans `C:\MATV_LOCAL\_Interne\Cleis\`.
- Langue : anglais pour l'interface et le code (interface traduite par `i18n/fr.json`) ; notes de
  version en anglais seulement (D-86).
- Chiffrement : aucune primitive écrite à la main ; tout passe par `cleis-crypto`, qui fixe les formats
  (octet de version en tête). Toute nouvelle combinaison reçoit un test avec un vecteur officiel.
- Code repris ou adapté d'un projet tiers : mention dans le fichier et dans `THIRD_PARTY.md`.
- Données d'essai fictives uniquement (domaines `example-*.com`, numéros en `99 00`).
- Pas de dépense (adhésion Apple, minutes CI macOS automatiques…) sans accord.

## Carte du code
- `crates/cleis-crypto` — Argon2id, AES-256-GCM, X25519 (enveloppe de clés), Ed25519, empreintes,
  RS256 (jetons Google / Microsoft, `ring`).
- `crates/cleis-core` — modèle sans E/S : `entry`, `contact`, `column`, `formula`, `money`, `totp`,
  `import_csv`, `vcard`, `phone` (format des numéros par pays, D-105), `health`, `dedupe`, `journal`, `generator`, `settings`, `text`, `autofill`
  (correspondance fenêtre ↔ éléments, séquences de remplissage), `edit` / `contact_edit` (saisie dans
  les inspecteurs), `open_code` (code à l'ouverture).
- `crates/cleis-autofill` — système : raccourcis globaux (`hotkeys.rs`), fenêtre au premier plan, adresse
  de l'onglet (UIA), remplissage (`run` + `Driver`) ; `unsafe` seulement dans
  `platform/{windows,macos,other}.rs`.
- `crates/cleis-store` — fichier coffre SQLite ; tout le sensible est scellé, AAD = `type/id/espace` ;
  `sync.rs` : fusion de deux copies sans déchiffrer (`changed`, `purged`), copie atomique ;
  `org.rs` : espaces d'organisation (`spaces.org`, `key_gen`, `records.pushed`), rechiffrement.
- `crates/cleis-net` — client HTTPS du serveur (rustls, certificat épinglé, HTTP/1.1 minimal).
- `crates/cleis-server` — serveur d'organisation (D-59, D-63) : `local.rs` (l'API rejouée dans
  l'appli pour un dossier partagé : horloge et hasard de l'opération, D-78), `api.rs` (requêtes), `db.rs`
  (schéma, journal chaîné), `tls.rs` (certificat), `config.rs` (`cleis-server.toml`), `delivery.rs`
  (codes et invitations par
  e-mail / SMS), `oidc.rs` (jetons d'identité), `main.rs`
  (ligne de commande, services systemd / launchd / Windows) ; `tests/flow.rs`.
- `crates/cleis-app` — appli GPUI : `theme.rs` (jetons de la charte), `ui.rs` (composants), `app.rs`
  (états premier lancement / verrouillé / ouvert), `autofill.rs` (raccourcis, remplissages en attente,
  liens appris), `editor.rs` / `contact_editor.rs` (cases des inspecteurs), `sync.rs` (place,
  sauvegardes, export), `kit.rs` (PDF du kit d'urgence), `tray.rs` (zone de notification, ouverture à la
  connexion), `org.rs` (organisation : adhésion, synchronisation, actions admin, rapports d'activité,
  sauvegardes), `host.rs` (serveur hébergé sur cet ordinateur), `remote.rs` / `sftp.rs` (places GitHub, SFTP), `oidc.rs` (connexion Google /
  Microsoft par le navigateur), `screens/ctx.rs` (menus du clic droit, raccourcis d'un élément),
  `update.rs` + `screens/update.rs` (mises à jour depuis GitHub, D-74), `nas.rs` (serveur installé
  sur un NAS par SSH : QNAP .qpkg, systemd, D-78), `folder_org.rs` (organisation dans un dossier
  partagé : opérations signées, copie locale rejouée par `cleis_server::local`, D-78),
  `scroll.rs` (barres de défilement, clic molette, D-105), `screens/wheel.rs` (roue chromatique), `opening.rs` (ouverture par
  Windows Hello, PIN ou sans rien, fichier `<coffre>.unlock`, D-76, D-83), `screens/pin.rs` (cases du
  PIN), `github_login.rs` + `screens/github.rs` (connexion GitHub par code, D-84 ; GitHub App, D-101),
  `cloud.rs` + `screens/cloud.rs` (coffre dans OneDrive / Google Drive : connexion PKCE par le
  navigateur, jetons renouvelés, D-104), `screens/profile.rs` (profil de Cleis en haut à droite, D-101), `screens/keys.rs`
  (Paramètres › Shortcuts, D-85), `screens/erase.rs` (suppression des sauvegardes et du compte, D-82), `instance.rs` (une seule
  copie, D-75), installateur `packaging/windows/` (NSIS),
  `i18n.rs` + `i18n/fr.json` (traduction, D-73 : `tr`, `trf!`, `tr_owned`, `tr_dynamic`),
  `screens/` (un fichier par écran ; `org.rs` : Members, Security policy, Encryption,
  Server, Activity, dialogues), `demo.rs` (données fictives, organisation de démo), `tests/e2e.rs`,
  `tests/org.rs` (deux fenêtres contre un vrai serveur).

## Pièges
- Texte affiché : en anglais dans le code, passé par `tr` / `trf!` (ou un composant de `ui.rs`
  qui traduit), puis l'entrée ajoutée à `i18n/fr.json`. Jamais de libellé traduit dans un id
  d'élément ni dans une donnée enregistrée (journal, coffre).
- RustCrypto 0.11 / dalek 3 : API différentes des exemples anciens (`hmac` 0.13 n'accepte plus un
  `D` générique simple : une fonction par algorithme, cf. `totp.rs`).
- `secure_delete` est activé : un enregistrement supprimé est réellement effacé du fichier.
- Arrondis monétaires : `round()` corrige l'erreur binaire (3,015 → 3,02).
- GPUI : une ligne de `uniform_list` doit avoir `.w_full()`, sinon les colonnes ne s'alignent pas.
- Essais e2e : `.test_support()` en dernier dans la chaîne (il change le type) ; sans lui l'élément est
  introuvable par `window.click(id)`.
- Argon2 non optimisé est très lent : `opt-level = 3` pour argon2/blake2/aes/sha2 en profil dev.
- Disque du conteneur : `target/` dépasse 20 Go en debug complet. Exporter `CARGO_INCREMENTAL=0
  CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only`.
- `cleis-crypto` n'emploie `ring` (RS256) qu'avec la fonction `rs256` (serveur) : sans elle,
  la vérification croisée de `cleis-autofill` reste possible (ring demande les en-têtes MSVC).
- Vérifier la compilation Windows/macOS de `cleis-autofill` sans machine :
  `cargo clippy -p cleis-autofill --target x86_64-pc-windows-msvc` (et `aarch64-apple-darwin`) ;
  l'appli entière ne se vérifie que sur la CI (SQLite demande le compilateur MSVC).
- Fenêtre ouverte depuis une mise à jour de `CleisApp` (Quick Access) : passer par `cx.defer`, sinon la
  nouvelle vue lit l'appli pendant qu'elle est mise à jour (panique « already being updated »).
- Une touche n'arrive à un `on_key_down` que si un élément de son sous-arbre a le focus : `track_focus`
  + `window.focus` (raccourcis Ctrl+L / Ctrl+F, enregistrement d'un raccourci).
- Captures et essais : `--demo` et les essais mettent `no_shortcuts` et `autofill.dry_run` ; jamais de
  raccourci système pris ni de frappe envoyée.
- Menus flottants : `menu_box` (avec `.occlude()`) dans `deferred(...)`, sinon le clic atteint aussi
  l'élément dessous. `Window::window_handle()` (GPUI) masque le trait de `raw-window-handle` : l'appeler
  par `HasWindowHandle::window_handle(window)`.
- Windows : jamais d'appui simulé sur Alt pour prendre le premier plan (l'appli devant affiche ses
  touches d'accès) ; `cleis_autofill::activate` utilise F22.
- Clic droit dans les essais : `w.right_click(id)` ; l'élément doit porter `.test_support()`. Les
  raccourcis d'un élément (Suppr, Ctrl+D…) ne partent que si `main_focus` a le clavier (clic sur
  une ligne ou ouverture d'un menu contextuel).
- Plusieurs organisations (D-69) : `Session::org` est celle montrée ; tout travail pour une autre
  (réponse du serveur, synchronisation, rapport) passe par `app.in_org(clé, |app| …)`. Ne pas
  lire `s.org` pour un élément : `s.org_of(espace)`.
- Minuteries dans les essais e2e : `cx.executor().advance_clock(...)` puis `run_until_parked()`.
- Réseau : jamais d'appel bloquant sur le fil de l'interface ; `cx.background_spawn` (trait
  `AppContext` à importer par `gpui_kit::prelude::*`), puis application au coffre dans `update`.
  Dans les essais, appeler `org_sync` directement (le minuteur d'une seconde ne tourne pas).
- Une boîte de saisie demande la fenêtre : créée au rendu suivant (`ensure_org_inputs`), pas dans une
  tâche de fond.
- Empreinte d'un certificat : `fingerprint_of_hash` (le hash est déjà fait), pas `fingerprint`.
- Essai SFTP : il faut `openssh-server` et le dossier `/run/sshd` (sinon l'essai est sauté ; avec
  `CLEIS_REQUIRE_ALL=1`, comme sur la CI Linux, un essai sauté échoue).
- Menus : tout menu flottant doit être dans `deferred(...)` : la couche « clic ailleurs »
  (`ctx::outside`) est sous les menus différés et au-dessus du reste.
- Licences des archives : `python3 tools/licenses.py -o FICHIER cleis-app cleis-server`.
- `target/debug/build/<crate>/<hash>/` garde les anciennes compilations : si le disque se remplit,
  `find target/debug/build -mindepth 2 -maxdepth 2 -type d -mmin +30 -exec rm -rf {} +`.
- Installateur Windows (D-75) : `makensis` existe aussi sous Linux (`apt install nsis`) ; essai
  local : `makensis -DVERSION=x -DSRC=dossier -DOUT=setup.exe packaging/windows/cleis.nsi`.
- Création d'un coffre (essais) : la clé de récupération s'affiche ensuite dans un dialogue qui
  couvre le centre : `w.click("recovery-key-done")` avant de cliquer ailleurs.
- Paquet QNAP (D-78) : `packaging/qnap/` construit par la CI avec le QDK de QNAP (licence GPL,
  hors dépôt) ; un .qpkg termine par le code 10 qu'il soit installé ou non : vérifier
  `/etc/init.d/CleisServer.sh`. Serveur Linux statique (musl) : `musl-tools` +
  `--target x86_64-unknown-linux-musl`.
- Dossier partagé (D-78) : une opération est rejouée à sa date ; deux opérations à la même seconde
  se départagent par la clé du signataire : l'appli date les siennes après la dernière appliquée.
  Le registre des copies ouvertes est global au processus : sa clé est le dossier de la copie.
- Un `div` qui doit être trouvé par les essais (`window_has`, `w.click`) porte `.test_support()`,
  boîtes de dialogue comprises. Une page de réglages plus haute que la fenêtre : faire défiler au-dessus
  d'un élément visible (`settings_bottom(cx, h, "<id>")`) avant de cliquer en bas.
- Un clic sur un élément qui ne prend pas le clavier ne retire pas le focus d'un champ : pour « clic
  ailleurs », `on_mouse_down_out` (dossier nommé sur place).
- Les champs `Input` gardent le clic droit pour le menu du système : un menu à nous se pose par
  `capture_any_mouse_down` sur leur conteneur (champ mot de passe, D-82).
- Essais OneDrive / Google Drive (D-104) : `app.cloud_hosts = CloudHosts::stand_in(adresse)` vers le faux
  de `tests/cloud.rs` ; le navigateur doit répondre avant `run_until_parked` (l'exécuteur d'essai
  déroulerait l'attente jusqu'à son terme : `sign_in_with`).
- Essais de la connexion GitHub : `app.github_hosts` vers `fake_github()` (device flow, `/user`, dépôt) ;
  jamais github.com depuis un essai.
- `appshots.sh` lance `cargo build` sans les variables du disque : les exporter avant (sinon tout se
  recompile dans un autre profil).
- Barre du haut (fenêtre sans cadre) : sous Windows, tout contrôle posé dedans doit avoir
  `.occlude()`, sinon le clic part en déplacement de fenêtre.
- Accès à l'organisation (D-98) : `OrgSession::pin` est la carte demandée (`PinPrompt::with` : PIN,
  mot de passe, Hello ; `here` : vérifié par l'appli, dossier partagé ou Hello). Les essais e2e qui
  attendent une empreinte Argon2id (64 Mio) bouclent sur `run_until_parked` + court sommeil.
- Essais e2e : une boîte ne reçoit `Blur` / `Focus` que dans une fenêtre active :
  `w.activate_window()` avant de cliquer ailleurs (enregistrement en quittant une case, D-100).
- GitHub App (D-101) : un jeton ne voit que les dépôts où l'application est installée (404 sinon, comme
  un dépôt absent) ; `GET /user/installations` distingue « jamais installée ». Essais : `fake_github_app`
  (`GhApp` : création refusée, installation, jetons qui expirent) ; jamais github.com.
- Un `div` porteur d'un `group` / d'un `id` change de type avec `.test_support()` : rendre
  `impl IntoElement` plutôt que `Stateful<Div>`.
- Listes par dossier (D-105) : `refilter` groupe `visible` par dossier quand `folder_lines()` ; les
  essais qui lisent la liste dans un seul tri appellent `no_folder_lines`. Un `set_value` fait par
  l'appli n'émet pas `InputEvent::Change` : l'émettre (`cx.emit`) si la liste doit suivre.
- Disque : ne pas effacer `target/debug/build/*/out` à la main (compilation cassée : `rm -rf
  target/debug` puis tout refaire).
- Dossier partagé supprimé (D-98) : seule `deleted.json` reste ; le registre `folder_org` a pour clé le
  chemin de la copie (`replica_dir(...).display()`), pas la clé de l'organisation.
