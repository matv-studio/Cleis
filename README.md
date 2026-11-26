# Cleis

Internal password manager and contact book for Windows and macOS: a personal vault on each
computer, and shared spaces through an organization server. Rust, GPU-rendered interface (GPUI).

- `crates/cleis-app` — the desktop app (Windows, macOS; Linux for development and captures).
- `crates/cleis-server` — the organization server (Linux, Windows, macOS, Docker): see
  `deploy/README.md`.
- `crates/cleis-core`, `cleis-crypto`, `cleis-store`, `cleis-net`, `cleis-autofill` — model,
  encryption, vault file, server client, system autofill.

Windows: `Cleis-x.y.z-…-windows-x64-setup.exe` (Releases) installs Cleis for the current user, no
administrator (`%LOCALAPPDATA%\Programs\Cleis`); later versions are offered in the app.

```sh
cargo test --workspace
cargo run -p cleis-app -- --demo --unlocked
cargo run -p cleis-server -- init --data /tmp/cleis --org "Example Studio"
```

See `CLAUDE.md` (how to work on the project), `docs/DECISIONS.md` and `docs/ETAT.md`. Third-party
licences: `THIRD_PARTY.md` (releases carry the full texts). Mockups:
`design/maquettes/index.html?s=01-coffre&theme=dark`.
