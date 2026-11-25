//! Password generator (inspector "Generate"). Uniform choice by rejection sampling over
//! the operating system's random generator; at least one character of each enabled class.

use cleis_crypto::{CryptoError, random_bytes};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratorOptions {
    pub length: usize,
    pub lower: bool,
    pub upper: bool,
    pub digits: bool,
    pub symbols: bool,
    /// Leaves out characters that look alike (O/0, l/1/I).
    pub avoid_ambiguous: bool,
}

impl Default for GeneratorOptions {
    fn default() -> Self {
        Self {
            length: 20,
            lower: true,
            upper: true,
            digits: true,
            symbols: true,
            avoid_ambiguous: true,
        }
    }
}

const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!#$%&*+-=?@^_~";
const AMBIGUOUS: &str = "O0oIl1|";

fn uniform(n: usize) -> Result<usize, CryptoError> {
    // Rejection sampling on u32 to avoid modulo bias.
    let n32 = n as u32;
    let limit = u32::MAX - (u32::MAX % n32);
    loop {
        let v = u32::from_le_bytes(random_bytes()?);
        if v < limit {
            return Ok((v % n32) as usize);
        }
    }
}

pub fn generate(o: &GeneratorOptions) -> Result<String, CryptoError> {
    let filter = |s: &str| -> Vec<char> {
        s.chars()
            .filter(|c| !(o.avoid_ambiguous && AMBIGUOUS.contains(*c)))
            .collect()
    };
    let mut classes: Vec<Vec<char>> = vec![];
    if o.lower {
        classes.push(filter(LOWER));
    }
    if o.upper {
        classes.push(filter(UPPER));
    }
    if o.digits {
        classes.push(filter(DIGITS));
    }
    if o.symbols {
        classes.push(filter(SYMBOLS));
    }
    if classes.is_empty() {
        classes.push(filter(LOWER));
    }
    let length = o.length.clamp(classes.len().max(4), 128);
    let all: Vec<char> = classes.concat();
    // `?`: a failing system RNG must stop generation, never fall back to a fixed index.
    let mut out: Vec<char> = classes
        .iter()
        .map(|c| uniform(c.len()).map(|i| c[i]))
        .collect::<Result<_, _>>()?;
    while out.len() < length {
        out.push(all[uniform(all.len())?]);
    }
    // Fisher–Yates so the guaranteed characters are not always first.
    for i in (1..out.len()).rev() {
        let j = uniform(i + 1)?;
        out.swap(i, j);
    }
    Ok(out.into_iter().collect())
}

/// Rough strength in bits: length × log2(alphabet size actually used).
pub fn entropy_bits(pw: &str) -> f64 {
    let mut alphabet = 0u32;
    if pw.chars().any(|c| c.is_ascii_lowercase()) {
        alphabet += 26;
    }
    if pw.chars().any(|c| c.is_ascii_uppercase()) {
        alphabet += 26;
    }
    if pw.chars().any(|c| c.is_ascii_digit()) {
        alphabet += 10;
    }
    if pw.chars().any(|c| c.is_ascii_punctuation() || c == ' ') {
        alphabet += 33;
    }
    if !pw.is_ascii() {
        alphabet += 64;
    }
    // Repetition penalty: count distinct characters at most.
    let distinct = pw.chars().collect::<std::collections::BTreeSet<_>>().len();
    let len = pw.chars().count().min(distinct * 2);
    if alphabet == 0 {
        0.0
    } else {
        len as f64 * f64::from(alphabet).log2()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_options() {
        for _ in 0..50 {
            let p = generate(&GeneratorOptions::default()).unwrap();
            assert_eq!(p.chars().count(), 20);
            assert!(p.chars().any(|c| c.is_ascii_lowercase()));
            assert!(p.chars().any(|c| c.is_ascii_uppercase()));
            assert!(p.chars().any(|c| c.is_ascii_digit()));
            assert!(p.chars().any(|c| SYMBOLS.contains(c)));
            assert!(!p.chars().any(|c| AMBIGUOUS.contains(c)));
        }
        let d = generate(&GeneratorOptions {
            length: 6,
            lower: false,
            upper: false,
            digits: true,
            symbols: false,
            avoid_ambiguous: false,
        })
        .unwrap();
        assert!(d.chars().all(|c| c.is_ascii_digit()) && d.len() == 6);
        assert_ne!(
            generate(&GeneratorOptions::default()).unwrap(),
            generate(&GeneratorOptions::default()).unwrap()
        );
    }

    #[test]
    fn entropy() {
        assert!(entropy_bits("aaaaaaaaaaaa") < 20.0);
        assert!(entropy_bits("password") < 40.0);
        assert!(entropy_bits("Tr0ub4dor&3-kX9!pq") > 80.0);
    }
}
