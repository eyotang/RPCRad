mod environments;
mod language;
use crate::i18n::Language;
use language::{localized_editor, localized_input, mask_button};
mod headers;
use environments::EnvironmentUi;
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
    menu::DropdownMenu,
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

gpui_kit::actions!(
    rpcrad,
    [
        Quit,
        Send,
        SearchServices,
        SaveWorkspace,
        About,
        CheckUpdates
    ]
);
const GREEN: u32 = 0x128252;
const INK: u32 = 0x25312d;
const MUTED: u32 = 0x7b8580;
const BORDER: u32 = 0xe4e9e6;
const SURFACE: u32 = 0xf6f8f7;
pub fn platform_text(mac: &'static str, other: &'static str) -> &'static str {
    if cfg!(target_os = "macos") {
        mac
    } else {
        other
    }
}
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
    #[serde(default)]
    language: Language,
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
    #[serde(default)]
    variables: crate::variables::Variables,
}

pub struct Workbench {
    environments: EnvironmentUi,
    environments_open: bool,
    about_open: bool,
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
    history_revealed: BTreeSet<String>,
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
    pub fn new(language: Option<Language>, window: &mut Window, cx: &mut Context<Self>) -> Self {
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
        language.unwrap_or(saved.language).activate();
        crate::install_menus(cx);
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
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder(tr!("Search services or methods")));
        let token = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(tr!("Enter Bearer token"))
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
        let environments = EnvironmentUi::new(saved.variables, window, cx);
        let mut state = Self {
            environments,
            environments_open: false,
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
            about_open: false,
            history_mode: false,
            history_open: false,
            history: vec![],
            history_detail: None,
            history_tab: 0,
            history_revealed: BTreeSet::new(),
            history_editor,
            reply: Reply::default(),
            busy: false,
            discovering: false,
            cancel: Arc::new(AtomicBool::new(false)),
            receiver: None,
            raw_output: String::new(),
            notice: tr!("Sample protos loaded · Connect to a local service to get started").into(),
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
            language: Language::current(),
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
            variables: self.environments.snapshot(cx),
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
            self.notice = tr_format!("Could not save: {error}", error = error);
        }
        self.dirty_since = None;
    }
    fn switch(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.discovering || index >= self.methods.len() {
            return;
        }
        self.save(cx);
        self.history_detail = None;
        self.about_open = false;
        self.environments_open = false;
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
        let variables = self.environments.snapshot(cx);
        self.resolved_connection(&variables.resolver()?, cx)
    }
    fn resolved_connection(
        &self,
        resolver: &crate::variables::Resolver<'_>,
        cx: &App,
    ) -> anyhow::Result<Connection> {
        let raw = resolver.text(&self.endpoint.read(cx).value())?;
        let endpoint = raw
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .to_string();
        if endpoint.is_empty() {
            anyhow::bail!(tr_format!("Enter a server address"));
        }
        Ok(Connection {
            endpoint,
            tls: self.tls || raw.starts_with("https://"),
            timeout: self.timeout,
            protos: self.protos.clone(),
            metadata: rpc::metadata(
                &resolver.json(&self.metadata.read(cx).value())?,
                &resolver.text(&self.token.read(cx).value())?,
            )?,
        })
    }
    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.history_detail.is_some() || self.about_open || self.environments_open {
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
            self.notice = tr!("Import a proto or discover services first").into();
            cx.notify();
            return;
        };
        if method.client_stream {
            self.notice = tr!("This preview supports unary and server streaming; client and bidirectional streaming are not yet available").into();
            cx.notify();
            return;
        }
        let prepared = (|| -> anyhow::Result<_> {
            let variables = self.environments.snapshot(cx);
            let resolver = variables.resolver()?;
            let body = resolver.json(&self.request.read(cx).value())?;
            let connection = self.resolved_connection(&resolver, cx)?;
            let plugin = self.plugins.execution(&resolver, cx)?;
            Ok((
                body,
                connection,
                plugin,
                variables.active_name().to_string(),
            ))
        })();
        let (body, mut connection, plugin, environment) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                self.notice = tr_format!("Not sent: {error}", error = error);
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
        self.notice = tr_format!("Calling {} …", method.name);
        self.active_call = Some(HistoryEntry::pending(
            &method,
            &connection,
            body.clone(),
            plugin
                .as_ref()
                .map(|p| p.plugin.manifest.name().to_string()),
            environment,
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
                            status: tr!("Plugin failed · Not sent").into(),
                            body: error.to_string(),
                            ..Default::default()
                        }));
                        return;
                    }
                }
            }
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(Event::Finished(Reply {
                    status: tr!("Cancelled").into(),
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
                    self.reply.status = tr!("Receiving").into();
                }
                Event::Finished(reply) => {
                    self.busy = false;
                    self.receiver = None;
                    self.reply = reply;
                    self.notice = if self.reply.status == "0 OK" {
                        tr!("Call completed").into()
                    } else {
                        tr_format!("Call ended · {}", self.reply.status)
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
                            self.notice = tr_format!("Loaded {} methods", self.methods.len());
                        }
                        Err(error) => {
                            self.notice = tr_format!("Could not load: {error}", error = error)
                        }
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
        self.notice = tr!("Loading service definitions …").into();
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
            prompt: Some(tr!("Select proto files").into()),
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
        if editor.read(cx).value().contains("{{") {
            self.notice = tr!(
                "Variable templates are preserved; JSON is resolved and validated before sending"
            )
            .into();
            cx.notify();
            return;
        }
        match serde_json::from_str::<serde_json::Value>(&editor.read(cx).value())
            .and_then(|v| serde_json::to_string_pretty(&v))
        {
            Ok(text) => {
                editor.update(cx, |state, cx| state.set_value(text, window, cx));
                self.notice = tr!("JSON formatted").into();
            }
            Err(error) => self.notice = tr_format!("Invalid JSON: {error}", error = error),
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
                .tooltip(tr!("Find and open a method"))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.search.update(cx, |s, cx| s.focus(window, cx))
                })),
        );
        let title = self
            .current()
            .map(|m| format!("{}  /  {}", m.service, m.name))
            .unwrap_or_else(|| tr!("Import service definitions to get started").into());
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
                            .tooltip(tr!("Favorite this method"))
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
                            .child(localized_input(&self.endpoint).prefix(icon(IconName::Server))),
                    )
                    .child(
                        Button::new("tls")
                            .label(if self.tls {
                                tr!("TLS on")
                            } else {
                                tr!("TLS off")
                            })
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
                            .min_w(px(122.))
                            .flex_shrink_0()
                            .icon(if self.busy {
                                IconName::Square
                            } else {
                                IconName::Play
                            })
                            .label(if self.busy {
                                tr!("Stop")
                            } else {
                                platform_text(tr!("Send  ⌘ ↵"), tr!("Send  Ctrl ↵"))
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
                            .aria_label(tr!("Call history"))
                            .h(px(36.))
                            .px_3()
                            .gap_2()
                            .cursor_pointer()
                            .child(icon(if self.history_open {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronRight
                            }))
                            .child(tr!("Call history"))
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
                                        .unwrap_or_else(|| tr!("No calls yet").into()),
                                ),
                            )
                            .child(self.clear_history_button("clear-history-drawer", cx))
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
                                        .aria_label(tr_format!(
                                            "View call details {} {}",
                                            h.method,
                                            h.reply.status
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
        let workspace = if self.environments_open {
            self.environments_panel(cx).into_any_element()
        } else if self.about_open {
            self.about_panel(cx).into_any_element()
        } else if self.history_detail.is_some() {
            self.history_panel(cx).into_any_element()
        } else {
            self.workspace(cx).into_any_element()
        };
        column()
            .size_full()
            .bg(rgb(0xffffff))
            .text_color(rgb(INK))
            .text_size(px(13.))
            .font_family(if cfg!(target_os = "macos") {
                ".SystemUIFont"
            } else if cfg!(windows) {
                "Segoe UI"
            } else {
                "Noto Sans"
            })
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
                this.notice = tr!("Workspace saved").into();
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
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .max_w(px(180.))
                            .truncate()
                            .child(tr!("Local workspace")),
                    )
                    .child(
                        div()
                            .text_color(rgb(MUTED))
                            .text_size(px(11.))
                            .child("RPCRad"),
                    )
                    .child(div().flex_1())
                    .when(!cfg!(target_os = "macos"), |s| {
                        s.child(
                            Button::new("app-menu")
                                .ghost()
                                .label("RPCRad")
                                .icon(IconName::ChevronDown)
                                .accessibility_label(tr!("RPCRad menu"))
                                .dropdown_menu(|menu, _, _| {
                                    menu.menu(tr!("About RPCRad…"), Box::new(About))
                                        .menu(tr!("Check for updates…"), Box::new(CheckUpdates))
                                        .separator()
                                        .menu(tr!("Quit RPCRad"), Box::new(Quit))
                                }),
                        )
                    })
                    .when_some(self.available_update.as_ref(), |s, release| {
                        s.child(
                            Button::new("update-banner")
                                .ghost()
                                .label(tr_format!("Update {}", release.version))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.show_about(false, cx);
                                })),
                        )
                    })
                    .child(self.language_selector(cx))
                    .child(self.environment_selector(cx))
                    .child(
                        Button::new("manage-variables")
                            .ghost()
                            .icon(IconName::Braces)
                            .label(tr!("Variables"))
                            .on_click(cx.listener(|this, _, _, cx| this.open_environments(cx))),
                    )
                    .child(
                        Button::new("command-search")
                            .ghost()
                            .icon(IconName::Search)
                            .label(platform_text(
                                tr!("Find method   ⌘ K"),
                                tr!("Find method   Ctrl K"),
                            ))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.search.update(cx, |s, cx| s.focus(window, cx))
                            })),
                    )
                    .child(
                        Button::new("settings")
                            .ghost()
                            .icon(IconName::Settings)
                            .tooltip(tr!("Settings and updates"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.about_open = false;
                                this.history_detail = None;
                                this.environments_open = false;
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
                        tr!("Loading services").to_string()
                    } else {
                        tr_format!(
                            "{} proto files · {} methods",
                            self.protos.len(),
                            self.methods.len()
                        )
                    })
                    .child(div().w(px(1.)).h(px(10.)).bg(rgb(BORDER)))
                    .child(div().flex_1().overflow_hidden().child(self.notice.clone()))
                    .child(if self.dirty_since.is_some() {
                        tr!("Saving…")
                    } else {
                        tr!("Saved locally")
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
        cancelled.status == tr!("Cancelled") && cancelled.elapsed_ms < 2000,
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
                row()
                    .px_2()
                    .py_3()
                    .text_color(rgb(MUTED))
                    .text_xs()
                    .child(tr!("This session"))
                    .child(div().flex_1())
                    .child(self.clear_history_button("clear-history-sidebar", cx)),
            );
            if self.history.is_empty() {
                tree = tree.child(
                    div()
                        .p_3()
                        .text_color(rgb(MUTED))
                        .child(tr!("Sent requests appear here.")),
                );
            }
            for (index, entry) in self.history.iter().enumerate() {
                tree = tree.child(
                    column()
                        .id(("history", index))
                        .role(Role::Button)
                        .aria_label(tr_format!(
                            "View call details {} {}",
                            entry.method,
                            entry.reply.status
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
                    .child(tr!("Favorites")),
            );
            if self.favorites.is_empty() {
                tree = tree.child(
                    div()
                        .px_2()
                        .py_2()
                        .text_size(px(11.))
                        .text_color(rgb(MUTED))
                        .child(tr!("Click ☆ next to a method to add a favorite")),
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
                            .label(tr!("Services"))
                            .flex_1()
                            .when(!self.history_mode, |s| s.primary())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.history_mode = false;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("history-mode")
                            .label(tr!("History"))
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
                    localized_input(&self.search)
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
                            .label(tr!("Import proto"))
                            .flex_1()
                            .disabled(self.busy || self.discovering)
                            .on_click(cx.listener(|this, _, w, cx| this.import(w, cx))),
                    )
                    .child(
                        Button::new("reflection")
                            .small()
                            .icon(IconName::RefreshCw)
                            .label(tr!("Server reflection"))
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
                            .child(tr!("Local workspace"))
                            .child(div().flex_1())
                            .child(div().text_color(rgb(GREEN)).child(icon(IconName::Check))),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(MUTED))
                            .child(tr!("Requests saved automatically · No sign-in")),
                    ),
            )
    }

    fn request_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut tabs = row().border_b_1().border_color(rgb(BORDER));
        for (index, label) in [
            tr!("Body"),
            "Metadata",
            tr!("Auth"),
            tr!("Settings"),
            tr!("Plugins"),
        ]
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
                                    .label(tr!("Generate example"))
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
                                .label(tr!("Format"))
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.format(window, cx)),
                                ),
                        ),
                )
                .child(
                    div().flex_1().min_h_0().pt_2().child(
                        localized_editor(if self.request_tab == 0 {
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
                            .child(tr!("Sent as authorization metadata with each request.")),
                    )
                    .child(localized_input(&self.token))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(tr!("Authentication is kept only for this session.")),
                    ),
            );
        } else {
            content = content.child(
                column()
                    .p_6()
                    .gap_5()
                    .id("settings-scroll")
                    .overflow_y_scroll()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(tr!("Connection settings")))
                    .child(
                        row().justify_between().child(tr!("TLS encryption")).child(
                            Button::new("tls-settings")
                                .label(if self.tls { tr!("On") } else { tr!("Off") })
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
                            .child(tr!("Verify the server certificate using system-trusted CAs.")),
                    )
                    .child(
                        row().justify_between().child(tr!("Request timeout")).child(
                            Button::new("timeout-settings")
                                .label(tr_format!("{} s", self.timeout))
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
                            .child(tr!("Click to cycle through 5 / 15 / 30 seconds. Also applies to streams.")),
                    )
                    .child(
                        div()
                            .border_t_1()
                            .border_color(rgb(BORDER))
                            .pt_4()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(tr!("Supports unary and server streaming. mTLS, client and bidirectional streaming are not yet available.")),
                    )
                    .child(self.update_panel(cx)),
            );
        }
        let variables = self.environments.snapshot(cx);
        let validation = variables
            .resolver()
            .and_then(|r| r.json(&self.request.read(cx).value()));
        let valid = validation.is_ok();
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
                    .child(match validation {
                        Ok(_) => tr!("Variables and JSON are valid").to_string(),
                        Err(error) => error.to_string(),
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
        for (index, label) in [tr!("Messages"), "Headers", "Trailers"]
            .into_iter()
            .enumerate()
        {
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
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(tr!("Response")),
                    )
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
                            .tooltip(tr!("Copy current response"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(
                                    this.response.read(cx).value().to_string(),
                                ));
                                this.notice = tr!("Copied to clipboard").into();
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
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(tr!("Ready for a response")),
                                    )
                                    .child(div().text_size(px(12.)).text_color(rgb(MUTED)).child(
                                        tr!("Send a request to view the server response here"),
                                    ))
                                    .child(badge(platform_text(
                                        tr!("⌘ Enter to send"),
                                        tr!("Ctrl Enter to send"),
                                    ))),
                            )
                        },
                    )
                    .when(
                        !self.reply.body.is_empty() || self.busy || self.response_tab != 0,
                        |s| {
                            s.pt_2().child(
                                localized_editor(&self.response)
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
                        tr_format!("Receiving · {} messages", self.reply.messages)
                    } else if self.reply.status.is_empty() {
                        tr!("Waiting for a call").into()
                    } else {
                        tr_format!("{} · {} messages", self.reply.status, self.reply.messages)
                    }),
            )
    }
}

pub fn plugin_smoke_test() -> anyhow::Result<()> {
    use crate::plugin::{self, Execution};
    use crate::variables::{Environment, Variable, Variables};
    let variable = |name: &str, value: &str, sensitive| Variable {
        name: name.into(),
        value: value.into(),
        sensitive,
    };
    let variables = Variables {
        globals: vec![
            variable("appid", "wrong-global-app", false),
            variable("endpoint", "127.0.0.1:50052", false),
        ],
        environments: vec![Environment {
            id: 1,
            name: "签名测试".into(),
            variables: vec![
                variable("appid", "rpcrad-test", false),
                variable("secret", "rpcrad-test-secret", true),
                variable("user_id", "signature_verified", false),
                variable("include_profile", "true", false),
            ],
        }],
        active: Some(1),
    };
    let resolver = variables.resolver()?;
    let connection = Connection {
        endpoint: resolver.text("{{endpoint}}")?,
        timeout: 5,
        protos: example_protos(),
        ..Default::default()
    };
    let method = rpc::discover(&connection)?
        .into_iter()
        .find(|m| m.name == "GetUser")
        .ok_or_else(|| anyhow::anyhow!("GetUser missing"))?;
    let (plugins, errors) = plugin::builtin_catalog();
    anyhow::ensure!(errors.is_empty(), "{errors:?}");
    let plugin = plugins
        .into_iter()
        .find(|p| p.manifest.id == "md5-appid-ts")
        .ok_or_else(|| anyhow::anyhow!("MD5 plugin missing"))?;
    let mut config = plugin.defaults();
    config.insert("appid".into(), resolver.text("{{appid}}")?);
    let execution = Execution {
        plugin,
        config,
        secret: zeroize::Zeroizing::new(resolver.text("{{secret}}")?),
    };
    let body =
        resolver.json(r#"{"user_id":"{{user_id}}","include_profile":{{include_profile}}}"#)?;
    let headers = execution.run(
        &method.path(),
        &connection.endpoint,
        &body,
        rpc::metadata(&resolver.json(r#"{"x-app":"{{appid}}"}"#)?, "")?,
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
            body.clone(),
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
                reply.status == "0 OK"
                    && reply.headers.contains("x-signature-verified: true")
                    && reply.body.contains("signature_verified")
                    && reply.body.contains("Platform"),
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
        "PASS: environment variables resolve in body, headers and plugin; actual grpcurl sends matching millisecond timestamp and MD5 headers; independent Go verifier accepts valid and rejects tampered signatures."
    );
    Ok(())
}
