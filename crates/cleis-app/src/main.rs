//! `cleis [--vault PATH] [--theme dark|light] [--demo] [--unlocked] [--tab T] [--page P]
//! [--quick-access] [--menu columns|new-column|open-code]
//! [--import FILE [--review]] [--try-autofill] [--background]`

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
    let import = value("--import").filter(|_| unlocked);
    let review = flag("--review");
    let background = flag("--background");
    gpui_kit::platform::application()
        .with_assets(cleis_app::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cleis_app::assets::load_fonts(cx);
            let mut appearance = cleis_app::app::load_appearance();
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
                    if demo && !try_autofill {
                        // Demo and screenshots: never take the system's shortcuts or type.
                        app.no_shortcuts = true;
                        app.autofill.dry_run = Some(vec![]);
                    }
                    if unlocked
                        && let Ok(v) = cleis_store::LockedVault::open(&path)
                            .and_then(|l| l.unlock(cleis_app::demo::PASSWORD))
                    {
                        app.enter(v, window, cx);
                        if let Some(s) = app.session() {
                            s.tab = match tab.as_deref() {
                                Some("contacts") => cleis_app::app::Tab::Contacts,
                                Some("health") => cleis_app::app::Tab::Health,
                                Some("settings") => cleis_app::app::Tab::Settings,
                                _ => cleis_app::app::Tab::Vault,
                            };
                            if page.as_deref() == Some("merge") {
                                s.contacts_ui.filter =
                                    cleis_app::screens::contacts::ContactFilter::Merge;
                            } else if let Some(p) = page.as_deref() {
                                s.settings_page = match p {
                                    "autofill" => "Autofill",
                                    "sync" => "Sync & backups",
                                    "security" => "Security",
                                    "import" => "Import & export",
                                    _ => "Preferences",
                                };
                            }
                        }
                    }
                    if let Some(f) = import.as_deref() {
                        // Screen 14 for screenshots: `--import FILE [--review]`.
                        cleis_app::screens::import::preload(&mut app, f.into(), cx);
                        if review && let Some(s) = app.session() {
                            s.import.step = 3;
                        }
                    }
                    if menu.as_deref() == Some("open-code") {
                        // Screen 06 dialog for screenshots.
                        app.toggle_open_code(window, cx);
                    } else if let Some(m) = menu.as_deref() {
                        // Screen 02 for screenshots.
                        if let Some(s) = app.session() {
                            s.selected = None;
                        }
                        app.toggle_columns_menu(cx);
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
