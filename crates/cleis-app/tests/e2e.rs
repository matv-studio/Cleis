//! End-to-end scenarios run against the real window tree (headless GPUI test platform).

use cleis_app::app::{CleisApp, Phase, Tab};
use cleis_app::theme::{self, Mode};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AppContext, Bounds, Entity, Point, TestAppContext, WindowBounds, WindowHandle, WindowOptions,
    base::Root, px, size,
};
use std::path::Path;

fn open(cx: &mut TestAppContext, path: &Path) -> (WindowHandle<Root>, Entity<CleisApp>) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::apply(Mode::Dark, cx);
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::default(),
                size: size(px(1440.), px(900.)),
            })),
            ..Default::default()
        };
        let path = path.to_owned();
        let (w, app) = gpui_kit::open_window(opts, cx, move |window, cx| {
            cx.new(|cx| CleisApp::new(path, Mode::Dark, window, cx))
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

    act(cx, h, |w, cx| w.click("lock", cx));
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
