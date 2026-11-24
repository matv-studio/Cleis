//! Activity log (D-30): one append-only chain per device. Each record is linked to the
//! previous one by SHA-256 and signed with the device's Ed25519 key. Devices also record
//! the latest head they have seen from the other devices ("witnesses"), which makes a
//! truncated or deleted log visible. Tamper-evident, not tamper-proof.
//!
//! The event details are opaque bytes here: the store seals them for the administrators
//! before they reach this layer.

use cleis_crypto::{SigningKeyPair, VerifyingKey, sha256, verify};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const DOMAIN: &str = "cleis/log/v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub [u8; 16]);

/// A device's statement about another device's chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Witness {
    pub device: DeviceId,
    pub seq: u64,
    pub hash: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub device: DeviceId,
    /// Starts at 1, no gaps.
    pub seq: u64,
    /// Unix seconds.
    pub at: i64,
    /// Hash of the previous record (zeros for the first).
    pub prev: [u8; 32],
    pub witnesses: Vec<Witness>,
    /// Sealed event (action, member, item, space…).
    pub payload: Vec<u8>,
    #[serde(with = "sig_serde")]
    pub signature: [u8; 64],
}

mod sig_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(v)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let v: Vec<u8> = Vec::deserialize(d)?;
        v.try_into()
            .map_err(|_| serde::de::Error::custom("signature must be 64 bytes"))
    }
}

impl Record {
    /// Canonical bytes covered by the hash and the signature.
    fn body(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(128 + self.payload.len());
        b.extend_from_slice(&self.device.0);
        b.extend_from_slice(&self.seq.to_be_bytes());
        b.extend_from_slice(&self.at.to_be_bytes());
        b.extend_from_slice(&self.prev);
        b.extend_from_slice(&(self.witnesses.len() as u32).to_be_bytes());
        for w in &self.witnesses {
            b.extend_from_slice(&w.device.0);
            b.extend_from_slice(&w.seq.to_be_bytes());
            b.extend_from_slice(&w.hash);
        }
        b.extend_from_slice(&(self.payload.len() as u64).to_be_bytes());
        b.extend_from_slice(&self.payload);
        b
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut b = self.body();
        b.extend_from_slice(&self.signature);
        sha256(&b)
    }
}

/// Writer for this device's chain.
pub struct Chain {
    device: DeviceId,
    key: SigningKeyPair,
    next_seq: u64,
    head: [u8; 32],
}

impl Chain {
    /// Resumes after `last` (or starts a new chain).
    pub fn resume(device: DeviceId, key: SigningKeyPair, last: Option<&Record>) -> Self {
        let (next_seq, head) = last.map_or((1, [0; 32]), |r| (r.seq + 1, r.hash()));
        Self {
            device,
            key,
            next_seq,
            head,
        }
    }

    pub fn append(&mut self, at: i64, payload: Vec<u8>, witnesses: Vec<Witness>) -> Record {
        let mut r = Record {
            device: self.device,
            seq: self.next_seq,
            at,
            prev: self.head,
            witnesses,
            payload,
            signature: [0; 64],
        };
        r.signature = self.key.sign(DOMAIN, &r.body());
        self.head = r.hash();
        self.next_seq += 1;
        r
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    UnknownDevice(DeviceId),
    BadSignature {
        device: DeviceId,
        seq: u64,
    },
    Gap {
        device: DeviceId,
        expected: u64,
        found: u64,
    },
    BrokenLink {
        device: DeviceId,
        seq: u64,
    },
    /// Another device saw records that are no longer there.
    Truncated {
        device: DeviceId,
        witnessed: u64,
        present: u64,
    },
    /// Another device saw a different record at that position.
    Rewritten {
        device: DeviceId,
        seq: u64,
    },
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub records: usize,
    pub devices: usize,
    pub problems: Vec<Problem>,
}

impl Report {
    pub fn is_intact(&self) -> bool {
        self.problems.is_empty()
    }
}

/// Verifies every chain and cross-checks the witnesses.
/// `keys` maps each enrolled device to its verifying key.
pub fn verify_all(
    chains: &BTreeMap<DeviceId, Vec<Record>>,
    keys: &BTreeMap<DeviceId, VerifyingKey>,
) -> Report {
    let mut rep = Report {
        devices: chains.len(),
        ..Default::default()
    };
    let mut hashes: BTreeMap<(DeviceId, u64), [u8; 32]> = BTreeMap::new();
    for (dev, recs) in chains {
        let Some(vk) = keys.get(dev) else {
            rep.problems.push(Problem::UnknownDevice(*dev));
            continue;
        };
        let mut prev = [0u8; 32];
        let mut expected = 1;
        for r in recs {
            rep.records += 1;
            if r.device != *dev || verify(vk, DOMAIN, &r.body(), &r.signature).is_err() {
                rep.problems.push(Problem::BadSignature {
                    device: *dev,
                    seq: r.seq,
                });
            }
            if r.seq != expected {
                rep.problems.push(Problem::Gap {
                    device: *dev,
                    expected,
                    found: r.seq,
                });
            } else if r.prev != prev {
                rep.problems.push(Problem::BrokenLink {
                    device: *dev,
                    seq: r.seq,
                });
            }
            prev = r.hash();
            expected = r.seq + 1;
            hashes.insert((*dev, r.seq), prev);
        }
    }
    for recs in chains.values() {
        for r in recs {
            for w in &r.witnesses {
                let present = chains
                    .get(&w.device)
                    .and_then(|c| c.last())
                    .map_or(0, |l| l.seq);
                match hashes.get(&(w.device, w.seq)) {
                    None if w.seq > present => {
                        let p = Problem::Truncated {
                            device: w.device,
                            witnessed: w.seq,
                            present,
                        };
                        if !rep.problems.contains(&p) {
                            rep.problems.push(p);
                        }
                    }
                    Some(h) if *h != w.hash => rep.problems.push(Problem::Rewritten {
                        device: w.device,
                        seq: w.seq,
                    }),
                    _ => {}
                }
            }
        }
    }
    rep
}

/// Current heads of every chain, to embed as witnesses in the next record.
pub fn heads(chains: &BTreeMap<DeviceId, Vec<Record>>, except: DeviceId) -> Vec<Witness> {
    chains
        .iter()
        .filter(|(d, _)| **d != except)
        .filter_map(|(d, c)| {
            c.last().map(|r| Witness {
                device: *d,
                seq: r.seq,
                hash: r.hash(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (
        BTreeMap<DeviceId, Vec<Record>>,
        BTreeMap<DeviceId, VerifyingKey>,
        DeviceId,
        DeviceId,
    ) {
        let (a, b) = (DeviceId([1; 16]), DeviceId([2; 16]));
        let (ka, kb) = (
            SigningKeyPair::generate().unwrap(),
            SigningKeyPair::generate().unwrap(),
        );
        let mut keys = BTreeMap::new();
        keys.insert(a, ka.verifying_key());
        keys.insert(b, kb.verifying_key());
        let mut chains: BTreeMap<DeviceId, Vec<Record>> = BTreeMap::new();
        let mut ca = Chain::resume(a, ka, None);
        let mut cb = Chain::resume(b, kb, None);
        for i in 0..3 {
            let r = ca.append(100 + i, format!("a{i}").into_bytes(), vec![]);
            chains.entry(a).or_default().push(r);
        }
        let w = heads(&chains, b);
        let r = cb.append(200, b"b0".to_vec(), w);
        chains.entry(b).or_default().push(r);
        (chains, keys, a, b)
    }

    #[test]
    fn intact() {
        let (c, k, _, _) = setup();
        let r = verify_all(&c, &k);
        assert!(r.is_intact(), "{r:?}");
        assert_eq!((r.records, r.devices), (4, 2));
    }

    #[test]
    fn detects_edit_deletion_truncation() {
        let (mut c, k, a, _) = setup();
        c.get_mut(&a).unwrap()[1].payload = b"edited".to_vec();
        let r = verify_all(&c, &k);
        assert!(
            r.problems
                .contains(&Problem::BadSignature { device: a, seq: 2 })
        );
        assert!(
            r.problems
                .contains(&Problem::BrokenLink { device: a, seq: 3 })
        );

        let (mut c, k, a, _) = setup();
        c.get_mut(&a).unwrap().remove(1);
        assert!(verify_all(&c, &k).problems.contains(&Problem::Gap {
            device: a,
            expected: 2,
            found: 3
        }));

        let (mut c, k, a, _) = setup();
        c.get_mut(&a).unwrap().pop();
        assert!(verify_all(&c, &k).problems.contains(&Problem::Truncated {
            device: a,
            witnessed: 3,
            present: 2
        }));

        let (c, mut k, a, _) = setup();
        k.remove(&a);
        assert!(
            verify_all(&c, &k)
                .problems
                .contains(&Problem::UnknownDevice(a))
        );
    }

    #[test]
    fn serde_round_trip() {
        let (c, _, a, _) = setup();
        let r = &c[&a][0];
        let back: Record = serde_json::from_str(&serde_json::to_string(r).unwrap()).unwrap();
        assert_eq!(&back, r);
    }
}
