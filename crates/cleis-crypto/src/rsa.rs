//! RS256 (RSASSA-PKCS1-v1_5 with SHA-256) verification, for the ID tokens of an identity provider
//! (Google, Microsoft: OpenID Connect, D-64). Delegates to `ring`.

use crate::CryptoError;
use ring::signature::{RSA_PKCS1_2048_8192_SHA256, RsaPublicKeyComponents};

/// Checks an RS256 signature with the public key given by its modulus `n` and exponent `e`
/// (big-endian bytes, as a JWK carries them once base64url-decoded). Keys under 2048 bits are
/// refused.
pub fn verify_rs256(
    n: &[u8],
    e: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), CryptoError> {
    RsaPublicKeyComponents { n, e }
        .verify(&RSA_PKCS1_2048_8192_SHA256, message, signature)
        .map_err(|_| CryptoError::Signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_encoding::BASE64URL_NOPAD as B64;

    /// RFC 7515, Appendix A.2 (example JWS using RSASSA-PKCS1-v1_5 SHA-256).
    const N: &str = "ofgWCuLjybRlzo0tZWJjNiuSfb4p4fAkd_wWJcyQoTbji9k0l8W26mPddxHmfHQp-Vaw-4qPCJrcS2mJPMEzP1Pt0Bm4d4QlL-yRT-SFd2lZS-pCgNMsD1W_YpRPEwOWvG6b32690r2jZ47soMZo9wGzjb_7OMg0LOL-bSf63kpaSHSXndS5z5rexMdbBYUsLA9e-KXBdQOS-UTo7WTBEMa2R2CapHg665xsmtdVMTBQY4uDZlxvb3qCo5ZwKh9kG4LT6_I5IhlJH7aGhyxXFvUK-DWNmoudF8NAco9_h9iaGNj8q2ethFkMLs91kzk2PAcDTW9gb54h4FRWyuXpoQ";
    const JWS: &str = "eyJhbGciOiJSUzI1NiJ9.eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ.cC4hiUPoj9Eetdgtv3hF80EGrhuB__dzERat0XF9g2VtQgr9PJbu3XOiZj5RZmh7AAuHIm4Bh-0Qc_lF5YKt_O8W2Fp5jujGbds9uJdbF9CUAr7t1dnZcAcQjbKBYNX4BAynRFdiuB--f_nZLgrnbyTyWzO75vRK5h6xBArLIARNPvkSjtQBMHlb1L07Qe7K0GarZRmB_eSN9383LcOLn6_dO--xi12jzDwusC-eOkHWEsqtFZESc6BfI7noOPqvhJ1phCnvWh6IeYI2w9QOYEUipUTI8np6LbgGY9Fs98rqVt5AXLIhWkWywlVmtVrBp0igcN_IoypGlUPQGe77Rw";

    #[test]
    fn rfc7515_a2() {
        let n = B64.decode(N.as_bytes()).unwrap();
        let e = B64.decode(b"AQAB").unwrap();
        let (input, sig) = JWS.rsplit_once('.').unwrap();
        let sig = B64.decode(sig.as_bytes()).unwrap();
        assert_eq!(verify_rs256(&n, &e, input.as_bytes(), &sig), Ok(()));
        let mut bad = input.as_bytes().to_vec();
        bad[3] ^= 1;
        assert_eq!(
            verify_rs256(&n, &e, &bad, &sig),
            Err(CryptoError::Signature)
        );
    }
}
