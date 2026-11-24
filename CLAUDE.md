# Cleis — mémoire de l'app

Gestionnaire de mots de passe et carnet de contacts interne (Windows, macOS), usage personnel et
d'équipe. Rust, interface GPUI (rendu GPU natif). Dépôt privé `matv-studio/cleis`.

## Reprise
1. Lire `docs/ETAT.md` (où on en est, prochaines étapes) puis `docs/DECISIONS.md` (D-01 à D-46,
   « En suspens »). Les décisions récentes priment sur les anciennes qu'elles remplacent.
2. Outils : Rust stable (voir `rust-toolchain.toml`), `cargo test --workspace`,
   `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`.
3. Maquettes : `design/maquettes/` (HTML statique, charte « Apps interne »). Captures :
   `python3 design/tools/shots.py all dark,light` (Playwright Chromium) → `target/captures/`.
   Canevas partagé : « Cleis — maquettes » (https://claude.ai/artifact/Y9eQTWzQgwtWhrfcXAVsCM).
4. Appli réelle : `cargo run -p cleis-app -- --demo [--unlocked] [--tab contacts|health|settings]
   [--theme light]`. Captures de l'appli : `design/tools/appshots.sh` (Linux : Xvfb, ImageMagick,
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
- `crates/cleis-crypto` — Argon2id, AES-256-GCM, X25519 (enveloppe de clés), Ed25519, empreintes.
- `crates/cleis-core` — modèle sans E/S : `entry`, `contact`, `column`, `formula`, `money`, `totp`,
  `import_csv`, `vcard`, `health`, `dedupe`, `journal`, `generator`, `settings`, `text`.
- `crates/cleis-store` — fichier coffre SQLite ; tout le sensible est scellé, AAD = `type/id/espace`.
- `crates/cleis-app` — appli GPUI : `theme.rs` (jetons de la charte), `ui.rs` (composants), `app.rs`
  (états premier lancement / verrouillé / ouvert), `screens/` (un fichier par écran), `demo.rs` (données
  fictives des maquettes), `tests/e2e.rs`.
- À venir : `crates/cleis-server` (binaire unique, SQLite, VPN).

## Pièges
- RustCrypto 0.11 / dalek 3 : API différentes des exemples anciens (`hmac` 0.13 n'accepte plus un
  `D` générique simple : une fonction par algorithme, cf. `totp.rs`).
- `secure_delete` est activé : un enregistrement supprimé est réellement effacé du fichier.
- Arrondis monétaires : `round()` corrige l'erreur binaire (3,015 → 3,02).
- GPUI : une ligne de `uniform_list` doit avoir `.w_full()`, sinon les colonnes ne s'alignent pas.
- Essais e2e : `.test_support()` en dernier dans la chaîne (il change le type) ; sans lui l'élément est
  introuvable par `window.click(id)`.
- Argon2 non optimisé est très lent : `opt-level = 3` pour argon2/blake2/aes/sha2 en profil dev.
