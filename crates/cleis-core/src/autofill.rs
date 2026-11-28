//! Autofill logic without input/output (D-31, screens 16, 17): which items match the window
//! in front, and the sequence of keys and values that fills a login form. The platform layer
//! (`cleis-autofill`) only executes these plans, so every rule here is tested on any OS.

use crate::entry::{Entry, LinkedApp};
use crate::ids::EntryId;
use crate::text::fold;

/// What is known about the window in front when a shortcut is pressed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Context {
    /// Program file name in lower case (`figma.exe`) or macOS bundle id (`com.figma.Desktop`).
    pub program: String,
    /// Human name of the app (`Figma`).
    pub app_name: String,
    /// Window title.
    pub title: String,
    /// Address of the active tab, for browsers when reading it is allowed.
    pub url: Option<String>,
}

const BROWSERS: [&str; 14] = [
    "chrome.exe",
    "msedge.exe",
    "firefox.exe",
    "brave.exe",
    "vivaldi.exe",
    "opera.exe",
    "arc.exe",
    "com.google.chrome",
    "com.microsoft.edgemac",
    "org.mozilla.firefox",
    "com.apple.safari",
    "com.brave.browser",
    "company.thebrowser.browser",
    "com.vivaldi.vivaldi",
];

impl Context {
    pub fn is_browser(&self) -> bool {
        let p = self.program.to_ascii_lowercase();
        BROWSERS.contains(&p.as_str())
    }

    /// Host of the active tab without `www.`.
    pub fn host(&self) -> Option<String> {
        self.url.as_deref().and_then(host_of)
    }
}

/// `https://www.Figma.com/login` or `figma.com/login` → `figma.com`.
pub fn host_of(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() || s.contains(char::is_whitespace) {
        return None;
    }
    let with_scheme = if s.contains("://") {
        s.to_owned()
    } else {
        format!("https://{s}")
    };
    let u = url::Url::parse(&with_scheme).ok()?;
    let h = u.host_str()?.to_ascii_lowercase();
    if !h.contains('.') && h != "localhost" {
        return None;
    }
    Some(h.strip_prefix("www.").unwrap_or(&h).to_owned())
}

/// `app.figma.com` matches an item saved for `figma.com`, not the other way round, and
/// `notfigma.com` never matches `figma.com`.
pub fn host_matches(page: &str, saved: &str) -> bool {
    page == saved || page.ends_with(&format!(".{saved}"))
}

/// Why an item is proposed, strongest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    /// The program was linked to this item (learned at the first fill).
    Linked,
    /// The tab address matches one of the item's websites.
    Website,
    /// One of the item's websites is on the same site as the tab, another sub-domain
    /// (`billing.example-books.com` for `app.example-books.com`).
    SameSite,
    /// The tab's site name is in the item's name (`example-books.com` → "Example Books"), the
    /// item having no website for it.
    SiteName,
    /// The window title or app name contains the item's name or site (outside browsers, or
    /// in a browser whose address is unknown).
    Name,
}

/// The site of a host: its registrable domain (`app.example-books.co.uk` →
/// `example-books.co.uk`), by the public suffix list.
pub fn site_of(host: &str) -> Option<String> {
    psl::domain_str(host).map(str::to_owned)
}

/// The name part of a site, letters and digits only (`example-books.com` → `examplebooks`).
fn site_word(site: &str) -> String {
    let suffix = psl::suffix_str(site).unwrap_or("");
    let label = site
        .strip_suffix(suffix)
        .map(|s| s.trim_end_matches('.'))
        .unwrap_or(site);
    let label = label.rsplit('.').next().unwrap_or(label);
    fold(label)
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub entry: EntryId,
    pub reason: Reason,
}

/// Items matching the window in front, best first; ties keep favorites, then name order.
pub fn rank(entries: &[Entry], ctx: &Context) -> Vec<Candidate> {
    let host = ctx.host();
    let browser = ctx.is_browser();
    let title = fold(&format!("{} {}", ctx.title, ctx.app_name));
    let mut out: Vec<(Reason, bool, String, EntryId)> = vec![];
    for e in entries {
        let linked = !browser
            && e.linked_apps
                .iter()
                .any(|l| l.program.eq_ignore_ascii_case(&ctx.program));
        let saved: Vec<String> = e.urls.iter().filter_map(|u| host_of(u)).collect();
        let site = host
            .as_deref()
            .is_some_and(|h| saved.iter().any(|s| host_matches(h, s)));
        let page_site = host.as_deref().and_then(site_of);
        let same_site = page_site
            .as_deref()
            .is_some_and(|ps| saved.iter().any(|s| site_of(s).as_deref() == Some(ps)));
        // The site's name in the item's name, for an item with no website elsewhere.
        let site_name = saved.is_empty()
            && page_site.as_deref().is_some_and(|ps| {
                let w = site_word(ps);
                let n: String = fold(&e.name)
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect();
                w.chars().count() >= 3 && n.contains(&w)
            });
        let name = {
            let n = fold(&e.name);
            let by_name = n.chars().count() >= 3 && contains_word(&title, &n);
            let by_host = e.host().is_some_and(|h| {
                let base = fold(h.split('.').next().unwrap_or(&h));
                base.chars().count() >= 3 && contains_word(&title, &base)
            });
            by_name || by_host
        };
        let reason = if linked {
            Reason::Linked
        } else if site {
            Reason::Website
        } else if same_site {
            Reason::SameSite
        } else if site_name {
            Reason::SiteName
        } else if name && host.is_none() {
            // In a browser whose address is known, the title alone is not trusted: a page can
            // name any brand in its title.
            Reason::Name
        } else {
            continue;
        };
        out.push((reason, !e.favorite, fold(&e.name), e.id));
    }
    out.sort();
    out.into_iter()
        .map(|(reason, _, _, entry)| Candidate { entry, reason })
        .collect()
}

/// `needle` appears in `hay` on word boundaries (`figma` in `figma — design`, not `figmatic`).
fn contains_word(hay: &str, needle: &str) -> bool {
    let is_word = |c: char| c.is_alphanumeric();
    hay.match_indices(needle).any(|(i, _)| {
        let before = hay[..i].chars().next_back();
        let after = hay[i + needle.len()..].chars().next();
        !before.is_some_and(is_word) && !after.is_some_and(is_word)
    })
}

/// The item "Fill best match" may fill without asking: exactly one item linked to the app or
/// saved for the site. A match by the window's title fills only when `ask_unlinked` is off;
/// one by the same site or the site's name never does (Quick Access lists them first, 0.2.4).
pub fn best(candidates: &[Candidate], ask_unlinked: bool) -> Option<EntryId> {
    let top = candidates.first()?;
    if matches!(top.reason, Reason::SameSite | Reason::SiteName)
        || (top.reason == Reason::Name && ask_unlinked)
    {
        return None;
    }
    let tied = candidates.iter().filter(|c| c.reason == top.reason).count();
    (tied == 1).then_some(top.entry)
}

/// Records that `ctx`'s program uses `entry` (Quick Access, first fill). Browsers are never
/// linked: one browser serves every site.
pub fn learn_link(entry: &mut Entry, ctx: &Context) -> bool {
    if ctx.program.is_empty()
        || ctx.is_browser()
        || entry
            .linked_apps
            .iter()
            .any(|l| l.program.eq_ignore_ascii_case(&ctx.program))
    {
        return false;
    }
    let display_name = if ctx.app_name.is_empty() {
        ctx.program.clone()
    } else {
        ctx.app_name.clone()
    };
    entry.linked_apps.push(LinkedApp {
        program: ctx.program.clone(),
        display_name,
    });
    true
}

/// A value Cleis puts into a field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Username,
    Password,
    /// Current 2FA code, computed when the step runs.
    Totp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Tab,
    /// Shift+Tab.
    BackTab,
    Enter,
}

/// Kind of the field that has the keyboard focus, when the system can tell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    /// No accessibility information (typing only, or the app exposes nothing).
    Unknown,
    EmptyText,
    Text,
    Password,
    /// Not a text field (a button, a link): typing there would go nowhere or somewhere wrong.
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Fill(Field),
    Press(Key),
    /// Go on if the focused field (after Tab from the username) is a password field, or when
    /// the system cannot tell (the classic auto-type sequence). The focus is read again for up
    /// to [`FOCUS_SETTLE_MS`]: browsers move it a moment after Tab. Still elsewhere: the page's
    /// own password field, when it has one, gets the keyboard and the fill goes on (a link
    /// between the two fields, 0.2.4 fix). Only a page without any password field is a two-page
    /// sign-in: back to the username field (Shift+Tab, so Enter is never pressed on the link or
    /// button Tab reached), Enter to send it, then wait up to [`TWO_PAGE_WAIT_MS`] for the
    /// password page, and go on there. When the system cannot search the page, or no password
    /// field comes in time, or another window comes in front: nothing is sent, the rest waits
    /// for the next press of "Fill best match" on the same window.
    ExpectPassword,
    /// Branch on whether the focused field is an empty text field (unknown counts as no).
    IfEmptyText {
        then: Vec<Action>,
        otherwise: Vec<Action>,
    },
}

/// How long a two-page sign-in may take to show its password page (see
/// [`Action::ExpectPassword`]).
pub const TWO_PAGE_WAIT_MS: u64 = 10_000;

/// How long the focus may take to reach the next field after Tab (see
/// [`Action::ExpectPassword`]).
pub const FOCUS_SETTLE_MS: u64 = 600;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanOptions {
    pub press_enter: bool,
    pub totp_after: bool,
}

/// Steps for one press, and the steps kept for later (the 2FA page that follows the password).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub now: Vec<Action>,
    pub later: Vec<Action>,
}

/// Sign-in sequence for an item, from the field that has the focus.
pub fn plan(has_username: bool, has_totp: bool, focus: Focus, o: PlanOptions) -> Plan {
    use Action::*;
    let enter = |v: &mut Vec<Action>| {
        if o.press_enter {
            v.push(Press(Key::Enter));
        }
    };
    let mut now = vec![];
    match (focus, has_username) {
        // Focus already in the password field: put the username in the field before it only
        // if that one is empty, then come back.
        (Focus::Password, true) => {
            now.push(Press(Key::BackTab));
            now.push(IfEmptyText {
                then: vec![Fill(Field::Username), Press(Key::Tab)],
                otherwise: vec![Press(Key::Tab)],
            });
            now.push(Fill(Field::Password));
        }
        (_, true) => {
            now.push(Fill(Field::Username));
            now.push(Press(Key::Tab));
            now.push(ExpectPassword);
            now.push(Fill(Field::Password));
        }
        (_, false) => now.push(Fill(Field::Password)),
    }
    enter(&mut now);
    let mut later = vec![];
    if has_totp && o.totp_after {
        later.push(Fill(Field::Totp));
        enter(&mut later);
    }
    Plan { now, later }
}

/// An item signed in with another account (D-113): its username only, where the focus is, and
/// no Enter (the page goes on with the other service's button, not with a password).
pub fn plan_sign_in() -> Plan {
    Plan {
        now: vec![Action::Fill(Field::Username)],
        later: vec![],
    }
}

/// Only the 2FA code (Quick Access "2FA code" or a pending step).
pub fn plan_totp(o: PlanOptions) -> Vec<Action> {
    let mut v = vec![Action::Fill(Field::Totp)];
    if o.press_enter {
        v.push(Action::Press(Key::Enter));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::SpaceId;

    #[test]
    fn signed_in_elsewhere_types_the_username_only() {
        let p = plan_sign_in();
        assert_eq!(p.now, vec![Action::Fill(Field::Username)]);
        assert!(p.later.is_empty());
    }

    fn entry(name: &str, url: &str) -> Entry {
        let mut e = Entry::new(SpaceId::new(), name);
        if !url.is_empty() {
            e.urls.push(url.into());
        }
        e.username = "design@example-studio.com".into();
        e
    }

    fn ctx(program: &str, title: &str, url: Option<&str>) -> Context {
        Context {
            program: program.into(),
            app_name: String::new(),
            title: title.into(),
            url: url.map(Into::into),
        }
    }

    #[test]
    fn hosts() {
        assert_eq!(
            host_of("https://www.Figma.com/login").as_deref(),
            Some("figma.com")
        );
        assert_eq!(
            host_of("app.example-books.com/x").as_deref(),
            Some("app.example-books.com")
        );
        assert_eq!(host_of("Search Google or type a URL"), None);
        assert_eq!(host_of("about:blank"), None);
        assert!(host_matches("app.figma.com", "figma.com"));
        assert!(!host_matches("notfigma.com", "figma.com"));
        assert!(!host_matches("figma.com", "app.figma.com"));
    }

    #[test]
    fn ranking_prefers_link_then_site_and_distrusts_titles_in_browsers() {
        let mut books = entry("Example Books", "https://app.example-books.com");
        books.linked_apps.push(LinkedApp {
            program: "examplebooks.exe".into(),
            display_name: "Example Books".into(),
        });
        let figma = entry("Figma", "https://figma.com");
        let all = vec![books.clone(), figma.clone()];

        let r = rank(&all, &ctx("ExampleBooks.exe", "Sign in", None));
        assert_eq!(
            r,
            vec![Candidate {
                entry: books.id,
                reason: Reason::Linked
            }]
        );

        let r = rank(
            &all,
            &ctx(
                "chrome.exe",
                "Figma - Google Chrome",
                Some("www.figma.com/login"),
            ),
        );
        assert_eq!(
            r,
            vec![Candidate {
                entry: figma.id,
                reason: Reason::Website
            }]
        );

        // A phishing page titled "Figma" on another host proposes nothing.
        let r = rank(
            &all,
            &ctx(
                "chrome.exe",
                "Figma - Sign in",
                Some("figma-login.example.net"),
            ),
        );
        assert!(r.is_empty());

        // A desktop app not linked yet: matched by its title, but not filled without asking.
        let r = rank(&all, &ctx("figma.exe", "Figma", None));
        assert_eq!(r[0].reason, Reason::Name);
        assert_eq!(best(&r, true), None);
        assert_eq!(best(&r, false), Some(figma.id));
        // Whole words only.
        assert!(rank(&all, &ctx("x.exe", "Figmatic", None)).is_empty());
    }

    /// 0.2.4 (mockup 48k): a page no item is saved for exactly. Same site first, then the
    /// site's name in an item's name; titles are not trusted in a browser; nothing fills alone.
    #[test]
    fn most_likely_by_the_site() {
        let billing = entry("Example Books — billing", "billing.example-books.com");
        let named = entry("Example Books", "");
        let test = entry("Example Books — test", "");
        let other = entry("Microsoft 365", "microsoft.com");
        let lookalike = entry("Example Bookshelf", "example-bookshelf.com");
        let all = vec![
            other.clone(),
            test.clone(),
            lookalike.clone(),
            named.clone(),
            billing.clone(),
        ];
        let c = ctx(
            "chrome.exe",
            "Sign in to Microsoft 365 — Example Books",
            Some("https://app.example-books.com/login"),
        );
        let r = rank(&all, &c);
        let got: Vec<(EntryId, Reason)> = r.iter().map(|x| (x.entry, x.reason)).collect();
        assert_eq!(
            got,
            vec![
                (billing.id, Reason::SameSite),
                (named.id, Reason::SiteName),
                (test.id, Reason::SiteName),
            ]
        );
        assert_eq!(best(&r, false), None, "Quick Access opens");
        // Two-level suffixes: the site is the registrable domain.
        assert_eq!(
            site_of("app.example-books.co.uk").as_deref(),
            Some("example-books.co.uk")
        );
        assert_eq!(site_word("example-books.co.uk"), "examplebooks");
    }

    #[test]
    fn best_needs_a_single_top_match() {
        let a = entry("GitHub", "https://github.com");
        let b = entry("GitHub — test", "https://github.com");
        let r = rank(
            &[a, b],
            &ctx("firefox.exe", "", Some("https://github.com/login")),
        );
        assert_eq!(r.len(), 2);
        assert_eq!(best(&r, true), None);
    }

    #[test]
    fn learning_links_skips_browsers_and_duplicates() {
        let mut e = entry("Example Books", "");
        assert!(learn_link(&mut e, &ctx("ExampleBooks.exe", "", None)));
        assert!(!learn_link(&mut e, &ctx("examplebooks.exe", "", None)));
        assert!(!learn_link(&mut e, &ctx("msedge.exe", "", None)));
        assert_eq!(e.linked_apps.len(), 1);
    }

    #[test]
    fn plans() {
        use Action::*;
        let o = PlanOptions {
            press_enter: true,
            totp_after: true,
        };
        let p = plan(true, true, Focus::Text, o);
        assert_eq!(
            p.now,
            vec![
                Fill(Field::Username),
                Press(Key::Tab),
                ExpectPassword,
                Fill(Field::Password),
                Press(Key::Enter)
            ]
        );
        assert_eq!(p.later, vec![Fill(Field::Totp), Press(Key::Enter)]);

        let p = plan(
            true,
            false,
            Focus::Password,
            PlanOptions {
                press_enter: false,
                totp_after: true,
            },
        );
        assert_eq!(p.now[0], Press(Key::BackTab));
        assert_eq!(p.now.last(), Some(&Fill(Field::Password)));
        assert!(p.later.is_empty());

        let p = plan(false, false, Focus::Unknown, o);
        assert_eq!(p.now, vec![Fill(Field::Password), Press(Key::Enter)]);
    }
}
