//! Time-based one-time passwords (RFC 6238, on HOTP RFC 4226) and `otpauth://` URIs
//! (Key Uri Format used by authenticator QR codes).

use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
pub enum Algorithm {
    Sha1,
    Sha256,
    Sha512,
}

#[derive(Debug, Error, PartialEq)]
pub enum TotpError {
    #[error("not an otpauth:// address")]
    NotOtpauth,
    #[error("only time-based codes (totp) are supported")]
    NotTotp,
    #[error("missing or invalid secret")]
    Secret,
    #[error("invalid parameter {0}")]
    Param(&'static str),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Totp {
    pub secret: Vec<u8>,
    pub algorithm: Algorithm,
    pub digits: u8,
    pub period: u32,
    #[zeroize(skip)]
    pub issuer: Option<String>,
    #[zeroize(skip)]
    pub account: Option<String>,
}

fn decode_base32(s: &str) -> Result<Vec<u8>, TotpError> {
    let clean: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect::<String>()
        .trim_end_matches('=')
        .to_owned();
    if clean.is_empty() {
        return Err(TotpError::Secret);
    }
    data_encoding::BASE32_NOPAD
        .decode(clean.as_bytes())
        .map_err(|_| TotpError::Secret)
}

impl Totp {
    /// From a base32 secret as shown under most QR codes ("Can't scan? Enter this key").
    pub fn from_base32(secret: &str) -> Result<Self, TotpError> {
        Ok(Self {
            secret: decode_base32(secret)?,
            algorithm: Algorithm::Sha1,
            digits: 6,
            period: 30,
            issuer: None,
            account: None,
        })
    }

    /// Parses `otpauth://totp/Issuer:account?secret=…&issuer=…&algorithm=…&digits=…&period=…`.
    pub fn from_uri(uri: &str) -> Result<Self, TotpError> {
        let u = url::Url::parse(uri.trim()).map_err(|_| TotpError::NotOtpauth)?;
        if u.scheme() != "otpauth" {
            return Err(TotpError::NotOtpauth);
        }
        if u.host_str().map(str::to_ascii_lowercase).as_deref() != Some("totp") {
            return Err(TotpError::NotTotp);
        }
        let label = percent_decode(u.path().trim_start_matches('/'));
        let (label_issuer, account) = match label.split_once(':') {
            Some((i, a)) => (Some(i.trim().to_owned()), Some(a.trim().to_owned())),
            None if label.is_empty() => (None, None),
            None => (None, Some(label.trim().to_owned())),
        };
        let mut t = Self {
            secret: vec![],
            algorithm: Algorithm::Sha1,
            digits: 6,
            period: 30,
            issuer: label_issuer,
            account,
        };
        for (k, v) in u.query_pairs() {
            match k.to_ascii_lowercase().as_str() {
                "secret" => t.secret = decode_base32(&v)?,
                "issuer" if !v.is_empty() => t.issuer = Some(v.into_owned()),
                "algorithm" => {
                    t.algorithm = match v.to_ascii_uppercase().as_str() {
                        "SHA1" => Algorithm::Sha1,
                        "SHA256" => Algorithm::Sha256,
                        "SHA512" => Algorithm::Sha512,
                        _ => return Err(TotpError::Param("algorithm")),
                    }
                }
                "digits" => {
                    t.digits = v.parse().map_err(|_| TotpError::Param("digits"))?;
                    if !(6..=10).contains(&t.digits) {
                        return Err(TotpError::Param("digits"));
                    }
                }
                "period" => {
                    t.period = v.parse().map_err(|_| TotpError::Param("period"))?;
                    if t.period == 0 || t.period > 3600 {
                        return Err(TotpError::Param("period"));
                    }
                }
                _ => {}
            }
        }
        if t.secret.is_empty() {
            return Err(TotpError::Secret);
        }
        Ok(t)
    }

    /// Accepts either an `otpauth://` URI or a bare base32 secret.
    pub fn parse(input: &str) -> Result<Self, TotpError> {
        if input
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("otpauth:")
        {
            Self::from_uri(input)
        } else {
            Self::from_base32(input)
        }
    }

    pub fn to_uri(&self) -> String {
        let label = match (&self.issuer, &self.account) {
            (Some(i), Some(a)) => format!("{}:{}", enc(i), enc(a)),
            (None, Some(a)) => enc(a),
            (Some(i), None) => enc(i),
            (None, None) => "Cleis".into(),
        };
        let alg = match self.algorithm {
            Algorithm::Sha1 => "SHA1",
            Algorithm::Sha256 => "SHA256",
            Algorithm::Sha512 => "SHA512",
        };
        let mut s = format!(
            "otpauth://totp/{label}?secret={}&algorithm={alg}&digits={}&period={}",
            data_encoding::BASE32_NOPAD.encode(&self.secret),
            self.digits,
            self.period
        );
        if let Some(i) = &self.issuer {
            s.push_str("&issuer=");
            s.push_str(&enc(i));
        }
        s
    }

    fn hotp(&self, counter: u64) -> u32 {
        let msg = counter.to_be_bytes();
        let digest: Vec<u8> = match self.algorithm {
            Algorithm::Sha1 => hmac_sha1(&self.secret, &msg),
            Algorithm::Sha256 => hmac_sha256(&self.secret, &msg),
            Algorithm::Sha512 => hmac_sha512(&self.secret, &msg),
        };
        let off = (digest[digest.len() - 1] & 0x0f) as usize;
        let bin = u32::from_be_bytes([
            digest[off] & 0x7f,
            digest[off + 1],
            digest[off + 2],
            digest[off + 3],
        ]);
        let modulo = 10u64.pow(u32::from(self.digits));
        (u64::from(bin) % modulo) as u32
    }

    /// Code valid at unix time `t`, zero-padded.
    pub fn code_at(&self, t: i64) -> String {
        let counter = (t.max(0) as u64) / u64::from(self.period);
        format!(
            "{:0width$}",
            self.hotp(counter),
            width = usize::from(self.digits)
        )
    }

    /// Seconds until the code changes.
    pub fn remaining(&self, t: i64) -> u32 {
        self.period - (t.max(0) as u64 % u64::from(self.period)) as u32
    }

    /// `482 913` / `9420 7082`: grouped for reading aloud and typing.
    pub fn display(code: &str) -> String {
        let mid = code.len() / 2;
        format!("{} {}", &code[..mid], &code[mid..])
    }
}

macro_rules! hmac_fn {
    ($name:ident, $d:ty) => {
        fn $name(key: &[u8], msg: &[u8]) -> Vec<u8> {
            let mut m =
                <Hmac<$d> as KeyInit>::new_from_slice(key).expect("HMAC accepts any key length");
            m.update(msg);
            m.finalize().into_bytes().to_vec()
        }
    };
}
hmac_fn!(hmac_sha1, sha1::Sha1);
hmac_fn!(hmac_sha256, sha2::Sha256);
hmac_fn!(hmac_sha512, sha2::Sha512);

fn enc(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes())
        .collect::<String>()
        .replace('+', "%20")
}

fn percent_decode(s: &str) -> String {
    url::form_urlencoded::parse(format!("x={}", s.replace('+', "%2B")).as_bytes())
        .next()
        .map(|(_, v)| v.into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(alg: Algorithm, seed: &[u8]) -> Totp {
        Totp {
            secret: seed.to_vec(),
            algorithm: alg,
            digits: 8,
            period: 30,
            issuer: None,
            account: None,
        }
    }

    /// RFC 6238 Appendix B.
    #[test]
    fn rfc6238_vectors() {
        let s1 = t(Algorithm::Sha1, b"12345678901234567890");
        let s256 = t(Algorithm::Sha256, b"12345678901234567890123456789012");
        let s512 = t(
            Algorithm::Sha512,
            b"1234567890123456789012345678901234567890123456789012345678901234",
        );
        let table: [(i64, &str, &str, &str); 6] = [
            (59, "94287082", "46119246", "90693936"),
            (1_111_111_109, "07081804", "68084774", "25091201"),
            (1_111_111_111, "14050471", "67062674", "99943326"),
            (1_234_567_890, "89005924", "91819424", "93441116"),
            (2_000_000_000, "69279037", "90698825", "38618901"),
            (20_000_000_000, "65353130", "77737706", "47863826"),
        ];
        for (time, a, b, c) in table {
            assert_eq!(s1.code_at(time), a, "sha1 @ {time}");
            assert_eq!(s256.code_at(time), b, "sha256 @ {time}");
            assert_eq!(s512.code_at(time), c, "sha512 @ {time}");
        }
    }

    /// RFC 4226 Appendix D (HOTP, 6 digits).
    #[test]
    fn rfc4226_vectors() {
        let mut h = t(Algorithm::Sha1, b"12345678901234567890");
        h.digits = 6;
        let expected = [
            755224, 287082, 359152, 969429, 338314, 254676, 287922, 162583, 399871, 520489,
        ];
        for (c, e) in expected.iter().enumerate() {
            assert_eq!(h.hotp(c as u64), *e);
        }
    }

    #[test]
    fn uri_round_trip() {
        let u = "otpauth://totp/Example%20Studio:lea@example-studio.com?secret=JBSWY3DPEHPK3PXP&issuer=Example%20Studio&algorithm=SHA256&digits=8&period=60";
        let t = Totp::from_uri(u).unwrap();
        assert_eq!(t.issuer.as_deref(), Some("Example Studio"));
        assert_eq!(t.account.as_deref(), Some("lea@example-studio.com"));
        assert_eq!(
            (t.algorithm, t.digits, t.period),
            (Algorithm::Sha256, 8, 60)
        );
        assert_eq!(t.secret, b"Hello!\xde\xad\xbe\xef");
        let back = Totp::from_uri(&t.to_uri()).unwrap();
        assert_eq!(back, t);
        assert_eq!(Totp::parse("jbsw y3dp ehpk 3pxp").unwrap().secret, t.secret);
        assert_eq!(
            Totp::from_uri("otpauth://hotp/x?secret=JBSWY3DP"),
            Err(TotpError::NotTotp)
        );
        assert_eq!(Totp::parse("not base32 !"), Err(TotpError::Secret));
        assert_eq!(t.remaining(60), 60);
        assert_eq!(Totp::display("482913"), "482 913");
    }
}
