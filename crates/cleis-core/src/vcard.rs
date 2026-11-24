//! vCard import/export. Reads 2.1, 3.0 (RFC 2426) and 4.0 (RFC 6350); writes 4.0.
//! Unknown properties are ignored on import; Cleis-only data (custom columns, links) is
//! not exported to vCard.

use crate::contact::{Address, Contact, ContactKind, Labeled, PartialDate};
use crate::ids::SpaceId;

/// One logical property line after unfolding.
#[derive(Debug, Clone, PartialEq)]
struct Prop {
    name: String,
    params: Vec<(String, String)>,
    value: String,
}

impl Prop {
    fn types(&self) -> Vec<String> {
        let mut out = vec![];
        for (k, v) in &self.params {
            if k == "TYPE" {
                out.extend(
                    v.split(',')
                        .map(|s| s.trim_matches('"').to_ascii_lowercase()),
                );
            } else if v.is_empty() {
                // vCard 2.1: `TEL;WORK;VOICE:…`
                out.push(k.to_ascii_lowercase());
            }
        }
        out
    }
    fn pref(&self) -> bool {
        self.types().iter().any(|t| t == "pref")
            || self.params.iter().any(|(k, v)| k == "PREF" && v == "1")
    }
    fn charset_qp(&self) -> bool {
        self.params.iter().any(|(k, v)| {
            (k == "ENCODING" && v.eq_ignore_ascii_case("QUOTED-PRINTABLE"))
                || (v.is_empty() && k == "QUOTED-PRINTABLE")
        })
    }
}

/// Undoes line folding (CRLF/LF followed by a space or tab) and quoted-printable soft breaks.
fn unfold(src: &str) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    for raw in src.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(rest) = line.strip_prefix([' ', '\t'])
            && let Some(last) = lines.last_mut()
        {
            last.push_str(rest);
            continue;
        }
        if let Some(last) = lines.last_mut()
            && last.ends_with('=')
            && last.to_ascii_uppercase().contains("QUOTED-PRINTABLE")
        {
            last.pop();
            last.push_str(line);
            continue;
        }
        lines.push(line.to_owned());
    }
    lines.into_iter().filter(|l| !l.trim().is_empty()).collect()
}

fn parse_line(line: &str) -> Option<Prop> {
    // Find the first ':' not inside double quotes.
    let mut in_q = false;
    let mut colon = None;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_q = !in_q,
            ':' if !in_q => {
                colon = Some(i);
                break;
            }
            _ => {}
        }
    }
    let colon = colon?;
    let (head, value) = (&line[..colon], &line[colon + 1..]);
    let mut parts = head.split(';');
    let name = parts.next()?;
    let name = name.rsplit('.').next()?.trim().to_ascii_uppercase(); // drop "item1." groups
    let params = parts
        .map(|p| match p.split_once('=') {
            Some((k, v)) => (k.trim().to_ascii_uppercase(), v.trim().to_owned()),
            None => (p.trim().to_ascii_uppercase(), String::new()),
        })
        .collect();
    Some(Prop {
        name,
        params,
        value: value.to_owned(),
    })
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n' | 'N') => out.push('\n'),
                Some(o) => out.push(o),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Splits on unescaped `sep`, then unescapes each component.
fn split_components(s: &str, sep: char) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut esc = false;
    for c in s.chars() {
        if esc {
            cur.push('\\');
            cur.push(c);
            esc = false;
        } else if c == '\\' {
            esc = true;
        } else if c == sep {
            out.push(unescape(&cur));
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    out.push(unescape(&cur));
    out
}

fn decode_qp(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'='
            && i + 2 < b.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn label_from(types: &[String], fallback: &str) -> String {
    for (t, l) in [
        ("cell", "Mobile"),
        ("mobile", "Mobile"),
        ("work", "Work"),
        ("home", "Personal"),
        ("fax", "Fax"),
        ("pager", "Pager"),
        ("main", "Main"),
    ] {
        if types.iter().any(|x| x == t) {
            return l.to_owned();
        }
    }
    types
        .iter()
        .find(|t| !["pref", "voice", "internet", "x400", "text", "uri"].contains(&t.as_str()))
        .map(|t| {
            let t = t.trim_start_matches("x-");
            let mut c = t.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect())
                .unwrap_or_default()
        })
        .unwrap_or_else(|| fallback.to_owned())
}

/// Parses every `BEGIN:VCARD … END:VCARD` block into contacts placed in `space`.
pub fn parse(src: &str, space: SpaceId) -> Vec<Contact> {
    let mut out = vec![];
    let mut cur: Option<Contact> = None;
    let mut fn_value = String::new();
    for line in unfold(src.trim_start_matches('\u{feff}')) {
        let Some(mut p) = parse_line(&line) else {
            continue;
        };
        if p.charset_qp() {
            p.value = decode_qp(&p.value);
        }
        match p.name.as_str() {
            "BEGIN" if p.value.eq_ignore_ascii_case("VCARD") => {
                cur = Some(Contact::new(space));
                fn_value.clear();
            }
            "END" if p.value.eq_ignore_ascii_case("VCARD") => {
                if let Some(mut c) = cur.take() {
                    if c.given.is_empty()
                        && c.family.is_empty()
                        && !fn_value.is_empty()
                        && c.kind == ContactKind::Person
                    {
                        let mut words: Vec<&str> = fn_value.split_whitespace().collect();
                        if words.len() >= 2 {
                            c.family = words.pop().unwrap_or_default().to_owned();
                            c.given = words.join(" ");
                        } else if c.company.is_empty() {
                            c.given = fn_value.clone();
                        }
                    }
                    out.push(c);
                }
            }
            _ => {
                let Some(c) = cur.as_mut() else { continue };
                let types = p.types();
                let pref = p.pref();
                match p.name.as_str() {
                    "FN" => fn_value = unescape(&p.value),
                    "N" => {
                        let n = split_components(&p.value, ';');
                        let g = |i: usize| {
                            n.get(i)
                                .cloned()
                                .unwrap_or_default()
                                .replace(',', " ")
                                .trim()
                                .to_owned()
                        };
                        c.family = g(0);
                        c.given = g(1);
                        c.additional = g(2);
                        c.prefix = g(3);
                        c.suffix = g(4);
                    }
                    "NICKNAME" => c.nickname = unescape(&p.value),
                    "ORG" => {
                        let o = split_components(&p.value, ';');
                        c.company = o.first().cloned().unwrap_or_default();
                        c.department = o.get(1).cloned().unwrap_or_default();
                    }
                    "TITLE" => c.job_title = unescape(&p.value),
                    "KIND" | "X-ABSHOWAS"
                        if p.value.eq_ignore_ascii_case("org")
                            || p.value.eq_ignore_ascii_case("company") =>
                    {
                        c.kind = ContactKind::Organization
                    }
                    "EMAIL" => c.emails.push(Labeled {
                        label: label_from(&types, "Email"),
                        value: unescape(&p.value).trim().to_owned(),
                        preferred: pref,
                    }),
                    "TEL" => {
                        let v = unescape(&p.value);
                        let v = v.strip_prefix("tel:").unwrap_or(&v).trim().to_owned();
                        c.phones.push(Labeled {
                            label: label_from(&types, "Phone"),
                            value: v,
                            preferred: pref,
                        });
                    }
                    "ADR" => {
                        let a = split_components(&p.value, ';');
                        let g = |i: usize| a.get(i).cloned().unwrap_or_default().trim().to_owned();
                        c.addresses.push(Labeled {
                            label: label_from(&types, "Address"),
                            value: Address {
                                po_box: g(0),
                                extended: g(1),
                                street: g(2),
                                city: g(3),
                                region: g(4),
                                postal_code: g(5),
                                country: g(6),
                            },
                            preferred: pref,
                        });
                    }
                    "URL" => c.urls.push(Labeled {
                        label: label_from(&types, "Website"),
                        value: unescape(&p.value),
                        preferred: pref,
                    }),
                    "BDAY" => c.birthday = PartialDate::parse(&p.value),
                    "ANNIVERSARY" | "X-ANNIVERSARY" => c.anniversary = PartialDate::parse(&p.value),
                    "NOTE" => {
                        if !c.notes.is_empty() {
                            c.notes.push('\n');
                        }
                        c.notes.push_str(&unescape(&p.value));
                    }
                    "CATEGORIES" => c.labels.extend(
                        split_components(&p.value, ',')
                            .into_iter()
                            .map(|s| s.trim().to_owned())
                            .filter(|s| !s.is_empty()),
                    ),
                    _ => {}
                }
            }
        }
    }
    out
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace(',', "\\,")
        .replace(';', "\\;")
}

/// Folds at 75 octets without splitting a UTF-8 character (RFC 6350 §3.2).
fn fold_line(line: &str, out: &mut String) {
    let mut count = 0;
    for ch in line.chars() {
        let n = ch.len_utf8();
        if count + n > 75 {
            out.push_str("\r\n ");
            count = 1;
        }
        out.push(ch);
        count += n;
    }
    out.push_str("\r\n");
}

fn type_param(label: &str) -> String {
    match crate::text::fold(label).as_str() {
        "mobile" => ";TYPE=cell".into(),
        "work" => ";TYPE=work".into(),
        "personal" | "home" => ";TYPE=home".into(),
        "fax" => ";TYPE=fax".into(),
        _ => String::new(),
    }
}

/// Serialises contacts as vCard 4.0.
pub fn write(contacts: &[&Contact]) -> String {
    let mut out = String::new();
    for c in contacts {
        let mut lines = vec!["BEGIN:VCARD".to_owned(), "VERSION:4.0".to_owned()];
        if c.kind == ContactKind::Organization {
            lines.push("KIND:org".into());
        }
        lines.push(format!("FN:{}", escape(&c.display_name())));
        lines.push(format!(
            "N:{};{};{};{};{}",
            escape(&c.family),
            escape(&c.given),
            escape(&c.additional),
            escape(&c.prefix),
            escape(&c.suffix)
        ));
        if !c.nickname.is_empty() {
            lines.push(format!("NICKNAME:{}", escape(&c.nickname)));
        }
        if !c.company.is_empty() || !c.department.is_empty() {
            lines.push(
                format!("ORG:{};{}", escape(&c.company), escape(&c.department))
                    .trim_end_matches(';')
                    .to_owned(),
            );
        }
        if !c.job_title.is_empty() {
            lines.push(format!("TITLE:{}", escape(&c.job_title)));
        }
        let pref = |p: bool| if p { ";PREF=1" } else { "" };
        for e in &c.emails {
            lines.push(format!(
                "EMAIL{}{}:{}",
                type_param(&e.label),
                pref(e.preferred),
                escape(&e.value)
            ));
        }
        for t in &c.phones {
            lines.push(format!(
                "TEL;VALUE=uri{}{}:tel:{}",
                type_param(&t.label),
                pref(t.preferred),
                t.value.replace(' ', "-")
            ));
        }
        for a in &c.addresses {
            let v = &a.value;
            lines.push(format!(
                "ADR{}{}:{};{};{};{};{};{};{}",
                type_param(&a.label),
                pref(a.preferred),
                escape(&v.po_box),
                escape(&v.extended),
                escape(&v.street),
                escape(&v.city),
                escape(&v.region),
                escape(&v.postal_code),
                escape(&v.country)
            ));
        }
        for u in &c.urls {
            lines.push(format!("URL{}:{}", type_param(&u.label), u.value));
        }
        if let Some(b) = c.birthday {
            lines.push(format!("BDAY:{}", b.to_vcard()));
        }
        if let Some(b) = c.anniversary {
            lines.push(format!("ANNIVERSARY:{}", b.to_vcard()));
        }
        if !c.labels.is_empty() {
            lines.push(format!(
                "CATEGORIES:{}",
                c.labels
                    .iter()
                    .map(|l| escape(l))
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        if !c.notes.is_empty() {
            lines.push(format!("NOTE:{}", escape(&c.notes)));
        }
        lines.push(format!("UID:urn:uuid:{}", c.id));
        lines.push("END:VCARD".into());
        for l in lines {
            fold_line(&l, &mut out);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const V3: &str = "BEGIN:VCARD\r\nVERSION:3.0\r\nN:Dubois;Camille;;;\r\nFN:Camille Dubois\r\nORG:Example Print;Sales\r\nTITLE:Account manager\r\nitem1.EMAIL;TYPE=INTERNET;TYPE=WORK;TYPE=pref:camille@example-print.com\r\nEMAIL;TYPE=HOME:camille.d@example.com\r\nTEL;TYPE=CELL:+33 6 99 00 12 35\r\nTEL;TYPE=WORK,VOICE:+33 1 99 00 12 34\r\nADR;TYPE=WORK:;;12 rue de l'Exemple;Lyon;;69002;France\r\nBDAY:--0312\r\nCATEGORIES:Supplier,VIP\r\nNOTE:Quotes within 48 h\\, prefers email\\nSecond line which is long enough to be folded by\r\n  the writer of this card\r\nEND:VCARD\r\n";

    #[test]
    fn parses_v3() {
        let cs = parse(V3, SpaceId::new());
        assert_eq!(cs.len(), 1);
        let c = &cs[0];
        assert_eq!(c.display_name(), "Camille Dubois");
        assert_eq!(
            (
                c.company.as_str(),
                c.department.as_str(),
                c.job_title.as_str()
            ),
            ("Example Print", "Sales", "Account manager")
        );
        assert_eq!(
            c.primary_email().as_deref(),
            Some("camille@example-print.com")
        );
        assert_eq!(c.emails[1].label, "Personal");
        assert_eq!(c.phones[0].label, "Mobile");
        assert_eq!(c.phones[1].label, "Work");
        assert_eq!(c.city().as_deref(), Some("Lyon"));
        assert_eq!(c.addresses[0].value.street, "12 rue de l'Exemple");
        assert_eq!(c.birthday, PartialDate::parse("--0312"));
        assert!(c.labels.contains("VIP"));
        assert_eq!(
            c.notes,
            "Quotes within 48 h, prefers email\nSecond line which is long enough to be folded by the writer of this card"
        );
    }

    #[test]
    fn round_trip_v4() {
        let mut c = parse(V3, SpaceId::new()).remove(0);
        c.notes = "Ligne avec accents éèà et ; virgule, et un texte assez long pour dépasser soixante-quinze octets".into();
        let s = write(&[&c]);
        assert!(s.lines().all(|l| l.len() <= 75));
        let back = parse(&s, c.space).remove(0);
        assert_eq!(back.display_name(), c.display_name());
        assert_eq!(back.notes, c.notes);
        assert_eq!(back.emails, c.emails);
        assert_eq!(back.phones.len(), 2);
        assert_eq!(back.addresses, c.addresses);
        assert_eq!(back.labels, c.labels);
        assert_eq!(back.birthday, c.birthday);
    }

    #[test]
    fn v21_and_fn_only() {
        let src = "BEGIN:VCARD\nVERSION:2.1\nFN:Paul Lef=C3=A8vre\nTEL;WORK;VOICE:0199001234\nEND:VCARD\nBEGIN:VCARD\nVERSION:2.1\nFN;ENCODING=QUOTED-PRINTABLE;CHARSET=UTF-8:Paul Lef=C3=A8vre\nEND:VCARD\n";
        let cs = parse(src, SpaceId::new());
        assert_eq!(cs.len(), 2);
        assert_eq!(cs[0].phones[0].label, "Work");
        assert_eq!(cs[1].display_name(), "Paul Lefèvre");
    }
}
