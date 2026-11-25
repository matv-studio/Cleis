//! Contacts from a CSV file (Google Contacts, Outlook, spreadsheets): columns are recognised by
//! their header, in English and French; each row becomes a contact.

use crate::contact::{Address, Contact, Labeled, PartialDate};
use crate::ids::SpaceId;
use crate::text::fold;

#[derive(Debug, Default)]
pub struct ContactImport {
    pub contacts: Vec<Contact>,
    /// (row number starting at 2, message)
    pub warnings: Vec<(usize, String)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Col {
    Full,
    Given,
    Middle,
    Family,
    Company,
    Title,
    Email(&'static str),
    Phone(&'static str),
    Street(&'static str),
    City(&'static str),
    Postal(&'static str),
    Region(&'static str),
    Country(&'static str),
    Birthday,
    Notes,
    Labels,
    Website,
    Skip,
}

/// Work / Home / Mobile / Other from words of a header or of a "Type" column.
fn kind(h: &str) -> &'static str {
    if h.contains("mobile") || h.contains("portable") || h.contains("cell") {
        "Mobile"
    } else if h.contains("home") || h.contains("domicile") || h.contains("perso") {
        "Home"
    } else if h.contains("other") || h.contains("autre") {
        "Other"
    } else {
        "Work"
    }
}

fn col(header: &str) -> Col {
    let h = fold(header);
    let has = |w: &str| h.contains(w);
    // Google writes "E-mail 1 - Type" next to "E-mail 1 - Value": the type column is skipped
    // (its value lands in the label through the header of the value column when present).
    if has(" - type") || has(" - label") {
        return Col::Skip;
    }
    match h.as_str() {
        "name" | "full name" | "nom complet" | "display name" | "nom et prenom" => {
            return Col::Full;
        }
        "first name" | "given name" | "prenom" => return Col::Given,
        "middle name" | "additional name" | "deuxieme prenom" => return Col::Middle,
        "last name" | "family name" | "nom" | "nom de famille" | "surname" => return Col::Family,
        "notes" | "note" | "remarques" => return Col::Notes,
        "birthday" | "anniversaire" | "date de naissance" => return Col::Birthday,
        _ => {}
    }
    if has("e-mail") || has("email") || has("courriel") {
        return Col::Email(kind(&h));
    }
    if has("phone") || has("telephone") || has("mobile") || has("portable") || h.starts_with("tel")
    {
        return Col::Phone(kind(&h));
    }
    if has("street") || has("rue") || (has("address") && !has("e-mail")) || has("adresse") {
        if has("city") || has("ville") {
            return Col::City(kind(&h));
        }
        if has("postal") || has("zip") {
            return Col::Postal(kind(&h));
        }
        if has("country") || has("pays") {
            return Col::Country(kind(&h));
        }
        if has("region") || has("state") {
            return Col::Region(kind(&h));
        }
        return Col::Street(kind(&h));
    }
    if has("city") || has("ville") {
        return Col::City(kind(&h));
    }
    if has("postal") || has("zip") || has("code postal") {
        return Col::Postal(kind(&h));
    }
    if has("country") || has("pays") {
        return Col::Country(kind(&h));
    }
    if has("company")
        || has("organization")
        || has("organisation")
        || has("societe")
        || has("entreprise")
    {
        if has("title") {
            return Col::Title;
        }
        return Col::Company;
    }
    if has("job title") || h == "title" || has("fonction") || has("poste") {
        return Col::Title;
    }
    if has("web") || has("site") || h == "url" {
        return Col::Website;
    }
    if has("label") || has("group") || has("categor") || has("etiquette") {
        return Col::Labels;
    }
    Col::Skip
}

fn addr_of(addr: &mut Vec<(&'static str, Address)>, k: &'static str) -> usize {
    match addr.iter().position(|(x, _)| *x == k) {
        Some(i) => i,
        None => {
            addr.push((k, Address::default()));
            addr.len() - 1
        }
    }
}

pub fn contacts_from_csv(data: &str, space: SpaceId) -> Result<ContactImport, csv::Error> {
    let data = data.trim_start_matches('\u{feff}');
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(crate::import_csv::sniff_delimiter(data))
        .flexible(true)
        .from_reader(data.as_bytes());
    let cols: Vec<Col> = rdr.headers()?.iter().map(col).collect();
    let mut out = ContactImport::default();
    for (ri, rec) in rdr.records().enumerate() {
        let line = ri + 2;
        let Ok(rec) = rec else { continue };
        let mut c = Contact::new(space);
        let mut full = String::new();
        let mut addr: Vec<(&'static str, Address)> = vec![];
        let mut idx = vec![];
        for (i, v) in rec.iter().enumerate() {
            let v = v.trim();
            if v.is_empty() {
                continue;
            }
            idx.push((i, v.to_owned()));
        }
        for (i, v) in idx {
            match cols.get(i).copied().unwrap_or(Col::Skip) {
                Col::Full => full = v,
                Col::Given => c.given = v,
                Col::Middle => c.additional = v,
                Col::Family => c.family = v,
                Col::Company => c.company = v,
                Col::Title => c.job_title = v,
                // Google joins several values of one cell with " ::: ".
                Col::Email(k) => {
                    for e in v.split(":::").map(str::trim).filter(|e| !e.is_empty()) {
                        c.emails.push(Labeled::new(k, e.to_owned()));
                    }
                }
                Col::Phone(k) => {
                    for p in v.split(":::").map(str::trim).filter(|p| !p.is_empty()) {
                        c.phones.push(Labeled::new(k, p.to_owned()));
                    }
                }
                Col::Street(k) => {
                    let a = addr_of(&mut addr, k);
                    addr[a].1.street = v;
                }
                Col::City(k) => {
                    let a = addr_of(&mut addr, k);
                    addr[a].1.city = v;
                }
                Col::Postal(k) => {
                    let a = addr_of(&mut addr, k);
                    addr[a].1.postal_code = v;
                }
                Col::Region(k) => {
                    let a = addr_of(&mut addr, k);
                    addr[a].1.region = v;
                }
                Col::Country(k) => {
                    let a = addr_of(&mut addr, k);
                    addr[a].1.country = v;
                }
                Col::Birthday => match PartialDate::read(&v) {
                    Some(d) => c.birthday = Some(d),
                    None => out
                        .warnings
                        .push((line, format!("birthday “{v}” not understood"))),
                },
                Col::Notes => c.notes = v,
                Col::Labels => {
                    for l in v.split([';', ',']).flat_map(|x| x.split(":::")) {
                        let l = l.trim().trim_start_matches('*').trim();
                        // Google adds its own system groups.
                        if !l.is_empty() && !fold(l).starts_with("mycontacts") && l != "myContacts"
                        {
                            c.labels.insert(l.to_owned());
                        }
                    }
                }
                Col::Website => c.urls.push(Labeled::new("Work", v)),
                Col::Skip => {}
            }
        }
        c.addresses = addr.into_iter().map(|(k, a)| Labeled::new(k, a)).collect();
        if c.given.is_empty() && c.family.is_empty() && !full.is_empty() {
            c.set_full_name(&full);
        }
        if c.display_name().trim().is_empty() {
            out.warnings.push((line, "empty row skipped".into()));
            continue;
        }
        out.contacts.push(c);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn google_contacts_export() {
        let csv = "First Name,Middle Name,Last Name,Birthday,Notes,Labels,E-mail 1 - Label,E-mail 1 - Value,Phone 1 - Label,Phone 1 - Value,Address 1 - Label,Address 1 - Street,Address 1 - City,Address 1 - Postal Code,Address 1 - Country,Organization Name,Organization Title\n\
Camille,,Dubois,1990-03-12,Quotes within 48 h,* myContacts ::: Supplier,Work,camille@example-print.com,Mobile,+33 6 99 00 12 35,Work,12 rue de l'Exemple,Lyon,69002,France,Example Print,Account manager\n\
,,,,,,,,,,,,,,,,\n";
        let r = contacts_from_csv(csv, SpaceId::new()).unwrap();
        assert_eq!(r.contacts.len(), 1);
        let c = &r.contacts[0];
        assert_eq!(c.display_name(), "Camille Dubois");
        assert_eq!(c.company, "Example Print");
        assert_eq!(c.job_title, "Account manager");
        assert_eq!(
            c.primary_email().as_deref(),
            Some("camille@example-print.com")
        );
        assert_eq!(c.phones[0].value, "+33 6 99 00 12 35");
        assert_eq!(c.city().as_deref(), Some("Lyon"));
        assert_eq!(c.addresses[0].value.postal_code, "69002");
        assert_eq!(c.birthday.unwrap().month, 3);
        assert!(c.labels.contains("Supplier") && c.labels.len() == 1);
    }

    #[test]
    fn outlook_french_export() {
        let csv = "Prénom;Nom;Société;Titre;Adresse de messagerie;Téléphone mobile;Téléphone (bureau);Rue (bureau);Ville (bureau);Code postal (bureau);Anniversaire;Catégories\n\
Paul;Lefèvre;Example Bakery;Owner;paul@example-bakery.com;+33 6 99 00 21 88;+33 4 99 00 21 80;3 place de l'Exemple;Annecy;74000;12/05/1980;Client;VIP\n";
        let r = contacts_from_csv(csv, SpaceId::new()).unwrap();
        let c = &r.contacts[0];
        assert_eq!(c.display_name(), "Paul Lefèvre");
        assert_eq!(c.company, "Example Bakery");
        assert_eq!(c.phones.len(), 2);
        assert_eq!(c.phones[0].label, "Mobile");
        assert_eq!(c.city().as_deref(), Some("Annecy"));
        assert_eq!(c.birthday.unwrap().year, Some(1980));
        assert!(c.labels.contains("Client"));
    }
}
