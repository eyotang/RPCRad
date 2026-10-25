use super::*;
use crate::plugin::{self, Execution, Plugin, Settings};
use zeroize::Zeroizing;

pub(super) struct PluginUi {
    catalog: Vec<Plugin>,
    errors: Vec<String>,
    settings: Settings,
    fields: Vec<(String, Entity<InputState>)>,
    secret: Entity<InputState>,
    subscriptions: Vec<Subscription>,
    preview: Option<Result<plugin::Headers, String>>,
    preview_receiver: Option<Receiver<Result<plugin::Headers, String>>>,
    preview_cancel: Arc<AtomicBool>,
    revealed: BTreeSet<String>,
    catalog_receiver: Option<Receiver<(Vec<Plugin>, Vec<String>)>>,
}
impl PluginUi {
    pub(super) fn new(
        settings: Settings,
        window: &mut Window,
        cx: &mut Context<Workbench>,
    ) -> Self {
        let (catalog, errors) = plugin::builtin_catalog();
        let secret = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("secret · 仅本次会话")
                .masked(true)
        });
        let mut state = Self {
            catalog,
            errors,
            settings,
            fields: vec![],
            secret,
            subscriptions: vec![],
            preview: None,
            preview_receiver: None,
            preview_cancel: Arc::new(AtomicBool::new(false)),
            revealed: BTreeSet::new(),
            catalog_receiver: None,
        };
        if state.settings.selected.is_empty() {
            state.settings.selected = "md5-appid-ts".into();
        }
        state.load_fields(window, cx);
        if let Some(path) = state.settings.directory.clone() {
            state.reload(path);
        }
        state
    }
    fn reload(&mut self, path: PathBuf) {
        self.invalidate();
        self.settings.directory = Some(path.clone());
        let (tx, rx) = mpsc::channel();
        self.catalog_receiver = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(plugin::catalog_from(&path));
        });
    }
    fn current(&self) -> Option<&Plugin> {
        self.catalog
            .iter()
            .find(|p| p.manifest.id == self.settings.selected)
    }
    pub(super) fn settings(&self, cx: &App) -> Settings {
        let mut settings = self.settings.clone();
        if self.current().is_some() {
            settings
                .values
                .insert(settings.selected.clone(), self.values(cx));
        }
        settings
    }
    fn values(&self, cx: &App) -> BTreeMap<String, String> {
        self.fields
            .iter()
            .map(|(key, input)| (key.clone(), input.read(cx).value().to_string()))
            .collect()
    }
    pub(super) fn invalidate(&mut self) {
        self.preview_cancel.store(true, Ordering::Relaxed);
        self.preview_receiver = None;
        self.preview = None;
        self.revealed.clear();
    }
    fn load_fields(&mut self, window: &mut Window, cx: &mut Context<Workbench>) {
        self.fields.clear();
        self.subscriptions.clear();
        self.invalidate();
        if let Some(plugin) = self.current() {
            let fields = plugin.manifest.fields.clone();
            let mut values = plugin.defaults();
            if let Some(saved) = self.settings.values.get(&self.settings.selected) {
                values.extend(saved.clone());
            }
            for field in fields {
                let input = cx.new(|cx| {
                    InputState::new(window, cx)
                        .default_value(values.get(&field.key).cloned().unwrap_or_default())
                });
                self.fields.push((field.key, input));
            }
        }
        for input in self
            .fields
            .iter()
            .map(|(_, input)| input)
            .chain(std::iter::once(&self.secret))
        {
            self.subscriptions
                .push(cx.subscribe(input, |this, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.plugins.invalidate();
                        this.dirty_since = Some(Instant::now());
                        cx.notify();
                    }
                }));
        }
    }
    pub(super) fn execution(&self, cx: &App) -> anyhow::Result<Option<Execution>> {
        if !self.settings.enabled {
            return Ok(None);
        }
        anyhow::ensure!(
            self.catalog_receiver.is_none(),
            "插件目录仍在加载，请稍后再试"
        );
        anyhow::ensure!(
            self.errors.is_empty(),
            "插件加载失败，请修正插件或恢复内置插件"
        );
        let plugin = self
            .current()
            .ok_or_else(|| anyhow::anyhow!("已启用的插件未找到，请重新选择插件"))?
            .clone();
        Ok(Some(Execution {
            plugin,
            config: self.values(cx),
            secret: Zeroizing::new(self.secret.read(cx).value().to_string()),
        }))
    }
    pub(super) fn poll(&mut self, window: &mut Window, cx: &mut Context<Workbench>) {
        if let Some((loaded, errors)) = self
            .catalog_receiver
            .as_ref()
            .and_then(|rx| rx.try_recv().ok())
        {
            self.catalog_receiver = None;
            self.settings = self.settings(cx);
            let (mut catalog, mut builtin_errors) = plugin::builtin_catalog();
            for plugin in loaded {
                catalog.retain(|p| p.manifest.id != plugin.manifest.id);
                catalog.push(plugin);
            }
            builtin_errors.extend(errors);
            self.catalog = catalog;
            self.errors = builtin_errors;
            self.load_fields(window, cx);
            cx.notify();
        }
        if let Some(result) = self
            .preview_receiver
            .as_ref()
            .and_then(|rx| rx.try_recv().ok())
        {
            self.preview_receiver = None;
            self.preview = Some(result);
            cx.notify();
        }
    }
    pub(super) fn received(&mut self, headers: plugin::Headers) {
        self.preview = Some(Ok(headers));
        self.revealed.clear();
    }
}
impl Drop for PluginUi {
    fn drop(&mut self) {
        self.preview_cancel.store(true, Ordering::Relaxed);
    }
}
impl Workbench {
    fn choose_plugin_directory(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("选择包含插件子目录的文件夹".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await
                && let Some(path) = paths.into_iter().next()
            {
                let _ = this.update_in(cx, |this, _, cx| {
                    this.plugins.reload(path);
                    this.save(cx);
                    cx.notify();
                });
            }
        })
        .detach();
    }
    fn preview_plugin(&mut self, cx: &mut Context<Self>) {
        let result = (|| -> anyhow::Result<_> {
            let execution = self
                .plugins
                .execution(cx)?
                .ok_or_else(|| anyhow::anyhow!("请先启用插件"))?;
            let method = self
                .current()
                .ok_or_else(|| anyhow::anyhow!("请先选择方法"))?
                .path();
            Ok((
                execution,
                self.connection(cx)?,
                method,
                self.request.read(cx).value().to_string(),
            ))
        })();
        match result {
            Ok((execution, connection, method, body)) => {
                self.plugins.invalidate();
                let (tx, rx) = mpsc::channel();
                self.plugins.preview_receiver = Some(rx);
                self.plugins.preview_cancel = Arc::new(AtomicBool::new(false));
                let cancel = self.plugins.preview_cancel.clone();
                std::thread::spawn(move || {
                    let result = execution
                        .run(
                            &method,
                            &connection.endpoint,
                            &body,
                            connection.metadata,
                            cancel,
                        )
                        .map_err(|e| e.to_string());
                    let _ = tx.send(result);
                });
            }
            Err(e) => self.plugins.preview = Some(Err(e.to_string())),
        }
        cx.notify();
    }
    pub(super) fn plugin_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut panel = column()
            .id("plugin-scroll")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_5()
            .gap_4()
            .child(
                row()
                    .justify_between()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("请求头插件"))
                    .child(
                        Button::new("enable-plugin")
                            .label(if self.plugins.settings.enabled {
                                "已启用"
                            } else {
                                "启用插件"
                            })
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.plugins.settings.enabled = !this.plugins.settings.enabled;
                                this.plugins.invalidate();
                                this.save(cx);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child("对工作区内所有业务调用生效；服务反射使用原始 Metadata。"),
            );
        let mut choices = row().flex_wrap().gap_2();
        for (index, plugin) in self.plugins.catalog.iter().enumerate() {
            choices = choices.child(
                Button::new(("plugin-choice", index))
                    .small()
                    .label(format!(
                        "{}{}",
                        if plugin.manifest.id == self.plugins.settings.selected {
                            "✓ "
                        } else {
                            ""
                        },
                        plugin.manifest.name
                    ))
                    .disabled(self.busy)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.plugins.settings = this.plugins.settings(cx);
                        this.plugins.settings.selected =
                            this.plugins.catalog[index].manifest.id.clone();
                        this.plugins
                            .secret
                            .update(cx, |s, cx| s.set_value("", window, cx));
                        this.plugins.load_fields(window, cx);
                        this.save(cx);
                        cx.notify();
                    })),
            );
        }
        panel = panel.child(choices);
        if let Some(plugin) = self.plugins.current() {
            panel = panel.child(
                div()
                    .text_sm()
                    .text_color(rgb(MUTED))
                    .child(plugin.manifest.description.clone()),
            );
            for (index, field) in plugin.manifest.fields.iter().enumerate() {
                let input = &self.plugins.fields[index].1;
                let mut field_row = row()
                    .gap_3()
                    .child(div().w(px(125.)).flex_shrink_0().child(field.label.clone()));
                if field.options.is_empty() {
                    field_row = field_row
                        .child(div().flex_1().child(Input::new(input).disabled(self.busy)));
                } else {
                    let value = input.read(cx).value();
                    let label = match value.as_ref() {
                        "milliseconds" => "当前时间 · 毫秒（13 位）",
                        "seconds" => "当前时间 · 秒（10 位）",
                        "lower" => "小写",
                        "upper" => "大写",
                        _ => value.as_ref(),
                    }
                    .to_string();
                    let options = field.options.clone();
                    field_row = field_row.child(
                        Button::new(("plugin-option", index))
                            .label(label)
                            .disabled(self.busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let input = &this.plugins.fields[index].1;
                                let current = input.read(cx).value();
                                let next = options
                                    .iter()
                                    .position(|o| o == current.as_ref())
                                    .map(|i| (i + 1) % options.len())
                                    .unwrap_or(0);
                                input.update(cx, |s, cx| {
                                    s.set_value(options[next].clone(), window, cx)
                                });
                                this.plugins.invalidate();
                                this.save(cx);
                                cx.notify();
                            })),
                    );
                }
                panel = panel.child(field_row);
            }
            panel = panel
                .child(
                    row()
                        .gap_3()
                        .child(div().w(px(125.)).flex_shrink_0().child("Secret"))
                        .child(
                            div().flex_1().child(
                                Input::new(&self.plugins.secret)
                                    .mask_toggle()
                                    .disabled(self.busy),
                            ),
                        ),
                )
                .child(div().text_xs().text_color(rgb(MUTED)).child(format!(
                    "密钥编码：{}。Secret 不保存；上面的普通参数会保存到本地。",
                    plugin.manifest.secret_encoding
                )));
        }
        panel = panel
            .child(
                row()
                    .gap_2()
                    .child(
                        Button::new("preview-plugin")
                            .primary()
                            .label(if self.plugins.preview_receiver.is_some() {
                                "计算中…"
                            } else {
                                "预览头部"
                            })
                            .disabled(
                                self.busy
                                    || self.plugins.preview_receiver.is_some()
                                    || !self.plugins.settings.enabled,
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.preview_plugin(cx))),
                    )
                    .child(
                        Button::new("reload-plugins")
                            .ghost()
                            .label(if self.plugins.settings.directory.is_some() {
                                "重新加载插件"
                            } else {
                                "加载本地插件…"
                            })
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.plugins.settings = this.plugins.settings(cx);
                                if let Some(path) = this.plugins.settings.directory.clone() {
                                    this.plugins.reload(path);
                                    cx.notify();
                                } else {
                                    this.choose_plugin_directory(window, cx);
                                }
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child("每次发送重新计算。预览仅是快照，不会复用到下一次请求。"),
            );
        if self.plugins.settings.directory.is_some() {
            panel = panel.child(
                Button::new("builtin-plugins")
                    .ghost()
                    .small()
                    .label("恢复内置插件")
                    .disabled(self.busy)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.plugins.settings = this.plugins.settings(cx);
                        this.plugins.settings.directory = None;
                        this.plugins.catalog_receiver = None;
                        (this.plugins.catalog, this.plugins.errors) = plugin::builtin_catalog();
                        this.plugins.settings.selected = "md5-appid-ts".into();
                        this.plugins
                            .secret
                            .update(cx, |s, cx| s.set_value("", window, cx));
                        this.plugins.load_fields(window, cx);
                        this.save(cx);
                        cx.notify();
                    })),
            );
        }
        if self.plugins.catalog_receiver.is_some() {
            panel = panel.child("正在后台读取插件目录…");
        }
        if let Some(result) = &self.plugins.preview {
            match result {
                Ok(headers) => {
                    panel = panel.child("最近生成的请求头");
                    for (index, (key, value)) in headers.iter().enumerate() {
                        let revealed = self.plugins.revealed.contains(key);
                        let mut label = row()
                            .gap_2()
                            .child(div().text_xs().text_color(rgb(MUTED)).child(key.clone()));
                        if super::headers::is_sensitive(key) {
                            let key = key.clone();
                            label = label.child(
                                Button::new(("preview-header-reveal", index))
                                    .ghost()
                                    .small()
                                    .label(if revealed { "隐藏" } else { "显示" })
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if !this.plugins.revealed.remove(&key) {
                                            this.plugins.revealed.insert(key.clone());
                                        }
                                        cx.notify();
                                    })),
                            );
                        }
                        panel = panel.child(column().gap_1().child(label).child(
                            div().text_sm().child(
                                super::headers::visible_value(key, value, revealed).to_string(),
                            ),
                        ));
                    }
                }
                Err(error) => {
                    panel = panel.child(div().text_color(rgb(0xc35246)).child(error.clone()))
                }
            }
        }
        for error in &self.plugins.errors {
            panel = panel.child(
                div()
                    .text_color(rgb(0xc35246))
                    .child(format!("插件加载错误：{error}")),
            );
        }
        panel
    }
}
