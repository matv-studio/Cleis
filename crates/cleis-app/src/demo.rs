//! Fictitious data from the mockups, for `--demo`, screenshots and end-to-end tests.

use cleis_core::column::Value;
use cleis_core::contact::{Address, Labeled, PartialDate};
use cleis_core::entry::Subscription;
use cleis_core::totp::Totp;
use cleis_core::{Contact, Entry, Money, Period};
use cleis_crypto::KdfParams;
use cleis_store::{EncryptionMode, SpaceKind, Vault};
use std::path::Path;

pub const PASSWORD: &str = "demo-password";

type Row<'a> = (
    &'a str,
    &'a str,
    Option<&'a str>,
    Subscription,
    Option<(i64, Period)>,
    Option<u32>,
    &'a str,
);

const ROWS: &[Row] = &[
    (
        "OVHcloud",
        "admin@example-studio.com",
        Some("JBSWY3DPEHPK3PXP"),
        Subscription::Active,
        Some((999, Period::Month)),
        Some(1),
        "ovhcloud.com",
    ),
    (
        "Cloudflare",
        "admin@example-studio.com",
        Some("KRSXG5CTMVRXEZLU"),
        Subscription::Free,
        None,
        None,
        "cloudflare.com",
    ),
    (
        "Figma",
        "design@example-studio.com",
        Some("GEZDGNBVGY3TQOJQ"),
        Subscription::Active,
        Some((1500, Period::Month)),
        Some(4),
        "figma.com",
    ),
    (
        "Adobe Creative Cloud",
        "studio@example-studio.com",
        None,
        Subscription::Active,
        Some((6999, Period::Month)),
        Some(2),
        "adobe.com",
    ),
    (
        "Notion",
        "team@example-studio.com",
        None,
        Subscription::RenewSoon,
        Some((1000, Period::Month)),
        Some(5),
        "notion.so",
    ),
    (
        "Google Workspace",
        "admin@example-studio.com",
        Some("MFRGGZDFMZTWQ2LK"),
        Subscription::Active,
        Some((680, Period::Month)),
        Some(6),
        "workspace.google.com",
    ),
    (
        "Microsoft 365",
        "accounting@example-studio.com",
        None,
        Subscription::Active,
        Some((1250, Period::Month)),
        Some(3),
        "microsoft365.com",
    ),
    (
        "Bank — business portal",
        "0042 1187",
        Some("NBSWY3DPO5XXE3DE"),
        Subscription::Free,
        None,
        None,
        "bank.example.com",
    ),
    (
        "GitHub",
        "dev@example-studio.com",
        Some("ONSWG4TFORZWK3DQ"),
        Subscription::Free,
        None,
        None,
        "github.com",
    ),
    (
        "Webflow",
        "design@example-studio.com",
        None,
        Subscription::Pending,
        Some((2900, Period::Month)),
        Some(1),
        "webflow.com",
    ),
    (
        "Slack",
        "team@example-studio.com",
        None,
        Subscription::Off,
        None,
        None,
        "slack.com",
    ),
    (
        "Office router",
        "admin",
        None,
        Subscription::Free,
        None,
        None,
        "192.168.1.1",
    ),
    (
        "Canva",
        "design@example-studio.com",
        None,
        Subscription::Active,
        Some((11000, Period::Year)),
        Some(1),
        "canva.com",
    ),
    (
        "Dropbox",
        "studio@example-studio.com",
        None,
        Subscription::Off,
        None,
        None,
        "dropbox.com",
    ),
];

/// given, family, company, title, email, phone, city, labels, favourite
type Person<'a> = (
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a [&'a str],
    bool,
);

const PEOPLE: &[Person] = &[
    (
        "Camille",
        "Dubois",
        "Example Print",
        "Account manager",
        "camille@example-print.com",
        "+33 1 99 00 12 34",
        "Lyon",
        &["Supplier"],
        true,
    ),
    (
        "Julien",
        "Moreau",
        "Example Legal",
        "Lawyer",
        "j.moreau@example-legal.com",
        "+33 1 99 00 45 10",
        "Paris",
        &["Advisor"],
        false,
    ),
    (
        "Nadia",
        "Benali",
        "Example Hosting",
        "Support lead",
        "support@example-hosting.com",
        "+33 1 99 00 77 02",
        "Lille",
        &["Supplier", "Tech"],
        false,
    ),
    (
        "Paul",
        "Lefèvre",
        "Example Bakery",
        "Owner",
        "paul@example-bakery.com",
        "+33 6 99 00 21 88",
        "Annecy",
        &["Client"],
        true,
    ),
    (
        "Sofia",
        "Rossi",
        "Example Films",
        "Producer",
        "sofia@example-films.com",
        "+39 02 9900 1122",
        "Milan",
        &["Client", "VIP"],
        false,
    ),
    (
        "Marc",
        "Girard",
        "Example Bank",
        "Business adviser",
        "marc.girard@example-bank.com",
        "+33 1 99 00 63 40",
        "Paris",
        &["Bank"],
        false,
    ),
    (
        "Léa",
        "Fontaine",
        "",
        "Photographer",
        "lea.fontaine@example.com",
        "+33 6 99 00 54 17",
        "Bordeaux",
        &["Freelancer"],
        false,
    ),
    (
        "Emma",
        "Laurent",
        "Example Retail",
        "Marketing director",
        "emma.laurent@example-retail.com",
        "+33 1 99 00 88 61",
        "Nantes",
        &["Prospect"],
        false,
    ),
];

/// Creates a vault at `path` filled with the mockup data. Cheap key derivation: demo only.
pub fn create(path: &Path) -> anyhow::Result<()> {
    let mut v = Vault::create(path, PASSWORD, KdfParams::insecure_for_tests())?;
    let home = v.home_space();
    let now = cleis_core::now();
    let mut entries = vec![];
    for (i, (name, user, totp, sub, price, lic, url)) in ROWS.iter().enumerate() {
        let mut e = Entry::new(home, *name);
        e.username = (*user).into();
        e.password = format!(
            "Demo-{}-{:04}!x",
            name.split_whitespace().next().unwrap_or("x"),
            1000 + i * 37
        );
        e.totp = totp.and_then(|t| Totp::from_base32(t).ok());
        e.subscription = *sub;
        e.price = price.map(|(c, p)| Money::new(c, "EUR", Some(p)));
        e.licenses = *lic;
        e.urls = vec![(*url).into()];
        // Health report (screen 12): a weak and reused password, an old weak one.
        match *name {
            "Notion" | "Slack" | "Dropbox" => e.password = "Studio2024".into(),
            "Office router" => e.password = "admin1234".into(),
            "Microsoft 365" => e.password = "Compta2025".into(),
            _ => {}
        }
        e.favorite = matches!(*name, "Figma" | "GitHub" | "OVHcloud");
        e.password_changed = now - (i as i64 * 41 + 3) * 86_400;
        if *name == "Figma" {
            e.notes = "Team account · invoices in the studio's name".into();
            e.renewal = Some("2026-12-03".into());
        }
        entries.push(e);
    }
    v.put_entries(&entries)?;
    v.create_space(
        "Example Studio — General",
        SpaceKind::Organization,
        EncryptionMode::Standard,
        false,
    )?;
    let contacts_space = v
        .spaces()?
        .into_iter()
        .find(|s| s.for_contacts)
        .map(|s| s.id)
        .unwrap_or(home);
    // A custom contact column, as in mockup 20 (VAT number).
    let mut ccols = v.contact_columns()?;
    let vat = cleis_core::column::add_custom(
        &mut ccols,
        cleis_core::column::Table::Contacts,
        "VAT number",
        cleis_core::column::ColumnKind::Text,
        cleis_core::column::ColumnScope::AllSpaces,
    )
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    // Shown in the inspector only, as in the mockup.
    if let Some(c) = ccols.iter_mut().find(|c| c.id == vat) {
        c.visible = false;
    }
    v.set_contact_columns(&ccols)?;
    let mut cs = vec![];
    for (g, f, co, title, email, phone, city, labels, fav) in PEOPLE {
        let mut c = Contact::person(contacts_space, g, f);
        c.company = (*co).into();
        c.job_title = (*title).into();
        c.emails.push(Labeled::new("Work", (*email).into()));
        c.phones.push(Labeled::new("Work", (*phone).into()));
        c.addresses.push(Labeled::new(
            "Work",
            Address {
                city: (*city).into(),
                ..Default::default()
            },
        ));
        c.labels = labels.iter().map(|s| (*s).to_owned()).collect();
        c.favorite = *fav;
        if *g == "Camille" {
            c.birthday = PartialDate::parse("--0312");
            c.notes = "Quotes within 48 h · prefers email".into();
            c.fields
                .insert(vat.clone(), Value::Text("FR00 999999999".into()));
        }
        cs.push(c);
    }
    // Organisation folders, and a duplicate for Merge & fix (screen 21).
    let suppliers = v.create_space(
        "Suppliers",
        SpaceKind::Organization,
        EncryptionMode::Standard,
        true,
    )?;
    v.create_space(
        "Clients",
        SpaceKind::Organization,
        EncryptionMode::Standard,
        true,
    )?;
    let mut dup = Contact::person(suppliers, "C.", "Dubois");
    dup.company = "Example Print".into();
    dup.emails
        .push(Labeled::new("Work", "camille@example-print.com".into()));
    dup.phones
        .push(Labeled::new("Mobile", "+33 6 99 00 12 35".into()));
    cs.push(dup);
    if let Some(c) = cs.iter_mut().find(|c| c.given == "Camille")
        && let Some(e) = entries.iter().find(|e| e.name == "Figma")
    {
        c.linked_entries.push(e.id);
    }
    v.put_contacts(&cs)?;
    Ok(())
}

/// Item for trying autofill against `design/tools/autofill-form.hta` (`--try-autofill`): its name
/// matches the form's window title. Fictitious values.
pub fn add_autofill_sample(path: &Path) -> anyhow::Result<()> {
    let v = cleis_store::LockedVault::open(path)?.unlock(PASSWORD)?;
    let mut e = Entry::new(v.home_space(), "Example Login");
    e.username = "ines@example-studio.com".into();
    e.password = "Sample-Pass-2026!".into();
    e.totp = Totp::from_base32("JBSWY3DPEHPK3PXP").ok();
    e.urls = vec!["https://login.example-studio.com".into()];
    v.put_entry(&e)?;
    Ok(())
}
