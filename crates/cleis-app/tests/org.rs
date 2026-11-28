//! Organization scenario against a real `cleis-server` on 127.0.0.1, two windows: Lea (first
//! admin) and Hugo (invited member).
//!
//! Lea creates her vault with the server's first invitation code, makes a space, invites Hugo;
//! Hugo joins with the code and receives the space's items once Lea's app wraps the key for him.
//! Lea requires a verification code: Hugo sets one up. Lea checks the activity log, then
//! removes Hugo, whose app forgets the space.

use cleis_app::app::{CleisApp, Phase, Tab};
use cleis_app::theme;
use cleis_core::org::AccessWith;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AppContext, Bounds, Entity, Point, TestAppContext, WindowBounds, WindowHandle, WindowOptions,
    base::Root, px, size,
};
use std::path::{Path, PathBuf};

fn open(cx: &mut TestAppContext, path: &Path) -> (WindowHandle<Root>, Entity<CleisApp>) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        // Animations at their end state: elements are where the test clicks.
        cx.set_reduce_motion(true);
        theme::apply(&theme::Appearance::default(), cx);
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::default(),
                size: size(px(1440.), px(900.)),
            })),
            ..Default::default()
        };
        let path = path.to_owned();
        let (w, app) = gpui_kit::open_window(opts, cx, move |window, cx| {
            cx.new(|cx| {
                let mut a = CleisApp::new(path, None, window, cx);
                a.no_shortcuts = true;
                a.autofill.dry_run = Some(vec![]);
                a
            })
        })
        .expect("window");
        (w.downcast::<Root>().expect("root"), app)
    })
}

fn act(
    cx: &mut TestAppContext,
    h: WindowHandle<Root>,
    f: impl FnOnce(&mut gpui_kit::Window, &mut gpui_kit::App),
) {
    cx.update_window(h.into(), |_, w, cx| f(w, cx)).unwrap();
    cx.run_until_parked();
}

/// A server in its own thread and runtime.
fn server(dir: PathBuf) -> (String, String) {
    let code = cleis_server::App::init(&dir, "Example Studio", &[]).unwrap();
    let app = cleis_server::App::open(&dir).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            tx.send(l.local_addr().unwrap().to_string()).unwrap();
            cleis_server::serve(app, l, std::future::pending())
                .await
                .unwrap();
        });
    });
    (rx.recv().unwrap(), code)
}

/// Runs syncs until the app has nothing left to do with the server.
fn window_has(cx: &mut TestAppContext, h: WindowHandle<Root>, id: &str) -> bool {
    let id = gpui_kit::SharedString::from(id.to_owned());
    cx.update_window(h.into(), move |_, w, _| w.try_find(id).is_some())
        .unwrap()
}

fn sync(cx: &mut TestAppContext, app: &Entity<CleisApp>) {
    for _ in 0..4 {
        app.update(cx, |a, cx| a.org_sync(None, cx));
        cx.run_until_parked();
    }
}

fn session<R>(
    cx: &mut TestAppContext,
    app: &Entity<CleisApp>,
    f: impl FnOnce(&cleis_app::app::Session) -> R,
) -> R {
    app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => f(s),
        _ => panic!("not unlocked"),
    })
}

fn first_launch(
    cx: &mut TestAppContext,
    h: WindowHandle<Root>,
    addr: &str,
    code: &str,
    name: Option<&str>,
) {
    act(cx, h, |w, cx| w.click("org-server", cx));
    act(cx, h, |w, cx| {
        w.click("org-address", cx);
        w.input(addr, cx);
        w.click("org-code", cx);
        w.input(code, cx);
    });
    if let Some(n) = name {
        act(cx, h, |w, cx| {
            w.click("org-name", cx);
            w.input(n, cx);
        });
    }
    act(cx, h, |w, cx| {
        w.click("new-password", cx);
        w.input("correct horse battery staple", cx);
        w.click("confirm-password", cx);
        w.input("correct horse battery staple", cx);
        w.click("create", cx);
    });
    // The recovery key shown once after the creation (D-76).
    act(cx, h, |w, cx| w.click("recovery-key-done", cx));
}

#[gpui_kit::test]
fn organization_admin_and_member(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (addr, code) = server(dir.path().join("server"));

    // Lea: first launch with the server's first code (no name in it: she types hers).
    let (lh, lea) = open(cx, &dir.path().join("lea.cleis"));
    cx.run_until_parked();
    first_launch(cx, lh, &addr, &code, Some("Lea Martin"));
    session(cx, &lea, |s| {
        let o = s.org.as_ref().expect("joined");
        assert_eq!(o.m.name, "Example Studio");
    });
    sync(cx, &lea);
    session(cx, &lea, |s| {
        let o = s.org.as_ref().unwrap();
        assert!(o.error.is_none(), "{:?}", o.error);
        let st = o.state.as_ref().unwrap();
        assert_eq!(st.me.role, cleis_core::org::SUPER_ADMIN);
        assert!(
            st.org.org_public.is_some(),
            "organization key made by the first admin"
        );
        assert!(o.org_key.is_some());
    });

    // New space from the sidebar.
    act(cx, lh, |w, cx| w.click("new-space", cx));
    act(cx, lh, |w, cx| {
        w.click("space-name", cx);
        w.input("General", cx);
        w.click("space-create", cx);
    });
    sync(cx, &lea);
    let general = session(cx, &lea, |s| {
        let o = s.org.as_ref().unwrap();
        let st = o.state.as_ref().unwrap();
        assert_eq!(st.spaces.len(), 1);
        assert_eq!(o.space_name(st.spaces[0].id), "General");
        st.spaces[0].id
    });
    // An item in it, sent at the next sync.
    lea.update(cx, |a, _| {
        if let Phase::Main(s) = &mut a.phase {
            let mut e = cleis_core::Entry::new(general, "Figma");
            e.username = "design@example-studio.com".into();
            e.password = "figma-pass-2026".into();
            s.vault.put_entry(&e).unwrap();
        }
    });
    sync(cx, &lea);
    session(cx, &lea, |s| {
        assert_eq!(
            s.org.as_ref().unwrap().state.as_ref().unwrap().spaces[0].items,
            1
        )
    });

    // Invitation from Settings › Members.
    act(cx, lh, |w, cx| w.click("tab-Settings", cx));
    act(cx, lh, |w, cx| w.click("set-Members", cx));
    act(cx, lh, |w, cx| w.click("invite", cx));
    act(cx, lh, |w, cx| {
        w.click("inv-email", cx);
        w.input("hugo@example-studio.com", cx);
        w.click("inv-name", cx);
        w.input("Hugo Bernard", cx);
        w.click(format!("inv-sp-{general}"), cx);
        w.click("inv-send", cx);
    });
    let hugo_code = session(cx, &lea, |s| {
        s.org
            .as_ref()
            .unwrap()
            .ui
            .invite_code
            .clone()
            .expect("code shown")
    });
    act(cx, lh, |w, cx| w.click("invite-done", cx));
    sync(cx, &lea);
    session(cx, &lea, |s| {
        assert_eq!(
            s.org
                .as_ref()
                .unwrap()
                .state
                .as_ref()
                .unwrap()
                .invites
                .len(),
            1
        )
    });

    // Hugo joins: his name comes with the invitation.
    let (hh, hugo) = open(cx, &dir.path().join("hugo.cleis"));
    cx.run_until_parked();
    first_launch(cx, hh, &addr, &hugo_code, None);
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        let st = s.org.as_ref().unwrap().state.as_ref().unwrap();
        assert_eq!(st.me.name, "Hugo Bernard");
        assert!(s.entries.iter().all(|e| e.space != general), "no key yet");
    });
    // Lea's app wraps the space key for him; Hugo receives the item.
    sync(cx, &lea);
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        let f = s
            .entries
            .iter()
            .find(|e| e.name == "Figma")
            .expect("shared item");
        assert_eq!(f.password, "figma-pass-2026");
        assert_eq!(f.space, general);
    });

    // Reserved space (D-61) from the sidebar dialog: hidden from Hugo until Lea, a Super Admin,
    // gives it to him; then his app gets its key from Lea's.
    act(cx, lh, |w, cx| w.click("tab-Vault", cx));
    act(cx, lh, |w, cx| w.click("new-space", cx));
    act(cx, lh, |w, cx| {
        w.click("space-name", cx);
        w.input("Accounting", cx);
        w.click("space-reserved", cx);
        w.click("space-create", cx);
    });
    sync(cx, &lea);
    let acc = session(cx, &lea, |s| {
        let sv = s
            .org
            .as_ref()
            .unwrap()
            .state
            .as_ref()
            .unwrap()
            .spaces
            .iter()
            .find(|x| x.reserved)
            .cloned()
            .expect("reserved space");
        assert_eq!(sv.mode, cleis_core::org::SpaceMode::EndToEnd);
        sv.id
    });
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        assert!(
            s.org
                .as_ref()
                .unwrap()
                .state
                .as_ref()
                .unwrap()
                .spaces
                .iter()
                .all(|x| x.id != acc)
        )
    });
    let hugo_member = session(cx, &hugo, |s| s.org.as_ref().unwrap().m.member);
    act(cx, lh, |w, cx| w.click("tab-Settings", cx));
    act(cx, lh, |w, cx| w.click("set-Members", cx));
    act(cx, lh, |w, cx| w.click(format!("spaces-{hugo_member}"), cx));
    act(cx, lh, |w, cx| {
        w.click(format!("msp-{hugo_member}-{acc}"), cx)
    });
    // The menu stays open for several spaces; a click outside closes it (audit U-2).
    act(cx, lh, |w, cx| w.click("set-Members", cx));
    session(cx, &lea, |s| {
        assert!(s.org.as_ref().unwrap().ui.menu.is_none());
    });
    sync(cx, &lea);
    sync(cx, &hugo);
    sync(cx, &lea);
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        assert!(
            s.spaces
                .iter()
                .any(|x| x.id == acc && x.name == "Accounting"),
            "key received"
        )
    });

    // Detail "Everything" (lower on the page: set directly), then Hugo copies the item: the log
    // keeps who, what, and the item's name sealed for the admins.
    lea.update(cx, |a, cx| {
        a.org_set_policy(|p| p.detail = cleis_core::org::LogDetail::Everything, cx)
    });
    cx.run_until_parked();
    sync(cx, &hugo);
    let figma = session(cx, &hugo, |s| {
        s.entries.iter().find(|e| e.name == "Figma").unwrap().id
    });
    // Its row first: the copy buttons show on the selected row.
    act(cx, hh, |w, cx| {
        w.click_at(
            format!("row-{figma}"),
            gpui_kit::point(px(30.), px(10.)),
            cx,
        )
    });
    act(cx, hh, |w, cx| w.click(format!("cu-{figma}"), cx));
    cx.run_until_parked();

    // Policy: verification code required. Hugo is asked to set one up at his next sign-in.
    act(cx, lh, |w, cx| w.click("set-Security policy", cx));
    act(cx, lh, |w, cx| w.click("code-req-1", cx));
    sync(cx, &lea);
    hugo.update(cx, |a, _| {
        if let Phase::Main(s) = &mut a.phase
            && let Some(o) = s.org.as_mut()
        {
            o.token = None;
        }
    });
    sync(cx, &hugo);
    let secret = session(cx, &hugo, |s| {
        s.org
            .as_ref()
            .unwrap()
            .code
            .as_ref()
            .and_then(|c| c.secret.clone())
            .expect("set-up dialog")
    });
    let now = cleis_core::now();
    let code_now = cleis_core::totp::Totp::from_base32(&secret)
        .unwrap()
        .code_at(now);
    act(cx, hh, |w, cx| {
        w.click("srv-code", cx);
        w.input(&code_now, cx);
        w.click("srv-code-ok", cx);
    });
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        let o = s.org.as_ref().unwrap();
        assert!(o.code.is_none());
        assert!(o.error.is_none(), "{:?}", o.error);
        assert!(o.state.as_ref().unwrap().me.two_fa);
    });

    // Activity: Lea sees Hugo's copy with the item's name.
    sync(cx, &lea);
    act(cx, lh, |w, cx| w.click("tab-Activity", cx));
    session(cx, &lea, |s| {
        assert_eq!(s.tab, Tab::Activity);
        let o = s.org.as_ref().unwrap();
        let page = o.ui.log.as_ref().expect("log loaded");
        let copy = page
            .events
            .iter()
            .find(|e| e.action == cleis_core::org::Action::Copied)
            .expect("copy written");
        let k = o.org_key.as_ref().unwrap();
        let d = cleis_crypto::open_from(
            k,
            cleis_core::org::DETAIL_CONTEXT,
            copy.detail.as_ref().unwrap(),
        )
        .unwrap();
        let d: cleis_core::org::EventDetail = serde_json::from_slice(&d).unwrap();
        assert_eq!(d.item, "Figma");
        assert!(
            page.events
                .iter()
                .any(|e| e.action == cleis_core::org::Action::Joined)
        );
    });
    act(cx, lh, |w, cx| w.click("log-verify", cx));
    session(cx, &lea, |s| {
        let i = s
            .org
            .as_ref()
            .unwrap()
            .state
            .as_ref()
            .unwrap()
            .integrity
            .clone()
            .unwrap();
        assert!(i.intact, "{i:?}");
    });

    // Super Admin (D-60): Lea transfers the role to Hugo from his row's menu, and becomes Admin;
    // Hugo transfers it back the same way.
    let lea_id = session(cx, &lea, |s| s.org.as_ref().unwrap().m.member);
    let hugo_id0 = session(cx, &hugo, |s| s.org.as_ref().unwrap().m.member);
    act(cx, lh, |w, cx| w.click("tab-Settings", cx));
    act(cx, lh, |w, cx| w.click("set-Members", cx));
    act(cx, lh, |w, cx| w.click(format!("more-{hugo_id0}"), cx));
    act(cx, lh, |w, cx| w.click(format!("transfer-{hugo_id0}"), cx));
    act(cx, lh, |w, cx| w.click("confirm-ok", cx));
    sync(cx, &lea);
    sync(cx, &hugo);
    session(cx, &lea, |s| {
        let st = s.org.as_ref().unwrap().state.as_ref().unwrap();
        assert_eq!(st.me.role, cleis_core::org::ADMIN);
    });
    session(cx, &hugo, |s| {
        let st = s.org.as_ref().unwrap().state.as_ref().unwrap();
        assert_eq!(st.me.role, cleis_core::org::SUPER_ADMIN);
    });
    act(cx, hh, |w, cx| w.click("tab-Settings", cx));
    act(cx, hh, |w, cx| w.click("set-Members", cx));
    // Hugo goes through the row's right-click menu (mockup 33d).
    act(cx, hh, |w, cx| {
        w.right_click(format!("member-{lea_id}"), cx)
    });
    act(cx, hh, |w, cx| w.click(format!("transfer-{lea_id}"), cx));
    act(cx, hh, |w, cx| w.click("confirm-ok", cx));
    sync(cx, &lea);
    session(cx, &lea, |s| {
        let st = s.org.as_ref().unwrap().state.as_ref().unwrap();
        assert_eq!(st.me.role, cleis_core::org::SUPER_ADMIN);
    });
    // Hugo is Admin now: back to Member before he is removed.
    lea.update(cx, |a, cx| {
        a.org_member(
            cleis_core::org::MemberChange {
                member: hugo_id0,
                role: Some("member".into()),
                ..Default::default()
            },
            cx,
        )
    });
    cx.run_until_parked();

    // Settings › Spaces: a space renamed and an empty one deleted.
    act(cx, lh, |w, cx| w.click("set-Spaces", cx));
    act(cx, lh, |w, cx| w.click(format!("rename-{general}"), cx));
    act(cx, lh, |w, cx| {
        w.click("rename-space", cx);
        w.press("ctrl-a", cx);
        w.input("Shared", cx);
        w.click("rename-go", cx);
    });
    sync(cx, &lea);
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        assert!(
            s.spaces
                .iter()
                .any(|x| x.id == general && x.name == "Shared"),
            "renamed for everyone"
        )
    });
    act(cx, lh, |w, cx| w.click("spaces-new", cx));
    act(cx, lh, |w, cx| {
        w.click("space-name", cx);
        w.input("Temp", cx);
        w.click("space-create", cx);
    });
    sync(cx, &lea);
    let temp = session(cx, &lea, |s| {
        s.spaces.iter().find(|x| x.name == "Temp").unwrap().id
    });
    act(cx, lh, |w, cx| w.click(format!("delspace-{temp}"), cx));
    act(cx, lh, |w, cx| w.click("del-space-go", cx));
    sync(cx, &lea);
    session(cx, &lea, |s| {
        assert!(s.spaces.iter().all(|x| x.id != temp));
        assert!(
            s.org
                .as_ref()
                .unwrap()
                .state
                .as_ref()
                .unwrap()
                .spaces
                .iter()
                .all(|x| x.id != temp)
        );
    });

    // Lea removes Hugo: his app forgets the space; the space is marked for re-encryption.
    let hugo_id = session(cx, &hugo, |s| s.org.as_ref().unwrap().m.member);
    act(cx, lh, |w, cx| w.click("tab-Settings", cx));
    act(cx, lh, |w, cx| w.click("set-Members", cx));
    act(cx, lh, |w, cx| w.click(format!("more-{hugo_id}"), cx));
    act(cx, lh, |w, cx| w.click(format!("revoke-{hugo_id}"), cx));
    act(cx, lh, |w, cx| w.click("confirm-ok", cx));
    sync(cx, &lea);
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        assert!(s.org.is_none(), "removed");
        assert!(s.entries.iter().all(|e| e.name != "Figma"));
    });

    // Re-encryption from Settings › Encryption, to end-to-end.
    act(cx, lh, |w, cx| w.click("set-Encryption", cx));
    session(cx, &lea, |s| {
        assert!(s.org.as_ref().unwrap().state.as_ref().unwrap().spaces[0].needs_rekey)
    });
    act(cx, lh, |w, cx| w.click(format!("mode-{general}-1"), cx));
    act(cx, lh, |w, cx| w.click("reencrypt", cx));
    act(cx, lh, |w, cx| w.click("rk-backup", cx));
    act(cx, lh, |w, cx| w.click("rk-go", cx));
    sync(cx, &lea);
    session(cx, &lea, |s| {
        let sv = &s.org.as_ref().unwrap().state.as_ref().unwrap().spaces[0];
        assert_eq!(sv.key_gen, 2);
        assert!(!sv.needs_rekey);
        assert_eq!(sv.mode, cleis_core::org::SpaceMode::EndToEnd);
        let f = s.entries.iter().find(|e| e.name == "Figma").unwrap();
        assert_eq!(f.password, "figma-pass-2026");
    });
}

/// Settings › Organization › Create on this computer: Cleis starts `cleis-server` beside it and
/// joins as the first administrator (D-63). Skipped when the server is not built beside the tests.
#[gpui_kit::test]
fn organization_hosted_here(cx: &mut TestAppContext) {
    if cleis_app::host::server_exe().is_none() {
        // Audit X-3: where everything must run (Linux CI), a missing server is a failure.
        assert!(
            std::env::var_os("CLEIS_REQUIRE_ALL").is_none(),
            "not run: cleis-server is not built"
        );
        eprintln!("skipped: cleis-server is not built");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    cleis_app::host::use_dir(dir.path().join("server"));
    let (h, app) = open(cx, &dir.path().join("lea.cleis"));
    cx.run_until_parked();
    act(cx, h, |w, cx| {
        w.click("local-only", cx);
        w.click("new-password", cx);
        w.input("correct horse battery staple", cx);
        w.click("confirm-password", cx);
        w.input("correct horse battery staple", cx);
        w.click("create", cx);
    });
    act(cx, h, |w, cx| w.click("recovery-key-done", cx));
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Organization", cx));
    act(cx, h, |w, cx| {
        // D-78: the server on this computer, not on a NAS.
        w.click("create-server", cx);
        w.click("create-where-2", cx);
        w.click("create-name", cx);
        w.input("Example Studio", cx);
        w.click("create-me", cx);
        w.input("Lea Martin", cx);
        w.click("create-org", cx);
    });
    for _ in 0..50 {
        cx.run_until_parked();
        if app.read_with(cx, |a, _| match &a.phase {
            Phase::Main(s) => s.org.is_some() || s.join.error.is_some(),
            _ => false,
        }) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    sync(cx, &app);
    session(cx, &app, |s| {
        assert!(s.join.error.is_none(), "{:?}", s.join.error);
        let o = s.org.as_ref().expect("created");
        assert_eq!(o.m.name, "Example Studio");
        assert!(o.m.address.starts_with("127.0.0.1:"));
        assert!(o.local.host_dir.is_some());
        assert!(o.error.is_none(), "{:?}", o.error);
        // Audit L-10: the name typed, not the computer's account.
        assert_eq!(o.me().map(|m| m.name.as_str()), Some("Lea Martin"));
        assert_eq!(
            o.role().map(|r| r.id.clone()).as_deref(),
            Some(cleis_core::org::SUPER_ADMIN)
        );
        assert_eq!(s.settings_page, "Server");
    });
    cleis_app::host::stop();
}

/// Second step by SMS (D-64): set up from Settings › Profile, then asked at the next sign-in. The
/// server keeps the codes instead of sending them.
#[gpui_kit::test]
fn sms_second_step(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let sdir = dir.path().join("server");
    let code = cleis_server::App::init(&sdir, "Example Studio", &[]).unwrap();
    std::fs::write(
        sdir.join("cleis-server.toml"),
        "[sms]\nurl = \"https://sms.example-gateway.com/send\"\nbody = \"{to} {code}\"\n",
    )
    .unwrap();
    let srv = cleis_server::App::open(&sdir).unwrap();
    srv.capture_codes();
    let (tx, rx) = std::sync::mpsc::channel();
    let app2 = srv.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            tx.send(l.local_addr().unwrap().to_string()).unwrap();
            cleis_server::serve(app2, l, std::future::pending())
                .await
                .unwrap();
        });
    });
    let addr = rx.recv().unwrap();
    let (h, lea) = open(cx, &dir.path().join("lea.cleis"));
    cx.run_until_parked();
    first_launch(cx, h, &addr, &code, Some("Lea Martin"));
    sync(cx, &lea);
    lea.update(cx, |a, cx| {
        a.org_set_policy(
            |p| {
                p.methods.insert(cleis_core::org::Method::Sms);
            },
            cx,
        )
    });
    cx.run_until_parked();
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Account", cx));
    // Below the account's own sections (Account holds what Profile was, 27/11/2026).
    act(cx, h, |w, cx| {
        w.scroll(
            "open-code",
            gpui_kit::ScrollDelta::Pixels(gpui_kit::point(gpui_kit::px(0.), gpui_kit::px(-4000.))),
            cx,
        )
    });
    act(cx, h, |w, cx| w.click("second-step-set", cx));
    act(cx, h, |w, cx| w.click("srv-method-1", cx));
    act(cx, h, |w, cx| {
        w.click("srv-phone", cx);
        w.input("+33 6 99 00 00 12", cx);
        w.click("srv-send", cx);
    });
    let sent = srv.sent_codes();
    assert_eq!(sent.len(), 1, "{sent:?}");
    act(cx, h, |w, cx| {
        w.click("srv-code", cx);
        w.input(&sent[0].code, cx);
        w.click("srv-code-ok", cx);
    });
    session(cx, &lea, |s| {
        assert!(s.org.as_ref().unwrap().code.is_none())
    });
    sync(cx, &lea);
    session(cx, &lea, |s| {
        let o = s.org.as_ref().unwrap();
        assert_eq!(o.me().unwrap().factor, Some(cleis_core::org::Method::Sms));
    });

    // Next sign-in (session dropped, device not remembered): a code goes by SMS.
    lea.update(cx, |a, _| {
        if let Phase::Main(s) = &mut a.phase {
            let o = s.org.as_mut().unwrap();
            o.token = None;
            o.local.remembered = None;
        }
    });
    lea.update(cx, |a, cx| a.org_sync(None, cx));
    cx.run_until_parked();
    session(cx, &lea, |s| {
        let c = s.org.as_ref().unwrap().code.as_ref().expect("code asked");
        assert!(c.message.as_deref().unwrap().contains("00 12"));
    });
    let code = srv.sent_codes().last().unwrap().code.clone();
    act(cx, h, |w, cx| {
        w.click("srv-code", cx);
        w.input(&code, cx);
        w.click("srv-code-ok", cx);
    });
    session(cx, &lea, |s| {
        let o = s.org.as_ref().unwrap();
        assert!(o.code.is_none());
        assert!(o.error.is_none(), "{:?}", o.error);
    });
}

/// Audit L-1, L-2: a read-only member's change refused by the server is put back; an
/// organization's item does not move to a personal folder.
#[gpui_kit::test]
fn refused_changes_and_moves(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (addr, code) = server(dir.path().join("server"));
    let (lh, lea) = open(cx, &dir.path().join("lea.cleis"));
    cx.run_until_parked();
    first_launch(cx, lh, &addr, &code, Some("Lea Martin"));
    sync(cx, &lea);
    act(cx, lh, |w, cx| w.click("new-space", cx));
    act(cx, lh, |w, cx| {
        w.click("space-name", cx);
        w.input("General", cx);
        w.click("space-create", cx);
    });
    sync(cx, &lea);
    let general = session(cx, &lea, |s| {
        s.org.as_ref().unwrap().state.as_ref().unwrap().spaces[0].id
    });
    let figma = lea.update(cx, |a, _| {
        let Phase::Main(s) = &mut a.phase else {
            panic!()
        };
        let mut e = cleis_core::Entry::new(general, "Figma");
        e.password = "figma-pass-2026".into();
        s.vault.put_entry(&e).unwrap();
        e.id
    });
    sync(cx, &lea);
    // 27/11/2026 (mockup 44a): a contact at the organization's level, outside any folder: its
    // "Uncategorized" space is made the first time, and goes to whoever is invited.
    let okey = session(cx, &lea, |s| s.org.as_ref().unwrap().m.key());
    act(cx, lh, |w, cx| w.click("tab-Contacts", cx));
    act(cx, lh, |w, cx| w.click(format!("cf-org-{okey}"), cx));
    act(cx, lh, |w, cx| w.click("new-contact", cx));
    sync(cx, &lea);
    let unfiled = session(cx, &lea, |s| {
        let sp = cleis_app::screens::contacts::unfiled_of(s, &okey).expect("made");
        let c = s
            .contacts
            .iter()
            .find(|c| Some(c.id) == s.selected_contact)
            .unwrap();
        assert_eq!(c.space, sp);
        sp
    });
    act(cx, lh, |w, cx| w.click("tab-Settings", cx));
    act(cx, lh, |w, cx| w.click("set-Members", cx));
    act(cx, lh, |w, cx| w.click("invite", cx));
    act(cx, lh, |w, cx| {
        w.click("inv-email", cx);
        w.input("hugo@example-studio.com", cx);
        w.click("inv-name", cx);
        w.input("Hugo Bernard", cx);
        w.click(format!("inv-sp-{general}"), cx);
        w.click("inv-send", cx);
    });
    let hugo_code = session(cx, &lea, |s| {
        s.org.as_ref().unwrap().ui.invite_code.clone().unwrap()
    });
    act(cx, lh, |w, cx| w.click("invite-done", cx));
    let (hh, hugo) = open(cx, &dir.path().join("hugo.cleis"));
    cx.run_until_parked();
    first_launch(cx, hh, &addr, &hugo_code, None);
    sync(cx, &hugo);
    sync(cx, &lea);
    sync(cx, &hugo);
    // Invited, Hugo has the organization's "Uncategorized" contacts too.
    session(cx, &hugo, |s| {
        assert!(
            s.contacts.iter().any(|c| c.space == unfiled),
            "the contact reached him"
        );
    });
    let hugo_id = session(cx, &hugo, |s| s.org.as_ref().unwrap().m.member);
    let lea_id = session(cx, &lea, |s| s.org.as_ref().unwrap().m.member);

    // Mockup 46f (D-101): Settings › Spaces, General unfolded; Lea takes from Hugo the editing
    // and the showing of passwords there (his role keeps both elsewhere). An administrator's
    // boxes cannot be clicked.
    act(cx, lh, |w, cx| w.click("set-Spaces", cx));
    act(cx, lh, |w, cx| w.click(format!("space-row-{general}"), cx));
    assert!(window_has(cx, lh, &format!("sm-edit-{general}-{hugo_id}")));
    act(cx, lh, |w, cx| {
        w.click(format!("sm-edit-{general}-{lea_id}"), cx)
    });
    act(cx, lh, |w, cx| {
        w.click(format!("sm-edit-{general}-{hugo_id}"), cx)
    });
    sync(cx, &lea);
    act(cx, lh, |w, cx| {
        w.click(format!("sm-show-{general}-{hugo_id}"), cx)
    });
    sync(cx, &lea);
    session(cx, &lea, |s| {
        let st = s.org.as_ref().unwrap().state.as_ref().unwrap();
        let h = st.members.iter().find(|m| m.id == hugo_id).unwrap();
        let l = h.limits.get(&general).copied().unwrap_or_default();
        assert!(l.no_edit && l.no_show, "{l:?}");
        let me = st.members.iter().find(|m| m.id == lea_id).unwrap();
        assert!(me.limits.is_empty(), "an administrator keeps everything");
    });
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        assert!(!s.may(general, cleis_core::org::Perm::Edit));
        assert!(!s.may(general, cleis_core::org::Perm::See));
        assert!(s.may(general, cleis_core::org::Perm::Fill));
    });
    // Written behind the interface's back: the server refuses it, the app puts it back.
    hugo.update(cx, |a, _| {
        let Phase::Main(s) = &mut a.phase else {
            panic!()
        };
        let e = s.entries.iter_mut().find(|e| e.id == figma).unwrap();
        e.password = "changed-in-general".into();
        e.touch(None);
        s.vault.put_entry(e).unwrap();
    });
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        let e = s.vault.entries().unwrap();
        assert_eq!(
            e.iter().find(|e| e.id == figma).unwrap().password,
            "figma-pass-2026"
        );
    });
    // Given back: Edit again.
    act(cx, lh, |w, cx| {
        w.click(format!("sm-edit-{general}-{hugo_id}"), cx)
    });
    sync(cx, &lea);
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        assert!(s.may(general, cleis_core::org::Perm::Edit))
    });

    // Mockup 46c (D-101): Lea's profile from the picture at the top right, a photo with it;
    // the organization shows them to Hugo.
    let photo = dir.path().join("lea.png");
    image::RgbImage::from_pixel(300, 200, image::Rgb([120, 90, 60]))
        .save(&photo)
        .unwrap();
    act(cx, lh, |w, cx| w.click("acc-avatar", cx));
    act(cx, lh, |w, cx| w.click("acc-menu-profile", cx));
    lea.update(cx, |a, cx| a.profile_photo_from(&photo, cx));
    act(cx, lh, |w, cx| {
        w.click("profile-me-name", cx);
        w.press("ctrl-a", cx);
        w.input("Léa Martin", cx);
        w.click("profile-me-email", cx);
        w.press("ctrl-a", cx);
        w.input("lea@example-studio.com", cx);
        w.click("profile-me-save", cx);
    });
    cx.run_until_parked();
    session(cx, &lea, |s| {
        if let Some(e) = s.profile_edit.as_ref() {
            panic!("not saved: {:?}", e.error);
        }
    });
    sync(cx, &lea);
    session(cx, &lea, |s| {
        assert_eq!(s.prefs.profile.name, "Léa Martin");
        assert!(
            s.prefs
                .profile
                .photo
                .as_deref()
                .is_some_and(|p| p.starts_with("data:image/jpeg"))
        );
    });
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        let st = s.org.as_ref().unwrap().state.as_ref().unwrap();
        let l = st.members.iter().find(|m| m.id == lea_id).unwrap();
        assert_eq!(l.name, "Léa Martin");
        assert!(l.photo.is_some(), "the photo reached the organization");
    });

    // The e-mail is optional (27/11/2026): emptied, the profile is saved and the organization
    // keeps the address it has.
    act(cx, lh, |w, cx| w.click("acc-avatar", cx));
    act(cx, lh, |w, cx| w.click("acc-menu-profile", cx));
    act(cx, lh, |w, cx| {
        w.click("profile-me-name", cx);
        w.press("ctrl-a", cx);
        w.input("Léa M.", cx);
        w.click("profile-me-email", cx);
        w.press("ctrl-a", cx);
        w.press("backspace", cx);
        w.click("profile-me-save", cx);
    });
    cx.run_until_parked();
    session(cx, &lea, |s| {
        if let Some(e) = s.profile_edit.as_ref() {
            panic!("not saved without an e-mail: {:?}", e.error);
        }
        assert_eq!(s.prefs.profile.email, "");
    });
    sync(cx, &lea);
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        let st = s.org.as_ref().unwrap().state.as_ref().unwrap();
        let l = st.members.iter().find(|m| m.id == lea_id).unwrap();
        assert_eq!(l.name, "Léa M.");
        assert_eq!(l.email, "lea@example-studio.com");
    });

    // Mockup 48g: an admin gives a space an icon; every member sees it.
    let space = session(cx, &hugo, |s| {
        s.spaces
            .iter()
            .find(|x| s.is_org_space(x.id) && !x.for_contacts)
            .map(|x| x.id)
            .expect("a space of the organization")
    });
    lea.update(cx, |a, cx| a.set_space_icon(space, Some("palette"), cx));
    cx.run_until_parked();
    sync(cx, &lea);
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        let sp = s.spaces.iter().find(|x| x.id == space).unwrap();
        assert_eq!(
            sp.icon.as_deref(),
            Some("palette"),
            "the icon reached the member"
        );
    });

    lea.update(cx, |a, cx| {
        a.org_member(
            cleis_core::org::MemberChange {
                member: hugo_id,
                role: Some("read_only".into()),
                ..Default::default()
            },
            cx,
        )
    });
    cx.run_until_parked();
    sync(cx, &hugo);
    // A change written behind the interface's back is refused, then put back.
    hugo.update(cx, |a, _| {
        let Phase::Main(s) = &mut a.phase else {
            panic!()
        };
        let e = s.entries.iter_mut().find(|e| e.id == figma).unwrap();
        e.password = "changed-by-hugo".into();
        e.touch(None);
        s.vault.put_entry(e).unwrap();
    });
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        let e = s.vault.entries().unwrap();
        let f = e.iter().find(|e| e.id == figma).unwrap();
        assert_eq!(f.password, "figma-pass-2026", "server's version put back");
    });
    // Through the interface, the role stops it before anything is written.
    hugo.update(cx, |a, cx| a.trash_entry(figma, cx));
    session(cx, &hugo, |s| {
        assert!(s.entries.iter().any(|e| e.id == figma))
    });

    // Lea cannot move it to her personal vault.
    let home = session(cx, &lea, |s| s.vault.home_space());
    lea.update(cx, |a, cx| a.move_entry(figma, home, cx));
    session(cx, &lea, |s| {
        let f = s.vault.entries().unwrap();
        assert_eq!(f.iter().find(|e| e.id == figma).unwrap().space, general);
    });

    // D-102: the organization's import makes a space per department; the server keeps it.
    let csv = dir.path().join("org.csv");
    std::fs::write(
        &csv,
        "Name,Department,Login,Password\nExample Payroll,Finance,pay@example-studio.com,Pay-2026\n",
    )
    .unwrap();
    act(cx, lh, |w, cx| w.click("tab-Settings", cx));
    act(cx, lh, |w, cx| {
        w.click("set-Organization import & export", cx)
    });
    lea.update(cx, |a, cx| a.load_import(&csv, cx));
    cx.run_until_parked();
    act(cx, lh, |w, cx| w.click("import-continue", cx));
    act(cx, lh, |w, cx| w.click("import-run", cx));
    cx.run_until_parked();
    sync(cx, &lea);
    session(cx, &lea, |s| {
        let o = s.org.as_ref().unwrap();
        let st = o.state.as_ref().unwrap();
        let fin = st
            .spaces
            .iter()
            .find(|sv| o.names.get(&sv.id).is_some_and(|n| n == "Finance"))
            .expect("the space reached the server");
        let e = s
            .entries
            .iter()
            .find(|e| e.name == "Example Payroll")
            .unwrap();
        assert_eq!(e.space, fin.id);
    });
}

/// Scenario (mockups 36g, 36h, D-79): Lea turns the organization PIN on and sets it; Hugo's
/// app closes the organization's spaces and asks the PIN in their place; a wrong PIN is refused,
/// the right one opens them again (the lock-out after too many is in the server's tests).
#[gpui_kit::test]
fn organization_pin(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (addr, code) = server(dir.path().join("server"));
    let (lh, lea) = open(cx, &dir.path().join("lea.cleis"));
    cx.run_until_parked();
    first_launch(cx, lh, &addr, &code, Some("Lea Martin"));
    sync(cx, &lea);
    act(cx, lh, |w, cx| w.click("new-space", cx));
    act(cx, lh, |w, cx| {
        w.click("space-name", cx);
        w.input("General", cx);
        w.click("space-create", cx);
    });
    sync(cx, &lea);
    let general = session(cx, &lea, |s| {
        s.org.as_ref().unwrap().state.as_ref().unwrap().spaces[0].id
    });
    lea.update(cx, |a, _| {
        let Phase::Main(s) = &mut a.phase else {
            panic!()
        };
        s.vault
            .put_entry(&cleis_core::Entry::new(general, "Figma"))
            .unwrap();
    });
    sync(cx, &lea);
    act(cx, lh, |w, cx| w.click("tab-Settings", cx));
    act(cx, lh, |w, cx| w.click("set-Members", cx));
    act(cx, lh, |w, cx| w.click("invite", cx));
    act(cx, lh, |w, cx| {
        w.click("inv-email", cx);
        w.input("hugo@example-studio.com", cx);
        w.click("inv-name", cx);
        w.input("Hugo Bernard", cx);
        w.click(format!("inv-sp-{general}"), cx);
        w.click("inv-send", cx);
    });
    let hugo_code = session(cx, &lea, |s| {
        s.org.as_ref().unwrap().ui.invite_code.clone().unwrap()
    });
    act(cx, lh, |w, cx| w.click("invite-done", cx));
    let (hh, hugo) = open(cx, &dir.path().join("hugo.cleis"));
    cx.run_until_parked();
    first_launch(cx, hh, &addr, &hugo_code, None);
    sync(cx, &hugo);
    sync(cx, &lea);
    sync(cx, &hugo);
    let sees_figma = |cx: &mut TestAppContext| {
        session(cx, &hugo, |s| s.entries.iter().any(|e| e.name == "Figma"))
    };
    assert!(sees_figma(cx));

    // Lea: Security policy › Accessing (mockups 42), on, with a PIN of 4 digits, set.
    act(cx, lh, |w, cx| w.click("set-Security policy", cx));
    act(cx, lh, |w, cx| w.click("pol-access", cx));
    sync(cx, &lea);
    act(cx, lh, |w, cx| w.click("pol-access-with", cx));
    act(cx, lh, |w, cx| w.click("pol-access-with-1", cx));
    sync(cx, &lea);
    act(cx, lh, |w, cx| w.click("pin-len-0", cx));
    sync(cx, &lea);
    // Set: typed twice in the boxes of 37f (the first ones have the keyboard).
    act(cx, lh, |w, cx| w.click("pin-set", cx));
    assert!(session(cx, &lea, |s| s
        .pin_setup
        .as_ref()
        .is_some_and(|d| d.org.is_some() && d.first.len == 4)));
    act(cx, lh, |w, cx| {
        w.input("4821", cx);
        w.input("4821", cx);
    });
    cx.run_until_parked();
    assert!(session(cx, &lea, |s| s.pin_setup.is_none()));
    sync(cx, &lea);
    session(cx, &lea, |s| {
        let st = s.org.as_ref().unwrap().state.as_ref().unwrap();
        assert!(st.policy.access.on && st.policy.access.with == AccessWith::Pin && st.pin_set);
        assert!(
            s.org.as_ref().unwrap().pin.is_none(),
            "not asked of the admin who set it"
        );
    });

    // Hugo: the next session asks it; the organization's items are closed.
    hugo.update(cx, |a, _| {
        if let Some(o) = a.session().and_then(|s| s.org.as_mut()) {
            o.token = None;
        }
    });
    sync(cx, &hugo);
    assert!(session(cx, &hugo, |s| s
        .org
        .as_ref()
        .unwrap()
        .pin
        .is_some()));
    assert!(!sees_figma(cx));
    let okey = session(cx, &hugo, |s| s.org.as_ref().unwrap().m.key());
    act(cx, hh, |w, cx| w.click(format!("org-{okey}"), cx));
    // The boxes take the server's length and go on once full (mockup 36h).
    assert_eq!(
        session(cx, &hugo, |s| s
            .org
            .as_ref()
            .unwrap()
            .pin
            .as_ref()
            .unwrap()
            .len),
        4
    );
    act(cx, hh, |w, cx| {
        w.click("org-pin", cx);
        w.input("0000", cx);
    });
    cx.run_until_parked();
    let err = session(cx, &hugo, |s| {
        s.org
            .as_ref()
            .unwrap()
            .pin
            .as_ref()
            .and_then(|p| p.error.clone())
    });
    assert!(err.is_some_and(|e| e.starts_with("Wrong PIN")));
    assert!(!sees_figma(cx));
    act(cx, hh, |w, cx| {
        w.click("org-pin", cx);
        w.input("4821", cx);
    });
    cx.run_until_parked();
    assert!(session(cx, &hugo, |s| s
        .org
        .as_ref()
        .unwrap()
        .pin
        .is_none()));
    assert!(sees_figma(cx));

    // D-98: a password in place of the PIN; asked again after 5 minutes without using Cleis.
    act(cx, lh, |w, cx| w.click("pol-access-with", cx));
    act(cx, lh, |w, cx| w.click("pol-access-with-0", cx));
    sync(cx, &lea);
    act(cx, lh, |w, cx| w.click("pol-pin-idle", cx));
    act(cx, lh, |w, cx| w.click("pol-pin-idle-1", cx));
    sync(cx, &lea);
    act(cx, lh, |w, cx| w.click("pin-set", cx));
    act(cx, lh, |w, cx| {
        w.click("access-pw", cx);
        w.input("short", cx);
        w.click("access-pw-confirm", cx);
        w.input("short", cx);
        w.click("access-pw-save", cx);
    });
    assert!(session(cx, &lea, |s| s
        .org
        .as_ref()
        .unwrap()
        .ui
        .access_pw
        .as_ref()
        .is_some_and(|f| f.error.is_some())));
    let org_pw = "studio access 2026";
    act(cx, lh, |w, cx| {
        w.click("access-pw", cx);
        w.press("ctrl-a", cx);
        w.input(org_pw, cx);
        w.click("access-pw-confirm", cx);
        w.press("ctrl-a", cx);
        w.input(org_pw, cx);
        w.click("access-pw-save", cx);
    });
    for _ in 0..100 {
        cx.run_until_parked();
        if session(cx, &lea, |s| s.org.as_ref().unwrap().ui.access_pw.is_none()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(session(cx, &lea, |s| s
        .org
        .as_ref()
        .unwrap()
        .ui
        .access_pw
        .is_none()));
    // Hugo's next session asks the password (a field, not boxes).
    hugo.update(cx, |a, _| {
        if let Some(o) = a.session().and_then(|s| s.org.as_mut()) {
            o.token = None;
        }
    });
    sync(cx, &hugo);
    assert!(session(cx, &hugo, |s| s
        .org
        .as_ref()
        .unwrap()
        .pin
        .as_ref()
        .is_some_and(|p| p.with == AccessWith::Password)));
    assert!(!sees_figma(cx));
    act(cx, hh, |w, cx| {
        w.click("org-password", cx);
        w.input(org_pw, cx);
        w.press("enter", cx);
    });
    for _ in 0..100 {
        cx.run_until_parked();
        if sees_figma(cx) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(sees_figma(cx));
    // Five minutes without using Cleis: asked again, here, without a server's word.
    hugo.update(cx, |a, cx| {
        if let Some(s) = a.session() {
            s.last_activity = std::time::Instant::now() - std::time::Duration::from_secs(301);
        }
        a.org_idle_check(cx);
    });
    assert!(!sees_figma(cx));
    // A wrong one is counted by the server; the right one opens again.
    act(cx, hh, |w, cx| {
        w.click("org-password", cx);
        w.input("not it at all", cx);
        w.press("enter", cx);
    });
    for _ in 0..100 {
        cx.run_until_parked();
        if session(cx, &hugo, |s| {
            s.org
                .as_ref()
                .unwrap()
                .pin
                .as_ref()
                .is_some_and(|p| p.error.is_some())
        }) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let err = session(cx, &hugo, |s| {
        s.org
            .as_ref()
            .unwrap()
            .pin
            .as_ref()
            .and_then(|p| p.error.clone())
    });
    assert!(
        err.as_ref()
            .is_some_and(|e| e.starts_with("Wrong password")),
        "{err:?}"
    );
    act(cx, hh, |w, cx| {
        w.click("org-password", cx);
        w.press("ctrl-a", cx);
        w.input(org_pw, cx);
        w.press("enter", cx);
    });
    for _ in 0..100 {
        cx.run_until_parked();
        if sees_figma(cx) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(sees_figma(cx));
}

/// Scenario (mockup 36i, D-78): no server. Lea creates the organization in a shared folder,
/// makes a space with an item, invites Hugo; Hugo joins from the same folder with the code (put in
/// the address box) and receives the item once Lea's app gives him the space's key.
#[gpui_kit::test]
fn organization_in_shared_folder(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let shared = dir.path().join("SharePoint/Example Studio");
    let (lh, lea) = open(cx, &dir.path().join("lea.cleis"));
    cx.run_until_parked();
    act(cx, lh, |w, cx| {
        w.click("local-only", cx);
        w.click("new-password", cx);
        w.input("correct horse battery staple", cx);
        w.click("confirm-password", cx);
        w.input("correct horse battery staple", cx);
        w.click("create", cx);
    });
    act(cx, lh, |w, cx| w.click("recovery-key-done", cx));
    act(cx, lh, |w, cx| w.click("tab-Settings", cx));
    act(cx, lh, |w, cx| w.click("set-Organization", cx));
    act(cx, lh, |w, cx| w.click("create-folder", cx));
    // The folder dialog of the system is not driven here: the folder is set as it would be.
    let s2 = shared.clone();
    lea.update(cx, |a, cx| {
        if let Phase::Main(s) = &mut a.phase {
            s.join.folder = Some((0, s2));
        }
        cx.notify();
    });
    act(cx, lh, |w, cx| {
        w.click("create-name", cx);
        w.input("Example Studio", cx);
        w.click("create-me", cx);
        w.input("Lea Martin", cx);
        w.click("create-org", cx);
    });
    for _ in 0..50 {
        cx.run_until_parked();
        if session(cx, &lea, |s| s.org.is_some() || s.join.error.is_some()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    session(cx, &lea, |s| {
        assert!(s.join.error.is_none(), "{:?}", s.join.error);
        assert!(s.org.as_ref().unwrap().m.in_folder());
    });
    assert!(shared.join("cleis-org.json").is_file());
    sync(cx, &lea);
    session(cx, &lea, |s| {
        let o = s.org.as_ref().unwrap();
        assert!(o.error.is_none(), "{:?}", o.error);
        assert_eq!(
            o.state.as_ref().unwrap().me.role,
            cleis_core::org::SUPER_ADMIN
        );
    });
    // Mockup 49l (28/11/2026): alone, Lea cannot leave: no one could let her back in. The
    // question offers Members or the organization's deletion instead; Cancel keeps everything.
    act(cx, lh, |w, cx| w.click("set-Server", cx));
    act(cx, lh, |w, cx| w.click("org-leave", cx));
    session(cx, &lea, |s| {
        let c = s.confirm.as_ref().expect("asked");
        assert_eq!(c.action, cleis_app::app::ConfirmAction::OpenDeleteOrg);
        assert!(
            c.alt
                .as_ref()
                .is_some_and(|a| a.2 == cleis_app::app::ConfirmAction::GoMembers)
        );
    });
    assert!(window_has(cx, lh, "confirm-alt"));
    act(cx, lh, |w, cx| w.click("confirm-alt", cx));
    session(cx, &lea, |s| {
        assert!(s.confirm.is_none() && s.org.is_some());
        assert_eq!(s.settings_page, "Members");
    });

    act(cx, lh, |w, cx| w.click("tab-Vault", cx));
    act(cx, lh, |w, cx| w.click("new-space", cx));
    act(cx, lh, |w, cx| {
        w.click("space-name", cx);
        w.input("General", cx);
        w.click("space-create", cx);
    });
    sync(cx, &lea);
    let general = session(cx, &lea, |s| {
        s.org.as_ref().unwrap().state.as_ref().unwrap().spaces[0].id
    });
    lea.update(cx, |a, _| {
        if let Phase::Main(s) = &mut a.phase {
            let mut e = cleis_core::Entry::new(general, "Figma");
            e.password = "figma-pass-2026".into();
            s.vault.put_entry(&e).unwrap();
        }
    });
    sync(cx, &lea);
    act(cx, lh, |w, cx| w.click("tab-Settings", cx));
    act(cx, lh, |w, cx| w.click("set-Members", cx));
    act(cx, lh, |w, cx| w.click("invite", cx));
    act(cx, lh, |w, cx| {
        w.click("inv-email", cx);
        w.input("hugo@example-studio.com", cx);
        w.click("inv-name", cx);
        w.input("Hugo Bernard", cx);
        w.click(format!("inv-sp-{general}"), cx);
        w.click("inv-send", cx);
    });
    let code = session(cx, &lea, |s| {
        s.org.as_ref().unwrap().ui.invite_code.clone().unwrap()
    });
    assert!(
        cleis_core::org::Invitation::parse(&code).is_some(),
        "{code}"
    );
    act(cx, lh, |w, cx| w.click("invite-done", cx));

    // Hugo: a vault of his own, then Settings › Organization › Join with the folder.
    let (hh, hugo) = open(cx, &dir.path().join("hugo.cleis"));
    cx.run_until_parked();
    act(cx, hh, |w, cx| {
        w.click("local-only", cx);
        w.click("new-password", cx);
        w.input("another horse battery staple", cx);
        w.click("confirm-password", cx);
        w.input("another horse battery staple", cx);
        w.click("create", cx);
    });
    act(cx, hh, |w, cx| w.click("recovery-key-done", cx));
    act(cx, hh, |w, cx| w.click("tab-Settings", cx));
    act(cx, hh, |w, cx| w.click("set-Organization", cx));
    let folder_text = shared.display().to_string();
    act(cx, hh, |w, cx| {
        w.click("org-address", cx);
        w.input(&folder_text, cx);
        w.click("org-code", cx);
        w.input(&code, cx);
    });
    // "Join" is below the window's fold here.
    hugo.update(cx, |a, cx| a.join_from_settings(cx));
    for _ in 0..50 {
        cx.run_until_parked();
        if session(cx, &hugo, |s| s.org.is_some() || s.join.error.is_some()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    session(cx, &hugo, |s| {
        assert!(s.join.error.is_none(), "{:?}", s.join.error)
    });
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        let st = s.org.as_ref().unwrap().state.as_ref().unwrap();
        assert_eq!(st.me.name, "Hugo Bernard");
    });
    // Lea's app wraps the space key for him; Hugo receives the item.
    sync(cx, &lea);
    sync(cx, &hugo);
    session(cx, &hugo, |s| {
        let f = s
            .entries
            .iter()
            .find(|e| e.name == "Figma")
            .expect("shared item");
        assert_eq!(f.password, "figma-pass-2026");
    });

    // D-98: a PIN in a shared folder. Set by Lea (only its digest goes to the folder); Hugo's app
    // checks it and counts wrong ones itself.
    act(cx, lh, |w, cx| w.click("set-Security policy", cx));
    act(cx, lh, |w, cx| w.click("pol-access", cx));
    sync(cx, &lea);
    act(cx, lh, |w, cx| w.click("pol-access-with", cx));
    act(cx, lh, |w, cx| w.click("pol-access-with-1", cx));
    sync(cx, &lea);
    act(cx, lh, |w, cx| w.click("pin-len-0", cx));
    sync(cx, &lea);
    act(cx, lh, |w, cx| w.click("pin-set", cx));
    act(cx, lh, |w, cx| {
        w.input("2468", cx);
        w.input("2468", cx);
    });
    for _ in 0..100 {
        cx.run_until_parked();
        if session(cx, &lea, |s| s.pin_setup.is_none()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(session(cx, &lea, |s| s.pin_setup.is_none()));
    // Nothing in clear in the folder.
    let mut found = false;
    for f in walk(&shared) {
        let text = std::fs::read(&f).unwrap_or_default();
        assert!(!text.windows(4).any(|w| w == b"2468"), "PIN in {f:?}");
        found = true;
    }
    assert!(found);
    sync(cx, &hugo);
    let sees = |cx: &mut TestAppContext| {
        session(cx, &hugo, |s| s.entries.iter().any(|e| e.name == "Figma"))
    };
    assert!(!sees(cx), "asked first");
    let okey = session(cx, &hugo, |s| s.org.as_ref().unwrap().m.key());
    act(cx, hh, |w, cx| w.click("tab-Vault", cx));
    act(cx, hh, |w, cx| w.click(format!("org-{okey}"), cx));
    act(cx, hh, |w, cx| {
        w.click("org-pin", cx);
        w.input("1111", cx);
    });
    for _ in 0..100 {
        cx.run_until_parked();
        if session(cx, &hugo, |s| s.org.as_ref().unwrap().local.fails == 1) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(
        session(cx, &hugo, |s| s.org.as_ref().unwrap().local.fails),
        1
    );
    assert!(!sees(cx));
    act(cx, hh, |w, cx| {
        w.click("org-pin", cx);
        w.input("2468", cx);
    });
    for _ in 0..100 {
        cx.run_until_parked();
        if sees(cx) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(sees(cx));

    // D-98: Lea, Super Admin, deletes the organization; the folder keeps only the mark and
    // Hugo's app forgets it at its next sync.
    act(cx, lh, |w, cx| w.click("set-Server", cx));
    act(cx, lh, |w, cx| {
        w.click("del-org", cx);
    });
    act(cx, lh, |w, cx| {
        w.click("del-org-export", cx);
        w.click("del-org-name", cx);
        w.input("Example Studio", cx);
        w.click("del-org-go", cx);
    });
    for _ in 0..100 {
        cx.run_until_parked();
        if session(cx, &lea, |s| s.org.is_none()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(session(cx, &lea, |s| s.org.is_none()));
    assert!(shared.join("deleted.json").is_file());
    assert!(walk(&shared.join("ops")).is_empty());
    sync(cx, &hugo);
    assert!(session(cx, &hugo, |s| s.org.is_none()));
    assert!(!sees(cx));
}

/// Every file under `dir`.
fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}
