//! Verification code asked when Cleis opens (screens 05, 06; D-08): an authenticator-app code
//! after the master password, with one-time recovery codes.
//!
//! The key is kept in the vault's preferences, so it is checked once the vault is open: it stops
//! someone who learnt the master password from using Cleis on this computer, not someone who
//! copies the vault file and the password (the file's protection is the master password).

use crate::text::fold;
use crate::totp::Totp;
use cleis_crypto::CryptoError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenCode {
    /// Base32 key, as typed into an authenticator app.
    pub secret: String,
    /// Unused recovery codes (`k7m2-x9pq`).
    pub recovery: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Check {
    Code,
    /// A recovery code: it is used up.
    Recovery,
    Wrong,
}

const RECOVERY_ALPHABET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";

fn recovery_code() -> Result<String, CryptoError> {
    let b: [u8; 8] = cleis_crypto::random_bytes()?;
    // 31 symbols: the modulo bias (256 % 31 = 8) is negligible for a one-time code.
    let c: String = b
        .iter()
        .map(|x| RECOVERY_ALPHABET[usize::from(*x) % RECOVERY_ALPHABET.len()] as char)
        .collect();
    Ok(format!("{}-{}", &c[..4], &c[4..]))
}

impl OpenCode {
    /// New key (160 bits, RFC 4226's recommended length) and 8 recovery codes.
    pub fn generate() -> Result<Self, CryptoError> {
        let key: [u8; 20] = cleis_crypto::random_bytes()?;
        let recovery = (0..8).map(|_| recovery_code()).collect::<Result<_, _>>()?;
        Ok(Self {
            secret: data_encoding::BASE32_NOPAD.encode(&key),
            recovery,
        })
    }

    pub fn totp(&self, account: &str) -> Totp {
        let mut t = Totp::from_base32(&self.secret).expect("generated base32 key");
        t.issuer = Some("Cleis".into());
        t.account = Some(account.to_owned());
        t
    }

    /// `ABCD EFGH …`: the key in groups of four for typing.
    pub fn grouped(&self) -> String {
        self.secret
            .as_bytes()
            .chunks(4)
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Checks a typed code at unix time `t` (seconds): the current code or the one before or
    /// after (clock drift), else an unused recovery code, which is then removed.
    pub fn check(&mut self, input: &str, t: i64) -> Check {
        let digits: String = input.chars().filter(char::is_ascii_digit).collect();
        if digits.len() == 6 {
            let totp = self.totp("");
            if [-30, 0, 30].iter().any(|d| totp.code_at(t + d) == digits) {
                return Check::Code;
            }
        }
        let typed = fold(input).replace(' ', "");
        match self.recovery.iter().position(|r| *r == typed) {
            Some(i) => {
                self.recovery.remove(i);
                Check::Recovery
            }
            None => Check::Wrong,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_and_recovery() {
        let mut o = OpenCode::generate().unwrap();
        assert_eq!(o.secret.len(), 32);
        assert_eq!(o.recovery.len(), 8);
        let t = 1_795_000_000;
        let code = o.totp("me").code_at(t);
        assert_eq!(o.check(&code, t), Check::Code);
        assert_eq!(
            o.check(&code, t + 30),
            Check::Code,
            "previous code still accepted"
        );
        assert_eq!(o.check(&code, t + 120), Check::Wrong);
        let r = o.recovery[3].clone();
        assert_eq!(o.check(&r.to_uppercase(), t), Check::Recovery);
        assert_eq!(o.recovery.len(), 7);
        assert_eq!(o.check(&r, t), Check::Wrong, "used once");
        assert!(
            o.totp("me")
                .to_uri()
                .starts_with("otpauth://totp/Cleis:me?")
        );
    }
}
