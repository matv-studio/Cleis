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
    scan_by(entries, |_| policy, now)
}

/// [`scan`] with a policy per item (an organization's rules for its items, audit L-3).
pub fn scan_by<'a>(
    entries: &[Entry],
    policy_of: impl Fn(&Entry) -> &'a Policy,
    now: Timestamp,
) -> (Vec<Finding>, Summary) {
    // Group by a hash so passwords are compared without being copied around.
    // An item signed in with another account (D-113) has no password of its own to judge.
    let mut by_hash: HashMap<[u8; 32], usize> = HashMap::new();
    for e in entries.iter().filter(|e| e.uses_password()) {
        *by_hash
            .entry(cleis_crypto::sha256(e.password.as_bytes()))
            .or_default() += 1;
    }
    let mut findings = vec![];
    let mut s = Summary::default();
    for e in entries {
        let policy = policy_of(e);
        let max_age = i64::from(policy.max_age_days) * 86_400;
        let mut issues = vec![];
        if e.uses_password() {
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
        // Its two-step verification is the other account's.
        if e.totp.is_none() && !e.username.is_empty() && e.sign_in.is_none() {
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

/// The items whose password is `pw` (Health › find a password, D-114, mockup 50d), in the
/// order given. Compared by hash, as the reused ones; the text is never kept.
pub fn using_password<'a>(entries: &'a [Entry], pw: &str) -> Vec<&'a Entry> {
    if pw.is_empty() {
        return vec![];
    }
    let h = cleis_crypto::sha256(pw.as_bytes());
    entries
        .iter()
        .filter(|e| !e.password.is_empty() && cleis_crypto::sha256(e.password.as_bytes()) == h)
        .collect()
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
        let found: Vec<_> = using_password(&entries, "Shared-Pass-2025!x")
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(found, ["Slack", "Dropbox"]);
        assert!(using_password(&entries, "shared-pass-2025!x").is_empty());
        assert!(using_password(&entries, "").is_empty());
    }

    #[test]
    fn signed_in_with_another_account() {
        let sp = SpaceId::new();
        let mut canva = Entry::new(sp, "Canva");
        canva.username = "design@example-studio.com".into();
        canva.password = "azerty".into();
        canva.password_changed = 0;
        let mut webflow = canva.clone();
        webflow.id = crate::EntryId::new();
        canva.sign_in = Some(crate::entry::SignIn::new("Google", "#e5683c"));
        // Canva's old password is neither weak, reused nor old, and needs no 2FA here.
        let (f, s) = scan(
            &[canva.clone(), webflow.clone()],
            &Policy::default(),
            1_000_000_000,
        );
        assert_eq!(s.ok, 1);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].entry, webflow.id);
        assert!(!f[0].issues.contains(&Issue::Reused { count: 2 }));
    }
}
