# Third-party software

Cleis builds on these projects. Their licences require keeping their notices: every release
archive carries `THIRD_PARTY_LICENSES.txt` (the licence files of each crate built in, made by
`tools/licenses.py`) and `OFL.txt` (Geist typeface).

| Component | Use in Cleis | Licence |
|---|---|---|
| RustCrypto `aes-gcm`, `argon2`, `sha1`, `sha2`, `hmac`, `hkdf` | encryption, key derivation, hashing, TOTP | MIT OR Apache-2.0 |
| dalek-cryptography `x25519-dalek`, `ed25519-dalek` | key wrapping, signatures | BSD-3-Clause |
| `getrandom` | operating-system randomness | MIT OR Apache-2.0 |
| `zeroize`, `subtle` | wiping secrets, constant-time comparison | MIT OR Apache-2.0 / BSD-3-Clause |
| `rusqlite` + SQLite (bundled) | vault file | MIT / public domain |
| `serde`, `serde_json`, `csv`, `uuid`, `url`, `data-encoding`, `thiserror`, `strsim` | data handling | MIT OR Apache-2.0 / MIT |
| rlibphonenumber (crate, port of Google's libphonenumber and its metadata) | phone numbers in each country's format (0.2.4) | Apache-2.0 |
| `zip` (zip-rs) | a NAS's server read from the version's server archive | MIT |
| psl (crate, Mozilla's Public Suffix List) | the site of an address for Fill best match (0.2.4) | MIT / Apache-2.0 (list: MPL-2.0) |
| Geist, Geist Mono (Vercel, `geist` npm 1.9.2) | interface typeface, embedded TTF (`crates/cleis-app/assets/fonts`, licence in `OFL.txt`) | SIL Open Font License 1.1 |
| GPUI (`gpui-pre`, Zed Industries), gpui-kit / gpui-component / gpui-base (Longbridge) | desktop interface | Apache-2.0 |
| AccessKit (through GPUI) | accessibility (UI Automation, NSAccessibility) | MIT OR Apache-2.0 |
| `directories` | standard data folder | MIT OR Apache-2.0 |
| Lucide (`settings` icon path, `design/maquettes/js/icons.js`, `crates/cleis-app/assets/icons/gear.svg`) | preferences cog icon | ISC |

| tauri-apps `global-hotkey` | system-wide shortcuts (Quick Access, Fill best match) | MIT OR Apache-2.0 |
| `enigo` | typing into other apps (autofill) | MIT |
| 1Password `arboard` | clipboard for the "Paste" fill method | MIT OR Apache-2.0 |
| `uiautomation` (Windows) | focused field kind, browser address, writing into native fields | Apache-2.0 |
| Microsoft `windows` (Windows) | window in front, bringing it back | MIT OR Apache-2.0 |
| `objc2` (MIT), `objc2-app-kit`, `objc2-foundation` (Zlib OR Apache-2.0 OR MIT) (macOS) | app in front, activation | see column 1 |
| `unicode-normalization` (includes Unicode data) | NFC form of the master password | (MIT OR Apache-2.0) AND Unicode-3.0 |
| `async-channel` | shortcut events to the interface thread | MIT OR Apache-2.0 |

| tauri-apps `tray-icon`, `muda` | notification-area icon and its menu | MIT OR Apache-2.0 |
| `qrcode` | QR code of verification-code keys | MIT OR Apache-2.0 |
| `windows-registry` (Windows) | open at login | MIT OR Apache-2.0 |
| `rustls`, `tokio-rustls`, `ring` (organization server and its client) | TLS with a pinned certificate | Apache-2.0 OR ISC OR MIT / ISC-style + OpenSSL notices (ring) |
| `rcgen` | the server's self-signed certificate | MIT OR Apache-2.0 |
| `axum`, `hyper`, `hyper-util`, `tower`, `tokio` | organization server | MIT |
| `chrono` | local time of the activity log | MIT OR Apache-2.0 |
| `toml` | server configuration file | MIT OR Apache-2.0 |
| `lettre` | verification codes by e-mail (server) | MIT |
| `ureq` | HTTPS calls to identity providers, SMS gateways, GitHub | MIT OR Apache-2.0 |
| `russh`, `russh-sftp` | SFTP sync place | Apache-2.0 |
| `semver` | comparing published versions (updates) | MIT OR Apache-2.0 |
| NSIS (Nullsoft Scriptable Install System), stub and plugins built into the Windows installer | installer (`packaging/windows/cleis.nsi`) | zlib/libpng |
| `ring` (in `cleis-crypto`) | RS256 check of identity-provider tokens | ISC-style + OpenSSL notices |
| Mullvad `windows-service` (Windows) | server as a Windows service | MIT OR Apache-2.0 |

## QNAP QDK (build tool, D-78)

The Cleis Server package for QNAP (`CleisServer_*.qpkg`) is assembled with QNAP's QDK
(https://github.com/qnap-dev/QDK, GNU GPL). QDK is not part of this repository; the package it
builds contains QDK's installer script (`qinstall.sh`, GPL) next to Cleis Server, as a separate
program. `packaging/qnap/` (configuration, start script) is Cleis' own.
