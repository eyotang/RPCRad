use super::*;
use gpui_kit::component::menu::PopupMenuItem;

impl Workbench {
    pub(super) fn language_selector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.weak_entity();
        Button::new("language-selector")
            .ghost()
            .label(Language::current().label())
            .icon(IconName::ChevronDown)
            .accessibility_label("语言 / Language")
            .tooltip("语言 / Language")
            .dropdown_menu(move |mut menu, _, _| {
                for language in [Language::Chinese, Language::English] {
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
                Language::English => Language::Chinese,
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
            s.set_placeholder(
                tr!("搜索服务或方法", "Search services or methods"),
                window,
                cx,
            );
        });
        self.token.update(cx, |s, cx| {
            s.set_placeholder(tr!("输入 Bearer token", "Enter Bearer token"), window, cx);
        });
        self.environments.refresh_language(window, cx);
        self.plugins.refresh_language(window, cx);
        self.refresh_history(window, cx);
        self.notice = tr!("界面语言已切换", "Interface language changed").into();
        self.save(cx);
        // Refresh update messages in the newly selected language.
        self.update_status = tr!("正在检查更新…", "Checking for updates…").into();
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
