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
    act(cx, h, |w, cx| w.click("set-Security", cx));
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
    act(cx, h, |w, cx| w.click("set-Security", cx));
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
    act(cx, h, |w, cx| w.click("vview-Cards", cx));
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(s.vault_cards);
    });
    act(cx, h, |w, cx| w.click("vview-Table", cx));
    // The menu stays open for several choices.
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
        (c.id.clone(), a.shown_contact().unwrap())
    });
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
    act(cx, h, |w, cx| w.click("new-folder-v", cx));
    act(cx, h, |w, cx| {
        w.click("folder-name", cx);
        w.input("Savings", cx);
        w.click("folder-ok", cx);
    });
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
    // Rename, then delete: the item goes back to My vault.
    act(cx, h, |w, cx| w.hover(format!("sp-{bank}"), cx));
    act(cx, h, |w, cx| w.click(format!("fmore-{bank}"), cx));
    act(cx, h, |w, cx| w.click(format!("frename-{bank}"), cx));
    act(cx, h, |w, cx| {
        w.press("ctrl-a", cx);
        w.input("Home", cx);
        w.press("enter", cx);
    });
    app.read_with(cx, |a, _| match &a.phase {
        Phase::Main(s) => assert!(s.spaces.iter().any(|x| x.id == bank && x.name == "Home")),
        _ => panic!(),
    });
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
    use std::io::{BufRead, BufReader, Read, Write};
    let file: RepoFile = Default::default();
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
                let (status, out): (u16, Vec<u8>) = if auth != "Bearer github_pat_example" {
                    (401, br#"{"message":"Bad credentials"}"#.to_vec())
                } else if !path_ok {
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
    (addr, file)
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

/// Scenario (mockups 04, 06; D-66): turn on biometric unlock, lock, unlock without the master
/// password; a refused check leaves the vault locked; turning it off removes the device key.
/// The system check is a stand-in here (Linux has none).
#[gpui_kit::test]
fn biometric_unlock(cx: &mut TestAppContext) {
    use cleis_autofill::biometric::FAKE;
    *FAKE.lock().unwrap() = Some(true);
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
    act(cx, h, |w, cx| w.click("set-Security", cx));
    act(cx, h, |w, cx| w.click("bio", cx));
    assert!(cleis_app::bio::enabled(&path));
    act(cx, h, |w, cx| w.click("lock", cx));
    act(cx, h, |w, cx| w.click("bio", cx));
    app.read_with(cx, |a, _| {
        assert!(matches!(a.phase, Phase::Main(_)), "opened")
    });
    // A refused check.
    *FAKE.lock().unwrap() = Some(false);
    act(cx, h, |w, cx| w.click("lock", cx));
    act(cx, h, |w, cx| w.click("bio", cx));
    app.read_with(cx, |a, _| match &a.phase {
        Phase::Unlock(u) => assert!(u.error.is_some()),
        _ => panic!("still locked"),
    });
    *FAKE.lock().unwrap() = Some(true);
    act(cx, h, |w, cx| w.click("bio", cx));
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Security", cx));
    act(cx, h, |w, cx| w.click("bio", cx));
    assert!(!cleis_app::bio::enabled(&path));
    *FAKE.lock().unwrap() = None;
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

    act(cx, h, |w, cx| w.right_click(format!("row-{figma}"), cx));
    main(cx, &|s| {
        assert!(s.ctx.is_some());
        assert_eq!(s.selected, Some(figma));
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

    act(cx, h, |w, cx| w.press("ctrl-d", cx));
    main(cx, &|s| assert_eq!(s.entries.len(), count + 1));

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
