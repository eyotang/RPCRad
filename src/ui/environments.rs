use super::*;
use crate::variables::{Environment, Variable, Variables};
use gpui_kit::component::menu::PopupMenuItem;

struct VariableRow {
    name: Entity<InputState>,
    value: Entity<InputState>,
    sensitive: bool,
}

pub(super) struct EnvironmentUi {
    data: Variables,
    editing: Option<u64>,
    name: Entity<InputState>,
    rows: Vec<VariableRow>,
    subscriptions: Vec<Subscription>,
}
impl EnvironmentUi {
    pub(super) fn refresh_language(&mut self, window: &mut Window, cx: &mut Context<Workbench>) {
        self.name.update(cx, |s, cx| {
            s.set_placeholder(tr!("Environment name"), window, cx)
        });
        for row in &self.rows {
            row.name
                .update(cx, |s, cx| s.set_placeholder(tr!("e.g. appid"), window, cx));
            row.value.update(cx, |s, cx| {
                s.set_placeholder(
                    if row.sensitive {
                        tr!("Session only; enter again after restart")
                    } else {
                        tr!("Value or {{other_variable}}")
                    },
                    window,
                    cx,
                )
            });
        }
    }
    pub fn new(mut data: Variables, window: &mut Window, cx: &mut Context<Workbench>) -> Self {
        if !data.environments.iter().any(|e| Some(e.id) == data.active) {
            data.active = None;
        }
        let name = cx.new(|cx| InputState::new(window, cx).placeholder(tr!("Environment name")));
        let mut state = Self {
            editing: data.active,
            data,
            name,
            rows: vec![],
            subscriptions: vec![],
        };
        state.load(window, cx);
        state
    }
    pub fn snapshot(&self, cx: &App) -> Variables {
        let mut data = self.data.clone();
        let variables = self
            .rows
            .iter()
            .map(|row| Variable {
                name: row.name.read(cx).value().to_string(),
                value: row.value.read(cx).value().to_string(),
                sensitive: row.sensitive,
            })
            .collect();
        if let Some(id) = self.editing {
            if let Some(env) = data.environments.iter_mut().find(|e| e.id == id) {
                let name = self.name.read(cx).value();
                env.name = if name.trim().is_empty() {
                    tr!("Untitled environment").into()
                } else {
                    name.to_string()
                };
                env.variables = variables;
            }
        } else {
            data.globals = variables;
        }
        data
    }
    fn load(&mut self, window: &mut Window, cx: &mut Context<Workbench>) {
        self.subscriptions.clear();
        let (name, variables) = self
            .editing
            .and_then(|id| self.data.environments.iter().find(|e| e.id == id))
            .map_or((tr!("Global variables"), &self.data.globals), |env| {
                (env.name.as_str(), &env.variables)
            });
        self.name.update(cx, |s, cx| s.set_value(name, window, cx));
        self.rows = variables
            .iter()
            .map(|v| VariableRow {
                name: cx.new(|cx| {
                    InputState::new(window, cx)
                        .placeholder(tr!("e.g. appid"))
                        .default_value(v.name.clone())
                }),
                value: cx.new(|cx| {
                    InputState::new(window, cx)
                        .placeholder(if v.sensitive {
                            tr!("Session only; enter again after restart")
                        } else {
                            tr!("Value or {{other_variable}}")
                        })
                        .default_value(v.value.clone())
                        .masked(v.sensitive)
                }),
                sensitive: v.sensitive,
            })
            .collect();
        for input in std::iter::once(&self.name)
            .chain(self.rows.iter().flat_map(|row| [&row.name, &row.value]))
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
}

impl Workbench {
    pub(super) fn open_environments(&mut self, cx: &mut Context<Self>) {
        self.environments_open = true;
        self.about_open = false;
        self.history_detail = None;
        cx.notify();
    }
    fn edit_environment(&mut self, id: Option<u64>, window: &mut Window, cx: &mut Context<Self>) {
        self.environments.data = self.environments.snapshot(cx);
        self.environments.editing = id;
        self.environments.load(window, cx);
        cx.notify();
    }
    fn activate_environment(&mut self, id: Option<u64>, cx: &mut Context<Self>) {
        if self.busy || self.discovering {
            return;
        }
        self.environments.data.active = id;
        self.plugins.invalidate();
        self.save(cx);
        self.notice = tr_format!(
            "Active environment: {}",
            self.environments.snapshot(cx).active_name()
        );
        cx.notify();
    }
    pub(super) fn environment_selector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let data = self.environments.snapshot(cx);
        let weak = cx.weak_entity();
        Button::new("environment-selector")
            .ghost()
            .label(data.active_name().to_string())
            .icon(IconName::ChevronDown)
            .disabled(self.busy || self.discovering)
            .accessibility_label(tr!("Switch environment"))
            .dropdown_menu(move |mut menu, _, _| {
                for (id, name) in
                    std::iter::once((None, tr!("No environment · Globals").to_string())).chain(
                        data.environments
                            .iter()
                            .map(|e| (Some(e.id), e.name.clone())),
                    )
                {
                    let target = weak.clone();
                    menu = menu.item(
                        PopupMenuItem::new(name)
                            .checked(data.active == id)
                            .on_click(move |_, _, cx| {
                                let _ =
                                    target.update(cx, |this, cx| this.activate_environment(id, cx));
                            }),
                    );
                }
                let target = weak.clone();
                menu.separator().item(
                    PopupMenuItem::new(tr!("Manage environments and variables…")).on_click(
                        move |_, _, cx| {
                            let _ = target.update(cx, |this, cx| this.open_environments(cx));
                        },
                    ),
                )
            })
    }
    fn add_environment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.environments.data = self.environments.snapshot(cx);
        let id = self
            .environments
            .data
            .environments
            .iter()
            .map(|e| e.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        self.environments.data.environments.push(Environment {
            id,
            name: tr_format!("New environment {id}", id = id),
            variables: vec![],
        });
        self.environments.editing = Some(id);
        self.environments.load(window, cx);
        self.save(cx);
        cx.notify();
    }
    fn add_variable(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.environments.data = self.environments.snapshot(cx);
        let data = &mut self.environments.data;
        let rows = if let Some(id) = self.environments.editing {
            let Some(env) = data.environments.iter_mut().find(|e| e.id == id) else {
                return;
            };
            &mut env.variables
        } else {
            &mut data.globals
        };
        rows.push(Variable::default());
        self.environments.load(window, cx);
        if let Some(row) = self.environments.rows.last() {
            row.name.update(cx, |s, cx| s.focus(window, cx));
        }
        self.save(cx);
        cx.notify();
    }
    pub(super) fn environments_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let data = self.environments.snapshot(cx);
        let editing = self.environments.editing;
        let mut scopes = column()
            .id("environment-scopes")
            .overflow_y_scroll()
            .w(px(180.))
            .flex_shrink_0()
            .gap_2()
            .p_4()
            .border_r_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child(tr!("Variable scope")),
            );
        for (index, (id, name)) in std::iter::once((None, tr!("Global variables").to_string()))
            .chain(
                data.environments
                    .iter()
                    .map(|e| (Some(e.id), e.name.clone())),
            )
            .enumerate()
        {
            scopes = scopes.child(
                Button::new(("environment-scope", index))
                    .ghost()
                    .label(format!("{}{}", if editing == id { "✓ " } else { "" }, name))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.edit_environment(id, window, cx)
                    })),
            );
        }
        scopes = scopes.child(
            Button::new("add-environment")
                .ghost()
                .icon(IconName::Plus)
                .label(tr!("New environment"))
                .on_click(cx.listener(|this, _, window, cx| this.add_environment(window, cx))),
        );
        let mut editor = column().flex_1().min_w_0().p_5().gap_4();
        if let Some(id) = editing {
            editor = editor.child(
                row()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .child(localized_input(&self.environments.name)),
                    )
                    .child(
                        Button::new("use-environment")
                            .label(if data.active == Some(id) {
                                tr!("Active environment")
                            } else {
                                tr!("Use this environment")
                            })
                            .disabled(self.busy || self.discovering)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.activate_environment(Some(id), cx)
                            })),
                    )
                    .child(
                        Button::new("delete-environment")
                            .ghost()
                            .icon(IconName::Trash)
                            .tooltip(tr!("Delete this environment and its variables"))
                            .disabled(self.busy || self.discovering)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.environments.data = this.environments.snapshot(cx);
                                this.environments.data.environments.retain(|e| e.id != id);
                                if this.environments.data.active == Some(id) {
                                    this.environments.data.active = None;
                                }
                                this.environments.editing = None;
                                this.environments.load(window, cx);
                                this.plugins.invalidate();
                                this.save(cx);
                                cx.notify();
                            })),
                    ),
            );
        } else {
            editor = editor.child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(tr!("Global variables")),
            );
        }
        editor = editor
            .child(div().text_sm().text_color(rgb(MUTED)).child(
                tr!("Reference values with {{name}}. Environment variables override globals with the same name. Without an environment, only globals are used."),
            ))
            .child(
                row()
                    .gap_3()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child(div().w(px(180.)).child(tr!("Name")))
                    .child(div().flex_1().child(tr!("Value")))
                    .child(div().w(px(158.)).child(tr!("Storage"))),
            );
        let mut rows = column()
            .id("variables-scroll")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .gap_2();
        for (index, v) in self.environments.rows.iter().enumerate() {
            rows = rows.child(
                row()
                    .gap_3()
                    .child(div().w(px(180.)).flex_shrink_0().child(localized_input(&v.name)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(localized_input(&v.value).when(v.sensitive, |s| s.suffix(mask_button(&v.value, cx)))),
                    )
                    .child(
                        Button::new(("variable-sensitive", index))
                            .ghost()
                            .small()
                            .label(if v.sensitive {
                                tr!("Secret · Session")
                            } else {
                                tr!("Plain · Saved")
                            })
                            .tooltip(tr!("Toggle plain and sensitive variables; sensitive values are never saved to disk"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let row = &mut this.environments.rows[index];
                                row.sensitive = !row.sensitive;
                                row.value
                                    .update(cx, |s, cx| s.set_masked(row.sensitive, window, cx));
                                this.plugins.invalidate();
                                this.save(cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new(("delete-variable", index))
                            .ghost()
                            .small()
                            .icon(IconName::Trash)
                            .tooltip(tr!("Delete variable"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.environments.rows.remove(index);
                                this.environments.data = this.environments.snapshot(cx);
                                this.environments.load(window, cx);
                                this.plugins.invalidate();
                                this.save(cx);
                                cx.notify();
                            })),
                    ),
            );
        }
        if self.environments.rows.is_empty() {
            rows = rows.child(div().py_6().text_color(rgb(MUTED)).child(
                tr!("No variables yet. Add endpoint, appid or user_id and set their values for each environment."),
            ));
        }
        editor=editor.child(rows).child(Button::new("add-variable").ghost().icon(IconName::Plus).label(tr!("Add variable"))
            .on_click(cx.listener(|this,_,window,cx|this.add_variable(window,cx))))
            .child(div().text_xs().text_color(rgb(MUTED)).child(tr!("Use variables in the address, metadata, Bearer token, plugin settings and Secret. In bodies, use \"{{user_id}}\" for strings and {{page_size}} for numbers or JSON. Sensitive values last only for this session.")));
        if let Err(error) = data.resolver() {
            editor = editor.child(div().text_color(rgb(0xc35246)).child(error.to_string()));
        }
        column()
            .size_full()
            .child(
                row()
                    .h(px(58.))
                    .px_5()
                    .gap_3()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        Button::new("environment-back")
                            .ghost()
                            .label(tr!("Back to requests"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.environments_open = false;
                                this.save(cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(tr!("Environments and variables")),
                    )
                    .child(div().flex_1())
                    .child(badge(tr!("Autosaved"))),
            )
            .child(
                row()
                    .items_start()
                    .flex_1()
                    .min_h_0()
                    .child(scopes.h_full())
                    .child(editor.h_full()),
            )
    }
}
