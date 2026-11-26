//! Organization scenario against a real `cleis-server` on 127.0.0.1, two windows: Lea (first
//! admin) and Hugo (invited member).
//!
//! Lea creates her vault with the server's first invitation code, makes a space, invites Hugo;
//! Hugo joins with the code and receives the space's items once Lea's app wraps the key for him.
//! Lea requires a verification code: Hugo sets one up. Lea checks the activity log, then
//! removes Hugo, whose app forgets the space.

use cleis_app::app::{CleisApp, Phase, Tab};
use cleis_app::theme;
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
    act(cx, h, |w, cx| w.click("set-Profile", cx));
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
    let hugo_id = session(cx, &hugo, |s| s.org.as_ref().unwrap().m.member);
    lea.update(cx, |a, cx| {
        a.org_member(
            cleis_core::org::MemberChange {
                member: hugo_id,
                role: Some("read_only".into()),
                spaces: None,
                export: None,
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
}
