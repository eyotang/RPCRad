use super::*;
use gpui_kit::component::menu::PopupMenuItem;

#[derive(Clone, Copy)]
pub(super) enum UtilityKind {
    Settings,
    Updates,
    About,
}

impl UtilityKind {
    fn title(self) -> &'static str {
        match self {
            Self::Settings => tr!("Settings"),
            Self::Updates => tr!("Check for updates"),
            Self::About => tr!("About RPCRad"),
        }
    }
}

struct UtilityWindow {
    kind: UtilityKind,
    workbench: WeakEntity<Workbench>,
    main_window: AnyWindowHandle,
    focus: FocusHandle,
    _subscription: Subscription,
}

impl Workbench {
    pub fn settings_smoke_test(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        let original = self.appearance;
        if original != Appearance::System {
            anyhow::ensure!(
                gpui_kit::component::Theme::global(cx).mode == original.mode(window.appearance()),
                "Saved theme was not restored at startup"
            );
        }
        let body = self.request.read(cx).value();
        for appearance in Appearance::ALL {
            self.set_appearance(appearance, window, cx);
            let saved: Saved = serde_json::from_slice(&std::fs::read(state_path())?)?;
            anyhow::ensure!(
                saved.appearance == appearance,
                "Theme preference was not saved"
            );
            if appearance != Appearance::System {
                anyhow::ensure!(
                    gpui_kit::component::Theme::global(cx).mode
                        == appearance.mode(window.appearance()),
                    "Theme did not change"
                );
            }
        }
        self.set_appearance(original, window, cx);
        anyhow::ensure!(
            self.request.read(cx).value() == body,
            "Theme changed request data"
        );
        println!("PASS: theme persistence and request preservation");
        Ok(())
    }

    pub fn open_settings(&mut self, cx: &mut Context<Self>) {
        self.open_utility(UtilityKind::Settings, cx);
    }
    pub fn open_updates(&mut self, cx: &mut Context<Self>) {
        self.check_update(cx);
        self.open_utility(UtilityKind::Updates, cx);
    }
    pub fn open_about(&mut self, cx: &mut Context<Self>) {
        self.open_utility(UtilityKind::About, cx);
    }

    pub(super) fn open_utility(&mut self, kind: UtilityKind, cx: &mut Context<Self>) {
        let target = cx.weak_entity();
        // Opening a native window renders it immediately. Release the workbench borrow first.
        cx.defer(move |cx| {
            if let Some(workbench) = target.upgrade()
                && let Err(error) = open_window(&workbench, kind, cx)
            {
                workbench.update(cx, |view, cx| {
                    view.notice = tr_format!("Cannot open RPCRad: {error}", error = error);
                    cx.notify();
                });
            }
        });
    }

    pub(super) fn set_appearance(
        &mut self,
        appearance: Appearance,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.appearance = appearance;
        appearance.apply(window, cx);
        self.save(cx);
        cx.notify();
    }
}

impl UtilityWindow {
    fn settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let workbench = self.workbench.clone();
        let main_window = self.main_window;
        let current = workbench
            .upgrade()
            .map(|view| view.read(cx).appearance)
            .unwrap_or_default();
        let languages = Button::new("language-selector")
            .w(px(180.))
            .flex_shrink_0()
            .label(Language::current().label())
            .icon(IconName::ChevronDown)
            .accessibility_label(tr!("Language"))
            .dropdown_menu(move |mut menu, _, _| {
                for language in Language::ALL {
                    let target = workbench.clone();
                    menu = menu.item(
                        PopupMenuItem::new(language.label())
                            .checked(Language::current() == language)
                            .on_click(move |_, _, cx| {
                                // Input entities belong to the request window, not this settings window.
                                let _ = main_window.update(cx, |_, window, cx| {
                                    let _ = target.update(cx, |view, cx| {
                                        view.set_language(language, window, cx)
                                    });
                                });
                            }),
                    );
                }
                menu
            });
        let mut themes = row().flex_wrap().gap_2();
        for (index, appearance) in Appearance::ALL.into_iter().enumerate() {
            let target = self.workbench.clone();
            themes = themes.child(
                Button::new(("appearance", index))
                    .label(appearance.label())
                    .when(current == appearance, |button| button.primary())
                    .on_click(move |_, _, cx| {
                        let _ = main_window.update(cx, |_, window, cx| {
                            let _ = target
                                .update(cx, |view, cx| view.set_appearance(appearance, window, cx));
                        });
                    }),
            );
        }
        column()
            .gap_6()
            .child(
                column()
                    .gap_3()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(tr!("Language")),
                    )
                    .child(row().child(languages)),
            )
            .child(
                column()
                    .gap_3()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(tr!("Appearance")),
                    )
                    .child(themes),
            )
            .child(div().text_sm().text_color(color(MUTED, cx)).child(tr!(
                "Changes apply immediately and are saved automatically."
            )))
    }
}

impl Render for UtilityWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_window_title(self.kind.title());
        let content = match self.kind {
            UtilityKind::Settings => self.settings(cx).into_any_element(),
            UtilityKind::Updates => self
                .workbench
                .update(cx, |view, cx| view.update_panel(cx).into_any_element())
                .unwrap_or_else(|_| div().into_any_element()),
            UtilityKind::About => column()
                .gap_4()
                .child(img("brand/logo.png").size(px(64.)))
                .child(
                    div()
                        .text_size(px(24.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("RPCRad"),
                )
                .child(tr_format!("Version {}", env!("CARGO_PKG_VERSION")))
                .child(
                    div()
                        .text_color(color(MUTED, cx))
                        .child(tr!("Local gRPC workbench")),
                )
                .child(
                    Button::new("about-repository")
                        .label(tr!("Project homepage"))
                        .on_click(|_, _, cx| cx.open_url("https://github.com/eyotang/RPCRad")),
                )
                .into_any_element(),
        };
        column()
            .size_full()
            .bg(color(0xffffff, cx))
            .text_color(color(INK, cx))
            .text_size(px(13.))
            .key_context("UtilityWindow")
            .track_focus(&self.focus)
            .on_action(|_: &CloseUtility, window, _| window.remove_window())
            .child(
                row()
                    .h(px(64.))
                    .px_6()
                    .border_b_1()
                    .border_color(color(BORDER, cx))
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.kind.title()),
                    ),
            )
            .child(
                column()
                    .id("utility-content")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_6()
                    .child(content),
            )
            .child(
                row()
                    .px_6()
                    .py_3()
                    .justify_end()
                    .border_t_1()
                    .border_color(color(BORDER, cx))
                    .child(
                        Button::new("close-utility")
                            .label(tr!("Close"))
                            .on_click(|_, window, _| window.remove_window()),
                    ),
            )
    }
}

fn open_window(
    workbench: &Entity<Workbench>,
    kind: UtilityKind,
    cx: &mut App,
) -> anyhow::Result<AnyWindowHandle> {
    let existing = workbench.read(cx).utility_windows[kind as usize];
    if let Some(handle) = existing
        && handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
    {
        return Ok(handle);
    }
    let main_window = workbench.read(cx).main_window;
    let options = WindowOptions {
        app_id: Some("dev.rpcrad.desktop".into()),
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(600.), px(460.)),
            cx,
        ))),
        window_min_size: Some(size(px(520.), px(360.))),
        titlebar: Some(TitlebarOptions {
            title: Some(kind.title().into()),
            ..Default::default()
        }),
        is_minimizable: false,
        ..Default::default()
    };
    let (handle, _) = gpui_kit::open_window(options, cx, |window, cx| {
        cx.new(|cx| {
            let focus = cx.focus_handle();
            window.focus(&focus, cx);
            UtilityWindow {
                kind,
                main_window,
                workbench: workbench.downgrade(),
                focus,
                _subscription: cx.observe(workbench, |_, _, cx| cx.notify()),
            }
        })
    })?;
    workbench.update(cx, |view, _| {
        view.utility_windows[kind as usize] = Some(handle)
    });
    Ok(handle)
}

pub fn window_smoke_test(workbench: &Entity<Workbench>, cx: &mut App) -> anyhow::Result<()> {
    for kind in [
        UtilityKind::Settings,
        UtilityKind::Updates,
        UtilityKind::About,
    ] {
        let first = open_window(workbench, kind, cx)?;
        anyhow::ensure!(
            first.window_id() != workbench.read(cx).main_window.window_id(),
            "Utility is not a separate window"
        );
        anyhow::ensure!(
            open_window(workbench, kind, cx)?.window_id() == first.window_id(),
            "Duplicate utility window"
        );
        first.update(cx, |_, window, _| window.remove_window())?;
        anyhow::ensure!(
            open_window(workbench, kind, cx)?.window_id() != first.window_id(),
            "Closed utility did not reopen"
        );
    }
    println!("PASS: independent utility window rendering, reuse and reopening");
    Ok(())
}
