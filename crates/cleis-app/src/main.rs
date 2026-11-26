//! `cleis [--vault PATH] [--theme dark|light] [--demo [--no-org]] [--unlocked] [--tab T]
//! [--page P] [--quick-access] [--menu columns|new-column|open-code|filter|invite|invite-code|
//! new-space|new-role|rekey|server-code] [--import FILE [--review]] [--try-autofill]
//! [--background] [--restore] [--org-welcome] [--lang fr] [--update VERSION [--menu update]]`

// Release builds on Windows are a GUI program: no console window behind Cleis.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

use cleis_app::app::CleisApp;
use cleis_app::theme::{self, Mode};
use gpui_kit::{AppContext, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};
use std::path::PathBuf;

fn default_vault() -> PathBuf {
    directories::ProjectDirs::from("com", "matv-studio", "Cleis")
        .map(|d| d.data_dir().join("vault.cleis"))
        .unwrap_or_else(|| PathBuf::from("vault.cleis"))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flag = |f: &str| args.iter().any(|a| a == f);
    let value = |f: &str| {
        args.iter()
            .position(|a| a == f)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let forced_mode = match value("--theme").as_deref() {
        Some("light") => Some(Mode::Light),
        Some("dark") => Some(Mode::Dark),
        _ => None,
    };
    let mut path = value("--vault")
        .map(PathBuf::from)
        .unwrap_or_else(default_vault);
    // `--try-autofill`: demo vault with real shortcuts and fills, plus a sample item for the test
    // form `design/tools/autofill-form.hta` (Windows).
    let try_autofill = flag("--try-autofill");
    let demo = flag("--demo") || try_autofill;
    if demo {
        path = std::env::temp_dir().join(format!("cleis-demo-{}.cleis", std::process::id()));
        let made = cleis_app::demo::create(&path).and_then(|()| {
            if try_autofill {
                cleis_app::demo::add_autofill_sample(&path)
            } else {
                Ok(())
            }
        });
        if let Err(e) = made {
            eprintln!("cleis: demo vault: {e}");
            std::process::exit(1);
        }
    }
    if try_autofill {
        // Steps and field kinds of each fill, never values, next to the program.
        let log = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("cleis-autofill.log")));
        cleis_autofill::set_log_file(log);
    }
    let unlocked = flag("--unlocked") && demo;
    let tab = value("--tab");
    let page = value("--page");
    let quick_access = flag("--quick-access") && unlocked;
    let menu = value("--menu").filter(|_| unlocked);
    // Screenshots of the unlock screen as Windows Hello or "nothing" shows it (D-76).
    let unlock_with = value("--unlock-with").filter(|_| demo && !unlocked);
    // Screenshots: a newer version shown as published (`--update 0.2.1`).
    let fake_update = value("--update");
    let import = value("--import").filter(|_| unlocked);
    let review = flag("--review");
    let background = flag("--background");
    // D-75: one Cleis per user. The installer's `--quit` closes the running copy; another launch
    // brings it in front (not at login: it is already there). The demo and the screenshots run
    // beside it.
    use cleis_app::instance::{self, Request};
    if flag("--quit") {
        if instance::ask(Request::Quit) {
            instance::wait_gone(std::time::Duration::from_secs(15));
        }
        return;
    }
    let single = if demo {
        None
    } else {
        match instance::claim() {
            Ok(Some(g)) => Some(g),
            Ok(None) => {
                if !background {
                    instance::ask(Request::Show);
                }
                return;
            }
            // The lock cannot be taken (unusual profile): Cleis runs anyway.
            Err(e) => {
                eprintln!("cleis: single instance: {e}");
                None
            }
        }
    };
    let restore = flag("--restore");
    let no_org = flag("--no-org");
    // D-69 captures: a second organization, the vaults in their own tabs.
    let two_orgs = flag("--two-orgs");
    let vault_tab = value("--vault-tab");
    // U-9 captures: `--lang fr`.
    let lang = value("--lang");
    let org_welcome = flag("--org-welcome");
    gpui_kit::platform::application()
        .with_assets(cleis_app::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cleis_app::assets::load_fonts(cx);
            let mut appearance = cleis_app::app::load_appearance();
            if let Some(l) = &lang {
                cleis_app::i18n::set_language(l);
            }
            if let Some(m) = forced_mode {
                appearance.mode = m;
            }
            theme::apply(&appearance, cx);
            let bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
            let opts = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                // Cleis draws its title bar (gpui-component `TitleBar`): the top bar of the
                // charter, with the system window buttons.
                titlebar: Some(TitlebarOptions {
                    title: Some("Cleis".into()),
                    traffic_light_position: Some(gpui_kit::point(px(14.), px(16.))),
                    ..gpui_kit::component::TitleBar::title_bar_options()
                }),
                app_owns_titlebar_drag: true,
                window_min_size: Some(size(px(960.), px(600.))),
                // Started at login: only the tray icon until the user opens Cleis.
                show: !background,
                ..Default::default()
            };
            let path = path.clone();
            let (tab, page, menu, import) =
                (tab.clone(), page.clone(), menu.clone(), import.clone());
            gpui_kit::open_window(opts, cx, move |window, cx| {
                let view = cx.new(|cx| {
                    let mut app = CleisApp::new(path.clone(), forced_mode, window, cx);
                    app.demo = demo;
                    // D-74: GitHub looked at for a newer version (never in the demo).
                    app.update.enabled = !demo;
                    if let Some(v) = &fake_update {
                        app.update.available = Some(cleis_app::update::Release {
                            version: v.clone(),
                            prerelease: true,
                            page: String::new(),
                            package: Some((String::new(), String::new())),
                        });
                        app.update.confirm = menu.as_deref() == Some("update");
                    }
                    if demo && !try_autofill {
                        // Demo and screenshots: never take the system's shortcuts or type.
                        app.no_shortcuts = true;
                        app.autofill.dry_run = Some(vec![]);
                    }
                    if let (Some(m), cleis_app::app::Phase::Unlock(u)) =
                        (unlock_with.as_deref(), &mut app.phase)
                    {
                        u.mode = match m {
                            "nothing" => cleis_app::opening::Mode::Nothing,
                            _ => cleis_app::opening::Mode::Hello,
                        };
                    }
                    if unlocked
                        && let Ok(v) = cleis_store::LockedVault::open(&path)
                            .and_then(|l| l.unlock(cleis_app::demo::PASSWORD))
                    {
                        app.enter(v, window, cx);
                        if let Some(s) = app.session() {
                            if !no_org {
                                cleis_app::demo::org(s);
                            }
                            // Screenshots: the first item open in the panel on the right.
                            if s.selected.is_none() {
                                s.selected = s.visible.first().map(|i| s.entries[*i].id);
                            }
                            if let Some(l) = &lang {
                                s.prefs.language = l.clone();
                                cleis_app::i18n::set_language(l);
                            }
                            if two_orgs {
                                cleis_app::demo::second_org(s);
                                s.prefs.vault_tabs = true;
                                s.scope = match vault_tab.as_deref() {
                                    Some("personal") => cleis_app::app::Scope::Personal,
                                    _ => s
                                        .org
                                        .as_ref()
                                        .map_or(cleis_app::app::Scope::Personal, |o| {
                                            cleis_app::app::Scope::Org(o.m.key())
                                        }),
                                };
                                s.refilter();
                            }
                            s.tab = match tab.as_deref() {
                                Some("contacts") => cleis_app::app::Tab::Contacts,
                                Some("health") => cleis_app::app::Tab::Health,
                                Some("activity") => cleis_app::app::Tab::Activity,
                                Some("settings") => cleis_app::app::Tab::Settings,
                                _ => cleis_app::app::Tab::Vault,
                            };
                            if page.as_deref() == Some("merge") {
                                s.contacts_ui.filter =
                                    cleis_app::screens::contacts::ContactFilter::Merge;
                            } else if let Some(p) = page.as_deref() {
                                s.settings_page = match p {
                                    "autofill" => "Autofill",
                                    "sync" | "org-backups" => "Sync & backups",
                                    "security" => "Security",
                                    "import" => "Import & export",
                                    "members" => "Members",
                                    "policy" => "Security policy",
                                    "encryption" => "Encryption",
                                    "server" | "server-here" => "Server",
                                    "join" => "Organization",
                                    "profile" => "Profile",
                                    "devices" => "Devices",
                                    "spaces" => "Spaces",
                                    _ => "Preferences",
                                };
                                if p == "org-backups"
                                    && let Some(o) = s.org.as_mut()
                                {
                                    o.ui.org_backups = true;
                                }
                                if p == "server-here"
                                    && let Some(o) = s.org.as_mut()
                                {
                                    // Screen 24b: the server runs on this computer.
                                    o.local.host_dir = Some("server".into());
                                    o.ui.host = Some((
                                        true,
                                        vec!["192.168.1.24:8443".into(), "10.8.0.3:8443".into()],
                                    ));
                                }
                            }
                        }
                    }
                    if restore {
                        // Screen 22 for screenshots.
                        app.start_restore(window, cx);
                    }
                    if let Some(f) = import.as_deref() {
                        // Screen 14 for screenshots: `--import FILE [--review]`.
                        cleis_app::screens::import::preload(&mut app, f.into(), cx);
                        if review && let Some(s) = app.session() {
                            s.import.step = 3;
                        }
                    }
                    if let Some(m) = menu.as_deref() {
                        cleis_app::screens::org::demo_dialog(&mut app, m, window, cx);
                    }
                    if org_welcome {
                        app.demo = true;
                        // Screen 03 with the server option chosen, for screenshots.
                        cleis_app::screens::org::demo_welcome(&mut app, window, cx);
                    }
                    if menu.as_deref() == Some("filter") {
                        // Screen 01b for screenshots.
                        if let Some(s) = app.session() {
                            s.filter_menu = true;
                            s.vault_filter.subs.insert(cleis_core::Subscription::Active);
                            s.refilter();
                        }
                    } else if menu.as_deref() == Some("open-code") {
                        // Screen 06 dialog for screenshots.
                        app.toggle_open_code(window, cx);
                    } else if let Some(m) = menu
                        .as_deref()
                        .filter(|m| matches!(*m, "columns" | "new-column"))
                    {
                        // Screen 02 for screenshots.
                        if let Some(s) = app.session() {
                            s.selected = None;
                        }
                        app.toggle_columns_menu(cleis_core::column::Table::Vault, cx);
                        if m == "new-column" {
                            app.start_new_column(window, cx);
                            app.demo_new_column(window, cx);
                        }
                    }
                    if quick_access {
                        // Screen 17 for screenshots: as if pressed over a desktop app.
                        app.open_quick_access(
                            cleis_autofill::Foreground {
                                window: cleis_autofill::WindowRef(0),
                                monitor: None,
                                context: cleis_core::autofill::Context {
                                    program: "figma.exe".into(),
                                    app_name: "Figma".into(),
                                    title: "Figma".into(),
                                    url: None,
                                },
                            },
                            cx,
                        );
                    }
                    if !(demo && !try_autofill) {
                        app.install_tray(cx);
                    }
                    if let Some(g) = single {
                        app.serve_instance(g, cx);
                    }
                    app
                });
                let weak = view.downgrade();
                window.on_window_should_close(cx, move |window, cx| {
                    weak.update(cx, |app, cx| app.on_main_close(window, cx))
                        .unwrap_or(true)
                });
                view
            })
            .expect("open window");
            if !background {
                cx.activate(true);
            }
        });
}
