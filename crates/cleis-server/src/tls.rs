//! The server's certificate: self-signed, Ed25519, made at `init` and on key rotation. Apps trust
//! it by its SHA-256 (pinned from the invitation code), not by a certificate authority, so its
//! names and dates only need to be acceptable to rustls.

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_ED25519};
use std::sync::Arc;
use tokio_rustls::rustls::{
    self, ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
};

pub struct Identity {
    pub cert: Vec<u8>,
    /// PKCS#8 private key.
    pub key: Vec<u8>,
}

impl Identity {
    pub fn generate(names: &[String]) -> Result<Self, String> {
        let key = KeyPair::generate_for(&PKCS_ED25519).map_err(|e| e.to_string())?;
        let mut sans = vec!["cleis-server".to_owned()];
        sans.extend(names.iter().cloned());
        let mut params = CertificateParams::new(sans).map_err(|e| e.to_string())?;
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "Cleis server");
        params.distinguished_name = dn;
        params.not_before = rcgen::date_time_ymd(2026, 1, 1);
        params.not_after = rcgen::date_time_ymd(2126, 1, 1);
        let cert = params.self_signed(&key).map_err(|e| e.to_string())?;
        Ok(Self {
            cert: cert.der().to_vec(),
            key: key.serialize_der(),
        })
    }

    pub fn hash(&self) -> [u8; 32] {
        cleis_crypto::sha256(&self.cert)
    }

    pub fn server_config(&self) -> Result<Arc<ServerConfig>, String> {
        let cfg =
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .map_err(|e| e.to_string())?
                .with_no_client_auth()
                .with_single_cert(
                    vec![CertificateDer::from(self.cert.clone())],
                    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.key.clone())),
                )
                .map_err(|e| e.to_string())?;
        Ok(Arc::new(cfg))
    }
}
