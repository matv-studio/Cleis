//! `cleis [--vault PATH] [--theme dark|light] [--demo] [--unlocked]`

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
    if flag("--demo") {
        path = std::env::temp_dir().join(format!("cleis-demo-{}.cleis", std::process::id()));
        if let Err(e) = cleis_app::demo::create(&path) {
            eprintln!("cleis: demo vault: {e}");
            std::process::exit(1);
        }
    }
    let unlocked = flag("--unlocked") && flag("--demo");
    let tab = value("--tab");
    let page = value("--page");
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
                titlebar: Some(TitlebarOptions {
                    title: Some("Cleis".into()),
                    ..Default::default()
                }),
                window_min_size: Some(size(px(960.), px(600.))),
                ..Default::default()
            };
            let path = path.clone();
            let (tab, page) = (tab.clone(), page.clone());
            gpui_kit::open_window(opts, cx, move |window, cx| {
                cx.new(|cx| {
                    let mut app = CleisApp::new(path.clone(), forced_mode, window, cx);
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
                            if let Some(p) = page.as_deref() {
                                s.settings_page = match p {
                                    "autofill" => "Autofill",
                                    "security" => "Security",
                                    _ => "Preferences",
                                };
                            }
                        }
                    }
                    app
                })
            })
            .expect("open window");
            cx.activate(true);
        });
}
