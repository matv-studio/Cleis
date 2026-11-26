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
    // A personal folder (D-62).
    v.create_space(
        "Banking",
        SpaceKind::Personal,
        EncryptionMode::EndToEnd,
        false,
    )?;
    // The organization's spaces of the mockups (01, 09).
    let mut org = std::collections::BTreeMap::new();
    for (name, mode) in [
        ("General", EncryptionMode::Standard),
        ("Design", EncryptionMode::Standard),
        ("Accounting", EncryptionMode::EndToEnd),
        ("Dev", EncryptionMode::Standard),
        ("Interns", EncryptionMode::Standard),
    ] {
        org.insert(
            name,
            v.create_space(name, SpaceKind::Organization, mode, false)?,
        );
    }
    let space_of = |name: &str| match name {
        "Figma" | "Adobe Creative Cloud" | "Webflow" => org["Design"],
        "Microsoft 365" | "Bank — business portal" => org["Accounting"],
        "GitHub" | "Cloudflare" | "OVHcloud" => org["Dev"],
        "Canva" => org["Interns"],
        "Dropbox" => home,
        _ => org["General"],
    };
    let mut entries = vec![];
    for (i, (name, user, totp, sub, price, lic, url)) in ROWS.iter().enumerate() {
        let mut e = Entry::new(space_of(name), *name);
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
            c.photo = Some(sample_photo());
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

/// A drawn photo for the demo (mockup 36e): a silhouette on a gradient, no real person.
fn sample_photo() -> String {
    let n = 256u32;
    let img = image::RgbImage::from_fn(n, n, |x, y| {
        let (fx, fy) = (x as f32 / n as f32, y as f32 / n as f32);
        let head = ((fx - 0.5).powi(2) + (fy - 0.40).powi(2)).sqrt() < 0.17;
        let body = ((fx - 0.5).powi(2) / 0.09 + (fy - 1.0).powi(2) / 0.16) < 1.;
        if head || body {
            image::Rgb([236, 226, 214])
        } else {
            let t = (fx + fy) / 2.;
            let mix = |a: f32, b: f32| (a + (b - a) * t) as u8;
            image::Rgb([mix(62., 196.), mix(120., 140.), mix(138., 80.)])
        }
    });
    let mut png = std::io::Cursor::new(Vec::new());
    img.write_to(&mut png, image::ImageFormat::Png)
        .expect("PNG in memory");
    crate::screens::contacts::portrait(png.get_ref()).expect("demo photo")
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

/// The organization of the mockups (07–11, 13, 18) shown without a server: members, roles,
/// policy, spaces and an activity log, for `--demo` and screenshots.
pub fn org(s: &mut crate::app::Session) {
    use cleis_core::org::*;
    use cleis_core::{MemberId, SpaceId};
    let now = cleis_core::now();
    let org_kp = cleis_crypto::KeyPair::generate().expect("random");
    let super_kp = cleis_crypto::KeyPair::generate().expect("random");
    let cert = cleis_crypto::sha256(b"Example Studio demo certificate");
    let spaces: Vec<(SpaceId, String, bool)> = s
        .spaces
        .iter()
        .filter(|x| x.kind == SpaceKind::Organization && !x.for_contacts)
        .map(|x| (x.id, x.name.clone(), x.mode == EncryptionMode::EndToEnd))
        .collect();
    let sp = |n: &str| {
        spaces
            .iter()
            .find(|x| x.1 == n)
            .map(|x| x.0)
            .unwrap_or_default()
    };
    let key = || {
        cleis_crypto::KeyPair::generate()
            .expect("random")
            .public()
            .0
    };
    let member = |name: &str,
                  email: &str,
                  role: &str,
                  sps: Vec<SpaceId>,
                  two_fa: bool,
                  export: bool,
                  last: i64| MemberView {
        id: MemberId::new(),
        name: name.into(),
        email: email.into(),
        role: role.into(),
        spaces: sps,
        two_fa,
        export,
        last_sign_in: Some(last),
        last_device: String::new(),
        revoked: false,
        account_public: key(),
        verify_key: key(),
        factor: two_fa.then_some(Method::AuthenticatorApp),
        pin_locked: false,
    };
    let all: Vec<SpaceId> = spaces.iter().map(|x| x.0).collect();
    let today = now - now.rem_euclid(86_400);
    let members = vec![
        member(
            "Lea Martin",
            "lea@example-studio.com",
            SUPER_ADMIN,
            all.clone(),
            true,
            true,
            today + 9 * 3600 + 12 * 60,
        ),
        member(
            "Hugo Bernard",
            "hugo@example-studio.com",
            "member",
            vec![sp("General"), sp("Design"), sp("Dev")],
            true,
            false,
            today + 10 * 3600 + 47 * 60,
        ),
        member(
            "Ines Petit",
            "ines@example-studio.com",
            "member",
            vec![sp("General"), sp("Accounting")],
            true,
            true,
            today - 86_400 + 17 * 3600 + 30 * 60,
        ),
        member(
            "Sarah Roux",
            "sarah@example-studio.com",
            "read_only",
            vec![sp("General")],
            false,
            false,
            today - 5 * 86_400 + 9 * 3600,
        ),
        member(
            "Tom Durand",
            "tom@example-studio.com",
            "intern",
            vec![sp("Interns")],
            true,
            false,
            today - 4 * 86_400 + 9 * 3600 + 3 * 60,
        ),
    ];
    let lea = members[0].id;
    let space_views = spaces
        .iter()
        .map(|(id, _, e2e)| SpaceView {
            id: *id,
            sealed: vec![],
            mode: if *e2e {
                SpaceMode::EndToEnd
            } else {
                SpaceMode::Standard
            },
            key_gen: 1,
            wrapped: None,
            org_wrapped: None,
            recovery_wrapped: None,
            reserved: *e2e,
            members: members
                .iter()
                .filter(|m| m.spaces.contains(id))
                .map(|m| m.id)
                .collect(),
            items: s.entries.iter().filter(|e| e.space == *id).count() as u32,
            needs_rekey: false,
        })
        .collect();
    let policy = Policy {
        code_required: true,
        detail: LogDetail::Everything,
        ..Default::default()
    };
    let mut events = vec![];
    let ev =
        |seq: u64, at: i64, who: usize, action: Action, item: &str, space: &str, device: &str| {
            Event {
                seq,
                at,
                member: Some(members[who].id),
                device: device.into(),
                action,
                space: (!space.is_empty()).then(|| sp(space)),
                item: None,
                detail: (!item.is_empty())
                    .then(|| {
                        let (i, f) = item.split_once(" · ").unwrap_or((item, ""));
                        serde_json::to_vec(&EventDetail {
                            item: i.into(),
                            field: f.into(),
                        })
                        .ok()
                    })
                    .flatten()
                    .and_then(|d| {
                        // Accounting is reserved: its details go to the Super Admin key.
                        let k = if space == "Accounting" {
                            &super_kp
                        } else {
                            &org_kp
                        };
                        cleis_crypto::seal_to(&k.public(), DETAIL_CONTEXT, &d).ok()
                    }),
                note: String::new(),
            }
        };
    let h = |hh: i64, mm: i64| today + hh * 3600 + mm * 60;
    let y = |hh: i64, mm: i64| today - 86_400 + hh * 3600 + mm * 60;
    events.push(ev(
        214,
        h(11, 31),
        1,
        Action::Copied,
        "Figma · password",
        "Design",
        "PC-HUGO · Windows 11",
    ));
    events.push(ev(
        213,
        h(11, 29),
        4,
        Action::Filled,
        "Canva",
        "Interns",
        "MAC-TOM · macOS 14",
    ));
    events.push(ev(
        212,
        h(10, 58),
        2,
        Action::Viewed,
        "Bank — business portal",
        "Accounting",
        "PC-INES · Windows 11",
    ));
    events.push(ev(
        211,
        h(10, 47),
        1,
        Action::SignedIn,
        "",
        "",
        "PC-HUGO · Windows 11",
    ));
    events.push(ev(
        210,
        h(9, 40),
        0,
        Action::Edited,
        "OVHcloud · password",
        "Dev",
        "MAC-LEA · macOS 15",
    ));
    events.push(ev(
        209,
        h(9, 38),
        0,
        Action::Created,
        "Webflow",
        "Design",
        "MAC-LEA · macOS 15",
    ));
    events.push(ev(
        208,
        y(18, 2),
        0,
        // Audit L-15: as the app records it (vCard export of shared contacts).
        Action::Exported,
        "5 contacts",
        "General",
        "MAC-LEA · macOS 15",
    ));
    events.push(ev(
        207,
        y(17, 30),
        2,
        Action::Copied,
        "Microsoft 365 · username",
        "Accounting",
        "PC-INES · Windows 11",
    ));
    events.push(ev(
        206,
        y(16, 12),
        1,
        Action::Deleted,
        "Old hosting",
        "Dev",
        "PC-HUGO · Windows 11",
    ));
    events.push(ev(
        205,
        y(15, 44),
        4,
        Action::Filled,
        "Google Workspace",
        "Interns",
        "MAC-TOM · macOS 14",
    ));
    events.push(ev(
        204,
        y(11, 20),
        3,
        Action::Viewed,
        "Slack",
        "General",
        "PC-SARAH · Windows 10",
    ));
    events.push(ev(
        203,
        y(9, 5),
        1,
        Action::Edited,
        "GitHub · 2FA code",
        "Dev",
        "PC-HUGO · Windows 11",
    ));
    events.push(ev(
        202,
        today - 4 * 86_400 + 9 * 3600 + 3 * 60,
        4,
        Action::SignedIn,
        "",
        "",
        "MAC-TOM · macOS 14",
    ));
    let me = members[0].clone();
    let st = State {
        org: OrgInfo {
            id: uuid::Uuid::from_u128(0x0c1e_1500),
            name: "Example Studio".into(),
            cert,
            next_cert: None,
            org_public: Some(org_kp.public().0.to_vec()),
            recovery_public: Some(key().to_vec()),
            recovery_saved: Some(now - 11 * 86_400),
            super_public: Some(super_kp.public().0.to_vec()),
        },
        me,
        roles: Role::defaults(),
        policy,
        members,
        invites: vec![InviteView {
            id: uuid::Uuid::from_u128(1),
            name: String::new(),
            email: "noa@example-studio.com".into(),
            role: "member".into(),
            spaces: vec![sp("General")],
            created: now - 3 * 86_400,
            expires: now + 4 * 86_400 + 3600,
        }],
        spaces: space_views,
        pending: vec![],
        org_key: None,
        pending_admins: vec![],
        super_key: None,
        pending_supers: vec![],
        seq: 0,
        integrity: Some(Integrity {
            checked: 214,
            intact: true,
            problem: None,
            at: h(11, 30).min(now),
        }),
        events: 214,
        methods: vec![Method::AuthenticatorApp, Method::Email, Method::Google],
        providers: vec![],
        pin_set: false,
    };
    // The organization's contact folders belong to it too (audit L-13).
    let names = spaces
        .iter()
        .map(|x| (x.0, x.1.clone()))
        .chain(
            s.spaces
                .iter()
                .filter(|x| x.kind == SpaceKind::Organization && x.for_contacts)
                .map(|x| (x.id, x.name.clone())),
        )
        .collect();
    let ui = crate::screens::org::OrgUi {
        devices: Some(vec![
            DeviceView {
                device: "MAC-LEA · macOS 15".into(),
                last_active: now,
                remembered_until: None,
                current: true,
            },
            DeviceView {
                device: "PC-STUDIO · Windows 11".into(),
                last_active: today - 86_400 + 18 * 3600 + 2 * 60,
                remembered_until: Some(today + 29 * 86_400),
                current: false,
            },
            DeviceView {
                device: "MAC-HOME · macOS 14".into(),
                last_active: today - 7 * 86_400 + 20 * 3600,
                remembered_until: None,
                current: false,
            },
        ]),
        log: Some(LogPage { total: 214, events }),
        log_days: Some(7),
        ..Default::default()
    };
    s.org = Some(crate::org::OrgSession {
        m: crate::org::Membership {
            org: st.org.id,
            name: "Example Studio".into(),
            address: "vault.example-studio.lan:8443".into(),
            pins: vec![cleis_crypto::hex(&cert)],
            member: lea,
        },
        token: None,
        local: crate::org::Local {
            remembered: None,
            last_ok: Some(now - 120),
            host_dir: None,
            offline_days: None,
            pin_hours: None,
            pin_until: None,
            folder: None,
        },
        state: Some(st),
        busy: false,
        again: false,
        last_attempt: None,
        error: None,
        org_key: Some(org_kp),
        super_key: Some(super_kp),
        full: vec![],
        code: None,
        names,
        pin: None,
        offline_locked: false,
        ui,
    });
}

/// A second organization for the vault-tabs mockups (32 to 32d, D-69): Lea is a member of
/// "Acme Partners", with two spaces and three items.
pub fn second_org(s: &mut crate::app::Session) {
    use cleis_core::org::*;
    let Some(first) = s.org.as_ref().and_then(|o| o.state.clone()) else {
        return;
    };
    let mut ids = vec![];
    for name in ["Shared", "Projects"] {
        let id = s
            .spaces
            .iter()
            .find(|x| x.name == name)
            .map(|x| x.id)
            .or_else(|| {
                s.vault
                    .create_space(
                        name,
                        SpaceKind::Organization,
                        EncryptionMode::Standard,
                        false,
                    )
                    .ok()
            });
        if let Some(id) = id {
            ids.push(id);
        }
    }
    if ids.len() < 2 {
        return;
    }
    for (name, user, sp, url) in [
        (
            "Acme portal",
            "lea@example-acme.com",
            0,
            "portal.example-acme.com",
        ),
        (
            "Shared drive",
            "partners@example-acme.com",
            0,
            "drive.example-acme.com",
        ),
        (
            "Jira",
            "lea.martin@example-acme.com",
            1,
            "acme.example-jira.com",
        ),
    ] {
        if s.entries.iter().any(|e| e.name == name) {
            continue;
        }
        let mut e = Entry::new(ids[sp], name);
        e.username = user.into();
        e.password = format!(
            "Demo-{}-2048!x",
            name.split_whitespace().next().unwrap_or("x")
        );
        e.urls = vec![url.into()];
        let _ = s.vault.put_entry(&e);
    }
    let _ = crate::sync::reload(s);
    let now = cleis_core::now();
    let mut me = first.me.clone();
    me.id = cleis_core::MemberId::new();
    me.role = "member".into();
    me.email = "lea@example-acme.com".into();
    me.spaces = ids.clone();
    let mut other = me.clone();
    other.id = cleis_core::MemberId::new();
    other.name = "Paul Lefèvre".into();
    other.email = "paul@example-acme.com".into();
    other.role = SUPER_ADMIN.into();
    let spaces = ids
        .iter()
        .map(|id| SpaceView {
            id: *id,
            sealed: vec![],
            mode: SpaceMode::Standard,
            key_gen: 1,
            wrapped: None,
            org_wrapped: None,
            recovery_wrapped: None,
            reserved: false,
            members: vec![me.id, other.id],
            items: s.entries.iter().filter(|e| e.space == *id).count() as u32,
            needs_rekey: false,
        })
        .collect();
    let mut st = first;
    st.org.id = uuid::Uuid::from_u128(0xac3e_0001);
    st.org.name = "Acme Partners".into();
    st.me = me.clone();
    st.members = vec![other, me.clone()];
    st.invites = vec![];
    st.spaces = spaces;
    st.events = 0;
    let names = ids
        .iter()
        .zip(["Shared", "Projects"])
        .map(|(id, n)| (*id, n.to_owned()))
        .collect();
    s.orgs_idle.push(crate::org::OrgSession {
        m: crate::org::Membership {
            org: st.org.id,
            name: "Acme Partners".into(),
            address: "vault.example-acme.com:8443".into(),
            pins: vec![],
            member: me.id,
        },
        token: None,
        local: crate::org::Local {
            last_ok: Some(now - 300),
            ..Default::default()
        },
        state: Some(st),
        busy: false,
        again: false,
        last_attempt: None,
        error: None,
        org_key: None,
        super_key: None,
        full: vec![],
        code: None,
        names,
        pin: None,
        offline_locked: false,
        ui: Default::default(),
    });
    s.refilter();
}
