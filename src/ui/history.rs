use super::*;
use crate::plugin::Headers;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub(super) struct HistoryEntry {
    pub method: String,
    pub environment: String,
    pub endpoint: String,
    pub body: String,
    pub reply: Reply,
    pub original_headers: Headers,
    pub dispatched_headers: Option<Headers>,
    pub plugin: Option<String>,
    pub tls: bool,
    pub timeout: u64,
    pub started_ms: u128,
}
impl HistoryEntry {
    pub fn pending(
        method: &Method,
        connection: &Connection,
        body: String,
        plugin: Option<String>,
        environment: String,
    ) -> Self {
        Self {
            environment,
            method: method.path(),
            endpoint: connection.endpoint.clone(),
            body,
            reply: Reply::default(),
            original_headers: connection.metadata.clone(),
            dispatched_headers: None,
            plugin,
            tls: connection.tls,
            timeout: connection.timeout,
            started_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        }
    }
    pub(super) fn time_label(&self) -> String {
        let seconds = self.started_ms / 1000 + 8 * 3600;
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            seconds / 3600 % 24,
            seconds / 60 % 60,
            seconds % 60,
            self.started_ms % 1000
        )
    }
    fn sensitive_keys(&self, tab: usize) -> BTreeSet<String> {
        let headers = match tab {
            0 => self.dispatched_headers.as_deref().unwrap_or_default(),
            1 => &self.original_headers,
            _ => return BTreeSet::new(),
        };
        headers
            .iter()
            .filter(|(key, _)| super::headers::is_sensitive(key))
            .map(|(key, _)| key.clone())
            .collect()
    }
    fn content(&self, tab: usize, revealed: &BTreeSet<String>) -> String {
        let headers = |values: &Headers| {
            let values: BTreeMap<_, _> = values
                .iter()
                .map(|(k, v)| (k, super::headers::visible_value(k, v, revealed.contains(k))))
                .collect();
            serde_json::to_string_pretty(&values).unwrap_or_default()
        };
        match tab {
            0 => self
                .dispatched_headers
                .as_ref()
                .map(headers)
                .unwrap_or_else(|| {
                    let reason = if self.reply.body.trim().is_empty() { self.reply.status.as_str() } else { self.reply.body.trim() };
                    tr_format!("The request was not handed to the sender, so there are no sent headers.\n\nReason: {reason}\n\nFix the issue and send again. The result will appear as a new history entry.", reason = reason)
                }),
            1 => headers(&self.original_headers),
            2 => self.body.clone(),
            3 => self.reply.body.clone(),
            4 => self.reply.headers.clone(),
            _ => self.reply.trailers.clone(),
        }
    }
    fn changes(&self) -> String {
        let Some(after) = &self.dispatched_headers else {
            return tr!("Not sent").into();
        };
        let before: BTreeMap<_, _> = self.original_headers.iter().map(|(k, v)| (k, v)).collect();
        let after: BTreeMap<_, _> = after.iter().map(|(k, v)| (k, v)).collect();
        let added = after.keys().filter(|k| !before.contains_key(*k)).count();
        let updated = after
            .iter()
            .filter(|(k, v)| before.get(*k).is_some_and(|old| old != *v))
            .count();
        let removed = before.keys().filter(|k| !after.contains_key(*k)).count();
        tr_format!(
            "Added {added} · Changed {updated} · Removed {removed}",
            added = added,
            removed = removed,
            updated = updated
        )
    }
}
impl Workbench {
    pub(super) fn clear_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.history.clear();
        self.history_detail = None;
        self.history_revealed.clear();
        self.history_editor
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.notice = tr!("History cleared").into();
        cx.notify();
    }
    pub(super) fn clear_history_button(&self, id: &'static str, cx: &mut Context<Self>) -> Button {
        Button::new(id)
            .ghost()
            .small()
            .icon(IconName::Trash)
            .accessibility_label(tr!("Clear history"))
            .tooltip(tr!("Clear history"))
            .disabled(self.history.is_empty())
            .on_click(cx.listener(|this, _, window, cx| {
                cx.stop_propagation();
                this.clear_history(window, cx);
            }))
    }
    pub(super) fn open_history(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(entry) = self.history.get(index) {
            self.about_open = false;
            self.environments_open = false;
            self.history_detail = Some(entry.clone());
            self.history_tab = 0;
            self.history_revealed.clear();
            self.refresh_history(window, cx);
            cx.notify();
        }
    }
    pub(super) fn refresh_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = &self.history_detail {
            let content = entry.content(self.history_tab, &self.history_revealed);
            self.history_editor
                .update(cx, |s, cx| s.set_value(content, window, cx));
        }
    }
    fn load_history_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.history_detail.clone() else {
            return;
        };
        let Some(index) = self.methods.iter().position(|m| m.path() == entry.method) else {
            self.notice =
                tr!("This method is missing from the current definitions; import the proto again")
                    .into();
            cx.notify();
            return;
        };
        self.history_detail = None;
        self.switch(index, window, cx);
        self.request
            .update(cx, |s, cx| s.set_value(entry.body, window, cx));
        self.endpoint
            .update(cx, |s, cx| s.set_value(entry.endpoint, window, cx));
        self.tls = entry.tls;
        self.timeout = entry.timeout;
        self.plugins.invalidate();
        self.notice = tr!("Address, body and connection settings loaded. Headers use the current settings and plugins run again when sending").into();
        self.save(cx);
        cx.notify();
    }
    pub(super) fn history_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut panel = column().size_full().min_w_0().bg(rgb(SURFACE));
        let Some(entry) = &self.history_detail else {
            return panel;
        };
        let reveal_buttons: Vec<_> = entry
            .sensitive_keys(self.history_tab)
            .into_iter()
            .enumerate()
            .map(|(index, key)| {
                let label = format!(
                    "{} {}",
                    if self.history_revealed.contains(&key) {
                        tr!("Hide")
                    } else {
                        tr!("Show")
                    },
                    key
                );
                Button::new(("history-header-reveal", index))
                    .ghost()
                    .small()
                    .label(label)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if !this.history_revealed.remove(&key) {
                            this.history_revealed.insert(key.clone());
                        }
                        this.refresh_history(window, cx);
                        cx.notify();
                    }))
            })
            .collect();
        panel = panel.child(row().h(px(58.)).px_5().gap_3().border_b_1().border_color(rgb(BORDER))
            .child(Button::new("history-back").ghost().label(tr!("Back to requests")).on_click(cx.listener(|this,_,_,cx| {this.history_detail = None; cx.notify();})))
            .child(div().font_weight(FontWeight::SEMIBOLD).child(tr!("Call details")))
            .child(badge(tr!("Read-only snapshot")))
            .child(badge(tr_format!("Environment: {}",entry.environment)))
            .child(div().flex_1())
            .child(Button::new("history-load").label(tr!("Load request")).disabled(self.busy || self.discovering)
                .tooltip(tr!("Load the address, body and connection settings; generate a fresh signature when sending"))
                .on_click(cx.listener(|this,_,window,cx|this.load_history_request(window,cx)))))
            .child(column().p_5().gap_3()
                .child(row().gap_3().child(div().font_weight(FontWeight::SEMIBOLD).child(entry.method.clone()))
                    .child(badge(entry.reply.status.clone())))
                .child(row().gap_4().text_sm().text_color(rgb(MUTED)).child(entry.endpoint.clone())
                    .child(if entry.tls { tr!("TLS on") } else { tr!("TLS off") })
                    .child(tr_format!("{} ms · {} messages · {} B", entry.reply.elapsed_ms, entry.reply.messages, entry.reply.bytes)))
                .child(row().gap_4().text_xs().text_color(rgb(MUTED))
                    .child(tr_format!("Called at {} (UTC+8)",entry.time_label()))
                    .child(tr_format!("Unix milliseconds {}",entry.started_ms)).child(tr_format!("Timeout {} s",entry.timeout)))
                .child(row().gap_3().child(badge(entry.plugin.clone().unwrap_or_else(|| tr!("No plugin enabled").into())))
                    .child(div().text_sm().text_color(rgb(GREEN)).child(entry.changes())))
                .child(div().text_xs().text_color(rgb(MUTED)).child(if entry.dispatched_headers.is_some() {
                    tr!("Final headers are the RPC metadata captured when the sender starts, including plugin results. Automatically added HTTP/2 protocol headers are excluded.")
                } else { tr!("This request was not handed to the sender. Plugin previews are not marked as sent.") })))
            .child(column().flex_1().min_h_0().mx_5().mb_5().bg(rgb(0xffffff)).border_1().border_color(rgb(BORDER)).rounded_md().overflow_hidden()
                .child({
                    let mut tabs = row().flex_wrap().border_b_1().border_color(rgb(BORDER));
                    for (index,label) in [tr!("Sent headers"), tr!("Before plugin"), tr!("Body"), tr!("Response"), tr!("Response headers"), "Trailers"].into_iter().enumerate() {
                        tabs = tabs.child(tab(("history-tab",index),label,self.history_tab==index)
                            .on_click(cx.listener(move|this,_,window,cx|{this.history_tab=index; this.refresh_history(window,cx); cx.notify();})));
                    }
                    tabs
                })
                .child(row().px_4().py_2().min_h(px(42.)).flex_wrap().gap_3().border_b_1().border_color(rgb(BORDER))
                    .child(div().text_xs().text_color(rgb(MUTED)).child(if self.history_tab <= 1 {
                        tr!("Plain headers are visible. Reveal sensitive fields individually. Copy matches what is shown.")
                    } else { tr!("Original snapshot of this call") }))
                    .child(div().flex_1())
                    .children(reveal_buttons)
                    .child(Button::new("history-copy").ghost().small().label(tr!("Copy current content"))
                        .on_click(cx.listener(|this,_,_,cx|{
                            if let Some(entry) = &this.history_detail { cx.write_to_clipboard(ClipboardItem::new_string(entry.content(this.history_tab,&this.history_revealed))); }
                        }))))
                .child(div().flex_1().min_h_0().pt_2().child(localized_editor(&self.history_editor).readonly(true).h(relative(1.)).bordered(false).appearance(false).text_size(px(13.)))))
            .child(div().px_5().pb_3().text_xs().text_color(rgb(MUTED)).child(tr!("Up to 50 calls are kept for this session only. Plugin secrets are never written to history.")));
        panel
    }
}

#[cfg(test)]
mod tests {
    use super::HistoryEntry;
    use crate::rpc::Reply;
    use std::collections::BTreeSet;
    fn entry() -> HistoryEntry {
        HistoryEntry {
            method: "S/M".into(),
            environment: "测试".into(),
            endpoint: "localhost".into(),
            body: "{}".into(),
            reply: Reply::default(),
            original_headers: vec![
                ("ts".into(), "old".into()),
                ("x-payload".into(), "private".into()),
            ],
            dispatched_headers: None,
            plugin: Some("MD5".into()),
            tls: false,
            timeout: 5,
            started_ms: 0,
        }
    }
    #[test]
    fn failures_have_no_dispatched_metadata() {
        let mut entry = entry();
        entry.reply.body = "请填写 secret；密钥仅保留在当前会话".into();
        assert!(entry.content(0, &BTreeSet::new()).contains("请填写 secret"));
        assert!(
            entry
                .content(0, &BTreeSet::new())
                .contains("请求未交给发送进程")
        );
        assert!(!entry.content(0, &BTreeSet::new()).contains("private"));
    }
    #[test]
    fn snapshot_keeps_final_headers_and_tracks_added_changed_removed() {
        let mut entry = entry();
        let mut headers = vec![
            ("ts".into(), "1792930678339".into()),
            ("sign".into(), "actual-signature".into()),
        ];
        entry.dispatched_headers = Some(headers.clone());
        let snapshot = entry.clone();
        headers[0].1 = "changed-later".into();
        entry.dispatched_headers = Some(headers);
        assert!(
            snapshot
                .content(0, &BTreeSet::new())
                .contains("1792930678339")
        );
        assert!(snapshot.content(1, &BTreeSet::new()).contains("old"));
        assert_eq!(snapshot.changes(), "新增 1 · 修改 1 · 移除 1");
    }
    #[test]
    fn sensitive_headers_reveal_individually_while_signatures_stay_visible() {
        let mut entry = entry();
        entry.dispatched_headers = Some(vec![
            ("authorization".into(), "Bearer test-token".into()),
            ("x-secret".into(), "private-secret".into()),
            ("sign".into(), "actual-sign".into()),
            ("ts".into(), "1792930678339".into()),
        ]);
        let mut revealed = BTreeSet::new();
        let content = entry.content(0, &revealed);
        assert!(content.contains("actual-sign") && content.contains("1792930678339"));
        assert!(!content.contains("test-token") && !content.contains("private-secret"));
        revealed.insert("authorization".into());
        let content = entry.content(0, &revealed);
        assert!(content.contains("Bearer test-token"));
        assert!(!content.contains("private-secret"));
        assert_eq!(
            entry.sensitive_keys(0),
            BTreeSet::from(["authorization".into(), "x-secret".into()])
        );
        assert!(entry.content(1, &revealed).contains("private"));
    }
}
