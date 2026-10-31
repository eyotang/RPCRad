use super::*;
use crate::updater;
impl Workbench {
    pub fn show_about(&mut self, check: bool, cx: &mut Context<Self>) {
        self.about_open = true;
        self.environments_open = false;
        if check {
            self.check_update(cx);
        }
        cx.notify();
    }
    pub(super) fn about_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        column()
            .size_full()
            .bg(rgb(SURFACE))
            .child(
                row()
                    .h(px(58.))
                    .px_5()
                    .gap_3()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(tr!("About RPCRad")),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("close-about")
                            .ghost()
                            .icon(IconName::X)
                            .accessibility_label(tr!("Close About and return to requests"))
                            .tooltip(tr!("Back to requests"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.about_open = false;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                column()
                    .id("about-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_6()
                    .gap_5()
                    .child(
                        row()
                            .gap_4()
                            .child(img("brand/logo.png").size(px(64.)))
                            .child(
                                column()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(24.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("RPCRad"),
                                    )
                                    .child(tr_format!("Version {}", env!("CARGO_PKG_VERSION")))
                                    .child(
                                        div()
                                            .text_color(rgb(MUTED))
                                            .child(tr!("Local gRPC workbench")),
                                    ),
                            ),
                    )
                    .child(self.update_panel(cx))
                    .child(
                        Button::new("about-repository")
                            .ghost()
                            .label(tr!("Project homepage"))
                            .on_click(|_, _, cx| cx.open_url("https://github.com/eyotang/RPCRad")),
                    ),
            )
    }
    pub(super) fn check_update(&mut self, cx: &mut Context<Self>) {
        if self.update_receiver.is_some() {
            return;
        }
        self.update_status = tr!("Checking for updates…").into();
        let (tx, rx) = mpsc::channel();
        self.update_receiver = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(updater::check().map_err(|e| format!("{e:#}")));
        });
        cx.notify();
    }
    pub(super) fn poll_update(&mut self, cx: &mut Context<Self>) {
        if let Some(result) = self
            .update_receiver
            .as_ref()
            .and_then(|rx| rx.try_recv().ok())
        {
            self.update_receiver = None;
            match result {
                Ok(release) => {
                    self.update_status = release
                        .as_ref()
                        .map(|r| tr_format!("Version {} available", r.version))
                        .unwrap_or_else(|| tr!("You are up to date").into());
                    self.available_update = release;
                }
                Err(error) => {
                    self.update_status = tr_format!("Update check failed: {error}", error = error)
                }
            }
            cx.notify();
        }
    }
    pub(super) fn update_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        column().gap_3().border_t_1().border_color(rgb(BORDER)).pt_4()
            .child(row().justify_between().child(format!("RPCRad {}",env!("CARGO_PKG_VERSION")))
                .child(Button::new("check-update").label(tr!("Check for updates")).disabled(self.update_receiver.is_some())
                    .on_click(cx.listener(|this,_,_,cx|this.check_update(cx)))))
            .child(div().text_sm().text_color(rgb(MUTED)).child(self.update_status.clone()))
            .when_some(self.available_update.as_ref(), |s,release| {
                let version = release.version.clone();
                s.child(div().text_sm().child(release.notes.clone()))
                    .child(Button::new("download-update").primary().label(tr_format!("Download {}",release.version))
                        .on_click(cx.listener(move |_,_,_,cx|{ if let Some(url)=updater::release_page(&version) { cx.open_url(&url); } })))
            })
            .child(div().text_xs().text_color(rgb(MUTED)).child(tr!("Updates are checked at startup. Download the package for your system from the release page. Your local workspace is preserved when upgrading.")))
    }
}
