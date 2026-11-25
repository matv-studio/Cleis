# Third-party software

Cleis builds on these projects. Their licences require keeping their notices; the full texts ship
with each crate (see `cargo tree` and the crates' `LICENSE*` files) and are bundled with releases.

| Component | Use in Cleis | Licence |
|---|---|---|
| RustCrypto `aes-gcm`, `argon2`, `sha1`, `sha2`, `hmac`, `hkdf` | encryption, key derivation, hashing, TOTP | MIT OR Apache-2.0 |
| dalek-cryptography `x25519-dalek`, `ed25519-dalek` | key wrapping, signatures | BSD-3-Clause |
| `getrandom` | operating-system randomness | MIT OR Apache-2.0 |
| `zeroize`, `subtle` | wiping secrets, constant-time comparison | MIT OR Apache-2.0 / BSD-3-Clause |
| `rusqlite` + SQLite (bundled) | vault file | MIT / public domain |
| `serde`, `serde_json`, `csv`, `uuid`, `url`, `data-encoding`, `thiserror`, `strsim` | data handling | MIT OR Apache-2.0 / MIT |
| Geist, Geist Mono (Vercel, `geist` npm 1.9.2) | interface typeface, embedded TTF (`crates/cleis-app/assets/fonts`, licence in `OFL.txt`) | SIL Open Font License 1.1 |
| GPUI (`gpui-pre`, Zed Industries), gpui-kit / gpui-component / gpui-base (Longbridge) | desktop interface | Apache-2.0 |
| AccessKit (through GPUI) | accessibility (UI Automation, NSAccessibility) | MIT OR Apache-2.0 |
| `directories` | standard data folder | MIT OR Apache-2.0 |
| Lucide (`settings` icon path, `design/maquettes/js/icons.js`, `crates/cleis-app/assets/icons/gear.svg`) | preferences cog icon | ISC |

Planned: tauri-apps `tray-icon` and `global-hotkey` (MIT OR Apache-2.0).
