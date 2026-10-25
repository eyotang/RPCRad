use super::*;
use crate::updater;
impl Workbench {
    pub fn show_about(&mut self, check: bool, cx: &mut Context<Self>) {
        self.about_open = true;
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
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("关于 RPCRad"))
                    .child(div().flex_1())
                    .child(
                        Button::new("close-about")
                            .ghost()
                            .icon(IconName::X)
                            .accessibility_label("关闭关于，返回调试")
                            .tooltip("返回调试")
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
                                    .child(format!("版本 {}", env!("CARGO_PKG_VERSION")))
                                    .child(
                                        div().text_color(rgb(MUTED)).child("本地 gRPC 调试工具"),
                                    ),
                            ),
                    )
                    .child(self.update_panel(cx))
                    .child(
                        Button::new("about-repository")
                            .ghost()
                            .label("项目主页")
                            .on_click(|_, _, cx| cx.open_url("https://github.com/eyotang/RPCRad")),
                    ),
            )
    }
    pub(super) fn check_update(&mut self, cx: &mut Context<Self>) {
        if self.update_receiver.is_some() {
            return;
        }
        self.update_status = "正在检查更新…".into();
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
                        .map(|r| format!("发现新版本 {}", r.version))
                        .unwrap_or_else(|| "当前已是最新版本".into());
                    self.available_update = release;
                }
                Err(error) => self.update_status = format!("检查失败：{error}"),
            }
            cx.notify();
        }
    }
    pub(super) fn update_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        column().gap_3().border_t_1().border_color(rgb(BORDER)).pt_4()
            .child(row().justify_between().child(format!("RPCRad {}",env!("CARGO_PKG_VERSION")))
                .child(Button::new("check-update").label("检查更新").disabled(self.update_receiver.is_some())
                    .on_click(cx.listener(|this,_,_,cx|this.check_update(cx)))))
            .child(div().text_sm().text_color(rgb(MUTED)).child(self.update_status.clone()))
            .when_some(self.available_update.as_ref(), |s,release| {
                let version = release.version.clone();
                s.child(div().text_sm().child(release.notes.clone()))
                    .child(Button::new("download-update").primary().label(format!("下载 {}",release.version))
                        .on_click(cx.listener(move |_,_,_,cx|{ if let Some(url)=updater::release_page(&version) { cx.open_url(&url); } })))
            })
            .child(div().text_xs().text_color(rgb(MUTED)).child("启动时自动检查。macOS 更新打开 GitHub 对应版本下载页；替换应用后保留本地工作区配置。"))
    }
}
