//! Health report (screen 12): weak, reused, old passwords and entries without a 2FA code.

use crate::entry::Entry;
use crate::ids::{EntryId, Timestamp};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Issue {
    Weak,
    Reused { count: usize },
    Old,
    NoTwoFactor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Policy {
    pub min_length: usize,
    /// Below this estimated strength a password is weak.
    pub min_entropy_bits: u32,
    pub max_age_days: u32,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            min_length: 12,
            min_entropy_bits: 60,
            max_age_days: 365,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub entry: EntryId,
    pub issues: Vec<Issue>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub weak: usize,
    pub reused: usize,
    pub old: usize,
    pub no_two_factor: usize,
    pub ok: usize,
}

const COMMON: [&str; 20] = [
    "password",
    "motdepasse",
    "123456",
    "12345678",
    "123456789",
    "azerty",
    "qwerty",
    "111111",
    "000000",
    "admin",
    "letmein",
    "welcome",
    "bienvenue",
    "soleil",
    "iloveyou",
    "monkey",
    "dragon",
    "football",
    "abc123",
    "passw0rd",
];

pub fn is_weak(pw: &str, policy: &Policy) -> bool {
    let lower = pw.to_lowercase();
    pw.chars().count() < policy.min_length
        || crate::generator::entropy_bits(pw) < f64::from(policy.min_entropy_bits)
        || COMMON.iter().any(|c| lower.contains(c))
}

pub fn scan(entries: &[Entry], policy: &Policy, now: Timestamp) -> (Vec<Finding>, Summary) {
    // Group by a hash so passwords are compared without being copied around.
    let mut by_hash: HashMap<[u8; 32], usize> = HashMap::new();
    for e in entries.iter().filter(|e| !e.password.is_empty()) {
        *by_hash
            .entry(cleis_crypto::sha256(e.password.as_bytes()))
            .or_default() += 1;
    }
    let max_age = i64::from(policy.max_age_days) * 86_400;
    let mut findings = vec![];
    let mut s = Summary::default();
    for e in entries {
        let mut issues = vec![];
        if !e.password.is_empty() {
            if is_weak(&e.password, policy) {
                issues.push(Issue::Weak);
                s.weak += 1;
            }
            let n = by_hash[&cleis_crypto::sha256(e.password.as_bytes())];
            if n > 1 {
                issues.push(Issue::Reused { count: n });
                s.reused += 1;
            }
            if now - e.password_changed > max_age {
                issues.push(Issue::Old);
                s.old += 1;
            }
        }
        if e.totp.is_none() && !e.username.is_empty() {
            issues.push(Issue::NoTwoFactor);
            s.no_two_factor += 1;
        }
        if issues.is_empty() {
            s.ok += 1;
        } else {
            findings.push(Finding {
                entry: e.id,
                issues,
            });
        }
    }
    findings.sort_by_key(|f| {
        std::cmp::Reverse(
            f.issues
                .iter()
                .filter(|i| !matches!(i, Issue::NoTwoFactor))
                .count(),
        )
    });
    (findings, s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::SpaceId;

    #[test]
    fn report() {
        let sp = SpaceId::new();
        let mk = |n: &str, pw: &str, age_days: i64| {
            let mut e = Entry::new(sp, n);
            e.username = "team@example-studio.com".into();
            e.password = pw.into();
            e.password_changed = 1_000_000_000 - age_days * 86_400;
            e
        };
        let mut strong = mk("GitHub", "kX9!pq-Tr4v#Lm2&Zr8w", 10);
        strong.totp = Some(crate::totp::Totp::from_base32("JBSWY3DPEHPK3PXP").unwrap());
        let entries = vec![
            mk("Notion", "azerty2024", 400),
            mk("Slack", "Shared-Pass-2025!x", 30),
            mk("Dropbox", "Shared-Pass-2025!x", 30),
            strong,
        ];
        let (f, s) = scan(&entries, &Policy::default(), 1_000_000_000);
        assert_eq!(
            s,
            Summary {
                weak: 1,
                reused: 2,
                old: 1,
                no_two_factor: 3,
                ok: 1
            }
        );
        assert_eq!(
            f[0].issues,
            vec![Issue::Weak, Issue::Old, Issue::NoTwoFactor]
        );
    }
}
