//! Audit U-9: the interface switches to French from Preferences, without a restart.
//! Its own test binary: the language is a process-wide setting, other scenarios read English.

use cleis_app::app::{CleisApp, Phase};
use cleis_app::theme;
use cleis_core::edit::Field;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AppContext, Bounds, Point, TestAppContext, WindowBounds, WindowOptions, base::Root, px, size,
};

#[gpui_kit::test]
fn switch_to_french(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.cleis");
    cleis_app::demo::create(&path).unwrap();
    let (h, app) = cx.update(|cx| {
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
        let path = path.clone();
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
    });
    let act = |cx: &mut TestAppContext, f: &dyn Fn(&mut gpui_kit::Window, &mut gpui_kit::App)| {
        cx.update_window(h.into(), |_, w, cx| f(w, cx)).unwrap();
        cx.run_until_parked();
    };
    cx.run_until_parked();
    act(cx, &|w, cx| {
        w.click("master-password", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.press("enter", cx);
    });
    assert!(!cleis_app::i18n::is_french());
    // The first item (a subscription with a monthly price) opens in the panel.
    let first = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        s.entries[s.visible[0]].id
    });
    act(cx, &|w, cx| {
        w.click_at(
            format!("row-{first}"),
            gpui_kit::point(px(30.), px(10.)),
            cx,
        )
    });
    let price = |cx: &mut TestAppContext| {
        app.read_with(cx, |a, cx| {
            let Phase::Main(s) = &a.phase else { panic!() };
            let ed = s.editor.as_ref().expect("an item is shown");
            ed.inputs[&Field::Price].read(cx).value().to_string()
        })
    };
    assert!(price(cx).ends_with(" / month"), "{}", price(cx));

    act(cx, &|w, cx| w.click("tab-Settings", cx));
    // Language and region is under the shortcuts' grid (mockup 45a).
    act(cx, &|w, cx| {
        w.scroll(
            "sc-all-0",
            gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(-600.))),
            cx,
        )
    });
    act(cx, &|w, cx| w.click("lang", cx));
    act(cx, &|w, cx| w.click("lang-fr", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.prefs.language, "fr");
    });
    assert!(cleis_app::i18n::is_french());
    assert_eq!(cleis_app::i18n::tr("Settings"), "Réglages");
    // The inspector's boxes are made again in French; the price still reads back.
    act(cx, &|w, cx| w.click("tab-Vault", cx));
    assert!(price(cx).ends_with(" / mois"), "{}", price(cx));
    let (_, money) = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let e = s.selected_entry().unwrap();
        (e.id, e.price.clone())
    });
    assert_eq!(
        money.unwrap().period,
        Some(cleis_core::money::Period::Month)
    );

    // Kept in the vault: the next opening speaks French.
    let saved = cleis_store::LockedVault::open(&path)
        .and_then(|l| l.unlock(cleis_app::demo::PASSWORD))
        .and_then(|v| v.preferences())
        .unwrap();
    assert_eq!(saved.language, "fr");

    // Back to English (the appearance file of this machine says English again).
    act(cx, &|w, cx| w.click("tab-Settings", cx));
    // Language and region is under the shortcuts' grid (mockup 45a).
    act(cx, &|w, cx| {
        w.scroll(
            "sc-all-0",
            gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(-600.))),
            cx,
        )
    });
    act(cx, &|w, cx| w.click("lang", cx));
    act(cx, &|w, cx| w.click("lang-en", cx));
    assert!(!cleis_app::i18n::is_french());
    assert_eq!(cleis_app::i18n::tr("Settings"), "Settings");
}
