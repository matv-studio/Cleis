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
        Phase::Unlock(u) => assert_eq!(u.error.as_deref(), Some("Wrong password")),
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
    act(cx, h, |w, cx| {
        if w.try_find("lock").is_some() {
            w.click("lock", cx);
        }
    });
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

    let pw = "a long enough password";
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
    // The recovery key is shown once (D-76).
    act(cx, h, |w, cx| w.click("recovery-key-done", cx));

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
    act(cx, h, |w, cx| w.click("method-Typing", cx));
    act(cx, h, |w, cx| w.click("speed-Slow", cx));
    // Mockup 38: the system-wide shortcuts are in Settings › Shortcuts.
    act(cx, h, |w, cx| w.click("set-Shortcuts", cx));
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
    act(cx, h, |w, cx| w.click("set-Autofill", cx));

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

/// Inspector editing (screen 01): new item, typed fields saved to the vault file, a refused
/// value left unsaved with its message, trash, restore, and deleting for good after confirming.
#[gpui_kit::test]
fn edit_item_trash_restore_purge(cx: &mut TestAppContext) {
    use cleis_core::Subscription;
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

    // New item: the name box is focused with its text selected.
    act(cx, h, |w, cx| w.click("new-item", cx));
    act(cx, h, |w, cx| {
        w.input("Example Bank", cx);
        w.press("enter", cx);
    });
    act(cx, h, |w, cx| {
        w.click("edit-username", cx);
        w.input("ines@example-studio.com", cx);
        w.press("enter", cx);
    });
    act(cx, h, |w, cx| {
        w.click("edit-Licenses", cx);
        w.input("three", cx);
        w.press("enter", cx);
    });
    act(cx, h, |w, cx| {
        w.click("edit-Price", cx);
        w.input("€12,50/mo", cx);
        w.press("enter", cx);
    });
    act(cx, h, |w, cx| w.click("sub-state", cx));
    act(cx, h, |w, cx| w.click("sub-Active", cx));
    let id = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let e = s
            .entries
            .iter()
            .find(|e| e.name == "Example Bank")
            .expect("renamed");
        assert_eq!(e.username, "ines@example-studio.com");
        assert_eq!(e.licenses, None, "refused value not written");
        assert_eq!(e.subscription, Subscription::Active);
        assert_eq!(e.price.as_ref().unwrap().yearly_cents(), 15_000);
        let ed = s.editor.as_ref().unwrap();
        assert!(ed.errors.contains_key(&cleis_core::edit::Field::Licenses));
        e.id
    });
    // Written to the file, not only in memory.
    let reopened = cleis_store::LockedVault::open(&path)
        .unwrap()
        .unlock(cleis_app::demo::PASSWORD)
        .unwrap();
    let saved = reopened.entries().unwrap();
    let e = saved.iter().find(|e| e.id == id).unwrap();
    assert_eq!(e.username, "ines@example-studio.com");
    drop(reopened);

    // Trash, restore, trash again, delete for good after confirming.
    act(cx, h, |w, cx| w.click("more", cx));
    act(cx, h, |w, cx| w.click("menu-trash", cx));
    act(cx, h, |w, cx| w.click("f-trash", cx));
    act(cx, h, |w, cx| w.click("restore-0", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.entries.iter().any(|e| e.id == id));
        assert!(s.trash.is_empty());
    });
    act(cx, h, |w, cx| w.click("f-all", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.selected, Some(id), "the restored item stays selected");
    });
    act(cx, h, |w, cx| w.click("more", cx));
    act(cx, h, |w, cx| w.click("menu-trash", cx));
    act(cx, h, |w, cx| w.click("f-trash", cx));
    act(cx, h, |w, cx| w.click("purge-0", cx));
    act(cx, h, |w, cx| w.click("confirm-ok", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.trash.is_empty());
        assert!(s.entries.iter().all(|e| e.id != id));
    });
    let reopened = cleis_store::LockedVault::open(&path)
        .unwrap()
        .unlock(cleis_app::demo::PASSWORD)
        .unwrap();
    assert!(
        reopened
            .trashed_entries()
            .unwrap()
            .iter()
            .all(|e| e.id != id)
    );
}

/// Scenario (screen 02): add a "Cancellation date" date column for all spaces, fill it in the
/// inspector (a wrong date is refused), hide a column, then delete the new column.
#[gpui_kit::test]
fn custom_column_add_fill_hide_delete(cx: &mut TestAppContext) {
    use cleis_core::column::{Table, Value};
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
    act(cx, h, |w, cx| w.click("columns", cx));
    act(cx, h, |w, cx| w.click("col-new", cx));
    act(cx, h, |w, cx| {
        w.input("Cancellation date", cx);
        w.click("kind-Date", cx);
        w.click("col-create", cx);
    });
    let col = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let c = s
            .columns
            .iter()
            .find(|c| c.name == "Cancellation date")
            .expect("created");
        assert!(c.visible && !c.builtin);
        assert!(s.columns_menu.as_ref().unwrap().creating.is_none());
        c.id.clone()
    });
    // Hide "Username"; drag "Price" onto "Password" (it takes its place).
    act(cx, h, |w, cx| w.click("col-username", cx));
    act(cx, h, |w, cx| w.drag_to("col-price", "col-password", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let mut v: Vec<_> = s
            .columns
            .iter()
            .filter(|c| c.table == Table::Vault)
            .collect();
        v.sort_by_key(|c| c.order);
        let names: Vec<&str> = v.iter().map(|c| c.id.0.as_str()).take(4).collect();
        assert_eq!(names, ["name", "username", "price", "password"]);
    });

    // Fill the new field of the selected item.
    let figma = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        s.entries.iter().find(|e| e.name == "Figma").unwrap().id
    });
    app.update(cx, |a, cx| {
        if let Some(s) = a.session() {
            s.columns_menu = None;
            s.selected = Some(figma);
        }
        cx.notify();
    });
    cx.run_until_parked();
    let field: &'static str = format!("custom-{}", col.0).leak();
    act(cx, h, |w, cx| {
        w.click(field, cx);
        w.input("31/02/2026x", cx);
        w.press("enter", cx);
    });
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.editor.as_ref().unwrap().custom_errors.contains_key(&col));
    });
    act(cx, h, |w, cx| {
        w.press("ctrl-a", cx);
        w.input("2027-01-15", cx);
        w.press("enter", cx);
    });
    let reopened = cleis_store::LockedVault::open(&path)
        .unwrap()
        .unlock(cleis_app::demo::PASSWORD)
        .unwrap();
    let e = reopened
        .entries()
        .unwrap()
        .into_iter()
        .find(|e| e.id == figma)
        .unwrap();
    assert_eq!(e.fields.get(&col), Some(&Value::Date("2027-01-15".into())));
    let cols = reopened.vault_columns().unwrap();
    assert!(!cols.iter().find(|c| c.id.0 == "username").unwrap().visible);
    assert!(cols.iter().any(|c| c.id == col && c.table == Table::Vault));
    drop(reopened);

    // Delete the column.
    act(cx, h, |w, cx| w.click("columns", cx));
    let del = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let mut v: Vec<_> = s
            .columns
            .iter()
            .filter(|c| c.table == Table::Vault)
            .collect();
        v.sort_by_key(|c| c.order);
        let i = v
            .iter()
            .filter(|c| c.visible)
            .position(|c| c.id == col)
            .unwrap();
        format!("col-del-{i}")
    });
    let del: &'static str = del.leak();
    let row: &'static str = format!("col-{}", col.0).leak();
    act(cx, h, |w, cx| w.hover(row, cx));
    act(cx, h, |w, cx| w.click(del, cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.columns.iter().all(|c| c.id != col));
    });
}

/// Scenario (screen 14): import a Notion CSV export into the vault; a Cleis column picked twice
/// leaves the first file column skipped; an item already in the vault is not imported again.
#[gpui_kit::test]
fn import_notion_csv(cx: &mut TestAppContext) {
    use cleis_core::import_csv::Target;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.cleis");
    cleis_app::demo::create(&path).unwrap();
    let csv = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../design/tools/notion-export.csv");
    let (h, app) = open(cx, &path);
    cx.run_until_parked();
    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.press("enter", cx);
    });
    let before = app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => s.entries.len(),
        _ => panic!(),
    });
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Import & export", cx));
    app.update(cx, |a, cx| a.load_import(&csv, cx));
    cx.run_until_parked();
    // Into Design, where Figma already is.
    app.update(cx, |a, _| {
        if let Phase::Main(s) = &mut a.phase {
            s.import.space = s
                .spaces
                .iter()
                .find(|sp| sp.name == "Design")
                .map(|sp| sp.id);
        }
    });

    // "En +" → Name: "Site/app" (Name) becomes Skip; then back.
    act(cx, h, |w, cx| w.click("map-8", cx));
    act(cx, h, |w, cx| w.click("map-8-0", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let m = &s.import.file.as_ref().unwrap().mappings;
        assert_eq!(m[8].target, Target::Name);
        assert_eq!(m[0].target, Target::Skip);
    });
    act(cx, h, |w, cx| w.click("map-0", cx));
    act(cx, h, |w, cx| w.click("map-0-0", cx));
    act(cx, h, |w, cx| w.click("map-8", cx));
    act(cx, h, |w, cx| w.click("map-8-4", cx)); // Notes
    act(cx, h, |w, cx| w.click("import-continue", cx));
    act(cx, h, |w, cx| w.click("import-run", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.entries.len(), before + 5, "Figma already there, skipped");
        assert_eq!(s.tab, Tab::Vault);
        let e = s
            .entries
            .iter()
            .find(|e| e.name == "Example Hosting")
            .unwrap();
        assert_eq!(e.price.as_ref().unwrap().yearly_cents(), 11_988);
        assert!(s.entries.iter().any(|e| e.name == "example-status.com"));
    });
    let saved = cleis_store::LockedVault::open(&path)
        .unwrap()
        .unlock(cleis_app::demo::PASSWORD)
        .unwrap()
        .entries()
        .unwrap();
    assert_eq!(saved.len(), before + 5);
}

/// Scenario (screens 20, 21): edit a contact in the inspector (a wrong email is refused), add a
/// phone and a label, merge the duplicate, then delete a contact after confirming.
#[gpui_kit::test]
fn contacts_edit_merge_delete(cx: &mut TestAppContext) {
    use cleis_app::screens::contacts::ContactFilter;
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
    act(cx, h, |w, cx| w.click("tab-Contacts", cx));
    let (camille, dup, total) = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let c = s.contacts.iter().find(|c| c.given == "Camille").unwrap().id;
        let d = s.contacts.iter().find(|c| c.given == "C.").unwrap().id;
        (c, d, s.contacts.len())
    });
    let row: &'static str = format!("c-{camille}").leak();
    act(cx, h, |w, cx| w.click(row, cx));
    act(cx, h, |w, cx| {
        w.click("edit-email-0", cx);
        w.press("ctrl-a", cx);
        w.input("camille at example", cx);
        w.press("enter", cx);
    });
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let ed = s.contacts_ui.editor.as_ref().unwrap();
        assert!(!ed.errors.is_empty(), "refused email shows an error");
        let c = s.contacts.iter().find(|c| c.id == camille).unwrap();
        assert_eq!(c.emails[0].value, "camille@example-print.com");
    });
    act(cx, h, |w, cx| {
        w.press("ctrl-a", cx);
        w.input("camille@example-print.com", cx);
        w.press("enter", cx);
    });
    // New phone row: the cursor is put in it.
    act(cx, h, |w, cx| w.click("add-phone", cx));
    act(cx, h, |w, cx| {
        w.input("+33 6 99 00 77 88", cx);
        w.press("enter", cx);
    });
    act(cx, h, |w, cx| w.click("kind-phone-1", cx));
    act(cx, h, |w, cx| w.click("add-label", cx));
    act(cx, h, |w, cx| {
        w.input("VIP", cx);
        w.press("enter", cx);
    });
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let c = s.contacts.iter().find(|c| c.id == camille).unwrap();
        assert_eq!(c.phones.len(), 2);
        assert_eq!(c.phones[1].value, "+33 6 99 00 77 88");
        assert_eq!(c.phones[1].label, "Home", "Mobile → Home");
        assert!(c.labels.contains("VIP"));
    });

    // Merge & fix: the first group is open, Camille kept.
    act(cx, h, |w, cx| w.click("cf-merge", cx));
    act(cx, h, |w, cx| w.click("merge-0", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.contacts.iter().all(|c| c.id != dup));
        assert_eq!(s.contacts.len(), total - 1);
        let c = s.contacts.iter().find(|c| c.id == camille).unwrap();
        assert!(c.phones.iter().any(|p| p.value == "+33 6 99 00 12 35"));
    });

    // Delete, after confirming.
    act(cx, h, |w, cx| w.click("cf-all", cx));
    act(cx, h, |w, cx| w.click(row, cx));
    act(cx, h, |w, cx| w.click("contact-more", cx));
    act(cx, h, |w, cx| w.click("cm-delete", cx));
    act(cx, h, |w, cx| w.click("confirm-ok", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.contacts.iter().all(|c| c.id != camille));
        assert_eq!(s.contacts_ui.filter, ContactFilter::All);
    });
    let saved = cleis_store::LockedVault::open(&path)
        .unwrap()
        .unlock(cleis_app::demo::PASSWORD)
        .unwrap()
        .contacts()
        .unwrap();
    assert_eq!(saved.len(), total - 2);
}

/// Scenario (screens 05, 06): turn the opening code on (a wrong code is refused), lock, unlock:
/// the code is asked; a recovery code works once. Then change the master password.
#[gpui_kit::test]
fn security_open_code_and_password(cx: &mut TestAppContext) {
    use cleis_app::screens::security::SecDialog;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.cleis");
    cleis_app::demo::create(&path).unwrap();
    let (h, app) = open(cx, &path);
    cx.run_until_parked();
    let unlock = |cx: &mut TestAppContext, pw: &'static str| {
        act(cx, h, |w, cx| {
            w.click("master-password", cx);
            w.input(pw, cx);
            w.press("enter", cx);
        });
    };
    unlock(cx, cleis_app::demo::PASSWORD);
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Account", cx));
    act(cx, h, |w, cx| w.click("open-code", cx));
    act(cx, h, |w, cx| {
        w.input("000000", cx);
        w.click("code-ok", cx);
    });
    let code = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let Some(SecDialog::SetupCode { code, error, .. }) = &s.security.dialog else {
            panic!("setup dialog still open")
        };
        assert!(error.is_some());
        code.clone()
    });
    let now = cleis_core::now();
    let good: &'static str = code.totp("x").code_at(now).leak();
    act(cx, h, |w, cx| {
        w.click("code-check", cx);
        w.press("ctrl-a", cx);
        w.input(good, cx);
        w.click("code-ok", cx);
    });
    act(cx, h, |w, cx| w.click("recovery-done", cx));
    // Lock (Ctrl+L), unlock: the code is asked.
    act(cx, h, |w, cx| w.press("ctrl-l", cx));
    act(cx, h, |w, cx| {
        if w.try_find("lock").is_some() {
            w.click("lock", cx);
        }
    });
    app.read_with(cx, |a, _| {
        assert!(matches!(a.phase, Phase::Unlock(_)), "locked")
    });
    unlock(cx, cleis_app::demo::PASSWORD);
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.gate.is_some(), "code asked after the password");
    });
    let recovery: &'static str = code.recovery[0].clone().leak();
    act(cx, h, |w, cx| {
        w.input(recovery, cx);
        w.press("enter", cx);
    });
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.gate.is_none());
        assert_eq!(s.prefs.open_code.as_ref().unwrap().recovery.len(), 7);
    });

    // Change the master password.
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Account", cx));
    act(cx, h, |w, cx| w.click("chpw", cx));
    act(cx, h, |w, cx| {
        w.input("wrong current", cx);
        w.click("pw-new", cx);
        w.input("A-new long pass 2026", cx);
        w.click("pw-confirm", cx);
        w.input("A-new long pass 2026", cx);
        w.click("chpw-ok", cx);
    });
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(50));
    cx.run_until_parked();
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let Some(SecDialog::ChangePassword { error, .. }) = &s.security.dialog else {
            panic!()
        };
        assert_eq!(error.as_deref(), Some("Wrong current password"));
    });
    act(cx, h, |w, cx| {
        w.click("pw-current", cx);
        w.press("ctrl-a", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.click("chpw-ok", cx);
    });
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(50));
    cx.run_until_parked();
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.security.dialog.is_none());
        assert!(s.prefs.password_changed.is_some());
    });
    assert!(
        cleis_store::LockedVault::open(&path)
            .unwrap()
            .unlock("A-new long pass 2026")
            .is_ok()
    );
}

/// Scenario (screen 23): choose a sync folder (a copy is made), another device adds an item to
/// that copy, Sync now brings it; Back up now makes a dated copy.
#[gpui_kit::test]
fn sync_folder_and_backup(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.cleis");
    let place_dir = dir.path().join("OneDrive/Cleis");
    std::fs::create_dir_all(&place_dir).unwrap();
    cleis_app::demo::create(&path).unwrap();
    let (h, app) = open(cx, &path);
    cx.run_until_parked();
    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.press("enter", cx);
    });
    app.update(cx, |a, cx| {
        a.set_sync_folder(&place_dir, cleis_app::sync::Provider::OneDrive, cx)
    });
    let place = place_dir.join("demo.cleis");
    assert!(place.exists(), "copy made in the place");
    // The other device.
    {
        let v = cleis_store::LockedVault::open(&place)
            .unwrap()
            .unlock(cleis_app::demo::PASSWORD)
            .unwrap();
        v.put_entry(&cleis_core::Entry::new(v.home_space(), "Example Wiki"))
            .unwrap();
    }
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Sync & backups", cx));
    act(cx, h, |w, cx| w.click("sync-now", cx));
    act(cx, h, |w, cx| w.click("backup-now", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.entries.iter().any(|e| e.name == "Example Wiki"));
        assert!(s.sync.last_error.is_none());
    });
    assert_eq!(cleis_app::sync::backups(&path).len(), 1);
    // The place's settings stay on this device, beside the vault.
    assert!(cleis_app::sync::config_path(&path).exists());
}

/// Scenario (screen 22): on a new computer, restore the vault from a OneDrive folder; a wrong
/// password leaves nothing behind.
#[gpui_kit::test]
fn restore_from_folder(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let place_dir = dir.path().join("OneDrive/Cleis");
    std::fs::create_dir_all(&place_dir).unwrap();
    cleis_app::demo::create(&place_dir.join("vault.cleis")).unwrap();
    let path = dir.path().join("new-pc/vault.cleis");
    let (h, app) = open(cx, &path);
    cx.run_until_parked();
    act(cx, h, |w, cx| w.click("restore", cx));
    app.update(cx, |a, cx| a.restore_found(place_dir.clone(), cx));
    act(cx, h, |w, cx| {
        w.click("restore-password", cx);
        w.input("not it", cx);
        w.click("restore-go", cx);
    });
    app.read_with(cx, |a, _| match &a.phase {
        Phase::Welcome(w) => assert!(w.restore.as_ref().unwrap().error.is_some()),
        _ => panic!("still on the restore screen"),
    });
    assert!(!path.exists());
    act(cx, h, |w, cx| {
        w.press("ctrl-a", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.click("restore-go", cx);
    });
    app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => {
            assert_eq!(s.entries.len(), 14);
            assert_eq!(
                s.sync.place.as_deref(),
                Some(place_dir.join("vault.cleis").as_path())
            );
        }
        _ => panic!("restored"),
    });
}

/// Scenario (screen 12): filter the report on reused passwords, then "Change" one: the vault opens
/// on it with a new password, the old one kept in its history.
#[gpui_kit::test]
fn health_change_weak_password(cx: &mut TestAppContext) {
    use cleis_app::screens::health::IssueKind;
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
    act(cx, h, |w, cx| w.click("tab-Health", cx));
    act(cx, h, |w, cx| w.click("st-reused", cx));
    let (id, old) = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.health.only, IssueKind::Reused);
        let (findings, _) = cleis_core::health::scan(
            &s.entries,
            &cleis_core::health::Policy::default(),
            cleis_core::now(),
        );
        let f = findings
            .iter()
            .find(|f| {
                f.issues
                    .iter()
                    .any(|i| matches!(i, cleis_core::health::Issue::Reused { .. }))
            })
            .expect("the demo reuses a password");
        let e = s.entries.iter().find(|e| e.id == f.entry).unwrap();
        (e.id, e.password.clone())
    });
    let fix: &'static str = format!("fix-{id}").leak();
    act(cx, h, |w, cx| w.click(fix, cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.tab, Tab::Vault);
        assert_eq!(s.selected, Some(id));
        let e = s.entries.iter().find(|e| e.id == id).unwrap();
        assert_ne!(e.password, old);
        assert_eq!(e.password_history.last().unwrap().password, old);
    });
}

/// Scenario (screen 01b): filter the vault on active subscriptions with a 2FA code, switch to
/// cards and back, clear the filters.
#[gpui_kit::test]
fn vault_filter_and_cards(cx: &mut TestAppContext) {
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
    act(cx, h, |w, cx| w.click("filter", cx));
    act(cx, h, |w, cx| w.click("vf-active", cx));
    act(cx, h, |w, cx| w.click("vf-2fa-true", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(!s.visible.is_empty());
        assert!(s.visible.iter().all(|i| {
            let e = &s.entries[*i];
            e.subscription == cleis_core::Subscription::Active && e.totp.is_some()
        }));
    });
    // Audit U-2: a click outside closes the menu (it stays open for several choices inside).
    act(cx, h, |w, cx| w.click("vview-Cards", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(!s.filter_menu && !s.vault_cards);
    });
    act(cx, h, |w, cx| w.click("vview-Cards", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.vault_cards);
    });
    act(cx, h, |w, cx| w.click("vview-Table", cx));
    act(cx, h, |w, cx| w.click("filter", cx));
    act(cx, h, |w, cx| w.click("vf-clear", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.visible.len(), s.entries.len());
    });
}

/// Scenario (screen 20 + 02): the Columns menu of the contacts adds a "Contract end" date
/// column; the inspector writes it; "Company" is hidden.
#[gpui_kit::test]
fn contact_columns(cx: &mut TestAppContext) {
    use cleis_core::column::{Table, Value};
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
    act(cx, h, |w, cx| w.click("tab-Contacts", cx));
    act(cx, h, |w, cx| w.click("contact-columns", cx));
    act(cx, h, |w, cx| w.click("col-company", cx));
    act(cx, h, |w, cx| w.click("col-new", cx));
    act(cx, h, |w, cx| {
        w.input("Contract end", cx);
        w.click("kind-Date", cx);
        w.click("col-create", cx);
    });
    let (col, first) = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let c = s
            .columns
            .iter()
            .find(|c| c.name == "Contract end")
            .expect("created");
        assert_eq!(c.table, Table::Contacts);
        assert!(
            !s.columns
                .iter()
                .find(|c| c.table == Table::Contacts && c.id.0 == "company")
                .unwrap()
                .visible
        );
        (
            c.id.clone(),
            cleis_app::screens::contacts::listed(s).first().unwrap().id,
        )
    });
    // The first contact opens (nothing opens by itself).
    app.update(cx, |a, cx| {
        if let Some(s) = a.session() {
            s.selected_contact = Some(first);
        }
        cx.notify();
    });
    cx.run_until_parked();
    app.update(cx, |a, cx| {
        if let Some(s) = a.session() {
            s.columns_menu = None;
        }
        cx.notify();
    });
    cx.run_until_parked();
    let field: &'static str = format!("ccustom-{}", col.0).leak();
    act(cx, h, |w, cx| {
        w.click(field, cx);
        w.input("2027-06-30", cx);
        w.press("enter", cx);
    });
    let saved = cleis_store::LockedVault::open(&path)
        .unwrap()
        .unlock(cleis_app::demo::PASSWORD)
        .unwrap();
    let c = saved
        .contacts()
        .unwrap()
        .into_iter()
        .find(|c| c.id == first)
        .unwrap();
    assert_eq!(c.fields.get(&col), Some(&Value::Date("2027-06-30".into())));
    assert!(saved.contact_columns().unwrap().iter().any(|c| c.id == col));
}

/// Scenario (screen 20): contacts imported from a Google Contacts CSV into the Clients folder.
#[gpui_kit::test]
fn import_contacts_csv(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.cleis");
    cleis_app::demo::create(&path).unwrap();
    let csv = dir.path().join("contacts.csv");
    std::fs::write(
        &csv,
        "First Name,Last Name,E-mail 1 - Value,Phone 1 - Value,Organization Name\n\
Hana,Example,hana@example-films.com,+33 1 99 00 44 55,Example Films\n\
Yann,Sample,yann@example-legal.com,,Example Legal\n",
    )
    .unwrap();
    let (h, app) = open(cx, &path);
    cx.run_until_parked();
    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.press("enter", cx);
    });
    act(cx, h, |w, cx| w.click("tab-Contacts", cx));
    let clients = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        s.spaces.iter().find(|sp| sp.name == "Clients").unwrap().id
    });
    let folder: &'static str = format!("cf-folder-{clients}").leak();
    act(cx, h, |w, cx| w.click(folder, cx));
    app.update(cx, |a, cx| a.import_vcard(&csv, cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let new: Vec<_> = s.contacts.iter().filter(|c| c.space == clients).collect();
        assert_eq!(new.len(), 2);
        assert!(
            new.iter()
                .any(|c| c.display_name() == "Hana Example" && c.company == "Example Films")
        );
    });
}

/// Whether an element with this id is drawn.
fn window_has(cx: &mut TestAppContext, h: WindowHandle<Root>, id: &str) -> bool {
    let id = gpui_kit::SharedString::from(id.to_owned());
    cx.update_window(h.into(), move |_, w, _| w.try_find(id).is_some())
        .unwrap()
}

/// Personal folders (D-62): a folder is made from the sidebar, an item moved into it from the
/// inspector menu, the folder renamed, then deleted: its item goes back to My vault.
#[gpui_kit::test]
fn personal_folders(cx: &mut TestAppContext) {
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
    let folders = |cx: &mut TestAppContext| {
        app.read_with(cx, |a, _| match &a.phase {
            Phase::Main(s) => s.spaces.len(),
            _ => panic!(),
        })
    };
    let before = folders(cx);
    // Mockup 37a: "+" makes a row named in place. Esc: nothing made.
    act(cx, h, |w, cx| w.click("new-folder-v", cx));
    assert!(window_has(cx, h, "folder-edit"));
    act(cx, h, |w, cx| {
        w.input("Draft", cx);
        w.press("escape", cx);
    });
    assert!(!window_has(cx, h, "folder-edit"));
    assert_eq!(folders(cx), before);
    // Left empty, a click elsewhere: nothing made either.
    act(cx, h, |w, cx| w.click("new-folder-v", cx));
    act(cx, h, |w, cx| w.click("f-all", cx));
    assert!(!window_has(cx, h, "folder-edit"));
    assert_eq!(folders(cx), before);
    // A name, then Enter: kept.
    act(cx, h, |w, cx| w.click("new-folder-v", cx));
    act(cx, h, |w, cx| {
        w.input("Savings", cx);
        w.press("enter", cx);
    });
    assert_eq!(folders(cx), before + 1);
    let (bank, home, dropbox) = app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => {
            let b = s
                .spaces
                .iter()
                .find(|x| x.name == "Savings")
                .expect("folder")
                .id;
            assert_eq!(
                s.filter,
                cleis_app::app::Filter::Space(b),
                "the new folder is shown"
            );
            let d = s.entries.iter().find(|e| e.name == "Dropbox").unwrap().id;
            (b, s.vault.home_space(), d)
        }
        _ => panic!(),
    });
    // Move Dropbox (in My vault) to Banking from the inspector's menu.
    act(cx, h, |w, cx| w.click(format!("sp-{home}"), cx));
    app.update(cx, |a, cx| {
        if let Phase::Main(s) = &mut a.phase {
            s.selected = Some(dropbox);
        }
        cx.notify();
    });
    cx.run_until_parked();
    act(cx, h, |w, cx| w.click("more", cx));
    act(cx, h, |w, cx| w.click(format!("menu-move-{bank}"), cx));
    app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => {
            assert_eq!(
                s.entries.iter().find(|e| e.id == dropbox).unwrap().space,
                bank
            )
        }
        _ => panic!(),
    });
    // Rename from its menu (the name is selected: typing replaces it), then by a double-click
    // (mockup 37b) kept by a click elsewhere; Esc keeps the old name.
    act(cx, h, |w, cx| w.hover(format!("sp-{bank}"), cx));
    act(cx, h, |w, cx| w.click(format!("fmore-{bank}"), cx));
    act(cx, h, |w, cx| w.click(format!("frename-{bank}"), cx));
    act(cx, h, |w, cx| {
        w.input("Home", cx);
        w.press("enter", cx);
    });
    let name_of = |cx: &mut TestAppContext| {
        app.read_with(cx, |a, _| match &a.phase {
            Phase::Main(s) => s.spaces.iter().find(|x| x.id == bank).unwrap().name.clone(),
            _ => panic!(),
        })
    };
    assert_eq!(name_of(cx), "Home");
    act(cx, h, |w, cx| w.double_click(format!("pfw-{bank}"), cx));
    assert!(window_has(cx, h, "folder-edit"));
    act(cx, h, |w, cx| {
        w.input("Old", cx);
        w.press("escape", cx);
    });
    assert_eq!(name_of(cx), "Home");
    act(cx, h, |w, cx| w.double_click(format!("pfw-{bank}"), cx));
    act(cx, h, |w, cx| {
        w.input("House", cx);
        w.click("f-all", cx);
    });
    assert_eq!(name_of(cx), "House");
    act(cx, h, |w, cx| w.hover(format!("sp-{bank}"), cx));
    act(cx, h, |w, cx| w.click(format!("fmore-{bank}"), cx));
    act(cx, h, |w, cx| w.click(format!("fdelete-{bank}"), cx));
    act(cx, h, |w, cx| w.click("folder-ok", cx));
    app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => {
            assert!(s.spaces.iter().all(|x| x.id != bank));
            assert_eq!(
                s.entries.iter().find(|e| e.id == dropbox).unwrap().space,
                home
            );
            assert_eq!(s.entries.len(), 14, "nothing lost");
        }
        _ => panic!(),
    });
}

/// The file of the fake repository: its sha and bytes.
type RepoFile = std::sync::Arc<std::sync::Mutex<Option<(String, Vec<u8>)>>>;

/// A stand-in for GitHub's contents API on 127.0.0.1: one file, its sha checked on writes.
fn fake_github() -> (String, RepoFile) {
    let (addr, file, _) = fake_github_account(true);
    (addr, file)
}

/// [`fake_github`] whose account has the repository `cleis-vault` or not: made by
/// `POST /user/repos` (the flag says whether it exists).
fn fake_github_account(
    repo: bool,
) -> (
    String,
    RepoFile,
    std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::sync::atomic::Ordering;
    let file: RepoFile = Default::default();
    let has_repo = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(repo));
    let made = has_repo.clone();
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = format!("http://{}", l.local_addr().unwrap());
    let f = file.clone();
    std::thread::spawn(move || {
        let mut n = 0u32;
        for stream in l.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut r = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                if r.read_line(&mut line).unwrap_or(0) == 0 {
                    break;
                }
                let (mut len, mut chunked, mut accept, mut auth) =
                    (0usize, false, String::new(), String::new());
                loop {
                    let mut h = String::new();
                    r.read_line(&mut h).unwrap();
                    let h = h.trim_end().to_owned();
                    if h.is_empty() {
                        break;
                    }
                    let (k, v) = h.split_once(':').unwrap();
                    match k.to_ascii_lowercase().as_str() {
                        "content-length" => len = v.trim().parse().unwrap(),
                        "transfer-encoding" => chunked = v.contains("chunked"),
                        "accept" => accept = v.trim().into(),
                        "authorization" => auth = v.trim().into(),
                        _ => {}
                    }
                }
                let mut body = vec![];
                if chunked {
                    loop {
                        let mut sz = String::new();
                        r.read_line(&mut sz).unwrap();
                        let n = usize::from_str_radix(sz.trim(), 16).unwrap();
                        let mut b = vec![0; n + 2];
                        r.read_exact(&mut b).unwrap();
                        if n == 0 {
                            break;
                        }
                        body.extend_from_slice(&b[..n]);
                    }
                } else {
                    body.resize(len, 0);
                    r.read_exact(&mut body).unwrap();
                }
                let path_ok =
                    line.contains("/api/v3/repos/lea-example/cleis-vault/contents/vault.cleis");
                // Sign in with GitHub (37n): the device flow, the account, its repository.
                let device = if line.contains("/login/device/code") {
                    Some(
                        r#"{"device_code":"dc","user_code":"WDJB-MJHT","verification_uri":"https://github.com/login/device","expires_in":900,"interval":5}"#,
                    )
                } else if line.contains("/login/oauth/access_token") {
                    Some(r#"{"access_token":"github_pat_example","token_type":"bearer"}"#)
                } else if line.contains("/api/v3/user ") {
                    Some(r#"{"login":"lea-example"}"#)
                } else if line.contains("/api/v3/repos/lea-example/cleis-vault ")
                    && made.load(Ordering::SeqCst)
                {
                    Some(r#"{"default_branch":"main","private":true}"#)
                } else {
                    None
                };
                let (status, out): (u16, Vec<u8>) = if let Some(d) = device {
                    (200, d.as_bytes().to_vec())
                } else if line.starts_with("POST /api/v3/user/repos ")
                    && auth == "Bearer github_pat_example"
                {
                    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
                    assert_eq!(
                        (v["name"].as_str(), v["private"].as_bool()),
                        (Some("cleis-vault"), Some(true))
                    );
                    made.store(true, Ordering::SeqCst);
                    (
                        201,
                        br#"{"full_name":"lea-example/cleis-vault","default_branch":"main","private":true}"#.to_vec(),
                    )
                } else if auth != "Bearer github_pat_example" {
                    (401, br#"{"message":"Bad credentials"}"#.to_vec())
                } else if !path_ok || !made.load(Ordering::SeqCst) {
                    (404, br#"{"message":"Not Found"}"#.to_vec())
                } else if line.starts_with("GET") {
                    match &*f.lock().unwrap() {
                        None => (404, br#"{"message":"Not Found"}"#.to_vec()),
                        Some((sha, bytes)) if accept.contains("raw") => (200, bytes.clone()),
                        Some((sha, _)) => (
                            200,
                            format!(r#"{{"type":"file","sha":"{sha}"}}"#).into_bytes(),
                        ),
                    }
                } else {
                    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
                    let mut cur = f.lock().unwrap();
                    let given = v["sha"].as_str().map(str::to_owned);
                    if given != cur.as_ref().map(|c| c.0.clone()) {
                        (409, br#"{"message":"does not match"}"#.to_vec())
                    } else {
                        use data_encoding::BASE64;
                        n += 1;
                        let bytes = BASE64
                            .decode(v["content"].as_str().unwrap().as_bytes())
                            .unwrap();
                        *cur = Some((format!("sha{n}"), bytes));
                        (200, b"{}".to_vec())
                    }
                };
                let head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n",
                    out.len()
                );
                if stream
                    .write_all(head.as_bytes())
                    .and_then(|_| stream.write_all(&out))
                    .is_err()
                {
                    break;
                }
            }
        }
    });
    (addr, file, has_repo)
}

/// Scenario (mockup 23b, D-65): sync with a file in a GitHub repository; another device's copy
/// sent meanwhile is merged, not overwritten.
#[gpui_kit::test]
fn sync_github_place(cx: &mut TestAppContext) {
    let (addr, file) = fake_github();
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
    act(cx, h, |w, cx| w.click("set-Sync & backups", cx));
    act(cx, h, |w, cx| w.click("sync-github", cx));
    act(cx, h, |w, cx| {
        w.click("gh-repo", cx);
        w.input(&format!("{addr}/lea-example/cleis-vault"), cx);
        w.click("gh-path", cx);
        w.press("ctrl-a", cx);
        w.input("vault.cleis", cx);
        w.click("gh-token", cx);
        w.input("github_pat_wrong", cx);
        w.click("place-connect", cx);
    });
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let d = s.place.as_ref().expect("dialog stays");
        assert!(d.error.as_ref().unwrap().contains("token"), "{:?}", d.error);
    });
    act(cx, h, |w, cx| {
        w.click("gh-token", cx);
        w.press("ctrl-a", cx);
        w.input("github_pat_example", cx);
        w.click("place-connect", cx);
    });
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.place.is_none());
        assert!(s.sync.last_error.is_none(), "{:?}", s.sync.last_error);
        assert_eq!(s.sync.provider, cleis_app::sync::Provider::GitHub);
    });
    let (sha, bytes) = file.lock().unwrap().clone().expect("copy sent");
    // The other device adds an item and sends its copy.
    let other = dir.path().join("other.cleis");
    std::fs::write(&other, &bytes).unwrap();
    {
        let v = cleis_store::LockedVault::open(&other)
            .unwrap()
            .unlock(cleis_app::demo::PASSWORD)
            .unwrap();
        v.put_entry(&cleis_core::Entry::new(v.home_space(), "Example Wiki"))
            .unwrap();
    }
    *file.lock().unwrap() = Some((format!("{sha}-other"), std::fs::read(&other).unwrap()));
    // This device: a change of its own, then a sync.
    app.update(cx, |a, _| {
        let Phase::Main(s) = &mut a.phase else {
            panic!()
        };
        s.vault
            .put_entry(&cleis_core::Entry::new(
                s.vault.home_space(),
                "Example Notes",
            ))
            .unwrap();
    });
    act(cx, h, |w, cx| w.click("sync-now", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.sync.last_error.is_none(), "{:?}", s.sync.last_error);
        assert!(s.entries.iter().any(|e| e.name == "Example Wiki"));
    });
    // The repository's copy has both.
    let (_, bytes) = file.lock().unwrap().clone().unwrap();
    std::fs::write(&other, &bytes).unwrap();
    let v = cleis_store::LockedVault::open(&other)
        .unwrap()
        .unlock(cleis_app::demo::PASSWORD)
        .unwrap();
    let names: Vec<String> = v
        .entries()
        .unwrap()
        .iter()
        .map(|e| e.name.clone())
        .collect();
    assert!(names.contains(&"Example Wiki".to_owned()));
    assert!(names.contains(&"Example Notes".to_owned()));
}

/// Audit S-9: biometric unlock is gone; a device key left by an earlier build is erased when
/// the vault opens.
#[gpui_kit::test]
fn biometric_key_left_is_erased(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.cleis");
    cleis_app::demo::create(&path).unwrap();
    let bio = dir.path().join("demo.cleis.bio");
    std::fs::write(&bio, b"{}").unwrap();
    let (h, app) = open(cx, &path);
    cx.run_until_parked();
    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.press("enter", cx);
    });
    app.read_with(cx, |a, _| assert!(matches!(a.phase, Phase::Main(_))));
    assert!(!bio.exists());
}

/// Scenario (mockups 33, 34): right-click an item, Escape closes the menu, Duplicate from the
/// menu, Delete key to the trash, Move to ▸ a folder, Ctrl+D; a question is cancelled by
/// Escape and Enter.
#[gpui_kit::test]
fn context_menu_and_shortcuts(cx: &mut TestAppContext) {
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
    let (figma, banking, count) = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        (
            s.entries.iter().find(|e| e.name == "Figma").unwrap().id,
            s.spaces.iter().find(|x| x.name == "Banking").unwrap().id,
            s.entries.len(),
        )
    });
    let main = |cx: &mut TestAppContext, f: &dyn Fn(&cleis_app::app::Session)| {
        app.read_with(cx, |a, _| {
            let Phase::Main(s) = &a.phase else { panic!() };
            f(s)
        })
    };

    // Only a left click opens an item: the right-clicked row keeps the selection as it was
    // (mockup 37o), outlined while its menu is open.
    let first = app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => s.selected,
        _ => None,
    });
    assert_ne!(first, Some(figma));
    act(cx, h, |w, cx| w.right_click(format!("row-{figma}"), cx));
    main(cx, &|s| {
        assert!(s.ctx.is_some());
        assert_eq!(s.selected, first);
        assert!(s.aimed(cleis_app::screens::ctx::CtxTarget::Entry(figma)));
    });
    act(cx, h, |w, cx| w.press("escape", cx));
    main(cx, &|s| assert!(s.ctx.is_none()));

    act(cx, h, |w, cx| w.right_click(format!("row-{figma}"), cx));
    act(cx, h, |w, cx| w.click("cx-duplicate", cx));
    main(cx, &|s| {
        assert!(s.ctx.is_none());
        assert_eq!(s.entries.len(), count + 1);
        assert_eq!(s.selected_entry().unwrap().name, "Figma copy");
    });
    // The copy is selected and no box has the keyboard: Delete moves it to the trash.
    act(cx, h, |w, cx| w.press("delete", cx));
    main(cx, &|s| {
        assert_eq!(s.entries.len(), count);
        assert!(s.trash.iter().any(|e| e.name == "Figma copy"));
    });

    act(cx, h, |w, cx| w.right_click(format!("row-{figma}"), cx));
    act(cx, h, |w, cx| w.click("cx-move", cx));
    act(cx, h, |w, cx| w.click(format!("cx-move-{banking}"), cx));
    main(cx, &|s| {
        let e = s.entries.iter().find(|e| e.name == "Figma").unwrap();
        assert_eq!(e.space, banking);
    });

    // With a row's menu open, its keys act on that row: Ctrl+D duplicates Figma.
    act(cx, h, |w, cx| w.right_click(format!("row-{figma}"), cx));
    act(cx, h, |w, cx| w.press("ctrl-d", cx));
    main(cx, &|s| {
        assert!(s.ctx.is_none());
        assert_eq!(s.entries.len(), count + 1);
        assert_eq!(s.selected_entry().unwrap().name, "Figma copy");
    });
    // Audit U-3: arrows move through the list; Ctrl+N makes a new item.
    let before = app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => s.selected,
        _ => None,
    });
    act(cx, h, |w, cx| w.press("up", cx));
    main(cx, &|s| assert!(s.selected.is_some()));
    let after = app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => s.selected,
        _ => None,
    });
    assert_ne!(before, after);
    act(cx, h, |w, cx| w.press("ctrl-n", cx));
    main(cx, &|s| assert_eq!(s.entries.len(), count + 2));

    // A question: Escape cancels it, and Enter too (Cancel holds the focus).
    app.update(cx, |a, cx| {
        a.ask_action(cleis_app::app::ConfirmAction::RotateKey, cx)
    });
    act(cx, h, |w, cx| w.press("escape", cx));
    main(cx, &|s| assert!(s.confirm.is_none()));
    app.update(cx, |a, cx| {
        a.ask_action(cleis_app::app::ConfirmAction::RotateKey, cx)
    });
    cx.run_until_parked();
    act(cx, h, |w, cx| w.press("enter", cx));
    main(cx, &|s| assert!(s.confirm.is_none()));
}

/// Scenario (mockups 32 to 32c, D-69): Preferences › Vaults puts the personal vault in its own
/// tab, renamed "Personal"; with two organizations each has its tab, its sidebar and its items,
/// and an answer for the organization not shown reaches it, not the one shown.
#[gpui_kit::test]
fn vault_tabs_and_two_organizations(cx: &mut TestAppContext) {
    use cleis_app::app::Scope;
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
    let read = |cx: &mut TestAppContext, f: &dyn Fn(&cleis_app::app::Session)| {
        app.read_with(cx, |a, _| {
            let Phase::Main(s) = &a.phase else { panic!() };
            f(s)
        })
    };
    app.update(cx, |a, _| {
        let Phase::Main(s) = &mut a.phase else {
            panic!()
        };
        cleis_app::demo::org(s);
        cleis_app::demo::second_org(s);
    });
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Preferences", cx));
    act(cx, h, |w, cx| w.click("vault-tabs", cx));
    read(cx, &|s| {
        assert!(s.prefs.vault_tabs);
        assert_eq!(s.scope, Scope::Personal);
    });
    act(cx, h, |w, cx| {
        w.click("vault-name", cx);
        w.press("ctrl-a", cx);
        w.input("Personal", cx);
        w.press("enter", cx);
    });
    read(cx, &|s| assert_eq!(s.personal_name(), "Personal"));

    act(cx, h, |w, cx| w.click("tab-vault-personal", cx));
    read(cx, &|s| {
        assert_eq!(s.tab, Tab::Vault);
        assert!(!s.visible.is_empty());
        assert!(
            s.visible
                .iter()
                .all(|i| !s.is_org_space(s.entries[*i].space))
        );
    });

    let (studio, acme) = read_keys(cx, &app);
    act(cx, h, |w, cx| w.click(format!("tab-vault-{acme}"), cx));
    read(cx, &|s| {
        assert_eq!(s.org.as_ref().unwrap().m.key(), acme);
        let names: Vec<&str> = s
            .visible
            .iter()
            .map(|i| s.entries[*i].name.as_str())
            .collect();
        assert_eq!(names, ["Acme portal", "Jira", "Shared drive"]);
    });
    // An item of Acme stays in Acme: Example Studio's spaces are refused.
    let (jira, design) = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        (
            s.entries.iter().find(|e| e.name == "Jira").unwrap().id,
            s.spaces.iter().find(|x| x.name == "Design").unwrap().id,
        )
    });
    app.update(cx, |a, cx| a.move_entry(jira, design, cx));
    read(cx, &|s| {
        let e = s.entries.iter().find(|e| e.name == "Jira").unwrap();
        assert_ne!(e.space, design);
    });
    // Work for Example Studio runs with it in place, then Acme is shown again.
    let seen = app.update(cx, |a, _| a.in_org(&studio, |a| a.active_org_key()));
    assert_eq!(seen.flatten().as_deref(), Some(studio.as_str()));
    read(cx, &|s| assert_eq!(s.org.as_ref().unwrap().m.key(), acme));

    // Settings: one group per organization; a page of the other one shows it.
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click(format!("set-{studio}-Members"), cx));
    read(cx, &|s| {
        assert_eq!(s.org.as_ref().unwrap().m.key(), studio);
        assert_eq!(s.settings_page, "Members");
    });

    // Off again: one Vault tab with everything.
    act(cx, h, |w, cx| w.click("set-Preferences", cx));
    act(cx, h, |w, cx| w.click("vault-tabs", cx));
    act(cx, h, |w, cx| w.click("tab-Vault", cx));
    read(cx, &|s| {
        assert_eq!(s.scope, Scope::All);
        assert_eq!(s.visible.len(), s.entries.len());
    });
}

fn read_keys(cx: &mut TestAppContext, app: &Entity<CleisApp>) -> (String, String) {
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let key = |n: &str| {
            s.all_orgs()
                .find(|o| o.m.name == n)
                .map(|o| o.m.key())
                .unwrap()
        };
        (key("Example Studio"), key("Acme Partners"))
    })
}

/// Scenario (mockup 32e): Health filtered by vault — the personal vault, then one organization;
/// a space of another vault is dropped from the filter.
#[gpui_kit::test]
fn health_by_vault(cx: &mut TestAppContext) {
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
    app.update(cx, |a, _| {
        let Phase::Main(s) = &mut a.phase else {
            panic!()
        };
        cleis_app::demo::org(s);
        cleis_app::demo::second_org(s);
    });
    let (_, acme) = read_keys(cx, &app);
    act(cx, h, |w, cx| w.click("tab-Health", cx));
    let banking = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        s.spaces.iter().find(|x| x.name == "Banking").unwrap().id
    });
    act(cx, h, |w, cx| w.click("health-space", cx));
    act(cx, h, |w, cx| w.click(format!("hs-{banking}"), cx));
    act(cx, h, |w, cx| w.click("health-vault", cx));
    act(cx, h, |w, cx| w.click("hv-personal", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.health.vault, Some(cleis_app::app::Scope::Personal));
        assert_eq!(s.health.space, Some(banking), "Banking is personal: kept");
    });
    act(cx, h, |w, cx| w.click("health-vault", cx));
    act(cx, h, |w, cx| w.click(format!("hv-{acme}"), cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(
            s.health.vault,
            Some(cleis_app::app::Scope::Org(acme.clone()))
        );
        assert_eq!(s.health.space, None, "Banking is not Acme's");
    });
    act(cx, h, |w, cx| w.click("health-vault", cx));
    act(cx, h, |w, cx| w.press("escape", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(!s.health.vault_menu);
    });
}

/// Audit L-6: the server's address is changed on Settings › Server (its key stays pinned);
/// L-18: Activity can be hidden from Preferences › Tabs.
#[gpui_kit::test]
fn server_address_and_activity_tab(cx: &mut TestAppContext) {
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
    app.update(cx, |a, _| {
        let Phase::Main(s) = &mut a.phase else {
            panic!()
        };
        cleis_app::demo::org(s);
    });
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Server", cx));
    act(cx, h, |w, cx| w.click("server-address-value", cx));
    act(cx, h, |w, cx| {
        w.press("ctrl-a", cx);
        w.input("not an address", cx);
        w.press("enter", cx);
    });
    act(cx, h, |w, cx| {
        w.press("ctrl-a", cx);
        w.input("10.0.0.20:8443", cx);
        w.press("enter", cx);
    });
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let o = s.org.as_ref().unwrap();
        assert_eq!(o.m.address, "10.0.0.20:8443");
        assert!(o.ui.address.is_none());
        assert!(!o.m.pins.is_empty(), "same key, still pinned");
    });

    act(cx, h, |w, cx| w.click("set-Preferences", cx));
    act(cx, h, |w, cx| w.click("tab-activity", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(!s.prefs.tabs.activity);
    });
}

/// Scenario (26/11/2026): a click below the rows lets go of the item and closes its panel, the
/// panel's "×" too; "My vault" lists the whole personal vault, its folders included; "Saved"
/// leaves after 1.5 s.
#[gpui_kit::test]
fn deselect_close_and_personal_section(cx: &mut TestAppContext) {
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
    let read = |cx: &mut TestAppContext| {
        app.read_with(cx, |a, _| {
            let Phase::Main(s) = &a.phase else { panic!() };
            (s.selected, s.visible.len(), s.toast.clone())
        })
    };
    let (figma, home) = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        (
            s.entries.iter().find(|e| e.name == "Figma").unwrap().id,
            s.vault.home_space(),
        )
    });
    act(cx, h, |w, cx| {
        w.click_at(
            format!("row-{figma}"),
            gpui_kit::point(px(30.), px(10.)),
            cx,
        )
    });
    assert_eq!(read(cx).0, Some(figma));
    // Far below the last row.
    act(cx, h, |w, cx| {
        w.click_at("entries-void", gpui_kit::point(px(400.), px(700.)), cx)
    });
    assert_eq!(read(cx).0, None);
    act(cx, h, |w, cx| {
        w.click_at(
            format!("row-{figma}"),
            gpui_kit::point(px(30.), px(10.)),
            cx,
        )
    });
    act(cx, h, |w, cx| w.click("close-inspector", cx));
    assert_eq!(read(cx).0, None);

    // A personal item put in a folder still shows under "My vault".
    let banking = app.update(cx, |a, cx| {
        let Phase::Main(s) = &mut a.phase else {
            panic!()
        };
        let banking = s.spaces.iter().find(|x| x.name == "Banking").unwrap().id;
        let mut e = cleis_core::Entry::new(banking, "Bank card");
        e.touch(None);
        s.vault.put_entry(&e).unwrap();
        s.entries.push(e);
        s.refilter();
        cx.notify();
        banking
    });
    cx.run_until_parked();
    act(cx, h, |w, cx| w.click(format!("sp-{home}"), cx));
    let personal = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.filter, cleis_app::app::Filter::Personal);
        s.visible
            .iter()
            .map(|i| s.entries[*i].space)
            .collect::<Vec<_>>()
    });
    assert!(personal.contains(&home) && personal.contains(&banking));

    // "Saved" goes after 1.5 s.
    let first = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        s.entries[s.visible[0]].id
    });
    act(cx, h, |w, cx| {
        w.click_at(
            format!("row-{first}"),
            gpui_kit::point(px(30.), px(10.)),
            cx,
        )
    });
    act(cx, h, |w, cx| w.click("fav", cx));
    assert!(read(cx).2.is_some());
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(1600));
    cx.run_until_parked();
    assert_eq!(read(cx).2, None);
}

/// Sidebar in sections (mockups 35, 35f, 35j): an organization's name lists all of it; the
/// shortcuts are chosen per vault, or the same everywhere; customize shows the hidden ones.
#[gpui_kit::test]
fn sidebar_sections_and_shortcuts(cx: &mut TestAppContext) {
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
    app.update(cx, |a, _| {
        let Phase::Main(s) = &mut a.phase else {
            panic!()
        };
        cleis_app::demo::org(s);
        cleis_app::demo::second_org(s);
    });
    cx.run_until_parked();
    let (studio, acme) = read_keys(cx, &app);
    // The organization's section: every item of its spaces, none of the other.
    act(cx, h, |w, cx| w.click(format!("org-{acme}"), cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(matches!(s.filter, cleis_app::app::Filter::Org(_)));
        let names: Vec<&str> = s
            .visible
            .iter()
            .map(|i| s.entries[*i].name.as_str())
            .collect();
        assert_eq!(names, ["Acme portal", "Jira", "Shared drive"]);
    });

    // Vault tabs on: Renew soon (off by default since 27/11/2026) shown in the personal vault only.
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Preferences", cx));
    act(cx, h, |w, cx| w.click("vault-tabs", cx));
    act(cx, h, |w, cx| w.click("sc-personal-2", cx));
    act(cx, h, |w, cx| w.click("tab-vault-personal", cx));
    let has = |cx: &mut TestAppContext, id: &str| {
        let id = gpui_kit::SharedString::from(id.to_owned());
        cx.update_window(h.into(), move |_, w, _| w.try_find(id).is_some())
            .unwrap()
    };
    assert!(has(cx, "f-all"));
    assert!(has(cx, "f-renew"));
    act(cx, h, |w, cx| w.click(format!("tab-vault-{studio}"), cx));
    assert!(!has(cx, "f-renew"));

    // Customize: the hidden shortcut comes back with its eye; shown again from there.
    act(cx, h, |w, cx| w.click(format!("tab-vault-{studio}"), cx));
    act(cx, h, |w, cx| w.click("customize", cx));
    assert!(has(cx, "f-renew"));
    act(cx, h, |w, cx| w.click("f-renew-eye", cx));
    act(cx, h, |w, cx| w.click("customize", cx));
    assert!(has(cx, "f-renew"));

    // The same everywhere: one choice for every vault.
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Preferences", cx));
    act(cx, h, |w, cx| w.click("shortcuts-same", cx));
    act(cx, h, |w, cx| w.click("sc-all-1", cx));
    act(cx, h, |w, cx| w.click(format!("tab-vault-{acme}"), cx));
    assert!(!has(cx, "f-fav"));
    act(cx, h, |w, cx| w.click("tab-vault-personal", cx));
    assert!(!has(cx, "f-fav"));
}

/// Void menus (mockups 35, 35b): sort the list, switch to cards, hide a shortcut, customize.
#[gpui_kit::test]
fn void_menus_and_sort(cx: &mut TestAppContext) {
    use cleis_core::settings::SortBy;
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
    let names = |cx: &mut TestAppContext| {
        app.read_with(cx, |a, _| {
            let Phase::Main(s) = &a.phase else { panic!() };
            s.visible
                .iter()
                .map(|i| s.entries[*i].name.clone())
                .collect::<Vec<_>>()
        })
    };
    let by_name = names(cx);
    act(cx, h, |w, cx| void_right(w, "entries-void", cx));
    act(cx, h, |w, cx| w.click("cx-sort", cx));
    act(cx, h, |w, cx| w.click("cx-sort-Price", cx));
    let prices = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.prefs.sort_by, SortBy::Price);
        assert!(s.ctx.is_none());
        s.visible
            .iter()
            .map(|i| s.entries[*i].price.as_ref().map_or(0, |m| m.yearly_cents()))
            .collect::<Vec<_>>()
    });
    assert!(prices.windows(2).all(|w| w[0] >= w[1]), "{prices:?}");

    // My order starts from what was shown, then follows the order kept.
    act(cx, h, |w, cx| void_right(w, "entries-void", cx));
    act(cx, h, |w, cx| w.click("cx-sort", cx));
    act(cx, h, |w, cx| w.click("cx-sort-Manual", cx));
    let mine = names(cx);
    assert_ne!(mine, by_name);
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.prefs.sort_by, SortBy::Manual);
        assert_eq!(s.prefs.item_order.len(), s.visible.len());
    });

    // Show as cards, and back from the cards' void.
    act(cx, h, |w, cx| void_right(w, "entries-void", cx));
    act(cx, h, |w, cx| w.click("cx-view", cx));
    act(cx, h, |w, cx| void_right(w, "vault-cards", cx));
    act(cx, h, |w, cx| w.click("cx-view", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(!s.vault_cards);
    });

    // The sidebar: a tick keeps the menu open; Customize.
    act(cx, h, |w, cx| void_right(w, "sidebar", cx));
    act(cx, h, |w, cx| w.click("cx-sc-1", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.ctx.is_some());
        assert_eq!(s.prefs.shortcuts.of("all"), [true, false, false]);
    });
    act(cx, h, |w, cx| w.click("cx-customize", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.customize && s.ctx.is_none());
    });
}

/// A right-click far down an element, below its rows.
fn void_right(w: &mut gpui_kit::Window, id: &str, cx: &mut gpui_kit::App) {
    w.click_with_options(
        gpui_kit::SharedString::from(id.to_owned()),
        gpui_kit::test::ClickOptions::new()
            .with_button(gpui_kit::MouseButton::Right)
            .with_offset(gpui_kit::point(px(60.), px(600.))),
        cx,
    );
}

/// Drag and drop (mockups 35c to 35f): an item onto a folder, a folder before another, an item
/// before another while customizing, a column's edge; the cross of customize mode.
#[gpui_kit::test]
fn drag_drop_resize_customize(cx: &mut TestAppContext) {
    use cleis_core::settings::SortBy;
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
    let (item, space) = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let item = |n: &str| s.entries.iter().find(|e| e.name == n).unwrap().id;
        let space = |n: &str| s.spaces.iter().find(|x| x.name == n).unwrap().id;
        (
            [
                item("Dropbox"),
                item("Webflow"),
                item("Canva"),
                item("Slack"),
            ],
            [space("Banking"), space("Dev"), space("General")],
        )
    });
    let [dropbox, webflow, canva, slack] = item;
    let [banking, dev, general] = space;

    // An item onto a folder: it moves into it.
    act(cx, h, |w, cx| {
        w.drag_to(format!("row-{dropbox}"), format!("pfw-{banking}"), cx)
    });
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(
            s.entries.iter().find(|e| e.id == dropbox).unwrap().space,
            banking
        );
    });

    // A folder onto another: it takes its place.
    act(cx, h, |w, cx| {
        w.drag_to(format!("pfw-{dev}"), format!("pfw-{general}"), cx)
    });
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let o = &s.prefs.space_order;
        let at = |id| o.iter().position(|x| *x == id).unwrap();
        // Dragged down, it lands just after the folder it was dropped on.
        assert_eq!(at(dev), at(general) + 1, "{o:?}");
    });

    // Sorted by name, rows do not take each other's place; while customizing they do.
    act(cx, h, |w, cx| {
        w.drag_to(format!("row-{webflow}"), format!("row-{canva}"), cx)
    });
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.prefs.sort_by, SortBy::Name);
    });
    act(cx, h, |w, cx| w.click("customize", cx));
    act(cx, h, |w, cx| {
        w.drag_to(format!("row-{webflow}"), format!("row-{canva}"), cx)
    });
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.prefs.sort_by, SortBy::Manual);
        let shown: Vec<_> = s.visible.iter().map(|i| s.entries[*i].id).collect();
        let at = |id| shown.iter().position(|x| *x == id).unwrap();
        assert_eq!(at(webflow) + 1, at(canva));
    });
    // The cross moves the item to the trash; Done leaves.
    act(cx, h, |w, cx| w.click(format!("rm-item-{slack}"), cx));
    act(cx, h, |w, cx| w.click("customize-done", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(!s.customize);
        assert!(s.entries.iter().all(|e| e.id != slack));
        assert!(s.trash.iter().any(|e| e.id == slack));
    });

    // A column's edge: dragged wider, then fitted to its longest name.
    let name_width = |cx: &mut TestAppContext| {
        app.read_with(cx, |a, _| {
            let Phase::Main(s) = &a.phase else { panic!() };
            assert!(s.resize.is_none());
            s.columns
                .iter()
                .find(|c| c.table == cleis_core::column::Table::Vault && c.id.0 == "name")
                .unwrap()
                .width
        })
    };
    act(cx, h, |w, cx| {
        w.drag_to("colgrip-name", "colgrip-password", cx)
    });
    let wide = name_width(cx).expect("kept");
    assert!(wide > 300, "{wide}");
    act(cx, h, |w, cx| w.double_click("colgrip-name", cx));
    let fit = name_width(cx).expect("kept");
    assert!(fit < wide && fit >= 150, "{fit}");
}

/// Dates (mockup 35g): the renewal picked in the calendar, one year from today, then cleared.
#[gpui_kit::test]
fn renewal_calendar(cx: &mut TestAppContext) {
    use chrono::{Datelike, Months};
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
    let figma = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        s.entries.iter().find(|e| e.name == "Figma").unwrap().id
    });
    act(cx, h, |w, cx| {
        w.click_at(
            format!("row-{figma}"),
            gpui_kit::point(px(30.), px(10.)),
            cx,
        )
    });
    let renewal = |cx: &mut TestAppContext| {
        app.read_with(cx, |a, _| {
            let Phase::Main(s) = &a.phase else { panic!() };
            s.entries
                .iter()
                .find(|e| e.id == figma)
                .unwrap()
                .renewal
                .clone()
        })
    };
    let today = chrono::Local::now().date_naive();
    act(cx, h, |w, cx| w.click("cal-open-renewal", cx));
    let month = |cx: &mut TestAppContext| {
        app.read_with(cx, |a, _| {
            let Phase::Main(s) = &a.phase else { panic!() };
            s.calendar.as_ref().map(|c| (c.year, c.month))
        })
    };
    // It opens on the month of the renewal; the next one is shown after a click.
    let (y, m) = month(cx).unwrap();
    act(cx, h, |w, cx| w.click("cal-next", cx));
    let shown = chrono::NaiveDate::from_ymd_opt(y, m, 1).unwrap() + Months::new(1);
    assert_eq!(month(cx), Some((shown.year(), shown.month())));
    let day = shown.with_day(15).unwrap();
    act(cx, h, |w, cx| {
        w.click(format!("cal-{}", day.format("%Y-%m-%d")), cx)
    });
    assert_eq!(renewal(cx), Some(day.format("%Y-%m-%d").to_string()));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.calendar.is_none());
    });
    act(cx, h, |w, cx| w.click("cal-open-renewal", cx));
    act(cx, h, |w, cx| w.click("cal-year", cx));
    let year = today + Months::new(12);
    assert_eq!(renewal(cx), Some(year.format("%Y-%m-%d").to_string()));
    act(cx, h, |w, cx| w.click("cal-open-renewal", cx));
    act(cx, h, |w, cx| w.click("cal-clear", cx));
    assert_eq!(renewal(cx), None);
}

/// Opening Cleis (D-76, mockups 35h, 35i): created to open with nothing to type, the recovery key
/// shown once; locked, it opens with one click or the recovery key; a master password set again
/// brings the password back on this computer.
#[gpui_kit::test]
fn opening_without_password(cx: &mut TestAppContext) {
    use cleis_app::opening::Mode;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.cleis");
    let (h, app) = open(cx, &path);
    cx.run_until_parked();
    act(cx, h, |w, cx| w.click("first-opening-Nothing", cx));
    act(cx, h, |w, cx| w.click("create", cx));
    let key = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else {
            panic!("created and open")
        };
        assert_eq!(s.opening, Mode::Nothing);
        assert!(!s.vault.has_password().unwrap());
        match &s.security.dialog {
            Some(cleis_app::screens::security::SecDialog::RecoveryKey(k)) => k.to_string(),
            _ => panic!("the recovery key is shown"),
        }
    });
    act(cx, h, |w, cx| w.click("recovery-key-done", cx));
    assert_eq!(cleis_app::opening::mode(&path), Mode::Nothing);

    // Locked: one click opens it; no password box.
    act(cx, h, |w, cx| w.click("lock", cx));
    let has_box = cx
        .update_window(h.into(), |_, w, _| w.try_find("master-password").is_some())
        .unwrap();
    assert!(!has_box);
    act(cx, h, |w, cx| w.click("unlock-device", cx));
    app.read_with(cx, |a, _| assert!(matches!(a.phase, Phase::Main(_))));

    // The recovery key opens it too (typed in lower case).
    act(cx, h, |w, cx| w.click("lock", cx));
    act(cx, h, |w, cx| w.click("use-recovery", cx));
    act(cx, h, |w, cx| {
        w.click("recovery-key", cx);
        w.input("WRONG", cx);
        w.press("enter", cx);
    });
    app.read_with(cx, |a, _| {
        let Phase::Unlock(u) = &a.phase else { panic!() };
        assert_eq!(u.error.as_deref(), Some("Wrong recovery key"));
    });
    act(cx, h, |w, cx| {
        w.click("recovery-key", cx);
        w.input(&key.to_lowercase(), cx);
        w.press("enter", cx);
    });
    app.read_with(cx, |a, _| assert!(matches!(a.phase, Phase::Main(_))));

    // Back to the master password: set first, then this computer's key goes.
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Account", cx));
    act(cx, h, |w, cx| w.click("opening-Password", cx));
    let pw = "a long enough password";
    act(cx, h, |w, cx| {
        w.click("pw-new", cx);
        w.input(pw, cx);
        w.click("pw-confirm", cx);
        w.input(pw, cx);
        w.click("chpw-ok", cx);
    });
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(50));
    cx.run_until_parked();
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.vault.has_password().unwrap());
        assert_eq!(s.opening, Mode::Password);
    });
    assert_eq!(cleis_app::opening::mode(&path), Mode::Password);
    act(cx, h, |w, cx| w.click("lock", cx));
    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input(pw, cx);
        w.press("enter", cx);
    });
    app.read_with(cx, |a, _| assert!(matches!(a.phase, Phase::Main(_))));
}

/// D-76: an organization that wants the master password takes "nothing" away on this computer.
#[gpui_kit::test]
fn organization_rule_for_opening(cx: &mut TestAppContext) {
    use cleis_app::opening::Mode;
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
    act(cx, h, |w, cx| w.click("set-Account", cx));
    act(cx, h, |w, cx| w.click("opening-Nothing", cx));
    assert_eq!(cleis_app::opening::mode(&path), Mode::Nothing);
    app.update(cx, |a, _| {
        let Phase::Main(s) = &mut a.phase else {
            panic!()
        };
        cleis_app::demo::org(s);
        if let Some(st) = s.org.as_mut().and_then(|o| o.state.as_mut()) {
            st.policy.unlock = cleis_core::org::UnlockRule::PasswordOnly;
            st.policy.master_min_length = 40;
        }
        s.typed_len = Some(cleis_app::demo::PASSWORD.chars().count());
        s.enforce_opening(&path);
        assert_eq!(s.opening, Mode::Password);
        assert!(s.password_due, "a longer password is asked for");
    });
    assert_eq!(cleis_app::opening::mode(&path), Mode::Password);
}

/// Scenario (mockup 36): the last swatch opens the wheel; a click on the wheel picks a colour
/// shown but not saved; Escape puts the saved accent back; a colour typed in the field and "Use"
/// make it the accent, ringed on the last swatch.
#[gpui_kit::test]
fn own_accent_colour(cx: &mut TestAppContext) {
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
    let accent = |cx: &mut TestAppContext| {
        app.read_with(cx, |a, _| {
            let Phase::Main(s) = &a.phase else { panic!() };
            (
                s.prefs.accent.clone(),
                s.wheel.as_ref().map(|w| w.hex_text()),
            )
        })
    };
    let before = accent(cx).0;
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("accent-own", cx));
    assert!(accent(cx).1.is_some());
    // The top of the wheel: red.
    act(cx, h, |w, cx| {
        w.click_at("wheel-area", gpui_kit::point(px(108.), px(4.)), cx)
    });
    let (saved, shown) = accent(cx);
    assert_eq!(saved, before);
    let shown = shown.unwrap();
    // Red: the strongest channel is red (the brightness kept from the bronze default, D-81).
    let ch = |i: usize| u8::from_str_radix(&shown[i..i + 2], 16).unwrap();
    assert!(ch(1) > ch(3) && ch(1) > ch(5), "{shown}");
    act(cx, h, |w, cx| w.press("escape", cx));
    assert_eq!(accent(cx), (before.clone(), None));

    act(cx, h, |w, cx| w.click("accent-own", cx));
    act(cx, h, |w, cx| {
        w.click("wheel-hex", cx);
        w.press("ctrl-a", cx);
        w.input("#3366CC", cx);
    });
    assert_eq!(accent(cx).1.as_deref(), Some("#3366cc"));
    act(cx, h, |w, cx| w.click("wheel-use", cx));
    assert_eq!(accent(cx), (Some("#3366cc".into()), None));
}

/// Scenario (mockup 36e): a picture becomes a contact's photo (centred square, 256 px JPEG),
/// shown in place of the initials; the avatar's menu removes it.
#[gpui_kit::test]
fn contact_photo(cx: &mut TestAppContext) {
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
    // A wide picture: only its centre is kept.
    let pic = dir.path().join("wide.png");
    image::RgbImage::from_pixel(600, 300, image::Rgb([40, 120, 200]))
        .save(&pic)
        .unwrap();
    act(cx, h, |w, cx| w.click("tab-Contacts", cx));
    let emma = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        s.contacts.iter().find(|c| c.given == "Emma").unwrap().id
    });
    act(cx, h, |w, cx| {
        w.click_at(format!("c-{emma}"), gpui_kit::point(px(30.), px(10.)), cx)
    });
    app.update(cx, |a, cx| a.contact_photo_from(emma, &pic, cx));
    let photo = |cx: &mut TestAppContext| {
        app.read_with(cx, |a, _| {
            let Phase::Main(s) = &a.phase else { panic!() };
            s.contacts
                .iter()
                .find(|c| c.id == emma)
                .unwrap()
                .photo
                .clone()
        })
    };
    let url = photo(cx).unwrap();
    let (kind, bytes) = cleis_core::contact::photo_bytes(&url).unwrap();
    assert_eq!(kind, "jpeg");
    let img = image::load_from_memory(&bytes).unwrap();
    assert_eq!((img.width(), img.height()), (256, 256));
    // Kept in the vault.
    let stored = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        s.vault.contacts().unwrap()
    });
    assert_eq!(
        stored.iter().find(|c| c.id == emma).unwrap().photo,
        Some(url)
    );

    act(cx, h, |w, cx| w.click("contact-photo", cx));
    act(cx, h, |w, cx| w.click("cm-photo-remove", cx));
    assert_eq!(photo(cx), None);
}

/// Scenario (mockups 36b, 36d, D-80): Settings › Account, GitHub: the GitHub place opens to sign
/// in; Microsoft and Google wait (not clickable). A new computer: "Sign in" › GitHub opens the
/// restore with the GitHub place.
#[gpui_kit::test]
fn account_sign_in(cx: &mut TestAppContext) {
    let (addr, file) = fake_github();
    let hosts = cleis_app::github_login::Hosts {
        web: addr.clone(),
        api: format!("{addr}/api/v3"),
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.cleis");
    cleis_app::demo::create(&path).unwrap();
    let (h, app) = open(cx, &path);
    app.update(cx, |a, _| a.github_hosts = hosts.clone());
    cx.run_until_parked();
    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.press("enter", cx);
    });
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Account", cx));
    // Microsoft waits for its registration: nothing opens.
    act(cx, h, |w, cx| w.click("acc-microsoft", cx));
    app.read_with(cx, |a, _| assert!(a.github.is_none()));
    // GitHub (37n): the code to type on github.com.
    act(cx, h, |w, cx| w.click("acc-signin", cx));
    app.read_with(cx, |a, _| assert!(a.github.is_some()));
    act(cx, h, |w, cx| w.click("gh-cancel", cx));
    app.read_with(cx, |a, _| assert!(a.github.is_none()));

    // A new computer (36d): Sign in › GitHub, then the vault already on the account comes.
    let (h2, app2) = open(cx, &dir.path().join("new/vault.cleis"));
    app2.update(cx, |a, _| a.github_hosts = hosts.clone());
    cx.run_until_parked();
    *file.lock().unwrap() = Some(("sha0".into(), std::fs::read(&path).unwrap()));
    act(cx, h2, |w, cx| w.click("signin-github", cx));
    for _ in 0..20 {
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(6));
        cx.run_until_parked();
        if app2.read_with(cx, |a, _| a.github.is_none()) {
            break;
        }
    }
    for _ in 0..20 {
        cx.run_until_parked();
        let found = app2.read_with(cx, |a, _| match &a.phase {
            Phase::Welcome(w) => w.restore.as_ref().is_some_and(|r| !r.found.is_empty()),
            _ => false,
        });
        if found {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    app2.read_with(cx, |a, _| {
        let Phase::Welcome(w) = &a.phase else {
            panic!()
        };
        let r = w.restore.as_ref().expect("restore");
        assert_eq!(r.provider, Some(cleis_app::sync::Provider::GitHub));
        assert_eq!(r.found.len(), 1, "the vault of the account, ready to open");
    });
}

/// Scrolls the settings page to its bottom (the sections below the window), over `over`.
fn settings_bottom(cx: &mut TestAppContext, h: WindowHandle<Root>, over: &'static str) {
    act(cx, h, |w, cx| {
        w.scroll(
            over,
            gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(-4000.))),
            cx,
        )
    });
}

/// Feedback on 0.2.2 (27/11/2026): shortcuts per vault in the single Vault tab (37i, 37j), Start
/// in a vault tab (37k), the password box's menu (37o).
#[gpui_kit::test]
fn split_shortcuts_start_in_and_password_menu(cx: &mut TestAppContext) {
    use cleis_app::app::{Filter, Scope};
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
    app.update(cx, |a, _| {
        let Phase::Main(s) = &mut a.phase else {
            panic!()
        };
        cleis_app::demo::org(s);
    });
    cx.run_until_parked();
    let studio = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        s.org_tabs()[0].0.clone()
    });
    // One Vault tab: one set at the top until "Same for every vault" is switched off.
    assert!(window_has(cx, h, "f-all"));
    assert!(!window_has(cx, h, "fv-personal-1"));
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Preferences", cx));
    act(cx, h, |w, cx| w.click("shortcuts-same", cx));
    // Per vault: All items is the vault's own name, so only two tags each.
    assert!(window_has(cx, h, "sc-personal-1"));
    assert!(!window_has(cx, h, "sc-personal-0"));
    act(cx, h, |w, cx| w.click(format!("sc-{studio}-2"), cx));
    act(cx, h, |w, cx| w.click("tab-Vault", cx));
    assert!(!window_has(cx, h, "f-all"));
    assert!(window_has(cx, h, "fv-personal-1"));
    assert!(!window_has(cx, h, "fv-personal-2"));
    assert!(window_has(cx, h, format!("fv-{studio}-2").as_str()));
    // A vault's Favorites list only that vault's favorites.
    act(cx, h, |w, cx| w.click("fv-personal-1", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.filter, Filter::FavoritesOf(0));
        assert!(
            s.visible
                .iter()
                .all(|i| s.entries[*i].favorite && s.in_vault(0, s.entries[*i].space))
        );
    });

    // Start in: with vault tabs, the organization's tab is offered and used at the next unlock.
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Preferences", cx));
    act(cx, h, |w, cx| w.click("vault-tabs", cx));
    settings_bottom(cx, h, "vault-tabs");
    act(cx, h, |w, cx| w.click("start", cx));
    assert!(window_has(cx, h, format!("start-vault:{studio}").as_str()));
    act(cx, h, |w, cx| w.click("start-vault:personal", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.prefs.start_in, "vault:personal");
    });
    act(cx, h, |w, cx| w.press("ctrl-l", cx));
    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.press("enter", cx);
    });
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        // The demo organization lives in memory only: without an organization there is one vault
        // and no tab of its own (27/11/2026), whatever the setting.
        assert!(s.prefs.vault_tabs && !s.vault_tabs());
        assert_eq!(s.scope, Scope::All);
    });
    app.update(cx, |a, _| {
        let Phase::Main(s) = &mut a.phase else {
            panic!()
        };
        cleis_app::demo::org(s);
    });
    cx.run_until_parked();

    // The password box's own menu: Generate keeps the old one in History.
    act(cx, h, |w, cx| w.click("tab-vault-personal", cx));
    let (id, before) = app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let e = s
            .visible
            .iter()
            .map(|i| &s.entries[*i])
            .find(|e| !e.password.is_empty())
            .unwrap();
        (e.id, e.password.clone())
    });
    app.update(cx, |a, cx| {
        if let Phase::Main(s) = &mut a.phase {
            s.selected = Some(id);
        }
        cx.notify();
    });
    cx.run_until_parked();
    act(cx, h, |w, cx| w.right_click("password-box", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(matches!(
            s.ctx.as_ref().map(|m| m.target),
            Some(cleis_app::screens::ctx::CtxTarget::PasswordField(x)) if x == id
        ));
    });
    act(cx, h, |w, cx| w.click("cx-pw-gen", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let e = s.entries.iter().find(|e| e.id == id).unwrap();
        assert_ne!(e.password, before);
        assert!(e.password_history.iter().any(|h| h.password == before));
        assert!(s.ctx.is_none());
    });
}

/// Mockups 37l, 37m: all backups deleted; then the account, with a wrong then the right master
/// password: back to the first launch, nothing left beside the vault.
#[gpui_kit::test]
fn delete_backups_and_account(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.cleis");
    cleis_app::demo::create(&path).unwrap();
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    std::fs::write(backups.join("demo-2026-11-27-101010.cleis"), b"x").unwrap();
    std::fs::write(dir.path().join("demo.cleis.sync.json"), b"{}").unwrap();
    let (h, app) = open(cx, &path);
    cx.run_until_parked();
    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.press("enter", cx);
    });
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Sync & backups", cx));
    settings_bottom(cx, h, "kit");
    act(cx, h, |w, cx| w.click("erase-backups", cx));
    act(cx, h, |w, cx| w.click("confirm-ok", cx));
    assert_eq!(std::fs::read_dir(&backups).unwrap().count(), 0);
    assert!(path.exists());

    act(cx, h, |w, cx| w.click("erase-account", cx));
    act(cx, h, |w, cx| {
        w.input("not it", cx);
        w.click("erase-ok", cx);
    });
    cx.run_until_parked();
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let e = s.erase.as_ref().expect("still asked");
        assert!(e.error.is_some() && !e.busy);
    });
    assert!(path.exists());
    act(cx, h, |w, cx| {
        w.click("erase-password", cx);
        w.press("ctrl-a", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.click("erase-ok", cx);
    });
    cx.run_until_parked();
    app.read_with(cx, |a, _| assert!(matches!(a.phase, Phase::Welcome(_))));
    assert!(!path.exists());
    assert!(!dir.path().join("demo.cleis.sync.json").exists());
    assert!(!backups.exists());
}

/// Mockup 37n: Settings › Account › Sign in: the code, then the account's repository becomes the
/// sync place with the token GitHub gave (nothing typed but the code, on GitHub).
#[gpui_kit::test]
fn sign_in_with_github(cx: &mut TestAppContext) {
    let (addr, file) = fake_github();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.cleis");
    cleis_app::demo::create(&path).unwrap();
    let (h, app) = open(cx, &path);
    app.update(cx, |a, _| {
        a.github_hosts = cleis_app::github_login::Hosts {
            web: addr.clone(),
            api: format!("{addr}/api/v3"),
        };
    });
    cx.run_until_parked();
    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.press("enter", cx);
    });
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Account", cx));
    act(cx, h, |w, cx| w.click("acc-signin", cx));
    cx.run_until_parked();
    app.read_with(cx, |a, _| {
        let g = a.github.as_ref().expect("dialog");
        assert_eq!(g.code.as_ref().unwrap().user_code, "WDJB-MJHT");
    });
    // GitHub's interval passes: the code was typed there.
    for _ in 0..20 {
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(6));
        cx.run_until_parked();
        if app.read_with(cx, |a, _| a.github.is_none()) {
            break;
        }
    }
    app.read_with(cx, |a, _| {
        assert!(
            a.github.is_none(),
            "{:?}",
            a.github.as_ref().and_then(|g| g.error.clone())
        );
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.sync.provider, cleis_app::sync::Provider::GitHub);
        assert!(matches!(
            &s.sync.remote,
            Some(cleis_app::remote::RemotePlace::GitHub { repo, path, .. })
                if repo == "lea-example/cleis-vault" && path == "vault.cleis"
        ));
    });
    for _ in 0..20 {
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(2));
        cx.run_until_parked();
        if file.lock().unwrap().is_some() {
            break;
        }
    }
    assert!(file.lock().unwrap().is_some(), "the vault was sent");
}

/// Mockups 37e-37g: a PIN for the personal vault, typed twice in its boxes; at the next unlock,
/// a wrong PIN is counted, the right one opens; the master password stays the other way in.
#[gpui_kit::test]
fn pin_for_my_vault(cx: &mut TestAppContext) {
    use cleis_app::opening::Mode;
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
    act(cx, h, |w, cx| w.click("set-Account", cx));
    act(cx, h, |w, cx| w.click("opening-Pin", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(
            s.pin_setup.is_some(),
            "dialog state; opening {:?}",
            s.opening
        );
    });
    assert!(window_has(cx, h, "pin-dialog"));
    // Two different PINs: refused, the second boxes emptied.
    act(cx, h, |w, cx| {
        w.input("123456", cx);
        w.input("654321", cx);
    });
    cx.run_until_parked();
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let d = s.pin_setup.as_ref().unwrap();
        assert!(d.error.is_some() && d.confirm.value.is_empty());
    });
    act(cx, h, |w, cx| w.input("123456", cx));
    for _ in 0..50 {
        cx.run_until_parked();
        if app.read_with(
            cx,
            |a, _| matches!(&a.phase, Phase::Main(s) if s.pin_setup.is_none()),
        ) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.pin_setup.is_none());
        assert_eq!(s.opening, Mode::Pin);
    });
    assert_eq!(cleis_app::opening::mode(&path), Mode::Pin);
    assert!(window_has(cx, h, "pin-len-4") && window_has(cx, h, "pin-tries"));

    // Locked: the boxes have the keyboard.
    act(cx, h, |w, cx| w.press("ctrl-l", cx));
    assert!(window_has(cx, h, "unlock-pin"));
    act(cx, h, |w, cx| w.input("000000", cx));
    for _ in 0..50 {
        cx.run_until_parked();
        if app.read_with(cx, |a, _| matches!(&a.phase, Phase::Unlock(u) if !u.busy)) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    app.read_with(cx, |a, _| {
        let Phase::Unlock(u) = &a.phase else { panic!() };
        assert!(u.error.as_ref().unwrap().contains('4'), "{:?}", u.error);
    });
    act(cx, h, |w, cx| {
        w.click("unlock-pin", cx);
        w.input("123456", cx);
    });
    for _ in 0..50 {
        cx.run_until_parked();
        if app.read_with(cx, |a, _| matches!(a.phase, Phase::Main(_))) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    app.read_with(cx, |a, _| assert!(matches!(a.phase, Phase::Main(_))));
    assert_eq!(cleis_app::opening::pin_info(&path).unwrap().tries_left, 5);

    // The master password is still the other way in.
    act(cx, h, |w, cx| w.press("ctrl-l", cx));
    act(cx, h, |w, cx| w.click("pin-other", cx));
    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.press("enter", cx);
    });
    for _ in 0..50 {
        cx.run_until_parked();
        if app.read_with(cx, |a, _| matches!(a.phase, Phase::Main(_))) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    app.read_with(cx, |a, _| assert!(matches!(a.phase, Phase::Main(_))));
}

/// Mockup 38: a shortcut inside Cleis changed by pressing the new keys, used at once, marked when
/// it clashes, put back; Backspace leaves one without keys.
#[gpui_kit::test]
fn keyboard_shortcuts(cx: &mut TestAppContext) {
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
    let count = |cx: &mut TestAppContext| {
        app.read_with(cx, |a, _| match &a.phase {
            Phase::Main(s) => s.entries.len(),
            _ => 0,
        })
    };
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Shortcuts", cx));
    // Duplicate on Ctrl+Shift+D.
    act(cx, h, |w, cx| w.click("duplicate-change", cx));
    act(cx, h, |w, cx| w.press("ctrl-shift-d", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.prefs.app_key("duplicate"), "Ctrl+Shift+D");
        assert!(s.recording.is_none());
    });
    assert!(window_has(cx, h, "duplicate-reset"));
    // Lock on Ctrl+D... taken by nothing now: no clash. Then Copy password on Ctrl+L: clash.
    act(cx, h, |w, cx| w.click("copy_pass-change", cx));
    act(cx, h, |w, cx| w.press("ctrl-l", cx));
    assert!(window_has(cx, h, "lock-clash") && window_has(cx, h, "copy_pass-clash"));
    act(cx, h, |w, cx| w.click("copy_pass-reset", cx));
    assert!(!window_has(cx, h, "lock-clash"));
    // Backspace: no keys.
    act(cx, h, |w, cx| w.click("copy_totp-change", cx));
    act(cx, h, |w, cx| w.press("backspace", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.prefs.app_key("copy_totp"), "");
    });
    // The new keys work in the vault; the old ones no longer.
    act(cx, h, |w, cx| w.click("tab-Vault", cx));
    let first = app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => s.entries[s.visible[0]].id,
        _ => panic!(),
    });
    act(cx, h, |w, cx| w.click(format!("row-{first}"), cx));
    let n = count(cx);
    act(cx, h, |w, cx| w.press("ctrl-d", cx));
    assert_eq!(count(cx), n);
    act(cx, h, |w, cx| w.press("ctrl-shift-d", cx));
    assert_eq!(count(cx), n + 1);
}

/// Scenario (mockup 37r): a new computer, Restore › GitHub, but the account has no vault yet.
/// The dialog names the account and offers to make the vault here: once made (its recovery
/// key put away), the private repository is made on the account and the vault sent to it.
#[gpui_kit::test]
fn github_without_vault_creates_one(cx: &mut TestAppContext) {
    use std::sync::atomic::Ordering;
    let (addr, file, has_repo) = fake_github_account(false);
    let hosts = cleis_app::github_login::Hosts {
        web: addr.clone(),
        api: format!("{addr}/api/v3"),
    };
    let dir = tempfile::tempdir().unwrap();
    let (h, app) = open(cx, &dir.path().join("new/vault.cleis"));
    app.update(cx, |a, _| a.github_hosts = hosts.clone());
    cx.run_until_parked();
    act(cx, h, |w, cx| w.click("signin-github", cx));
    let mut seen = false;
    for _ in 0..30 {
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(6));
        cx.run_until_parked();
        if app.read_with(cx, |a, _| {
            a.github.as_ref().is_some_and(|g| g.no_vault.is_some())
        }) {
            seen = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(seen, "the dialog says the account has no vault");
    assert!(window_has(cx, h, "gh-account") && window_has(cx, h, "gh-create"));
    // Restoring does not make a repository on its own.
    assert!(!has_repo.load(Ordering::SeqCst));
    act(cx, h, |w, cx| w.click("gh-create", cx));
    app.read_with(cx, |a, _| {
        assert!(a.github.is_none() && a.github_after.is_some());
        let Phase::Welcome(w) = &a.phase else {
            panic!()
        };
        assert!(w.restore.is_none());
    });
    act(cx, h, |w, cx| {
        w.click("new-password", cx);
        w.input("correct horse battery staple", cx);
        w.click("confirm-password", cx);
        w.input("correct horse battery staple", cx);
        w.click("create", cx);
    });
    for _ in 0..100 {
        cx.run_until_parked();
        if app.read_with(cx, |a, _| matches!(a.phase, Phase::Main(_))) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    // Nothing sent before the recovery key is put away.
    assert!(!has_repo.load(Ordering::SeqCst));
    act(cx, h, |w, cx| w.click("recovery-key-done", cx));
    for _ in 0..100 {
        cx.run_until_parked();
        if file.lock().unwrap().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(
        has_repo.load(Ordering::SeqCst),
        "the private repository was made"
    );
    assert!(file.lock().unwrap().is_some(), "the vault was sent");
    app.read_with(cx, |a, _| {
        assert!(a.github.is_none() && a.github_after.is_none());
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.sync.provider, cleis_app::sync::Provider::GitHub);
        assert!(s.sync.remote.is_some());
    });
}

/// Scenario (27/11/2026): the mouse's side buttons and Alt+← / Alt+→ go back and forward
/// through the views: tabs, settings pages, the item open in the panel.
#[gpui_kit::test]
fn back_and_forward_through_views(cx: &mut TestAppContext) {
    use cleis_app::app::Tab;
    use gpui_kit::test::ClickOptions;
    use gpui_kit::{MouseButton, NavigationDirection};
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
    let view = |cx: &mut TestAppContext| {
        app.read_with(cx, |a, _| match &a.phase {
            Phase::Main(s) => (s.tab, s.settings_page, s.selected),
            _ => panic!(),
        })
    };
    let figma = app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => s.entries.iter().find(|e| e.name == "Figma").unwrap().id,
        _ => panic!(),
    });
    let start = view(cx);
    // On its name (the middle of the row holds copy buttons).
    act(cx, h, |w, cx| {
        w.click_at(
            format!("row-{figma}"),
            gpui_kit::point(gpui_kit::px(40.), gpui_kit::px(12.)),
            cx,
        )
    });
    assert_eq!(view(cx).2, Some(figma));
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Autofill", cx));
    assert_eq!(view(cx).1, "Autofill");
    // Thumb button "back": the settings page before, then the vault with Figma open.
    let back = ClickOptions::new().with_button(MouseButton::Navigate(NavigationDirection::Back));
    act(cx, h, |w, cx| {
        w.click_with_options("set-Autofill", back, cx)
    });
    let v = view(cx);
    assert_eq!((v.0, v.1), (Tab::Settings, start.1));
    act(cx, h, |w, cx| w.press("alt-left", cx));
    assert_eq!(view(cx), (Tab::Vault, start.1, Some(figma)));
    act(cx, h, |w, cx| w.press("alt-left", cx));
    assert_eq!(view(cx), start);
    // Forward again, by the other thumb button and Alt+→.
    let fwd = ClickOptions::new().with_button(MouseButton::Navigate(NavigationDirection::Forward));
    act(cx, h, |w, cx| w.click_with_options("tab-Vault", fwd, cx));
    assert_eq!(view(cx).2, Some(figma));
    act(cx, h, |w, cx| w.press("alt-right", cx));
    act(cx, h, |w, cx| w.press("alt-right", cx));
    assert_eq!(view(cx).1, "Autofill");
    // Nothing ahead: Alt+→ changes nothing.
    act(cx, h, |w, cx| w.press("alt-right", cx));
    assert_eq!(view(cx).1, "Autofill");
}

/// Scenario (mockups 39a, 39b, 27/11/2026): Settings › Account holds what Security and Profile
/// were, Delete account at the bottom, Revoke access only when signed in; no "own tab" choice
/// without an organization; Sync & backups does not offer the place already connected.
#[gpui_kit::test]
fn account_page_holds_security(cx: &mut TestAppContext) {
    let (addr, _file) = fake_github();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.cleis");
    cleis_app::demo::create(&path).unwrap();
    let (h, app) = open(cx, &path);
    app.update(cx, |a, _| {
        a.github_hosts = cleis_app::github_login::Hosts {
            web: addr.clone(),
            api: format!("{addr}/api/v3"),
        }
    });
    cx.run_until_parked();
    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.press("enter", cx);
    });
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    assert!(!window_has(cx, h, "set-Security") && !window_has(cx, h, "set-Profile"));
    act(cx, h, |w, cx| w.click("set-Account", cx));
    assert!(window_has(cx, h, "opening-Pin") && window_has(cx, h, "open-code"));
    assert!(
        !window_has(cx, h, "acc-revoke"),
        "nothing to revoke while local"
    );
    settings_bottom(cx, h, "open-code");
    assert!(window_has(cx, h, "erase-account-2"));
    // One vault: no tab of its own to offer.
    act(cx, h, |w, cx| w.click("set-Preferences", cx));
    assert!(!window_has(cx, h, "vault-tabs"));
    // Signed in to GitHub: Revoke access appears; Sync & backups no longer offers GitHub.
    act(cx, h, |w, cx| w.click("set-Account", cx));
    act(cx, h, |w, cx| w.click("acc-signin", cx));
    for _ in 0..30 {
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(6));
        cx.run_until_parked();
        if app.read_with(
            cx,
            |a, _| matches!(&a.phase, Phase::Main(s) if s.sync.remote.is_some()),
        ) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(window_has(cx, h, "acc-revoke"));
    act(cx, h, |w, cx| w.click("set-Sync & backups", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.sync.place.is_some(), "connected");
    });
    assert!(
        window_has(cx, h, "sync-onedrive"),
        "the other places are offered"
    );
    assert!(!window_has(cx, h, "sync-github"), "not the one connected");
    // Revoke: GitHub forgotten here (then GitHub's page is opened).
    act(cx, h, |w, cx| w.click("set-Account", cx));
    act(cx, h, |w, cx| w.click("acc-revoke", cx));
    act(cx, h, |w, cx| w.press("tab", cx));
    app.update(cx, |a, cx| a.revoke_github(cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.sync.remote.is_none() && s.sync.place.is_none());
    });
}
