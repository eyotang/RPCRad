use super::*;
use gpui_kit::component::menu::PopupMenuItem;

// GPUI's built-in input menus only ship a subset of our supported languages.
fn input_menu(
    menu: gpui_kit::component::native_menu::NativeMenu,
    capabilities: gpui_kit::base::input::InputContextMenuCapabilities,
) -> gpui_kit::component::native_menu::NativeMenu {
    use gpui_kit::base::input::{Copy, Cut, GoToDefinition, Paste, SelectAll, ToggleCodeActions};
    let enabled = !capabilities.is_disabled();
    let editable = enabled && !capabilities.is_readonly();
    let menu = if capabilities.is_code_editor() {
        menu.menu_with_disabled(
            tr!("Go to definition"),
            !(enabled && capabilities.has_definition()),
            Box::new(GoToDefinition),
        )
        .menu_with_disabled(
            tr!("Show code actions"),
            !(editable && capabilities.has_code_actions()),
            Box::new(ToggleCodeActions),
        )
        .separator()
    } else {
        menu
    };
    menu.menu_with_disabled(
        tr!("Cut"),
        !(editable && capabilities.is_copyable()),
        Box::new(Cut),
    )
    .menu_with_disabled(tr!("Copy"), !capabilities.is_copyable(), Box::new(Copy))
    .menu_with_disabled(tr!("Paste"), !editable, Box::new(Paste))
    .separator()
    .menu(tr!("Select all"), Box::new(SelectAll))
}

pub(super) fn localized_input(state: &Entity<InputState>) -> Input {
    let target = state.clone();
    Input::new(state).context_menu(move |menu, _, cx| {
        input_menu(menu, target.read(cx).context_menu_capabilities())
    })
}
pub(super) fn localized_editor(state: &Entity<EditorState>) -> Editor {
    let target = state.clone();
    Editor::new(state).context_menu(move |menu, _, cx| {
        input_menu(menu, target.read(cx).context_menu_capabilities())
    })
}
pub(super) fn mask_button(state: &Entity<InputState>, cx: &App) -> Button {
    let masked = state.read(cx).presentation().is_masked();
    let target = state.clone();
    Button::new("toggle-mask")
        .icon(if masked {
            IconName::Eye
        } else {
            IconName::EyeOff
        })
        .accessibility_label(if masked {
            tr!("Show password")
        } else {
            tr!("Hide password")
        })
        .xsmall()
        .text()
        .tab_stop(false)
        .on_click(move |_, window, cx| {
            target.update(cx, |s, cx| {
                s.set_masked(!s.presentation().is_masked(), window, cx);
            })
        })
}

impl Workbench {
    pub(super) fn language_selector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.weak_entity();
        Button::new("language-selector")
            .ghost()
            .label(Language::current().label())
            .icon(IconName::ChevronDown)
            .accessibility_label(tr!("Language"))
            .tooltip(tr!("Language"))
            .dropdown_menu(move |mut menu, _, _| {
                for language in Language::ALL {
                    let target = weak.clone();
                    menu = menu.item(
                        PopupMenuItem::new(language.label())
                            .checked(Language::current() == language)
                            .on_click(move |_, window, cx| {
                                let _ = target.update(cx, |this, cx| {
                                    this.set_language(language, window, cx);
                                });
                            }),
                    );
                }
                menu
            })
    }

    pub fn language_smoke_test(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        let language = Language::current();
        let body = self.request.read(cx).value();
        let metadata = self.metadata.read(cx).value();
        let token = self.token.read(cx).value();
        let variables = serde_json::to_value(self.environments.snapshot(cx))?;
        let settings = serde_json::to_value(self.plugins.settings(cx))?;
        self.set_language(
            match language {
                Language::Chinese => Language::English,
                _ => Language::Chinese,
            },
            window,
            cx,
        );
        self.set_language(language, window, cx);
        anyhow::ensure!(
            self.request.read(cx).value() == body,
            "Language switch changed request data"
        );
        anyhow::ensure!(
            self.metadata.read(cx).value() == metadata,
            "Language switch changed metadata"
        );
        anyhow::ensure!(
            self.token.read(cx).value() == token,
            "Language switch changed authentication"
        );
        anyhow::ensure!(
            serde_json::to_value(self.environments.snapshot(cx))? == variables,
            "Language switch changed variables"
        );
        anyhow::ensure!(
            serde_json::to_value(self.plugins.settings(cx))? == settings,
            "Language switch changed plugin configuration"
        );
        let saved: Saved = serde_json::from_slice(&std::fs::read(state_path())?)?;
        anyhow::ensure!(
            saved.language == language,
            "Language preference was not saved"
        );
        println!("PASS: language switching preserves request data and saves the preference");
        Ok(())
    }

    fn set_language(&mut self, language: Language, window: &mut Window, cx: &mut Context<Self>) {
        if language == Language::current() {
            return;
        }
        language.activate();
        crate::install_menus(cx);
        self.search.update(cx, |s, cx| {
            s.set_placeholder(tr!("Search services or methods"), window, cx);
        });
        self.token.update(cx, |s, cx| {
            s.set_placeholder(tr!("Enter Bearer token"), window, cx);
        });
        self.environments.refresh_language(window, cx);
        self.plugins.refresh_language(window, cx);
        self.refresh_history(window, cx);
        self.notice = tr!("Interface language changed").into();
        self.save(cx);
        // Refresh update messages in the newly selected language.
        self.update_status = tr!("Checking for updates…").into();
        self.check_update(cx);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::Saved;
    use crate::i18n::Language;

    #[test]
    fn old_workspaces_keep_data_and_default_to_chinese() {
        let mut saved = serde_json::to_value(Saved::default()).unwrap();
        saved.as_object_mut().unwrap().remove("language");
        saved["endpoint"] = "{{endpoint}}".into();
        saved["drafts"] = serde_json::json!({"demo/Call": "{\"name\":\"中文 {{name}}\"}"});
        let mut restored: Saved = serde_json::from_value(saved.clone()).unwrap();
        assert_eq!(restored.language, Language::Chinese);
        restored.language = Language::English;
        let roundtrip = serde_json::to_value(&restored).unwrap();
        assert_eq!(roundtrip["language"], "en");
        assert_eq!(roundtrip["endpoint"], saved["endpoint"]);
        assert_eq!(roundtrip["drafts"], saved["drafts"]);
    }
}
