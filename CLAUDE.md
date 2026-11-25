# Cleis — mémoire de l'app

Gestionnaire de mots de passe et carnet de contacts interne (Windows, macOS), usage personnel et
d'équipe. Rust, interface GPUI (rendu GPU natif). Dépôt privé `matv-studio/cleis`.

## Reprise
1. Lire `docs/ETAT.md` (où on en est, prochaines étapes) puis `docs/DECISIONS.md` (D-01 à D-60,
   « En suspens »). Les décisions récentes priment sur les anciennes qu'elles remplacent.
2. Outils : Rust stable (voir `rust-toolchain.toml`), `cargo test --workspace`,
   `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`.
3. Maquettes : `design/maquettes/` (HTML statique, charte « Apps interne »). Captures :
   `python3 design/tools/shots.py all dark` (Playwright Chromium) → `target/captures/`.
   Canevas partagé : « Cleis — maquettes » (https://claude.ai/artifact/Y9eQTWzQgwtWhrfcXAVsCM).
4. Appli réelle : `cargo run -p cleis-app -- --demo [--unlocked] [--tab contacts|health|settings]
   [--page autofill|security|sync|import|merge|members|policy|encryption|server|org-backups|join|
   profile|devices|spaces|server-here] [--menu filter|columns|new-column|open-code|invite|invite-code|
   new-role|new-space|rekey|server-code|super-admin|rename-space|delete-space|new-folder|folder-menu|
   delete-folder|create-elsewhere|server-sms|server-email|server-identity|place-github|place-sftp]
   [--no-org] [--org-welcome] [--import FICHIER [--review]] [--quick-access] [--restore]
   [--theme light]` (`--tab activity` aussi). Serveur : `cargo run -p cleis-server -- init --data DIR
   --org NOM` puis `run --data DIR --listen 127.0.0.1:8443` (code du premier admin affiché) ;
   `install [--print]`, `backup`, `restore`, `invite`, `status` ; configuration `cleis-server.toml`,
   Docker et guide dans `deploy/`. Captures de l'appli : `design/tools/appshots.sh`, thème sombre seulement
   (demande du 25/11/2026 ; `THEMES="dark light"` pour les deux) (Linux : Xvfb, ImageMagick,
   `mesa-vulkan-drivers` pour le rendu logiciel llvmpipe ; paquets de compilation GPUI listés dans la CI).
5. Toute fonction d'interface nouvelle ou refaite passe d'abord par une maquette validée.

## Règles propres à l'app
- **Auteur des commits : `Claude <noreply@anthropic.com>`.** Aucun nom ni adresse de compte personnel
  dans l'historique (D-41). Messages en anglais, à l'impératif, préfixés par la zone :
  `crypto: …`, `core: …`, `store: …`, `app: …`, `server: …`, `docs: …`, `ci: …`. Terminer par les
  lignes `Co-Authored-By` et `Claude-Session`.
- Branches : `feat/<sujet>`, `fix/<sujet>`, fusion par PR. `main` reste vert.
- Langue : anglais pour l'interface et le code ; anglais + français à la sortie des versions.
- Chiffrement : aucune primitive écrite à la main ; tout passe par `cleis-crypto`, qui fixe les formats
  (octet de version en tête). Toute nouvelle combinaison reçoit un test avec un vecteur officiel.
- Code repris ou adapté d'un projet tiers : mention dans le fichier et dans `THIRD_PARTY.md`.
- Données d'essai fictives uniquement (domaines `example-*.com`, numéros en `99 00`).
- Pas de dépense (adhésion Apple, minutes CI macOS automatiques…) sans accord.

## Carte du code
- `crates/cleis-crypto` — Argon2id, AES-256-GCM, X25519 (enveloppe de clés), Ed25519, empreintes,
  RS256 (jetons Google / Microsoft, `ring`).
- `crates/cleis-core` — modèle sans E/S : `entry`, `contact`, `column`, `formula`, `money`, `totp`,
  `import_csv`, `vcard`, `health`, `dedupe`, `journal`, `generator`, `settings`, `text`, `autofill`
  (correspondance fenêtre ↔ éléments, séquences de remplissage), `edit` / `contact_edit` (saisie dans
  les inspecteurs), `open_code` (code à l'ouverture).
- `crates/cleis-autofill` — système : raccourcis globaux (`hotkeys.rs`), fenêtre au premier plan, adresse
  de l'onglet (UIA), remplissage (`run` + `Driver`), `platform/{windows,macos,other}.rs` (seul `unsafe`).
- `crates/cleis-store` — fichier coffre SQLite ; tout le sensible est scellé, AAD = `type/id/espace` ;
  `sync.rs` : fusion de deux copies sans déchiffrer (`changed`, `purged`), copie atomique ;
  `org.rs` : espaces d'organisation (`spaces.org`, `key_gen`, `records.pushed`), rechiffrement.
- `crates/cleis-net` — client HTTPS du serveur (rustls, certificat épinglé, HTTP/1.1 minimal).
- `crates/cleis-server` — serveur d'organisation (D-59, D-63) : `api.rs` (requêtes), `db.rs`
  (schéma, journal chaîné), `tls.rs` (certificat), `config.rs` (`cleis-server.toml`), `delivery.rs` (codes par
  e-mail / SMS), `oidc.rs` (jetons d'identité), `main.rs`
  (ligne de commande, services systemd / launchd / Windows) ; `tests/flow.rs`.
- `crates/cleis-app` — appli GPUI : `theme.rs` (jetons de la charte), `ui.rs` (composants), `app.rs`
  (états premier lancement / verrouillé / ouvert), `autofill.rs` (raccourcis, remplissages en attente,
  liens appris), `editor.rs` / `contact_editor.rs` (cases des inspecteurs), `sync.rs` (place,
  sauvegardes, export), `kit.rs` (PDF du kit d'urgence), `tray.rs` (zone de notification, ouverture à la
  connexion), `org.rs` (organisation : adhésion, synchronisation, actions admin, rapports d'activité,
  sauvegardes), `host.rs` (serveur hébergé sur cet ordinateur), `remote.rs` / `sftp.rs` (places GitHub, SFTP), `oidc.rs` (connexion Google / Microsoft par le
  navigateur), `screens/` (un fichier par écran ; `org.rs` : Members, Security policy, Encryption,
  Server, Activity, dialogues), `demo.rs` (données fictives, organisation de démo), `tests/e2e.rs`,
  `tests/org.rs` (deux fenêtres contre un vrai serveur).

## Pièges
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
- Minuteries dans les essais e2e : `cx.executor().advance_clock(...)` puis `run_until_parked()`.
- Réseau : jamais d'appel bloquant sur le fil de l'interface ; `cx.background_spawn` (trait
  `AppContext` à importer par `gpui_kit::prelude::*`), puis application au coffre dans `update`.
  Dans les essais, appeler `org_sync` directement (le minuteur d'une seconde ne tourne pas).
- Une boîte de saisie demande la fenêtre : créée au rendu suivant (`ensure_org_inputs`), pas dans une
  tâche de fond.
- Empreinte d'un certificat : `fingerprint_of_hash` (le hash est déjà fait), pas `fingerprint`.
- Essai SFTP : il faut `openssh-server` et le dossier `/run/sshd` (sinon l'essai est sauté).
- `target/debug/build/<crate>/` garde les anciennes compilations : supprimer les dossiers de plus
  d'une heure si le disque se remplit.
