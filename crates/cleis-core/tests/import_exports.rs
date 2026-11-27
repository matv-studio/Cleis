//! Audit of 25/11/2026: imports run against files shaped like real exports (all data fictitious:
//! `example-*.com` domains, `99 00` phone numbers). One test per source, so a regression
//! names the tool that broke.

use cleis_core::entry::Subscription;
use cleis_core::import_csv::{Target, build, decode, preview};
use cleis_core::money::{Money, Period};
use cleis_core::{SpaceId, vcard};

fn import(bytes: &[u8]) -> (Vec<Target>, cleis_core::import_csv::Outcome) {
    let text = decode(bytes);
    let p = preview(&text).expect("preview");
    let t = p.mappings.iter().map(|m| m.target.clone()).collect();
    let o = build(&p, &p.mappings, SpaceId::new(), "EUR", &[]);
    (t, o)
}

/// Notion "Export › Markdown & CSV" of a database: UTF-8 with BOM, dates in Notion's display
/// format, checkboxes as Yes/No, multi-selects joined with ", ", line breaks kept in cells.
#[test]
fn notion_database() {
    let csv = "\u{feff}Name,Login,Password,URL,Status,Price,Renewal,Tags,Shared,Notes\n\
Figma,design@example-studio.com,s3cret-Figma!,https://www.figma.com,Active,€15.00,\"December 3, 2026\",\"Design, SaaS\",Yes,\"Team plan\nAdmin: Ines\"\n\
Example Books,accounting@example-studio.com,Bk-2026!,https://app.example-books.com,Renew soon,\"€120.00\",\"January 15, 2027 10:00 AM\",Accounting,No,\n\
Webflow,design@example-studio.com,wf-pass,webflow.com,Pending,29,\"March 1, 2027 → March 31, 2027\",,No,\n";
    let (t, o) = import(csv.as_bytes());
    assert_eq!(
        t[..7],
        [
            Target::Name,
            Target::Username,
            Target::Password,
            Target::Website,
            Target::Subscription,
            Target::Price,
            Target::Renewal
        ]
    );
    assert!(o.warnings.is_empty(), "{:?}", o.warnings);
    assert_eq!(o.entries.len(), 3);
    let f = &o.entries[0];
    assert_eq!(f.renewal.as_deref(), Some("2026-12-03"));
    // D-102: the columns Cleis does not know follow the notes, under their name.
    assert_eq!(
        f.notes,
        "Team plan\nAdmin: Ines\nTags: Design, SaaS\nShared: Yes"
    );
    assert_eq!(f.subscription, Subscription::Active);
    assert_eq!(o.entries[1].renewal.as_deref(), Some("2027-01-15"));
    assert_eq!(o.entries[1].subscription, Subscription::RenewSoon);
    assert_eq!(o.entries[2].renewal.as_deref(), Some("2027-03-01"));
}

/// Notion with a French interface: dates like « 3 décembre 2026 ».
#[test]
fn notion_database_french() {
    let csv = "\u{feff}Nom,Identifiant,Mot de passe,Site web,Échéance\n\
Figma,design@example-studio.com,x,figma.com,3 décembre 2026\n\
Canva,design@example-studio.com,y,canva.com,1 févr. 2027\n";
    let (_, o) = import(csv.as_bytes());
    assert!(o.warnings.is_empty(), "{:?}", o.warnings);
    assert_eq!(o.entries[0].renewal.as_deref(), Some("2026-12-03"));
    assert_eq!(o.entries[1].renewal.as_deref(), Some("2027-02-01"));
}

/// Excel in French, "CSV (séparateur : point-virgule)": Windows-1252, no BOM, `;`, decimal comma,
/// narrow no-break space as thousands separator in « 1 234,56 € ».
#[test]
fn excel_french_windows_1252() {
    let text = "Nom;Identifiant;Mot de passe;Prix (annuel);Licences;Remarques\r\n\
Société Générale — portail;0042 1187;Bq-2026!;1\u{202f}234,56 €;2;Accès « pro »\r\n\
Microsoft 365;accounting@example-studio.com;Ms-365!;150,00 €;5;\r\n";
    // Encode to Windows-1252 the way Excel writes it (é = E9, — = 97, « = AB, € = 80).
    let bytes: Vec<u8> = text
        .chars()
        .map(|c| match c {
            '€' => 0x80,
            '—' => 0x97,
            '\u{202f}' => 0xA0, // Excel writes a plain no-break space in this code page
            c if (c as u32) < 0x100 => c as u8,
            c => panic!("not in cp1252: {c}"),
        })
        .collect();
    assert!(
        std::str::from_utf8(&bytes).is_err(),
        "fixture must not be UTF-8"
    );
    let (t, o) = import(&bytes);
    assert_eq!(t[3], Target::Price);
    assert!(o.warnings.is_empty(), "{:?}", o.warnings);
    let e = &o.entries[0];
    assert_eq!(e.name, "Société Générale — portail");
    assert_eq!(e.notes, "Accès « pro »");
    assert_eq!(
        e.price,
        Some(Money::new(123_456, "EUR", Some(Period::Year)))
    );
    assert_eq!(e.licenses, Some(2));
}

/// Excel "Unicode text": UTF-16 LE with BOM, tab separated.
#[test]
fn excel_unicode_text_utf16() {
    let text = "Name\tUsername\tPassword\r\nCanva\tdesign@example-studio.com\tCv-Pass!\r\n";
    let mut bytes = vec![0xFF, 0xFE];
    for u in text.encode_utf16() {
        bytes.extend_from_slice(&u.to_le_bytes());
    }
    let (_, o) = import(&bytes);
    assert_eq!(o.entries.len(), 1);
    assert_eq!(o.entries[0].password, "Cv-Pass!");
}

/// Google Sheets "Download › CSV": UTF-8 without BOM, quoted header with a comma inside.
#[test]
fn google_sheets() {
    let csv = "\"Service, app\",Email,Password,Monthly price,Notes\n\
Slack,team@example-studio.com,\"pa,ss\"\"word\",\"$8.75\",\n";
    let (t, o) = import(csv.as_bytes());
    assert_eq!(
        t[0],
        Target::Name,
        "the comma in a quoted header must not change the delimiter"
    );
    let e = &o.entries[0];
    assert_eq!(e.password, "pa,ss\"word");
    assert_eq!(e.price, Some(Money::new(875, "USD", Some(Period::Month))));
}

/// Bitwarden "Export vault › .csv".
#[test]
fn bitwarden() {
    let csv = "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp\n\
Work,1,login,GitHub,,,0,https://github.com,dev@example-studio.com,Gh-2026!,otpauth://totp/GitHub:dev?secret=JBSWY3DPEHPK3PXP&issuer=GitHub\n\
,,login,Cloudflare,Main account,,0,https://dash.cloudflare.com,admin@example-studio.com,Cf-2026!,JBSW Y3DP EHPK 3PXP\n";
    let (t, o) = import(csv.as_bytes());
    assert!(t.contains(&Target::Totp));
    assert!(o.warnings.is_empty(), "{:?}", o.warnings);
    assert!(o.entries.iter().all(|e| e.totp.is_some()));
    assert_eq!(o.entries[1].notes, "Main account");
}

/// 1Password 8 CSV export.
#[test]
fn one_password() {
    let csv = "Title,Url,Username,Password,OTPAuth,Favorite,Archived,Tags,Notes\n\
Figma,https://figma.com,design@example-studio.com,Fg-2026!,,false,false,Design,\n";
    let (t, o) = import(csv.as_bytes());
    assert_eq!(
        &t[..5],
        &[
            Target::Name,
            Target::Website,
            Target::Username,
            Target::Password,
            Target::Totp
        ]
    );
    assert_eq!(o.entries[0].urls, vec!["https://figma.com".to_owned()]);
}

/// Chrome / Edge "Export passwords" and Firefox "Export logins" (no name column).
#[test]
fn browsers() {
    let chrome = "name,url,username,password,note\n\
figma.com,https://www.figma.com/login,design@example-studio.com,Fg!,\n";
    let (_, o) = import(chrome.as_bytes());
    assert_eq!(o.entries[0].name, "figma.com");
    let firefox = "\"url\",\"username\",\"password\",\"httpRealm\",\"formActionOrigin\",\"guid\",\"timeCreated\",\"timeLastUsed\",\"timePasswordChanged\"\n\
\"https://app.example-books.com\",\"accounting@example-studio.com\",\"Bk!\",,\"https://app.example-books.com\",\"{0a1b}\",\"1732000000000\",\"1732000000000\",\"1732000000000\"\n";
    let (t, o) = import(firefox.as_bytes());
    assert_eq!(t.iter().filter(|t| **t == Target::Password).count(), 1);
    let e = &o.entries[0];
    assert_eq!(e.password, "Bk!");
    assert_eq!(e.name, "app.example-books.com");
}

/// Apple Contacts: vCard 3.0, grouped properties with custom labels, folded lines, photo.
#[test]
fn vcard_apple() {
    let src = "BEGIN:VCARD\r\nVERSION:3.0\r\nPRODID:-//Apple Inc.//macOS 15//EN\r\nN:Dubois;Camille;;;\r\n\
FN:Camille Dubois\r\nORG:Example Print;\r\nTITLE:Account manager\r\n\
item1.EMAIL;type=INTERNET;type=pref:camille@example-print.com\r\nitem1.X-ABLabel:_$!<Work>!$_\r\n\
TEL;type=CELL;type=VOICE;type=pref:+33 6 99 00 12 35\r\n\
item2.ADR;type=WORK:;;12 rue de l'Exemple;Lyon;;69002;France\r\nitem2.X-ABADR:fr\r\n\
NOTE:Met at the print fair\\, Lyon.\\nPrefers email.\r\n\
PHOTO;ENCODING=b;TYPE=JPEG:/9j/4AAQSkZJRgABAQ\r\n AAAQABAAD/2wBDAAMCAgM\r\nEND:VCARD\r\n";
    let cs = vcard::parse(src, SpaceId::new());
    assert_eq!(cs.len(), 1);
    let c = &cs[0];
    assert_eq!(c.emails[0].value, "camille@example-print.com");
    assert_eq!(c.phones[0].value, "+33 6 99 00 12 35");
    assert_eq!(c.notes, "Met at the print fair, Lyon.\nPrefers email.");
}

/// Google Contacts "Export › vCard (for iOS Contacts)".
#[test]
fn vcard_google() {
    let src = "BEGIN:VCARD\nVERSION:3.0\nFN:Julien Moreau\nN:Moreau;Julien;;;\n\
EMAIL;TYPE=INTERNET;TYPE=WORK:j.moreau@example-legal.com\nTEL;TYPE=WORK:+33 1 99 00 45 10\n\
ORG:Example Legal\nTITLE:Lawyer\nCATEGORIES:myContacts,Advisors\nEND:VCARD\n\
BEGIN:VCARD\nVERSION:3.0\nFN:Léa Fontaine\nN:Fontaine;Léa;;;\nTEL;TYPE=CELL:+33 6 99 00 54 17\nBDAY:1990-04-12\nEND:VCARD\n";
    let cs = vcard::parse(src, SpaceId::new());
    assert_eq!(cs.len(), 2);
    assert_eq!(cs[1].phones[0].value, "+33 6 99 00 54 17");
}

/// Outlook "Save as vCard": 2.1, quoted-printable in Windows-1252 for accented names.
#[test]
fn vcard_outlook() {
    let src = "BEGIN:VCARD\r\nVERSION:2.1\r\nN;CHARSET=Windows-1252;ENCODING=QUOTED-PRINTABLE:Lef=E8vre;Paul\r\n\
FN;CHARSET=Windows-1252;ENCODING=QUOTED-PRINTABLE:Paul Lef=E8vre\r\nORG:Example Bakery\r\n\
TEL;CELL;VOICE:+33 6 99 00 21 88\r\nEMAIL;PREF;INTERNET:paul@example-bakery.com\r\nEND:VCARD\r\n";
    let cs = vcard::parse(src, SpaceId::new());
    assert_eq!(cs[0].display_name(), "Paul Lefèvre");
    assert_eq!(cs[0].emails[0].value, "paul@example-bakery.com");
}
