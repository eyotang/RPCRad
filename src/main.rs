#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
extern crate gpui_kit as gpui;
#[macro_use]
mod i18n;
mod assets;
mod paths;
mod plugin;
mod rpc;
mod ui;
mod updater;
mod variables;
use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::*;

fn main() {
    let language =
        std::env::args().find_map(|arg| arg.strip_prefix("--language=").map(str::to_owned));
    let language = language.map(|code| match code.as_str() {
        "zh-CN" => i18n::Language::Chinese,
        "en" => i18n::Language::English,
        _ => {
            eprintln!("Unsupported language: {code}. Use --language=zh-CN or --language=en.");
            std::process::exit(2);
        }
    });
    language.unwrap_or_default().activate();
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
            Theme::change(ThemeMode::Light, None, cx);
            let theme = Theme::global_mut(cx);
            theme.colors.primary = rgb(0x128252).into();
            theme.colors.primary_hover = rgb(0x0c7045).into();
            theme.colors.primary_active = rgb(0x095c38).into();
            theme.colors.ring = rgb(0x128252).into();
            theme.tokens.button_primary = Hsla::from(rgb(0x128252)).into();
            theme.tokens.button_primary_hover = Hsla::from(rgb(0x0c7045)).into();
            theme.tokens.button_primary_active = Hsla::from(rgb(0x095c38)).into();
            theme.font_size = px(13.);
            theme.mono_font_size = px(13.);
            theme.radius = px(6.);
            theme.shadow = false;
            cx.bind_keys([
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
                            workbench.update(cx, |this, cx| this.language_smoke_test(window, cx))
                        });
                        if let Err(error) = checked.and_then(|result| result) {
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
                        let _ = about.update(cx, |view, cx| view.show_about(false, cx));
                        let _ = window.update(cx, |_, window, _| window.activate_window());
                    });
                    let updates = workbench.downgrade();
                    cx.on_action(move |_: &ui::CheckUpdates, cx| {
                        let _ = updates.update(cx, |view, cx| view.show_about(true, cx));
                        let _ = window.update(cx, |_, window, _| window.activate_window());
                    });
                    cx.activate(true);
                }
                Err(error) => {
                    eprintln!(
                        "{}",
                        tr_format!("无法打开 RPCRad: {error}", "Cannot open RPCRad: {error}")
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
        MenuItem::action(tr!("关于 RPCRad…", "About RPCRad…"), ui::About),
        MenuItem::action(tr!("检查更新…", "Check for updates…"), ui::CheckUpdates),
        MenuItem::separator(),
        MenuItem::action(tr!("退出 RPCRad", "Quit RPCRad"), ui::Quit),
    ])]);
}
