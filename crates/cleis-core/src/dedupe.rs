//! "Merge & fix" for contacts (screen 21): finds probable duplicates and merges them.

use crate::contact::{Contact, Labeled};
use crate::ids::ContactId;
use crate::text::fold;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    SameEmail,
    SamePhone,
    SimilarName,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateGroup {
    pub contacts: Vec<ContactId>,
    pub reason: Reason,
}

fn norm_email(e: &str) -> String {
    e.trim().to_lowercase()
}

/// Compares phone numbers on their last 9 digits, which absorbs national prefixes
/// (`06 99…` vs `+33 6 99…`) without guessing a country.
fn norm_phone(p: &str) -> Option<String> {
    let d: String = p.chars().filter(char::is_ascii_digit).collect();
    (d.len() >= 8).then(|| d[d.len().saturating_sub(9)..].to_owned())
}

fn name_key(c: &Contact) -> Option<(String, String)> {
    let family = fold(&c.family);
    let given = fold(&c.given);
    (!family.is_empty()).then_some((family, given))
}

fn similar_names(a: &Contact, b: &Contact) -> bool {
    let (Some((fa, ga)), Some((fb, gb))) = (name_key(a), name_key(b)) else {
        let (x, y) = (fold(&a.display_name()), fold(&b.display_name()));
        return !x.is_empty() && x == y;
    };
    if strsim::jaro_winkler(&fa, &fb) < 0.94 {
        return false;
    }
    let ga = ga.trim_end_matches('.');
    let gb = gb.trim_end_matches('.');
    if ga.is_empty() || gb.is_empty() {
        return true;
    }
    // "C." vs "Camille", or close spellings.
    (ga.len() == 1 && gb.starts_with(ga))
        || (gb.len() == 1 && ga.starts_with(gb))
        || strsim::jaro_winkler(ga, gb) >= 0.92
}

/// Union-find over contacts; each found group is reported with the strongest reason.
pub fn find_duplicates(
    contacts: &[Contact],
    dismissed: &BTreeSet<(ContactId, ContactId)>,
) -> Vec<DuplicateGroup> {
    let n = contacts.len();
    let mut parent: Vec<usize> = (0..n).collect();
    let mut reason: BTreeMap<usize, Reason> = BTreeMap::new();
    fn find(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    let mut link = |p: &mut Vec<usize>, a: usize, b: usize, r: Reason| {
        let pair = if contacts[a].id < contacts[b].id {
            (contacts[a].id, contacts[b].id)
        } else {
            (contacts[b].id, contacts[a].id)
        };
        if dismissed.contains(&pair) {
            return;
        }
        let (ra, rb) = (find(p, a), find(p, b));
        if ra != rb {
            p[rb] = ra;
        }
        let root = find(p, a);
        let e = reason.entry(root).or_insert(r);
        *e = (*e).min(r);
    };
    let mut by_email: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_phone: BTreeMap<String, usize> = BTreeMap::new();
    for (i, c) in contacts.iter().enumerate() {
        for e in &c.emails {
            let k = norm_email(&e.value);
            if k.is_empty() {
                continue;
            }
            match by_email.get(&k) {
                Some(&j) if j != i => link(&mut parent, j, i, Reason::SameEmail),
                Some(_) => {}
                None => {
                    by_email.insert(k, i);
                }
            }
        }
        for p in &c.phones {
            if let Some(k) = norm_phone(&p.value) {
                match by_phone.get(&k) {
                    Some(&j) if j != i => link(&mut parent, j, i, Reason::SamePhone),
                    Some(_) => {}
                    None => {
                        by_phone.insert(k, i);
                    }
                }
            }
        }
    }
    // Name similarity: bucket by the first letter of the folded family name to stay ~linear.
    let mut buckets: BTreeMap<char, Vec<usize>> = BTreeMap::new();
    for (i, c) in contacts.iter().enumerate() {
        let k = name_key(c)
            .map(|(f, _)| f)
            .unwrap_or_else(|| fold(&c.display_name()));
        if let Some(ch) = k.chars().next() {
            buckets.entry(ch).or_default().push(i);
        }
    }
    for idx in buckets.values() {
        for (x, &a) in idx.iter().enumerate() {
            for &b in &idx[x + 1..] {
                if similar_names(&contacts[a], &contacts[b]) {
                    link(&mut parent, a, b, Reason::SimilarName);
                }
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<ContactId>> = BTreeMap::new();
    for (i, c) in contacts.iter().enumerate() {
        let r = find(&mut parent, i);
        groups.entry(r).or_default().push(c.id);
    }
    let mut out: Vec<DuplicateGroup> = groups
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(root, contacts)| {
            // The recorded reason may sit on an older root; take the best among members.
            let r = reason
                .iter()
                .filter(|(k, _)| find(&mut parent, **k) == root)
                .map(|(_, r)| *r)
                .min()
                .unwrap_or(Reason::SimilarName);
            DuplicateGroup {
                contacts,
                reason: r,
            }
        })
        .collect();
    out.sort_by_key(|g| g.reason);
    out
}

fn merge_list<T: PartialEq + Clone>(
    into: &mut Vec<Labeled<T>>,
    from: &[Labeled<T>],
    same: impl Fn(&T, &T) -> bool,
) {
    for f in from {
        if !into.iter().any(|x| same(&x.value, &f.value)) {
            let mut v = f.clone();
            v.preferred = false;
            into.push(v);
        }
    }
}

/// Merges `other` into `keep`: empty fields of `keep` are filled, lists are united,
/// notes are concatenated. `keep` keeps its id and folder.
pub fn merge(keep: &mut Contact, other: &Contact) {
    let fill = |a: &mut String, b: &String| {
        if a.trim().is_empty() {
            a.clone_from(b);
        }
    };
    fill(&mut keep.prefix, &other.prefix);
    fill(&mut keep.given, &other.given);
    fill(&mut keep.additional, &other.additional);
    fill(&mut keep.family, &other.family);
    fill(&mut keep.suffix, &other.suffix);
    fill(&mut keep.nickname, &other.nickname);
    fill(&mut keep.company, &other.company);
    fill(&mut keep.department, &other.department);
    fill(&mut keep.job_title, &other.job_title);
    merge_list(&mut keep.emails, &other.emails, |a, b| {
        norm_email(a) == norm_email(b)
    });
    merge_list(&mut keep.phones, &other.phones, |a, b| {
        norm_phone(a).is_some() && norm_phone(a) == norm_phone(b)
    });
    merge_list(&mut keep.addresses, &other.addresses, |a, b| {
        fold(&a.one_line()) == fold(&b.one_line())
    });
    merge_list(&mut keep.urls, &other.urls, |a, b| fold(a) == fold(b));
    keep.birthday = keep.birthday.or(other.birthday);
    keep.anniversary = keep.anniversary.or(other.anniversary);
    keep.labels.extend(other.labels.iter().cloned());
    keep.favorite |= other.favorite;
    if !other.notes.trim().is_empty() && !keep.notes.contains(other.notes.trim()) {
        if !keep.notes.is_empty() {
            keep.notes.push_str("\n\n");
        }
        keep.notes.push_str(other.notes.trim());
    }
    for e in &other.linked_entries {
        if !keep.linked_entries.contains(e) {
            keep.linked_entries.push(*e);
        }
    }
    for (k, v) in &other.fields {
        keep.fields.entry(k.clone()).or_insert_with(|| v.clone());
    }
    keep.updated = crate::ids::now();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contact::PartialDate;
    use crate::ids::SpaceId;

    fn p(g: &str, f: &str, email: Option<&str>, phone: Option<&str>) -> Contact {
        let mut c = Contact::person(SpaceId::new(), g, f);
        if let Some(e) = email {
            c.emails.push(Labeled::new("Work", e.to_owned()));
        }
        if let Some(t) = phone {
            c.phones.push(Labeled::new("Work", t.to_owned()));
        }
        c
    }

    #[test]
    fn finds_groups_by_reason() {
        let cs = vec![
            p(
                "Camille",
                "Dubois",
                Some("camille@example-print.com"),
                Some("+33 1 99 00 12 34"),
            ),
            p("C.", "Dubois", Some(" Camille@Example-Print.com "), None),
            p("Nadia", "Benali", None, Some("+33 1 99 00 77 02")),
            p("N", "Bnali", None, Some("01 99 00 77 02")),
            p("Paul", "Lefèvre", None, None),
            p("Paul", "Lefevre", None, None),
            p("Julien", "Moreau", None, None),
            p("Julie", "Morel", None, None),
        ];
        let g = find_duplicates(&cs, &BTreeSet::new());
        assert_eq!(g.len(), 3, "{g:?}");
        assert_eq!(g[0].reason, Reason::SameEmail);
        assert_eq!(g[1].reason, Reason::SamePhone);
        assert_eq!(g[2].reason, Reason::SimilarName);
        let mut dismissed = BTreeSet::new();
        let (a, b) = (cs[4].id, cs[5].id);
        dismissed.insert(if a < b { (a, b) } else { (b, a) });
        assert_eq!(find_duplicates(&cs, &dismissed).len(), 2);
    }

    #[test]
    fn merges() {
        let mut a = p(
            "Camille",
            "Dubois",
            Some("camille@example-print.com"),
            Some("+33 1 99 00 12 34"),
        );
        let mut b = p(
            "C.",
            "Dubois",
            Some("CAMILLE@example-print.com"),
            Some("+33 6 99 00 12 35"),
        );
        b.birthday = PartialDate::parse("--0312");
        b.company = "Example Print".into();
        b.labels.insert("Supplier".into());
        merge(&mut a, &b);
        assert_eq!(a.given, "Camille");
        assert_eq!(a.emails.len(), 1);
        assert_eq!(a.phones.len(), 2);
        assert_eq!(a.company, "Example Print");
        assert!(a.birthday.is_some() && a.labels.contains("Supplier"));
    }
}
