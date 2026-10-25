mod history;
mod update;
use history::HistoryEntry;
mod plugins;
use crate::rpc::{self, Connection, Event, Method, Reply};
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    Disableable, Icon, Sizable,
    button::{Button, ButtonVariants},
    input::{Editor, EditorState, Input, InputEvent, InputState},
    resizable::{h_resizable, resizable_panel},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use plugins::PluginUi;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    time::{Duration, Instant},
};

gpui_kit::actions!(rpcrad, [Quit, Send, SearchServices, SaveWorkspace]);
const GREEN: u32 = 0x128252;
const INK: u32 = 0x25312d;
const MUTED: u32 = 0x7b8580;
const BORDER: u32 = 0xe4e9e6;
const SURFACE: u32 = 0xf6f8f7;
fn row() -> Div {
    div().flex().items_center()
}
fn column() -> Div {
    div().flex().flex_col()
}
fn icon(name: IconName) -> Icon {
    Icon::new(name).size(px(16.))
}
fn badge(text: impl Into<SharedString>) -> Div {
    div()
        .px_2()
        .py(px(3.))
        .rounded(px(4.))
        .bg(rgb(0xeff3f1))
        .text_color(rgb(MUTED))
        .text_size(px(11.))
        .child(text.into())
}
fn tab(id: impl Into<ElementId>, label: impl Into<SharedString>, active: bool) -> Stateful<Div> {
    let label: SharedString = label.into();
    row()
        .id(id)
        .role(Role::Tab)
        .aria_label(label.clone())
        .aria_selected(active)
        .h(px(44.))
        .px_3()
        .gap_2()
        .cursor_pointer()
        .border_b_2()
        .border_color(if active { rgb(GREEN) } else { rgb(0xffffff) })
        .text_color(if active { rgb(GREEN) } else { rgb(MUTED) })
        .hover(|s| s.bg(rgb(0xf5f8f6)))
        .child(label)
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct Saved {
    endpoint: String,
    tls: bool,
    timeout: u64,
    protos: Vec<PathBuf>,
    methods: Vec<Method>,
    drafts: BTreeMap<String, String>,
    favorites: BTreeSet<String>,
    selected: String,
    #[serde(default)]
    plugin: crate::plugin::Settings,
}

pub struct Workbench {
    plugins: PluginUi,
    methods: Vec<Method>,
    protos: Vec<PathBuf>,
    selected: usize,
    tabs: Vec<usize>,
    drafts: BTreeMap<String, String>,
    responses: BTreeMap<String, Reply>,
    favorites: BTreeSet<String>,
    collapsed: BTreeSet<String>,
    endpoint: Entity<InputState>,
    search: Entity<InputState>,
    token: Entity<InputState>,
    request: Entity<EditorState>,
    metadata: Entity<EditorState>,
    response: Entity<EditorState>,
    tls: bool,
    timeout: u64,
    request_tab: usize,
    response_tab: usize,
    history_mode: bool,
    history_open: bool,
    history: Vec<HistoryEntry>,
    history_detail: Option<HistoryEntry>,
    history_tab: usize,
    history_reveal: bool,
    history_editor: Entity<EditorState>,
    reply: Reply,
    busy: bool,
    discovering: bool,
    cancel: Arc<AtomicBool>,
    receiver: Option<Receiver<Event>>,
    raw_output: String,
    notice: String,
    dirty_since: Option<Instant>,
    _subscriptions: Vec<Subscription>,
    active_call: Option<HistoryEntry>,
    update_receiver: Option<Receiver<Result<Option<crate::updater::Release>, String>>>,
    available_update: Option<crate::updater::Release>,
    update_status: String,
}
fn state_path() -> PathBuf {
    crate::paths::state_path()
}
fn example_protos() -> Vec<PathBuf> {
    crate::paths::examples()
}

impl Workbench {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let saved = std::fs::read(state_path())
            .or_else(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    std::fs::read(crate::paths::legacy_state_path())
                } else {
                    Err(error)
                }
            })
            .ok()
            .and_then(|data| serde_json::from_slice::<Saved>(&data).ok())
            .unwrap_or_default();
        let protos = if saved.protos.is_empty() && saved.methods.is_empty() {
            example_protos()
        } else {
            saved
                .protos
                .iter()
                .map(|p| crate::paths::load_proto(p))
                .collect()
        };
        let methods = if saved.methods.is_empty() {
            rpc::discover(&Connection {
                protos: protos.clone(),
                timeout: 5,
                ..Default::default()
            })
            .unwrap_or_default()
        } else {
            saved.methods
        };
        let selected = methods
            .iter()
            .position(|m| m.path() == saved.selected)
            .or_else(|| methods.iter().position(|m| m.name == "GetUser"))
            .unwrap_or(0);
        let mut drafts = saved.drafts;
        for method in &methods {
            drafts
                .entry(method.path())
                .or_insert_with(|| match method.name.as_str() {
                    "GetUser" | "WatchUsers" => {
                        "{\n  \"user_id\": \"usr_1024\",\n  \"include_profile\": true\n}".into()
                    }
                    "CreateUser" => {
                        "{\n  \"name\": \"Alex Chen\",\n  \"email\": \"alex@example.com\"\n}".into()
                    }
                    "ListUsers" => "{\n  \"page_size\": 20\n}".into(),
                    "GetOrder" => "{\n  \"order_id\": \"ord_2048\"\n}".into(),
                    _ => method.sample.clone(),
                });
        }
        let body = methods
            .get(selected)
            .and_then(|m| drafts.get(&m.path()))
            .cloned()
            .unwrap_or_else(|| "{}".into());
        let endpoint = cx.new(|cx| {
            InputState::new(window, cx).default_value(if saved.endpoint.is_empty() {
                "localhost:50051".into()
            } else {
                saved.endpoint
            })
        });
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索服务或方法"));
        let token = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("输入 Bearer token")
                .masked(true)
        });
        let request = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("json")
                .default_value(body)
        });
        let metadata = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("json")
                .default_value("{\n  \"x-client\": \"rpcrad\"\n}")
        });
        let history_editor = cx.new(|cx| EditorState::new(window, cx).language("json"));
        let response = cx.new(|cx| EditorState::new(window, cx).language("json"));
        let mut subscriptions = Vec::new();
        for input in [&endpoint, &search] {
            subscriptions.push(cx.subscribe(input, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.plugins.invalidate();
                    this.dirty_since = Some(Instant::now());
                    cx.notify();
                }
            }));
        }
        subscriptions.push(cx.subscribe(&request, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.plugins.invalidate();
                this.dirty_since = Some(Instant::now());
                cx.notify();
            }
        }));
        subscriptions.push(cx.subscribe(&token, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.plugins.invalidate();
                cx.notify();
            }
        }));
        subscriptions.push(cx.subscribe(&metadata, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.plugins.invalidate();
                cx.notify();
            }
        }));
        let mut tabs = if methods.is_empty() {
            vec![]
        } else {
            vec![selected]
        };
        if let Some(i) = methods.iter().position(|m| m.name == "WatchUsers")
            && i != selected
        {
            tabs.push(i);
        }
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(60))
                    .await;
                if this
                    .update_in(cx, |this, window, cx| this.poll(window, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        let weak = cx.weak_entity();
        window.on_window_should_close(cx, move |_, cx| {
            let _ = weak.update(cx, |this, cx| this.save(cx));
            true
        });
        let plugins = PluginUi::new(saved.plugin, window, cx);
        let mut state = Self {
            plugins,
            methods,
            protos,
            selected,
            tabs,
            drafts,
            responses: BTreeMap::new(),
            favorites: saved.favorites,
            collapsed: BTreeSet::from(["order.proto".into(), "health.proto".into()]),
            endpoint,
            search,
            token,
            request,
            metadata,
            response,
            tls: saved.tls,
            timeout: if saved.timeout == 0 { 5 } else { saved.timeout },
            request_tab: 0,
            response_tab: 0,
            history_mode: false,
            history_open: false,
            history: vec![],
            history_detail: None,
            history_tab: 0,
            history_reveal: false,
            history_editor,
            reply: Reply::default(),
            busy: false,
            discovering: false,
            cancel: Arc::new(AtomicBool::new(false)),
            receiver: None,
            raw_output: String::new(),
            notice: "示例 Proto 已加载 · 连接本地服务开始调试".into(),
            dirty_since: None,
            _subscriptions: subscriptions,
            active_call: None,
            update_receiver: None,
            available_update: None,
            update_status: String::new(),
        };
        state.check_update(cx);
        state
    }
    fn current(&self) -> Option<&Method> {
        self.methods.get(self.selected)
    }
    fn save(&mut self, cx: &App) {
        if let Some(method) = self.current() {
            self.drafts
                .insert(method.path(), self.request.read(cx).value().to_string());
        }
        let saved = Saved {
            endpoint: self.endpoint.read(cx).value().to_string(),
            tls: self.tls,
            timeout: self.timeout,
            protos: self
                .protos
                .iter()
                .map(|p| crate::paths::save_proto(p))
                .collect(),
            methods: self.methods.clone(),
            drafts: self.drafts.clone(),
            favorites: self.favorites.clone(),
            selected: self.current().map(Method::path).unwrap_or_default(),
            plugin: self.plugins.settings(cx),
        };
        let result = (|| -> anyhow::Result<()> {
            let path = state_path();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(
                path.with_extension("tmp"),
                serde_json::to_vec_pretty(&saved)?,
            )?;
            std::fs::rename(path.with_extension("tmp"), &path)?;
            Ok(())
        })();
        if let Err(error) = result {
            self.notice = format!("保存失败：{error}");
        }
        self.dirty_since = None;
    }
    fn switch(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.discovering || index >= self.methods.len() {
            return;
        }
        self.save(cx);
        self.history_detail = None;
        self.plugins.invalidate();
        self.selected = index;
        if !self.tabs.contains(&index) {
            self.tabs.push(index);
        }
        let method = self.methods[index].clone();
        let body = self
            .drafts
            .get(&method.path())
            .cloned()
            .unwrap_or_else(|| method.sample.clone());
        self.request
            .update(cx, |state, cx| state.set_value(body, window, cx));
        self.reply = self
            .responses
            .get(&method.path())
            .cloned()
            .unwrap_or_default();
        self.response_tab = 0;
        self.refresh_response(window, cx);
        self.notice = format!("{} · {}", method.name, method.kind());
        self.save(cx);
        cx.notify();
    }
    fn connection(&self, cx: &App) -> anyhow::Result<Connection> {
        let raw = self.endpoint.read(cx).value();
        let endpoint = raw
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .to_string();
        if endpoint.is_empty() {
            anyhow::bail!("请输入服务地址");
        }
        Ok(Connection {
            endpoint,
            tls: self.tls || raw.starts_with("https://"),
            timeout: self.timeout,
            protos: self.protos.clone(),
            metadata: rpc::metadata(
                &self.metadata.read(cx).value(),
                &self.token.read(cx).value(),
            )?,
        })
    }
    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.history_detail.is_some() {
            return;
        }
        if self.busy {
            self.cancel.store(true, Ordering::Relaxed);
            return;
        }
        if self.discovering {
            return;
        }
        let Some(method) = self.current().cloned() else {
            self.notice = "请先导入 Proto 或发现服务".into();
            cx.notify();
            return;
        };
        if method.client_stream {
            self.notice = "预览版支持 Unary 和服务端流；客户端流与双向流尚未开放".into();
            cx.notify();
            return;
        }
        let body = self.request.read(cx).value().to_string();
        if let Err(error) = serde_json::from_str::<serde_json::Value>(&body) {
            self.notice = format!("JSON 格式错误：{error}");
            cx.notify();
            return;
        }
        let mut connection = match self.connection(cx) {
            Ok(c) => c,
            Err(error) => {
                self.notice = error.to_string();
                cx.notify();
                return;
            }
        };
        let plugin = match self.plugins.execution(cx) {
            Ok(plugin) => plugin,
            Err(error) => {
                self.notice = error.to_string();
                cx.notify();
                return;
            }
        };
        self.plugins.invalidate();
        self.save(cx);
        self.busy = true;
        self.reply = Reply::default();
        self.response_tab = 0;
        self.raw_output.clear();
        self.notice = format!("正在调用 {} …", method.name);
        self.active_call = Some(HistoryEntry::pending(
            &method,
            &connection,
            body.clone(),
            plugin.as_ref().map(|p| p.plugin.manifest.name.clone()),
        ));
        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.cancel.clone();
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        self.refresh_response(window, cx);
        std::thread::spawn(move || {
            if let Some(plugin) = plugin {
                match plugin.run(
                    &method.path(),
                    &connection.endpoint,
                    &body,
                    connection.metadata.clone(),
                    cancel.clone(),
                ) {
                    Ok(headers) => {
                        connection.metadata = headers.clone();
                        let _ = tx.send(Event::PluginPrepared(headers));
                    }
                    Err(error) => {
                        let _ = tx.send(Event::Finished(Reply {
                            status: "插件失败 · 未发送".into(),
                            body: error.to_string(),
                            ..Default::default()
                        }));
                        return;
                    }
                }
            }
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(Event::Finished(Reply {
                    status: "已取消".into(),
                    ..Default::default()
                }));
                return;
            }
            rpc::invoke(connection, method, body, cancel, tx);
        });
        cx.notify();
    }
    fn poll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.plugins.poll(window, cx);
        self.poll_update(cx);
        let events: Vec<_> = self
            .receiver
            .as_ref()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();
        let mut changed = false;
        for event in events {
            changed = true;
            match event {
                Event::PluginPrepared(headers) => self.plugins.received(headers),
                Event::RequestDispatched(headers) => {
                    if let Some(call) = &mut self.active_call {
                        call.dispatched_headers = Some(headers);
                    }
                }
                Event::Progress(line) => {
                    self.raw_output.push_str(&line);
                    self.raw_output.push('\n');
                    self.reply = rpc::parse_output(&self.raw_output);
                    self.reply.status = "接收中".into();
                }
                Event::Finished(reply) => {
                    self.busy = false;
                    self.receiver = None;
                    self.reply = reply;
                    self.notice = if self.reply.status == "0 OK" {
                        "调用完成".into()
                    } else {
                        format!("调用结束 · {}", self.reply.status)
                    };
                    if let Some(mut entry) = self.active_call.take() {
                        self.responses
                            .insert(entry.method.clone(), self.reply.clone());
                        entry.reply = self.reply.clone();
                        self.history.insert(0, entry);
                        self.history.truncate(50);
                    }
                }
                Event::Discovered(result, paths) => {
                    self.discovering = false;
                    self.receiver = None;
                    match result {
                        Ok(methods) => {
                            self.save(cx);
                            self.methods = methods;
                            self.protos = paths;
                            self.tabs.clear();
                            self.selected = 0;
                            // Avoid saving the previous method's editor into the newly loaded first method.
                            let body = self
                                .methods
                                .first()
                                .map(|m| {
                                    self.drafts
                                        .get(&m.path())
                                        .cloned()
                                        .unwrap_or_else(|| m.sample.clone())
                                })
                                .unwrap_or_default();
                            self.request
                                .update(cx, |s, cx| s.set_value(body, window, cx));
                            self.switch(0, window, cx);
                            self.notice = format!("已加载 {} 个方法", self.methods.len());
                        }
                        Err(error) => self.notice = format!("加载失败：{error}"),
                    }
                }
            }
        }
        if changed {
            self.refresh_response(window, cx);
            cx.notify();
        }
        if self
            .dirty_since
            .is_some_and(|t| t.elapsed() > Duration::from_secs(1))
        {
            self.save(cx);
            cx.notify();
        }
    }
    fn refresh_response(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = match self.response_tab {
            1 => self.reply.headers.clone(),
            2 => self.reply.trailers.clone(),
            _ => self.reply.body.clone(),
        };
        self.response
            .update(cx, |state, cx| state.set_value(text, window, cx));
    }
    fn discover(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        if self.busy || self.discovering {
            return;
        }
        let mut connection = match self.connection(cx) {
            Ok(c) => c,
            Err(error) => {
                self.notice = error.to_string();
                cx.notify();
                return;
            }
        };
        connection.protos = paths.clone();
        self.discovering = true;
        self.notice = "正在解析服务定义 …".into();
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        std::thread::spawn(move || {
            let result = rpc::discover(&connection).map_err(|e| format!("{e:#}"));
            let _ = tx.send(Event::Discovered(result, paths));
        });
        cx.notify();
    }
    fn import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("选择 Proto 文件".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await {
                let _ = this.update_in(cx, |this, _, cx| {
                    let mut all = this.protos.clone();
                    for path in paths {
                        if path.extension().is_some_and(|e| e == "proto") && !all.contains(&path) {
                            all.push(path);
                        }
                    }
                    this.discover(all, cx);
                });
            }
        })
        .detach();
    }
    fn format(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let editor = if self.request_tab == 1 {
            self.metadata.clone()
        } else {
            self.request.clone()
        };
        match serde_json::from_str::<serde_json::Value>(&editor.read(cx).value())
            .and_then(|v| serde_json::to_string_pretty(&v))
        {
            Ok(text) => {
                editor.update(cx, |state, cx| state.set_value(text, window, cx));
                self.notice = "JSON 已格式化".into();
            }
            Err(error) => self.notice = format!("JSON 格式错误：{error}"),
        }
        cx.notify();
    }
}

impl Drop for Workbench {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Workbench {
    fn workspace(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut tabs = row()
            .h(px(42.))
            .bg(rgb(SURFACE))
            .border_b_1()
            .border_color(rgb(BORDER))
            .px_2()
            .gap_1();
        for &index in &self.tabs {
            if let Some(method) = self.methods.get(index) {
                tabs = tabs.child(
                    row()
                        .id(("work-tab", index))
                        .role(Role::Tab)
                        .aria_label(method.name.clone())
                        .aria_selected(index == self.selected)
                        .h_full()
                        .px_4()
                        .gap_2()
                        .cursor_pointer()
                        .border_t_2()
                        .border_color(if index == self.selected {
                            rgb(GREEN)
                        } else {
                            rgb(SURFACE)
                        })
                        .bg(if index == self.selected {
                            rgb(0xffffff)
                        } else {
                            rgb(SURFACE)
                        })
                        .text_color(if index == self.selected {
                            rgb(GREEN)
                        } else {
                            rgb(MUTED)
                        })
                        .child(icon(if method.server_stream {
                            IconName::Activity
                        } else {
                            IconName::GitFork
                        }))
                        .child(method.name.clone())
                        .child(
                            Button::new(("close-tab", index))
                                .ghost()
                                .xsmall()
                                .icon(IconName::X)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    if this.busy || this.discovering || this.tabs.len() <= 1 {
                                        return;
                                    }
                                    this.tabs.retain(|i| *i != index);
                                    if this.selected == index {
                                        this.switch(this.tabs[0], window, cx);
                                    }
                                    cx.notify();
                                })),
                        )
                        .on_click(
                            cx.listener(move |this, _, window, cx| this.switch(index, window, cx)),
                        ),
                );
            }
        }
        tabs = tabs.child(
            Button::new("find-method")
                .ghost()
                .small()
                .icon(IconName::Plus)
                .tooltip("查找并打开方法")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.search.update(cx, |s, cx| s.focus(window, cx))
                })),
        );
        let title = self
            .current()
            .map(|m| format!("{}  /  {}", m.service, m.name))
            .unwrap_or_else(|| "导入服务定义开始调试".into());
        let kind = self.current().map(Method::kind).unwrap_or("gRPC");
        let favorite = self
            .current()
            .is_some_and(|m| self.favorites.contains(&m.path()));
        let left = self.request_panel(cx).into_any_element();
        let right = self.response_panel(cx).into_any_element();
        column()
            .size_full()
            .min_w_0()
            .child(tabs)
            .child(
                row()
                    .h(px(58.))
                    .px_5()
                    .gap_3()
                    .child(div().text_color(rgb(GREEN)).child(icon(IconName::GitFork)))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                    .child(badge(kind))
                    .child(div().flex_1())
                    .child(
                        Button::new("favorite-toggle")
                            .ghost()
                            .small()
                            .icon(IconName::Star)
                            .when(favorite, |s| s.text_color(rgb(GREEN)))
                            .tooltip("收藏此方法")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(method) = this.current() {
                                    let path = method.path();
                                    if !this.favorites.remove(&path) {
                                        this.favorites.insert(path);
                                    }
                                    this.save(cx);
                                    cx.notify();
                                }
                            })),
                    ),
            )
            .child(
                row()
                    .px_5()
                    .pb_4()
                    .gap_2()
                    .child(badge("gRPC"))
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(&self.endpoint).prefix(icon(IconName::Server))),
                    )
                    .child(
                        Button::new("tls")
                            .label(if self.tls { "TLS 开启" } else { "TLS 关闭" })
                            .icon(if self.tls {
                                IconName::Lock
                            } else {
                                IconName::LockOpen
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.tls = !this.tls;
                                this.save(cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("timeout")
                            .label(format!("{} s", self.timeout))
                            .icon(IconName::Clock)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.timeout = match this.timeout {
                                    5 => 15,
                                    15 => 30,
                                    _ => 5,
                                };
                                this.save(cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("send")
                            .primary()
                            .w(px(122.))
                            .icon(if self.busy {
                                IconName::Square
                            } else {
                                IconName::Play
                            })
                            .label(if self.busy {
                                "停止"
                            } else {
                                "发送  ⌘ ↵"
                            })
                            .disabled(self.discovering)
                            .on_click(cx.listener(|this, _, window, cx| this.send(window, cx))),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .mx_4()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded(px(8.))
                    .overflow_hidden()
                    .child(
                        h_resizable("editors")
                            .child(
                                resizable_panel()
                                    .size(px(520.))
                                    .size_range(px(280.)..px(1600.))
                                    .child(left),
                            )
                            .child(
                                resizable_panel()
                                    .size_range(px(300.)..px(1600.))
                                    .child(right),
                            ),
                    ),
            )
            .child(
                column()
                    .mx_4()
                    .my_3()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded(px(6.))
                    .overflow_hidden()
                    .child(
                        row()
                            .id("history-drawer")
                            .role(Role::Button)
                            .aria_label("调用记录")
                            .h(px(36.))
                            .px_3()
                            .gap_2()
                            .cursor_pointer()
                            .child(icon(if self.history_open {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronRight
                            }))
                            .child("调用记录")
                            .child(badge(self.history.len().to_string()))
                            .child(div().flex_1())
                            .child(
                                div().text_xs().text_color(rgb(MUTED)).child(
                                    self.history
                                        .first()
                                        .map(|h| {
                                            format!(
                                                "{}   ·   {}   ·   {} ms",
                                                h.method.rsplit('/').next().unwrap_or(""),
                                                h.reply.status,
                                                h.reply.elapsed_ms
                                            )
                                        })
                                        .unwrap_or_else(|| "尚无调用".into()),
                                ),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.history_open = !this.history_open;
                                cx.notify();
                            })),
                    )
                    .when(self.history_open, |s| {
                        s.child(
                            column()
                                .id("history-rows")
                                .max_h(px(120.))
                                .overflow_y_scroll()
                                .children(self.history.iter().enumerate().map(|(index, h)| {
                                    row()
                                        .id(("history-row", index))
                                        .role(Role::Button)
                                        .aria_label(format!(
                                            "查看调用详情 {} {}",
                                            h.method, h.reply.status
                                        ))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(rgb(0xeaf0ec)))
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.open_history(index, window, cx)
                                        }))
                                        .px_4()
                                        .py_2()
                                        .gap_4()
                                        .border_t_1()
                                        .border_color(rgb(BORDER))
                                        .text_xs()
                                        .child(h.time_label())
                                        .child(h.method.clone())
                                        .child(div().flex_1())
                                        .child(h.endpoint.clone())
                                        .child(h.reply.status.clone())
                                        .child(format!("{} ms", h.reply.elapsed_ms))
                                })),
                        )
                    }),
            )
    }
}

impl Render for Workbench {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar = self.sidebar(cx).into_any_element();
        let workspace = if self.history_detail.is_some() {
            self.history_panel(cx).into_any_element()
        } else {
            self.workspace(cx).into_any_element()
        };
        column()
            .size_full()
            .bg(rgb(0xffffff))
            .text_color(rgb(INK))
            .text_size(px(13.))
            .font_family(".SystemUIFont")
            .on_action(cx.listener(|this, _: &Quit, _, cx| {
                this.save(cx);
                cx.quit();
            }))
            .on_action(cx.listener(|this, _: &Send, window, cx| this.send(window, cx)))
            .on_action(cx.listener(|this, _: &SearchServices, window, cx| {
                this.search.update(cx, |s, cx| s.focus(window, cx))
            }))
            .on_action(cx.listener(|this, _: &SaveWorkspace, _, cx| {
                this.save(cx);
                this.notice = "工作区已保存".into();
                cx.notify();
            }))
            .child(
                row()
                    .h(px(58.))
                    .px_5()
                    .gap_3()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(img("brand/logo.png").size(px(42.)).flex_shrink_0())
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("本地工作区"))
                    .child(
                        div()
                            .text_color(rgb(MUTED))
                            .text_size(px(11.))
                            .child("RPCRad"),
                    )
                    .child(div().flex_1())
                    .when_some(self.available_update.as_ref(), |s, release| {
                        s.child(
                            Button::new("update-banner")
                                .ghost()
                                .label(format!("新版本 {}", release.version))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.history_detail = None;
                                    this.request_tab = 3;
                                    cx.notify();
                                })),
                        )
                    })
                    .child(badge("●  Local"))
                    .child(
                        Button::new("command-search")
                            .ghost()
                            .icon(IconName::Search)
                            .label("搜索方法   ⌘ K")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.search.update(cx, |s, cx| s.focus(window, cx))
                            })),
                    )
                    .child(
                        Button::new("settings")
                            .ghost()
                            .icon(IconName::Settings)
                            .tooltip("设置与更新")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.history_detail = None;
                                this.request_tab = 3;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div().flex_1().min_h_0().child(
                    h_resizable("layout")
                        .child(
                            resizable_panel()
                                .size(px(260.))
                                .size_range(px(230.)..px(420.))
                                .child(sidebar),
                        )
                        .child(resizable_panel().child(workspace)),
                ),
            )
            .child(
                row()
                    .h(px(28.))
                    .px_4()
                    .gap_3()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(SURFACE))
                    .text_size(px(10.))
                    .text_color(rgb(MUTED))
                    .child(div().text_color(rgb(GREEN)).child("●"))
                    .child(if self.discovering {
                        "正在加载服务".to_string()
                    } else {
                        format!(
                            "{} 个定义文件 · {} 个方法",
                            self.protos.len(),
                            self.methods.len()
                        )
                    })
                    .child(div().w(px(1.)).h(px(10.)).bg(rgb(BORDER)))
                    .child(div().flex_1().overflow_hidden().child(self.notice.clone()))
                    .child(if self.dirty_since.is_some() {
                        "保存中…"
                    } else {
                        "已保存到本地"
                    })
                    .child(concat!("v", env!("CARGO_PKG_VERSION")))
                    .child("UTF-8"),
            )
    }
}

pub fn smoke_test() -> anyhow::Result<()> {
    let connection = Connection {
        endpoint: "127.0.0.1:50051".into(),
        timeout: 5,
        protos: example_protos(),
        ..Default::default()
    };
    let methods = rpc::discover(&connection)?;
    anyhow::ensure!(
        methods.len() == 8,
        "Expected 8 methods, got {}",
        methods.len()
    );
    let method = methods
        .iter()
        .find(|m| m.name == "GetUser")
        .ok_or_else(|| anyhow::anyhow!("GetUser missing"))?
        .clone();
    let (tx, rx) = mpsc::channel();
    rpc::invoke(
        connection.clone(),
        method,
        r#"{"user_id":"smoke_123","include_profile":true}"#.into(),
        Arc::new(AtomicBool::new(false)),
        tx,
    );
    let reply = rx
        .into_iter()
        .find_map(|e| {
            if let Event::Finished(r) = e {
                Some(r)
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("No response"))?;
    anyhow::ensure!(
        reply.status == "0 OK" && reply.body.contains("smoke_123"),
        "Unary failed: {reply:?}"
    );
    let reflected = rpc::discover(&Connection {
        protos: vec![],
        ..connection.clone()
    })?;
    anyhow::ensure!(
        reflected.iter().any(|m| m.name == "GetUser"),
        "Reflection did not discover GetUser"
    );
    let stream = methods
        .iter()
        .find(|m| m.name == "WatchUsers")
        .ok_or_else(|| anyhow::anyhow!("Stream missing"))?
        .clone();
    let (tx, rx) = mpsc::channel();
    rpc::invoke(
        connection.clone(),
        stream,
        r#"{"user_id":"usr_1024"}"#.into(),
        Arc::new(AtomicBool::new(false)),
        tx,
    );
    let reply = rx
        .into_iter()
        .find_map(|e| {
            if let Event::Finished(r) = e {
                Some(r)
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("No stream response"))?;
    anyhow::ensure!(
        reply.status == "0 OK" && reply.messages == 5,
        "Stream failed: {reply:?}"
    );
    let method = methods
        .iter()
        .find(|m| m.name == "GetUser")
        .ok_or_else(|| anyhow::anyhow!("GetUser missing"))?
        .clone();
    let (tx, rx) = mpsc::channel();
    rpc::invoke(
        connection.clone(),
        method,
        r#"{"user_id":"missing"}"#.into(),
        Arc::new(AtomicBool::new(false)),
        tx,
    );
    let error_reply = rx
        .into_iter()
        .find_map(|e| {
            if let Event::Finished(r) = e {
                Some(r)
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("No error response"))?;
    anyhow::ensure!(
        error_reply.status == "NotFound",
        "Expected NotFound: {error_reply:?}"
    );
    let stream = methods
        .iter()
        .find(|m| m.name == "WatchUsers")
        .ok_or_else(|| anyhow::anyhow!("Stream missing"))?
        .clone();
    let (tx, rx) = mpsc::channel();
    rpc::invoke(
        Connection {
            timeout: 1,
            ..connection.clone()
        },
        stream.clone(),
        "{}".into(),
        Arc::new(AtomicBool::new(false)),
        tx,
    );
    let timeout_reply = rx
        .into_iter()
        .find_map(|e| {
            if let Event::Finished(r) = e {
                Some(r)
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("No timeout response"))?;
    anyhow::ensure!(
        timeout_reply.status == "DeadlineExceeded",
        "Expected deadline: {timeout_reply:?}"
    );
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let signal = cancel.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(800));
        signal.store(true, Ordering::Relaxed);
    });
    rpc::invoke(connection, stream, "{}".into(), cancel, tx);
    let cancelled = rx
        .into_iter()
        .find_map(|e| {
            if let Event::Finished(r) = e {
                Some(r)
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("No cancelled response"))?;
    anyhow::ensure!(
        cancelled.status == "已取消" && cancelled.elapsed_ms < 2000,
        "Cancellation failed: {cancelled:?}"
    );
    println!(
        "PASS: Proto import (8 methods), unary, reflection, streaming, NotFound, deadline, cancellation."
    );
    Ok(())
}

impl Workbench {
    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search.read(cx).value().to_lowercase();
        let mut tree = column()
            .id("service-tree")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px_3()
            .gap_1();
        if self.history_mode {
            tree = tree.child(
                div()
                    .px_2()
                    .py_3()
                    .text_color(rgb(MUTED))
                    .text_xs()
                    .child("本次会话"),
            );
            if self.history.is_empty() {
                tree = tree.child(
                    div()
                        .p_3()
                        .text_color(rgb(MUTED))
                        .child("发送请求后，调用记录会显示在这里。"),
                );
            }
            for (index, entry) in self.history.iter().enumerate() {
                tree = tree.child(
                    column()
                        .id(("history", index))
                        .role(Role::Button)
                        .aria_label(format!(
                            "查看调用详情 {} {}",
                            entry.method, entry.reply.status
                        ))
                        .p_3()
                        .gap_1()
                        .rounded_md()
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0xeaf0ec)))
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(MUTED))
                                .child(entry.time_label()),
                        )
                        .child(entry.method.rsplit('/').next().unwrap_or("").to_string())
                        .child(div().text_xs().text_color(rgb(MUTED)).child(format!(
                            "{} · {} ms",
                            entry.reply.status, entry.reply.elapsed_ms
                        )))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_history(index, window, cx)
                        })),
                );
            }
        } else {
            tree = tree.child(
                row()
                    .px_2()
                    .pt_4()
                    .pb_2()
                    .justify_between()
                    .text_size(px(10.))
                    .text_color(rgb(MUTED))
                    .child("PROTO / SERVICES")
                    .child(badge(self.methods.len().to_string())),
            );
            let mut files: Vec<String> = self
                .methods
                .iter()
                .map(|m| m.source.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            files.sort_by_key(|s| match s.as_str() {
                "user.proto" => 0,
                "order.proto" => 1,
                _ => 2,
            });
            for (file_index, file) in files.iter().enumerate() {
                let file_methods: Vec<_> = self
                    .methods
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| {
                        &m.source == file
                            && (query.is_empty() || m.path().to_lowercase().contains(&query))
                    })
                    .collect();
                if file_methods.is_empty() {
                    continue;
                }
                let collapsed = self.collapsed.contains(file) && query.is_empty();
                let toggle_file = file.clone();
                tree = tree.child(
                    row()
                        .id(("file", file_index))
                        .role(Role::Button)
                        .aria_label(file.clone())
                        .h(px(36.))
                        .px_1()
                        .gap_2()
                        .cursor_pointer()
                        .child(icon(if collapsed {
                            IconName::ChevronRight
                        } else {
                            IconName::ChevronDown
                        }))
                        .child(icon(IconName::FileCode))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(file.clone()),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !this.collapsed.remove(&toggle_file) {
                                this.collapsed.insert(toggle_file.clone());
                            }
                            cx.notify();
                        })),
                );
                if collapsed {
                    continue;
                }
                let mut last_service = String::new();
                for (index, method) in file_methods {
                    if method.service != last_service {
                        last_service = method.service.clone();
                        tree = tree.child(
                            column()
                                .ml_7()
                                .mt_1()
                                .mb_1()
                                .gap_1()
                                .child(
                                    div().text_size(px(10.)).text_color(rgb(MUTED)).child(
                                        method
                                            .service
                                            .rsplit_once('.')
                                            .map(|s| s.0)
                                            .unwrap_or("")
                                            .to_string(),
                                    ),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(
                                            method
                                                .service
                                                .rsplit('.')
                                                .next()
                                                .unwrap_or("")
                                                .to_string(),
                                        ),
                                ),
                        );
                    }
                    tree = tree.child(
                        row()
                            .id(("method", index))
                            .role(Role::Button)
                            .aria_label(method.name.clone())
                            .ml_5()
                            .h(px(34.))
                            .px_2()
                            .gap_2()
                            .rounded(px(5.))
                            .bg(if self.selected == index {
                                rgb(0xe0f0e7)
                            } else {
                                rgb(SURFACE)
                            })
                            .text_color(if self.selected == index {
                                rgb(0x096c42)
                            } else {
                                rgb(INK)
                            })
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(0xe8efea)))
                            .child(icon(if method.server_stream {
                                IconName::Activity
                            } else {
                                IconName::GitFork
                            }))
                            .child(div().flex_1().text_size(px(12.)).child(method.name.clone()))
                            .child(div().text_size(px(9.)).text_color(rgb(MUTED)).child(
                                if method.server_stream {
                                    "Stream"
                                } else {
                                    "Unary"
                                },
                            ))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.switch(index, window, cx)
                            })),
                    );
                }
                tree = tree.child(div().h_2());
            }
            tree = tree.child(
                row()
                    .mt_6()
                    .pt_4()
                    .px_2()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .gap_2()
                    .text_color(rgb(MUTED))
                    .child(icon(IconName::Star))
                    .child("收藏"),
            );
            if self.favorites.is_empty() {
                tree = tree.child(
                    div()
                        .px_2()
                        .py_2()
                        .text_size(px(11.))
                        .text_color(rgb(MUTED))
                        .child("点击方法右侧的 ☆ 添加收藏"),
                );
            }
            for (index, method) in self
                .methods
                .iter()
                .enumerate()
                .filter(|(_, m)| self.favorites.contains(&m.path()))
            {
                tree = tree.child(
                    row()
                        .id(("favorite", index))
                        .p_2()
                        .gap_2()
                        .cursor_pointer()
                        .rounded_md()
                        .hover(|s| s.bg(rgb(0xe8efea)))
                        .child(icon(IconName::Bookmark))
                        .child(method.name.clone())
                        .on_click(
                            cx.listener(move |this, _, window, cx| this.switch(index, window, cx)),
                        ),
                );
            }
        }
        column()
            .size_full()
            .bg(rgb(SURFACE))
            .border_r_1()
            .border_color(rgb(BORDER))
            .child(
                row()
                    .p_3()
                    .gap_1()
                    .child(
                        Button::new("services-mode")
                            .label("服务")
                            .flex_1()
                            .when(!self.history_mode, |s| s.primary())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.history_mode = false;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("history-mode")
                            .label("历史")
                            .flex_1()
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.history_mode = true;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div().px_3().pb_3().child(
                    Input::new(&self.search)
                        .small()
                        .prefix(icon(IconName::Search)),
                ),
            )
            .child(
                row()
                    .px_3()
                    .gap_2()
                    .child(
                        Button::new("import")
                            .small()
                            .icon(IconName::Plus)
                            .label("导入 Proto")
                            .flex_1()
                            .disabled(self.busy || self.discovering)
                            .on_click(cx.listener(|this, _, w, cx| this.import(w, cx))),
                    )
                    .child(
                        Button::new("reflection")
                            .small()
                            .icon(IconName::RefreshCw)
                            .label("服务反射")
                            .flex_1()
                            .disabled(self.busy || self.discovering)
                            .on_click(cx.listener(|this, _, _, cx| this.discover(vec![], cx))),
                    ),
            )
            .child(tree)
            .child(
                column()
                    .p_4()
                    .gap_2()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .child(
                        row()
                            .gap_2()
                            .child(icon(IconName::Folder))
                            .child("本地工作区")
                            .child(div().flex_1())
                            .child(div().text_color(rgb(GREEN)).child(icon(IconName::Check))),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(MUTED))
                            .child("请求自动保存 · 无需登录"),
                    ),
            )
    }

    fn request_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut tabs = row().border_b_1().border_color(rgb(BORDER));
        for (index, label) in ["请求体", "Metadata", "认证", "设置", "插件"]
            .into_iter()
            .enumerate()
        {
            tabs = tabs.child(
                tab(("request-tab", index), label, self.request_tab == index).on_click(
                    cx.listener(move |this, _, _, cx| {
                        this.request_tab = index;
                        cx.notify();
                    }),
                ),
            );
        }
        let mut content = column().flex_1().min_h_0();
        if self.request_tab <= 1 {
            content = content
                .child(
                    row()
                        .h(px(40.))
                        .px_4()
                        .gap_2()
                        .border_b_1()
                        .border_color(rgb(0xf0f3f1))
                        .child(div().text_xs().text_color(rgb(MUTED)).child("JSON"))
                        .child(div().flex_1())
                        .when(self.request_tab == 0, |s| {
                            s.child(
                                Button::new("sample")
                                    .ghost()
                                    .small()
                                    .icon(IconName::FileCode)
                                    .label("生成示例")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        if let Some(method) = this.current() {
                                            let sample = method.sample.clone();
                                            this.request.update(cx, |state, cx| {
                                                state.set_value(sample, window, cx)
                                            });
                                        }
                                    })),
                            )
                        })
                        .child(
                            Button::new("format")
                                .ghost()
                                .small()
                                .icon(IconName::Braces)
                                .label("格式化")
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.format(window, cx)),
                                ),
                        ),
                )
                .child(
                    div().flex_1().min_h_0().pt_2().child(
                        Editor::new(if self.request_tab == 0 {
                            &self.request
                        } else {
                            &self.metadata
                        })
                        .h(relative(1.))
                        .bordered(false)
                        .appearance(false)
                        .text_size(px(13.)),
                    ),
                );
        } else if self.request_tab == 4 {
            content = content.child(self.plugin_panel(cx));
        } else if self.request_tab == 2 {
            content = content.child(
                column()
                    .p_6()
                    .gap_4()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Bearer Token"),
                    )
                    .child(
                        div()
                            .text_color(rgb(MUTED))
                            .text_sm()
                            .child("随请求发送到 authorization Metadata。"),
                    )
                    .child(Input::new(&self.token))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child("认证信息仅保留在本次会话。"),
                    ),
            );
        } else {
            content = content.child(
                column()
                    .p_6()
                    .gap_5()
                    .id("settings-scroll")
                    .overflow_y_scroll()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("连接设置"))
                    .child(
                        row().justify_between().child("TLS 加密").child(
                            Button::new("tls-settings")
                                .label(if self.tls { "已开启" } else { "已关闭" })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.tls = !this.tls;
                                    this.save(cx);
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child("开启后使用系统信任的 CA 验证服务证书。"),
                    )
                    .child(
                        row().justify_between().child("请求超时").child(
                            Button::new("timeout-settings")
                                .label(format!("{} 秒", self.timeout))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.timeout = match this.timeout {
                                        5 => 15,
                                        15 => 30,
                                        _ => 5,
                                    };
                                    this.save(cx);
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child("点击切换 5 / 15 / 30 秒，流式请求也受此限制。"),
                    )
                    .child(
                        div()
                            .border_t_1()
                            .border_color(rgb(BORDER))
                            .pt_4()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child("预览版支持 Unary 与服务端流。mTLS、客户端流和双向流尚未开放。"),
                    )
                    .child(self.update_panel(cx)),
            );
        }
        let valid =
            serde_json::from_str::<serde_json::Value>(&self.request.read(cx).value()).is_ok();
        column()
            .size_full()
            .bg(rgb(0xffffff))
            .child(tabs)
            .child(content)
            .child(
                row()
                    .h(px(38.))
                    .px_4()
                    .gap_2()
                    .text_size(px(11.))
                    .text_color(rgb(if valid { MUTED } else { 0xc35246 }))
                    .child(icon(if valid {
                        IconName::CircleCheck
                    } else {
                        IconName::CircleAlert
                    }))
                    .child(if valid {
                        "JSON 格式有效"
                    } else {
                        "JSON 格式有误，请检查请求体"
                    }),
            )
    }

    fn response_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let status_color = if self.reply.status == "0 OK" || self.busy {
            GREEN
        } else {
            0xc35246
        };
        let mut tabs = row()
            .h(px(40.))
            .px_2()
            .border_b_1()
            .border_color(rgb(0xf0f3f1));
        for (index, label) in ["消息", "Headers", "Trailers"].into_iter().enumerate() {
            tabs = tabs.child(
                tab(("response-tab", index), label, self.response_tab == index)
                    .h(px(40.))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.response_tab = index;
                        this.refresh_response(window, cx);
                        cx.notify();
                    })),
            );
        }
        tabs =
            tabs.child(div().flex_1())
                .child(div().pr_3().text_xs().text_color(rgb(MUTED)).child(
                    if self.response_tab == 0 {
                        "JSON"
                    } else {
                        "TEXT"
                    },
                ));
        column()
            .size_full()
            .bg(rgb(0xffffff))
            .child(
                row()
                    .h(px(44.))
                    .px_4()
                    .gap_3()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("响应"))
                    .when(!self.reply.status.is_empty(), |s| {
                        s.child(
                            row()
                                .gap_1()
                                .text_color(rgb(status_color))
                                .text_xs()
                                .child("●")
                                .child(self.reply.status.clone()),
                        )
                    })
                    .when(!self.reply.status.is_empty() && !self.busy, |s| {
                        s.child(
                            div()
                                .text_xs()
                                .text_color(rgb(MUTED))
                                .child(format!("{} ms", self.reply.elapsed_ms)),
                        )
                    })
                    .when(self.reply.bytes > 0, |s| {
                        s.child(
                            div()
                                .text_xs()
                                .text_color(rgb(MUTED))
                                .child(format!("{} B", self.reply.bytes)),
                        )
                    })
                    .child(div().flex_1())
                    .child(
                        Button::new("copy-response")
                            .ghost()
                            .small()
                            .icon(IconName::Copy)
                            .tooltip("复制当前响应")
                            .on_click(cx.listener(|this, _, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(
                                    this.response.read(cx).value().to_string(),
                                ));
                                this.notice = "已复制到剪贴板".into();
                                cx.notify();
                            })),
                    ),
            )
            .child(tabs)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .when(
                        self.reply.body.is_empty() && !self.busy && self.response_tab == 0,
                        |s| {
                            s.child(
                                column()
                                    .size_full()
                                    .items_center()
                                    .justify_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .size(px(48.))
                                            .rounded(px(12.))
                                            .bg(rgb(0xf0f6f2))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_color(rgb(GREEN))
                                            .child(
                                                Icon::new(IconName::ArrowDownToLine).size(px(22.)),
                                            ),
                                    )
                                    .child(
                                        div().font_weight(FontWeight::MEDIUM).child("准备接收响应"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(rgb(MUTED))
                                            .child("发送请求，在这里查看服务返回的数据"),
                                    )
                                    .child(badge("⌘ Enter 发送请求")),
                            )
                        },
                    )
                    .when(
                        !self.reply.body.is_empty() || self.busy || self.response_tab != 0,
                        |s| {
                            s.pt_2().child(
                                Editor::new(&self.response)
                                    .h(relative(1.))
                                    .readonly(true)
                                    .bordered(false)
                                    .appearance(false)
                                    .text_size(px(13.)),
                            )
                        },
                    ),
            )
            .child(
                row()
                    .h(px(38.))
                    .px_4()
                    .gap_2()
                    .text_size(px(11.))
                    .text_color(rgb(MUTED))
                    .child(icon(if self.busy {
                        IconName::Activity
                    } else {
                        IconName::CircleCheck
                    }))
                    .child(if self.busy {
                        format!("接收中 · {} 条消息", self.reply.messages)
                    } else if self.reply.status.is_empty() {
                        "等待调用".into()
                    } else {
                        format!("{} · {} 条消息", self.reply.status, self.reply.messages)
                    }),
            )
    }
}

pub fn plugin_smoke_test() -> anyhow::Result<()> {
    use crate::plugin::{self, Execution};
    let connection = Connection {
        endpoint: "127.0.0.1:50052".into(),
        timeout: 5,
        protos: example_protos(),
        ..Default::default()
    };
    let method = rpc::discover(&connection)?
        .into_iter()
        .find(|m| m.name == "GetUser")
        .ok_or_else(|| anyhow::anyhow!("GetUser missing"))?;
    let (plugins, errors) = plugin::catalog();
    anyhow::ensure!(errors.is_empty(), "{errors:?}");
    let plugin = plugins
        .into_iter()
        .find(|p| p.manifest.id == "md5-appid-ts")
        .ok_or_else(|| anyhow::anyhow!("MD5 plugin missing"))?;
    let mut config = plugin.defaults();
    config.insert("appid".into(), "rpcrad-test".into());
    let execution = Execution {
        plugin,
        config,
        secret: zeroize::Zeroizing::new("rpcrad-test-secret".into()),
    };
    let body = r#"{"user_id":"signature_verified"}"#;
    let headers = execution.run(
        &method.path(),
        &connection.endpoint,
        body,
        vec![],
        Arc::new(AtomicBool::new(false)),
    )?;
    for valid in [true, false] {
        let mut headers = headers.clone();
        if !valid {
            for (k, v) in &mut headers {
                if k == "sign" {
                    *v = "tampered".into();
                }
            }
        }
        let (tx, rx) = mpsc::channel();
        let expected_headers = headers.clone();
        rpc::invoke(
            Connection {
                metadata: headers,
                ..connection.clone()
            },
            method.clone(),
            body.into(),
            Arc::new(AtomicBool::new(false)),
            tx,
        );
        let events: Vec<_> = rx.into_iter().collect();
        let dispatched = events.iter().find_map(|event| {
            if let Event::RequestDispatched(headers) = event {
                Some(headers)
            } else {
                None
            }
        });
        anyhow::ensure!(
            dispatched == Some(&expected_headers),
            "Recorded headers differ from outgoing headers"
        );
        let reply = events
            .into_iter()
            .find_map(|e| {
                if let Event::Finished(r) = e {
                    Some(r)
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow::anyhow!("No response"))?;
        if valid {
            anyhow::ensure!(
                reply.status == "0 OK" && reply.headers.contains("x-signature-verified: true"),
                "Signature rejected: {reply:?}"
            );
        } else {
            anyhow::ensure!(
                reply.status == "Unauthenticated",
                "Tampered signature accepted: {reply:?}"
            );
        }
    }
    println!(
        "PASS: actual grpcurl sends matching millisecond timestamp and MD5 headers; independent Go verifier accepts valid and rejects tampered signatures."
    );
    Ok(())
}
