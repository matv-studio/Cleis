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
| Geist font (Vercel) | interface typeface (design charter) | SIL Open Font License 1.1 |

Planned: GPUI and gpui-component (Apache-2.0), AccessKit (MIT OR Apache-2.0), tauri-apps `tray-icon`
and `global-hotkey` (MIT OR Apache-2.0).
