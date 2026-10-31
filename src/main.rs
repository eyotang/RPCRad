#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
extern crate gpui_kit as gpui;
#[macro_use]
mod i18n;
mod assets;
mod paths;
mod plugin;
mod rpc;
mod theme;
mod ui;
mod updater;
mod variables;
use gpui_kit::*;

fn main() {
    let language =
        std::env::args().find_map(|arg| arg.strip_prefix("--language=").map(str::to_owned));
    let language = language.map(|code| {
        i18n::LanguagePreference::from_code(&code).unwrap_or_else(|| {
            eprintln!("Unsupported language: {code}. Use auto, zh-CN, en, ja, ko, fr, de or ru.");
            std::process::exit(2);
        })
    });
    language.unwrap_or_default().resolve().activate();
    if std::env::args().any(|arg| arg == "--check-update") {
        match updater::check() {
            Ok(Some(release)) => println!("Update available: {}", release.version),
            Ok(None) => println!("Up to date: {}", env!("CARGO_PKG_VERSION")),
            Err(error) => {
                eprintln!("{error:#}");
                std::process::exit(1);
            }
        }
        return;
    }
    if std::env::args().any(|arg| arg == "--plugin-smoke-test") {
        if let Err(error) = ui::plugin_smoke_test() {
            eprintln!("{error:#}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|arg| arg == "--smoke-test") {
        if let Err(error) = ui::smoke_test() {
            eprintln!("{error:#}");
            std::process::exit(1);
        }
        return;
    }
    let ui_smoke = std::env::args().any(|arg| arg == "--ui-smoke-test");
    gpui_kit::application()
        .with_assets(assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.set_app_identity("dev.rpcrad.desktop", "RPCRad");
            cx.bind_keys([
                KeyBinding::new(ui::platform_text("cmd-,", "ctrl-,"), ui::OpenSettings, None),
                KeyBinding::new("escape", ui::CloseUtility, Some("UtilityWindow")),
                KeyBinding::new(
                    ui::platform_text("cmd-w", "ctrl-w"),
                    ui::CloseUtility,
                    Some("UtilityWindow"),
                ),
                KeyBinding::new(ui::platform_text("cmd-q", "ctrl-q"), ui::Quit, None),
                KeyBinding::new(ui::platform_text("cmd-enter", "ctrl-enter"), ui::Send, None),
                KeyBinding::new(
                    ui::platform_text("cmd-k", "ctrl-k"),
                    ui::SearchServices,
                    None,
                ),
                KeyBinding::new(
                    ui::platform_text("cmd-s", "ctrl-s"),
                    ui::SaveWorkspace,
                    None,
                ),
            ]);
            cx.on_action(|_: &ui::Quit, cx| cx.quit());
            let options = WindowOptions {
                app_id: Some("dev.rpcrad.desktop".into()),
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1440.), px(900.)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some("RPCRad".into()),
                    appears_transparent: false,
                    ..Default::default()
                }),
                window_min_size: Some(size(px(1080.), px(680.))),
                ..Default::default()
            };
            match gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| ui::Workbench::new(language, window, cx))
            }) {
                Ok((window, workbench)) => {
                    if ui_smoke {
                        let checked = window.update(cx, |_, window, cx| {
                            workbench.update(cx, |this, cx| {
                                this.language_smoke_test(window, cx)?;
                                this.settings_smoke_test(window, cx)
                            })
                        });
                        if let Err(error) = checked
                            .and_then(|result| result)
                            .and_then(|_| ui::window_smoke_test(&workbench, cx))
                        {
                            eprintln!("{error:#}");
                            std::process::exit(1);
                        }
                        let _ = window.update(cx, |_, window, _| {
                            window.on_next_frame(|window, _| {
                                window.on_next_frame(|_, cx| {
                                    if let Some(path) = std::env::var_os("RPCRAD_UI_SMOKE_MARKER")
                                        && let Err(error) = std::fs::write(
                                            path,
                                            format!(
                                                "native frames rendered: {}",
                                                i18n::Language::current().label()
                                            ),
                                        )
                                    {
                                        eprintln!("{error}");
                                        std::process::exit(1);
                                    }
                                    println!("PASS: native window rendered");
                                    cx.quit();
                                })
                            });
                        });
                    }
                    let about = workbench.downgrade();
                    cx.on_action(move |_: &ui::About, cx| {
                        let _ = about.update(cx, |view, cx| view.open_about(cx));
                    });
                    let updates = workbench.downgrade();
                    cx.on_action(move |_: &ui::CheckUpdates, cx| {
                        let _ = updates.update(cx, |view, cx| view.open_updates(cx));
                    });
                    let settings = workbench.downgrade();
                    cx.on_action(move |_: &ui::OpenSettings, cx| {
                        let _ = settings.update(cx, |view, cx| view.open_settings(cx));
                    });
                    cx.activate(true);
                }
                Err(error) => {
                    eprintln!(
                        "{}",
                        tr_format!("Cannot open RPCRad: {error}", error = error)
                    );
                    if ui_smoke {
                        std::process::exit(1);
                    }
                    cx.quit();
                }
            }
        });
}

fn install_menus(_cx: &mut App) {
    #[cfg(target_os = "macos")]
    _cx.set_menus([Menu::new("RPCRad").items([
        MenuItem::action(tr!("About RPCRad…"), ui::About),
        MenuItem::action(tr!("Settings…"), ui::OpenSettings),
        MenuItem::action(tr!("Check for updates…"), ui::CheckUpdates),
        MenuItem::separator(),
        MenuItem::action(tr!("Quit RPCRad"), ui::Quit),
    ])]);
}
