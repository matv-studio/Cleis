//! End-to-end scenarios run against the real window tree (headless GPUI test platform).

use cleis_app::app::{CleisApp, Phase, Tab};
use cleis_app::theme;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AppContext, Bounds, Entity, Point, TestAppContext, WindowBounds, WindowHandle, WindowOptions,
    base::Root, px, size,
};
use std::path::Path;

fn open(cx: &mut TestAppContext, path: &Path) -> (WindowHandle<Root>, Entity<CleisApp>) {
    cx.update(|cx| {
        gpui_kit::init(cx);
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
                // Never take the machine's real shortcuts or type into its windows.
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

/// Scenario: open an existing vault, mistype the password, unlock, search, copy, lock.
#[gpui_kit::test]
fn unlock_search_copy_lock(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.cleis");
    cleis_app::demo::create(&path).unwrap();
    let (h, app) = open(cx, &path);
    cx.run_until_parked();
    app.read_with(cx, |a, _| assert!(matches!(a.phase, Phase::Unlock(_))));

    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input("not the password", cx);
        w.click("unlock", cx);
    });
    app.read_with(cx, |a, _| match &a.phase {
        Phase::Unlock(u) => assert_eq!(u.error.as_deref(), Some("Wrong master password")),
        _ => panic!("should still be locked"),
    });

    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.press("enter", cx);
    });
    let figma = app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => {
            assert_eq!(s.visible.len(), 14);
            assert_eq!(s.tab, Tab::Vault);
            s.entries.iter().find(|e| e.name == "Figma").unwrap().id
        }
        _ => panic!("should be unlocked"),
    });

    act(cx, h, |w, cx| {
        w.click("search", cx);
        w.input("fig", cx);
    });
    app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => {
            assert_eq!(s.visible.len(), 1);
            assert_eq!(s.selected, Some(figma));
        }
        _ => unreachable!(),
    });

    act(cx, h, |w, cx| w.click(format!("cu-{figma}"), cx));
    let clip = cx.update(|cx| cx.read_from_clipboard().and_then(|c| c.text()));
    assert_eq!(clip.as_deref(), Some("design@example-studio.com"));

    // Ctrl+L locks from anywhere in the main window (tooltip of the lock button).
    act(cx, h, |w, cx| w.press("ctrl-l", cx));
    app.read_with(cx, |a, _| assert!(matches!(a.phase, Phase::Unlock(_))));
}

/// Scenario: first launch creates a vault, then the vault survives a restart.
#[gpui_kit::test]
fn first_launch_creates_vault(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub").join("vault.cleis");
    let (h, app) = open(cx, &path);
    cx.run_until_parked();
    app.read_with(cx, |a, _| assert!(matches!(a.phase, Phase::Welcome(_))));

    act(cx, h, |w, cx| {
        w.click("new-password", cx);
        w.input("short", cx);
        w.click("confirm-password", cx);
        w.input("short", cx);
        w.click("create", cx);
    });
    app.read_with(cx, |a, _| match &a.phase {
        Phase::Welcome(wl) => assert_eq!(wl.error.as_deref(), Some("At least 10 characters")),
        _ => panic!(),
    });

    let pw = "a long enough master password";
    act(cx, h, |w, cx| {
        // Select what was typed before and replace it, as a person would.
        w.click("new-password", cx);
        w.press("ctrl-a", cx);
        w.input(pw, cx);
        w.click("confirm-password", cx);
        w.press("ctrl-a", cx);
        w.input(pw, cx);
        w.click("create", cx);
    });
    app.read_with(cx, |a, _| {
        assert!(
            matches!(a.phase, Phase::Main(_)),
            "vault should be created and open"
        )
    });
    assert!(path.exists());

    // New item, favourite, then preferences persist across lock/unlock.
    act(cx, h, |w, cx| w.click("new-item", cx));
    act(cx, h, |w, cx| w.click("fav", cx));
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("tab-contacts", cx));
    act(cx, h, |w, cx| w.click("lock", cx));
    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input(pw, cx);
        w.press("enter", cx);
    });
    app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => {
            assert_eq!(s.entries.len(), 1);
            assert!(s.entries[0].favorite);
            assert!(
                !s.prefs.tabs.contacts,
                "the Contacts tab switch should be saved"
            );
        }
        Phase::Unlock(u) => panic!("should be unlocked: error={:?} busy={}", u.error, u.busy),
        _ => panic!("welcome?"),
    });
}

/// Scenarios of screens 16 and 17: settings of Autofill, then Quick Access over a desktop app
/// that is not linked yet; the first fill links it, the link shows in Settings and can be removed.
#[gpui_kit::test]
fn autofill_settings_and_quick_access(cx: &mut TestAppContext) {
    use cleis_autofill::{Foreground, WindowRef};
    use cleis_core::autofill::{Context, Reason};
    use cleis_core::settings::{FillMethod, TypingSpeed};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.cleis");
    cleis_app::demo::create(&path).unwrap();
    let (h, app) = open(cx, &path);
    cx.run_until_parked();
    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.press("enter", cx);
    });
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Autofill", cx));
    act(cx, h, |w, cx| w.click("method-Type", cx));
    act(cx, h, |w, cx| w.click("speed-Slow", cx));
    act(cx, h, |w, cx| w.click("qa-change", cx));
    act(cx, h, |w, cx| w.press("ctrl-shift-k", cx));
    app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => {
            let af = &s.prefs.autofill;
            assert_eq!(af.method, FillMethod::Type);
            assert_eq!(af.typing_speed, TypingSpeed::Slow);
            assert_eq!(af.quick_access.keys, "Ctrl+Shift+K");
            assert_eq!(s.recording, None);
        }
        _ => panic!("locked"),
    });

    let fg = Foreground {
        window: WindowRef(42),
        monitor: None,
        context: Context {
            program: "figma.exe".into(),
            app_name: "Figma".into(),
            title: "Figma".into(),
            url: None,
        },
    };
    let figma = app.read_with(cx, |a, _| {
        let items = cleis_app::screens::quick_access::items(a, &fg, "");
        assert_eq!(items[0].name, "Figma");
        assert_eq!(items[0].matched, Some(Reason::Name));
        assert!(items.len() >= 14, "every item stays reachable by search");
        items[0].id
    });
    let fg2 = fg.clone();
    app.update(cx, |a, cx| a.open_quick_access(fg2, cx));
    cx.run_until_parked();
    let qa = app
        .read_with(cx, |a, _| a.autofill.quick_access)
        .expect("Quick Access window");
    let qa = qa.downcast::<Root>().expect("root");
    act(cx, qa, |w, cx| w.press("enter", cx));
    app.read_with(cx, |a, _| {
        assert_eq!(a.autofill.dry_run.as_deref(), Some(&[(figma, None)][..]));
        assert!(a.autofill.quick_access.is_none(), "closes after filling");
        let Phase::Main(s) = &a.phase else { panic!() };
        let e = s.entries.iter().find(|e| e.id == figma).unwrap();
        assert_eq!(e.linked_apps[0].program, "figma.exe");
        let items = cleis_app::screens::quick_access::items(a, &fg, "");
        assert_eq!(items[0].matched, Some(Reason::Linked));
    });

    // Search narrows the list; Shift+Enter types.
    let fg3 = fg.clone();
    app.update(cx, |a, cx| a.open_quick_access(fg3, cx));
    cx.run_until_parked();
    let qa = app.read_with(cx, |a, _| a.autofill.quick_access).unwrap();
    let qa = qa.downcast::<Root>().unwrap();
    act(cx, qa, |w, cx| w.input("slack", cx));
    act(cx, qa, |w, cx| w.press("shift-enter", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let slack = s.entries.iter().find(|e| e.name == "Slack").unwrap().id;
        let log = a.autofill.dry_run.as_ref().unwrap();
        assert_eq!(log.last(), Some(&(slack, Some(FillMethod::Type))));
    });

    // The link appears in Settings › Autofill and can be removed.
    act(cx, h, |w, cx| w.click("unlink-0", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.entries.iter().all(|e| e.linked_apps.is_empty()));
    });
}
