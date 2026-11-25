//! CSV import (screen 14): reads exports from Notion, spreadsheets, browsers and other
//! password managers, proposes a column mapping, and builds entries.

use crate::column::{ColumnDef, Value};
use crate::entry::{Entry, Subscription};
use crate::ids::{ColumnId, SpaceId};
use crate::money::{Money, Period, period_hint};
use crate::text::fold;
use crate::totp::Totp;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Name,
    Username,
    Password,
    Website,
    Notes,
    Totp,
    Subscription,
    Price,
    Licenses,
    Renewal,
    /// Into an existing or new custom column.
    Custom(ColumnId),
    Skip,
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("the file could not be read as CSV: {0}")]
    Csv(#[from] csv::Error),
    #[error("the file has no header row")]
    NoHeader,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mapping {
    pub header: String,
    pub target: Target,
    /// True when the target was guessed (shown as "matched automatically").
    pub automatic: bool,
    pub sample: String,
}

#[derive(Debug)]
pub struct Preview {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub mappings: Vec<Mapping>,
    pub delimiter: u8,
}

/// Header synonyms (folded). Order matters: the first rule that matches wins.
type Rule = (&'static [&'static str], fn() -> Target);
const RULES: &[Rule] = &[
    (
        &[
            "cout total",
            "total cost",
            "yearly cost",
            "cout annuel",
            "total",
        ],
        || Target::Skip,
    ),
    (
        &[
            "site/app",
            "site / app",
            "name",
            "nom",
            "title",
            "titre",
            "service",
            "site",
            "app",
            "application",
            "compte",
        ],
        || Target::Name,
    ),
    (
        &[
            "username",
            "login",
            "identifiant",
            "id",
            "user",
            "utilisateur",
            "email",
            "e-mail",
            "mail",
            "login_username",
        ],
        || Target::Username,
    ),
    (
        &[
            "password",
            "mdp",
            "mot de passe",
            "pass",
            "pwd",
            "login_password",
        ],
        || Target::Password,
    ),
    (
        &[
            "url",
            "website",
            "site web",
            "lien",
            "link",
            "adresse",
            "login_uri",
        ],
        || Target::Website,
    ),
    (
        &[
            "totp",
            "2fa",
            "otp",
            "otpauth",
            "authenticator",
            "login_totp",
        ],
        || Target::Totp,
    ),
    (
        &["abonnement", "subscription", "statut", "status", "etat"],
        || Target::Subscription,
    ),
    (
        &[
            "n° de licences",
            "nb licences",
            "licences",
            "licenses",
            "seats",
            "nombre de licences",
            "postes",
        ],
        || Target::Licenses,
    ),
    (
        &[
            "prix", "price", "tarif", "cout", "cost", "montant", "amount",
        ],
        || Target::Price,
    ),
    (
        &[
            "renouvellement",
            "renewal",
            "echeance",
            "expiration",
            "date de fin",
        ],
        || Target::Renewal,
    ),
    (
        &[
            "notes",
            "note",
            "en +",
            "commentaire",
            "commentaires",
            "comments",
            "remarques",
            "extra",
            "infos",
        ],
        || Target::Notes,
    ),
];

fn guess(header: &str) -> Option<Target> {
    let h = fold(header);
    let base = h.split('(').next().unwrap_or(&h).trim().to_owned();
    for (keys, t) in RULES {
        if keys.iter().any(|k| base == *k) {
            return Some(t());
        }
    }
    for (keys, t) in RULES {
        if keys
            .iter()
            .filter(|k| k.len() >= 4)
            .any(|k| base.contains(k))
        {
            return Some(t());
        }
    }
    None
}

/// Bytes of a file as text: UTF-8 (with or without BOM), UTF-16 with BOM (Excel "Unicode
/// text"), otherwise Windows-1252 (Excel "CSV" on a French or Western Windows).
pub fn decode(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    let utf16 = |rest: &[u8], le: bool| {
        let units: Vec<u16> = rest
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| {
                if le {
                    u16::from_le_bytes(*c)
                } else {
                    u16::from_be_bytes(*c)
                }
            })
            .collect();
        String::from_utf16_lossy(&units)
    };
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return utf16(rest, true);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return utf16(rest, false);
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) => bytes.iter().map(|&b| cp1252(b)).collect(),
    }
}

/// Windows-1252: Latin-1 plus typographic characters in 0x80–0x9F.
pub(crate) fn cp1252(b: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}',
        'ž', 'Ÿ',
    ];
    match b {
        0x80..=0x9F => HIGH[usize::from(b - 0x80)],
        _ => char::from(b),
    }
}

/// Picks the separator that occurs most in the header row, ignoring quoted text (a header
/// such as `"Service, app"` must not turn a `;` file into a `,` one).
pub(crate) fn sniff_delimiter(sample: &str) -> u8 {
    let mut counts = [0usize; 3];
    let mut quoted = false;
    for c in sample.chars() {
        match c {
            '"' => quoted = !quoted,
            '\n' | '\r' if !quoted => break,
            ',' if !quoted => counts[0] += 1,
            ';' if !quoted => counts[1] += 1,
            '\t' if !quoted => counts[2] += 1,
            _ => {}
        }
    }
    let i = (0..3).max_by_key(|&i| (counts[i], 3 - i)).unwrap_or(0);
    b",;\t"[i]
}

/// Reads the file and proposes a mapping. Each target is used once; later duplicates
/// are skipped so the user decides.
pub fn preview(data: &str) -> Result<Preview, ImportError> {
    let data = data.trim_start_matches('\u{feff}');
    let delimiter = sniff_delimiter(data);
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_reader(data.as_bytes());
    let headers: Vec<String> = rdr.headers()?.iter().map(|h| h.trim().to_owned()).collect();
    if headers.iter().all(String::is_empty) {
        return Err(ImportError::NoHeader);
    }
    let rows: Vec<Vec<String>> = rdr
        .records()
        .filter_map(Result::ok)
        .map(|r| r.iter().map(|c| c.trim().to_owned()).collect::<Vec<_>>())
        .filter(|r| r.iter().any(|c| !c.is_empty()))
        .collect();
    let mut used: Vec<Target> = vec![];
    let mappings = headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let sample = rows
                .iter()
                .map(|r| r.get(i).cloned().unwrap_or_default())
                .find(|s| !s.is_empty())
                .unwrap_or_default();
            let sample = if matches!(guess(h), Some(Target::Password)) {
                "••••••••".to_owned()
            } else {
                sample
            };
            match guess(h) {
                Some(t) if t == Target::Skip || !used.contains(&t) => {
                    used.push(t.clone());
                    Mapping {
                        header: h.clone(),
                        target: t,
                        automatic: true,
                        sample,
                    }
                }
                _ => Mapping {
                    header: h.clone(),
                    target: Target::Skip,
                    automatic: false,
                    sample,
                },
            }
        })
        .collect();
    Ok(Preview {
        headers,
        rows,
        mappings,
        delimiter,
    })
}

fn subscription(v: &str) -> Subscription {
    let f = fold(v);
    match f.as_str() {
        "" => Subscription::None,
        s if s.contains("renouvel") || s.contains("renew") || s.contains("bientot") => {
            Subscription::RenewSoon
        }
        s if s.starts_with("actif")
            || s.starts_with("active")
            || s == "oui"
            || s == "yes"
            || s == "paye" =>
        {
            Subscription::Active
        }
        s if s.contains("gratuit") || s.contains("free") => Subscription::Free,
        s if s.contains("en cours")
            || s.contains("pending")
            || s.contains("attente")
            || s.contains("essai")
            || s.contains("trial") =>
        {
            Subscription::Pending
        }
        s if s == "off"
            || s.contains("inactif")
            || s.contains("resilie")
            || s.contains("annule")
            || s.contains("cancel")
            || s == "non"
            || s == "no" =>
        {
            Subscription::Off
        }
        _ => Subscription::None,
    }
}

/// Month number from an English or French name or abbreviation (`December`, `déc.`, `Feb`).
pub(crate) fn month(word: &str) -> Option<u32> {
    let w = fold(word.trim_end_matches('.'));
    const NAMES: [(&str, &str); 12] = [
        ("jan", "janv"),
        ("feb", "fev"),
        ("mar", "mars"),
        ("apr", "avr"),
        ("may", "mai"),
        ("jun", "juin"),
        ("jul", "juil"),
        ("aug", "aout"),
        ("sep", "sept"),
        ("oct", "oct"),
        ("nov", "nov"),
        ("dec", "dec"),
    ];
    if w.len() < 3 {
        return None;
    }
    NAMES
        .iter()
        .position(|(en, fr)| w.starts_with(en) || w.starts_with(fr))
        .map(|i| i as u32 + 1)
}

/// ISO date from `2026-12-03`, `03/12/2026` (day first), `12/3/2026` when unambiguous, and
/// the display formats of Notion and spreadsheets: `December 3, 2026`, `3 décembre 2026`,
/// `Dec 3, 2026 10:00 AM`. A range (`A → B`) gives its start.
pub fn parse_date(v: &str) -> Option<String> {
    let v = v.split('→').next().unwrap_or(v).trim();
    let ok = |y: u32, m: u32, d: u32| {
        let y = if y < 100 { 2000 + y } else { y };
        ((1..=12).contains(&m) && (1..=31).contains(&d) && (1900..=2200).contains(&y))
            .then(|| format!("{y:04}-{m:02}-{d:02}"))
    };
    // Words: "December 3, 2026 10:00 AM" / "3 décembre 2026" / "1er janv. 2027".
    let words: Vec<&str> = v
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| !w.is_empty())
        .collect();
    if let Some(mi) = words.iter().position(|w| month(w).is_some()) {
        let m = month(words[mi])?;
        let num = |w: &str| -> Option<u32> {
            w.trim_end_matches("er")
                .trim_end_matches(['s', 't', 'n', 'd', 'h', 'r'])
                .parse()
                .ok()
        };
        let nums: Vec<u32> = words
            .iter()
            .enumerate()
            .filter(|(i, w)| *i != mi && !w.contains(':'))
            .filter_map(|(_, w)| num(w))
            .collect();
        let (d, y) = match nums.as_slice() {
            [a, b, ..] if *a <= 31 => (*a, *b),
            [a, b, ..] => (*b, *a),
            _ => return None,
        };
        return ok(y, m, d);
    }
    let date = v.split([' ', 'T']).next().unwrap_or(v);
    let parts: Vec<&str> = date.split(['-', '/', '.']).collect();
    if parts.len() != 3 {
        return None;
    }
    let n: Vec<u32> = parts
        .iter()
        .map(|p| p.trim().parse().ok())
        .collect::<Option<_>>()?;
    let (y, m, d) = if parts[0].len() == 4 {
        (n[0], n[1], n[2])
    } else if n[1] > 12 {
        (n[2], n[0], n[1])
    } else {
        (n[2], n[1], n[0])
    };
    ok(y, m, d)
}

#[derive(Debug, Default)]
pub struct Outcome {
    pub entries: Vec<Entry>,
    /// (row number starting at 2 like a spreadsheet, message)
    pub warnings: Vec<(usize, String)>,
}

/// Builds entries with the (possibly user-edited) mappings. Values for custom columns are read
/// with the column's type (`cols`).
pub fn build(
    p: &Preview,
    mappings: &[Mapping],
    space: SpaceId,
    currency: &str,
    cols: &[ColumnDef],
) -> Outcome {
    let mut out = Outcome::default();
    for (ri, row) in p.rows.iter().enumerate() {
        let line = ri + 2;
        let mut e = Entry::new(space, "");
        for (ci, m) in mappings.iter().enumerate() {
            let v = row.get(ci).map(String::as_str).unwrap_or("").trim();
            if v.is_empty() {
                continue;
            }
            match &m.target {
                Target::Name => e.name = v.to_owned(),
                Target::Username => e.username = v.to_owned(),
                Target::Password => e.password = v.to_owned(),
                Target::Website => e.urls.push(v.to_owned()),
                Target::Notes => {
                    if !e.notes.is_empty() {
                        e.notes.push('\n');
                    }
                    e.notes.push_str(v);
                }
                Target::Totp => match Totp::parse(v) {
                    Ok(t) => e.totp = Some(t),
                    Err(_) => out
                        .warnings
                        .push((line, format!("{}: 2FA secret not recognised", m.header))),
                },
                Target::Subscription => {
                    e.subscription = subscription(v);
                    if e.subscription == Subscription::None {
                        out.warnings
                            .push((line, format!("{}: unknown status “{v}”", m.header)));
                    }
                }
                Target::Price => {
                    match Money::parse(v, currency, period_hint(&m.header).or(Some(Period::Month)))
                    {
                        Some(mo) => e.price = Some(mo),
                        None => {
                            if !matches!(fold(v).as_str(), "gratuit" | "free" | "0" | "-") {
                                out.warnings.push((
                                    line,
                                    format!("{}: price “{v}” not understood", m.header),
                                ));
                            }
                        }
                    }
                }
                Target::Licenses => match v.trim().parse::<u32>() {
                    Ok(n) => e.licenses = Some(n),
                    Err(_) => out
                        .warnings
                        .push((line, format!("{}: “{v}” is not a number", m.header))),
                },
                Target::Renewal => match parse_date(v) {
                    Some(d) => e.renewal = Some(d),
                    None => out
                        .warnings
                        .push((line, format!("{}: date “{v}” not understood", m.header))),
                },
                Target::Custom(id) => match cols.iter().find(|c| &c.id == id) {
                    Some(c) => match Value::parse(&c.kind, v) {
                        Ok(Some(val)) => {
                            e.fields.insert(id.clone(), val);
                        }
                        Ok(None) => {}
                        Err(_) => out.warnings.push((
                            line,
                            format!("{}: “{v}” does not fit the column {}", m.header, c.name),
                        )),
                    },
                    None => {
                        e.fields.insert(id.clone(), Value::Text(v.to_owned()));
                    }
                },
                Target::Skip => {}
            }
        }
        if e.name.is_empty() {
            if let Some(h) = e.host() {
                e.name = h;
            } else if !e.username.is_empty() {
                e.name = e.username.clone();
            } else {
                out.warnings.push((line, "empty row skipped".into()));
                continue;
            }
        }
        out.entries.push(e);
    }
    out
}

/// Items of `new` already in `existing`: same space, name and username (letter case and
/// accents ignored).
pub fn already_there(new: &[Entry], existing: &[Entry]) -> Vec<bool> {
    let key = |e: &Entry| (e.space, fold(&e.name), fold(&e.username));
    let have: std::collections::HashSet<_> = existing.iter().map(key).collect();
    new.iter().map(|e| have.contains(&key(e))).collect()
}

/// Display names of the import targets, in menu order.
pub fn target_label(t: &Target) -> &'static str {
    match t {
        Target::Name => "Name",
        Target::Username => "Username",
        Target::Password => "Password",
        Target::Website => "Website",
        Target::Notes => "Notes",
        Target::Totp => "2FA code",
        Target::Subscription => "Subscription",
        Target::Price => "Price",
        Target::Licenses => "Licenses",
        Target::Renewal => "Renewal",
        Target::Custom(_) => "Custom",
        Target::Skip => "Skip",
    }
}

/// Built-in targets offered in the mapping menu (custom columns are added after them).
pub const TARGETS: [Target; 11] = [
    Target::Name,
    Target::Username,
    Target::Password,
    Target::Website,
    Target::Notes,
    Target::Totp,
    Target::Subscription,
    Target::Price,
    Target::Licenses,
    Target::Renewal,
    Target::Skip,
];

#[cfg(test)]
mod tests {
    use super::*;

    const NOTION: &str = "\u{feff}Site/app,ID,MDP,URL,Abonnement,Prix (mensuel),N° de licences,Cout total,En +\n\
Figma,design@example-studio.com,s3cret-Figma!,https://figma.com,actif,15 €,4,60 €,Compte équipe\n\
Cloudflare,admin@example-studio.com,cf-Pass-2026,cloudflare.com,gratuit,,,,\n\
Webflow,design@example-studio.com,wf,webflow.com,en cours,\"29,00 €\",1,29 €,\n\
,,,,,,,,\n";

    #[test]
    fn notion_export() {
        let p = preview(NOTION).unwrap();
        let t: Vec<Target> = p.mappings.iter().map(|m| m.target.clone()).collect();
        assert_eq!(
            t,
            vec![
                Target::Name,
                Target::Username,
                Target::Password,
                Target::Website,
                Target::Subscription,
                Target::Price,
                Target::Licenses,
                Target::Skip,
                Target::Notes
            ]
        );
        assert_eq!(p.mappings[2].sample, "••••••••");
        let o = build(&p, &p.mappings, SpaceId::new(), "EUR", &[]);
        assert_eq!(o.entries.len(), 3, "{:?}", o.warnings);
        let f = &o.entries[0];
        assert_eq!(
            (f.name.as_str(), f.username.as_str(), f.password.as_str()),
            ("Figma", "design@example-studio.com", "s3cret-Figma!")
        );
        assert_eq!(f.price, Some(Money::new(1500, "EUR", Some(Period::Month))));
        assert_eq!(
            (f.licenses, f.subscription),
            (Some(4), Subscription::Active)
        );
        assert_eq!(f.notes, "Compte équipe");
        assert_eq!(o.entries[1].subscription, Subscription::Free);
        assert_eq!(o.entries[2].subscription, Subscription::Pending);
        assert_eq!(o.entries[2].price.as_ref().unwrap().cents, 2900);
    }

    #[test]
    fn semicolons_and_dates() {
        let p =
            preview("name;login_username;login_password;renewal\nBank;0042 1187;x;03/12/2026\n")
                .unwrap();
        assert_eq!(p.delimiter, b';');
        let o = build(&p, &p.mappings, SpaceId::new(), "EUR", &[]);
        assert_eq!(o.entries[0].renewal.as_deref(), Some("2026-12-03"));
        assert_eq!(parse_date("2026-12-03").as_deref(), Some("2026-12-03"));
        assert_eq!(parse_date("12/31/2026").as_deref(), Some("2026-12-31"));
        assert_eq!(parse_date("nope"), None);
    }
}
