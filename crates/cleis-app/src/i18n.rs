//! Interface language (audit U-9, D-71): English in the code, French from `i18n/fr.json`
//! (English text → French text). `tr` gives the text of the language chosen in Preferences;
//! `trf!` fills `{}` (or `{0}`, `{1}`…) in a translated template. A text missing from the file
//! stays in English. The same English word with two meanings carries its context after `@@`
//! (`Copy@@place`: a copy, not the verb), shown without it.

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU8, Ordering};

static LANG: AtomicU8 = AtomicU8::new(0);

static FR: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let map: HashMap<String, String> =
        serde_json::from_str(include_str!("../i18n/fr.json")).unwrap_or_default();
    map.into_iter()
        .map(|(k, v)| -> (&'static str, &'static str) { (k.leak(), v.leak()) })
        .collect()
});

/// Languages offered: `(tag, name in that language)`.
pub const LANGUAGES: [(&str, &str); 2] = [("en", "English"), ("fr", "Français")];

/// Sets the language from a BCP 47 tag (`en`, `fr`, `fr-FR`); anything else is English.
pub fn set_language(tag: &str) {
    let fr = tag
        .split(['-', '_'])
        .next()
        .is_some_and(|p| p.eq_ignore_ascii_case("fr"));
    LANG.store(u8::from(fr), Ordering::Relaxed);
}

/// The tag of the language shown (`en` or `fr`).
pub fn current_tag() -> &'static str {
    if is_french() { "fr" } else { "en" }
}

pub fn is_french() -> bool {
    LANG.load(Ordering::Relaxed) == 1
}

/// The text in the chosen language.
pub fn tr(s: &'static str) -> &'static str {
    let plain = s.split("@@").next().unwrap_or(s);
    if is_french() {
        FR.get(s).copied().unwrap_or(plain)
    } else {
        plain
    }
}

/// A text known only at run time (a label from a list): translated when the file has it.
pub fn tr_owned(s: &str) -> String {
    let plain = s.split("@@").next().unwrap_or(s);
    if is_french() {
        FR.get(s)
            .map_or_else(|| plain.to_owned(), |t| (*t).to_owned())
    } else {
        plain.to_owned()
    }
}

/// Fills `{}` in order, or `{0}`, `{1}`… by position.
pub fn fill(template: &str, args: &[&dyn Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut next = 0;
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('}') {
            Some(j) if after[..j].chars().all(|c| c.is_ascii_digit()) => {
                let at = if j == 0 {
                    next += 1;
                    next - 1
                } else {
                    after[..j].parse().unwrap_or(usize::MAX)
                };
                if let Some(a) = args.get(at) {
                    out.push_str(&a.to_string());
                }
                rest = &after[j + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A message made elsewhere (an error from another crate): the same text in the chosen
/// language, its one `{0}` or `{}` part kept (`the server cannot be reached: {0}`).
pub fn tr_dynamic(s: &str) -> String {
    if is_french() {
        dynamic_in(&FR, s).unwrap_or_else(|| s.to_owned())
    } else {
        s.to_owned()
    }
}

fn dynamic_in(map: &HashMap<&'static str, &'static str>, s: &str) -> Option<String> {
    if let Some(t) = map.get(s) {
        return Some((*t).to_owned());
    }
    // The template whose fixed text matches the most wins (the map has no order).
    map.iter()
        .filter_map(|(en, fr)| {
            let parts = matches(en, s)?;
            let fixed = en.len() - 2 * parts.len();
            let args: Vec<&dyn Display> = parts.iter().map(|p| p as &dyn Display).collect();
            Some((fixed, fill(fr, &args)))
        })
        .max_by_key(|(fixed, _)| *fixed)
        .map(|(_, t)| t)
}

/// The parts of `s` standing for the placeholders of `template` (`{}` or `{N}`, in order),
/// when the fixed text around them matches; none when the template has no placeholder or
/// less than four fixed characters.
fn matches<'a>(template: &str, s: &'a str) -> Option<Vec<&'a str>> {
    let mut fixed = Vec::new();
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        let j = rest[i..].find('}')? + i;
        if !rest[i + 1..j].chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        fixed.push(&rest[..i]);
        rest = &rest[j + 1..];
    }
    fixed.push(rest);
    if fixed.len() < 2 || fixed.iter().map(|f| f.len()).sum::<usize>() < 4 {
        return None;
    }
    let mut at = s.strip_prefix(fixed[0])?;
    let mut parts = Vec::new();
    for (k, f) in fixed.iter().enumerate().skip(1) {
        let end = if k == fixed.len() - 1 {
            at.strip_suffix(f).map(str::len)?
        } else {
            // A placeholder takes at least one character.
            at.get(1..)?.find(f)? + 1
        };
        if end == 0 {
            return None;
        }
        parts.push(&at[..end]);
        at = &at[end + f.len()..];
    }
    Some(parts)
}

/// A space's name as shown: the default names of the personal vault and contacts are translated.
pub fn space(name: &str) -> String {
    if name == "My vault" || name == "My contacts" {
        tr_owned(name)
    } else {
        name.to_owned()
    }
}

/// `trf!("Remove {}?", name)`: the template translated, then filled.
#[macro_export]
macro_rules! trf {
    ($t:literal $(, $a:expr)* $(,)?) => {
        $crate::i18n::fill($crate::i18n::tr($t), &[$(&$a as &dyn ::std::fmt::Display),*])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_messages_from_elsewhere() {
        let d = |s: &str| dynamic_in(&FR, s);
        assert_eq!(d("wrong password").unwrap(), "mot de passe erroné");
        assert_eq!(
            d("the server cannot be reached: timed out").unwrap(),
            "le serveur est injoignable : timed out"
        );
        assert_eq!(d("something else"), None);
        assert_eq!(
            d("Price: price “a lot” not understood").unwrap(),
            "Price : prix « a lot » non compris"
        );
    }

    #[test]
    fn context_after_two_ats() {
        // English shows the word alone; French has its own text for this meaning.
        assert_eq!(tr("Copy@@place"), "Copy");
        assert_eq!(FR.get("Copy@@place").copied(), Some("Copie"));
        assert_eq!(FR.get("Copy").copied(), Some("Copier"));
    }

    #[test]
    fn fills_and_translates() {
        assert_eq!(fill("Remove {}?", &[&"Hugo"]), "Remove Hugo?");
        assert_eq!(fill("{1} then {0}", &[&1, &2]), "2 then 1");
        assert_eq!(fill("a {b} c", &[]), "a {b} c");
        // Every French template keeps the placeholders of its English one.
        for (en, fr) in FR.iter() {
            let count = |s: &str| s.matches("{}").count();
            let indexed = fr.contains("{0}") || fr.contains("{1}");
            assert!(
                indexed || count(en) == count(fr),
                "placeholders differ: {en:?} → {fr:?}"
            );
        }
    }
}
