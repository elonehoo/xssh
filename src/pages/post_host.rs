use anyhow::{Context as AnyhowContext, Result, anyhow};
use gpui::{
    App, ClickEvent, Context, Entity, FocusHandle, Focusable, IntoElement, MouseButton, Render,
    SharedString, Subscription, Window, div, prelude::*, px, rgb,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    input::{Input, InputState},
    select::{Select, SelectEvent, SelectState},
};

use crate::{
    ipc::{AuthenticationMode, ServerDraft, ServerResource},
    ui::{BASE_FONT_SIZE, Language, TextKey, ThemeMode, icons},
};

use super::Xssh;

pub(super) struct CreateHostWindow {
    parent: Entity<Xssh>,
    server_id: Option<i32>,
    language: Language,
    theme: ThemeMode,
    name_input: Entity<InputState>,
    host_input: Entity<InputState>,
    port_input: Entity<InputState>,
    username_input: Entity<InputState>,
    password_input: Entity<InputState>,
    password_revealed: bool,
    auth_select: Entity<SelectState<Vec<&'static str>>>,
    selected_authentication: AuthenticationMode,
    status: SharedString,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl CreateHostWindow {
    pub(super) fn new(
        parent: Entity<Xssh>,
        language: Language,
        theme: ThemeMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::build(parent, None, language, theme, window, cx)
    }

    pub(super) fn edit(
        parent: Entity<Xssh>,
        language: Language,
        theme: ThemeMode,
        server: ServerResource,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::build(parent, Some(server), language, theme, window, cx)
    }

    pub(super) fn name_focus_handle(&self, cx: &App) -> FocusHandle {
        self.name_input.focus_handle(cx)
    }

    fn build(
        parent: Entity<Xssh>,
        server: Option<ServerResource>,
        language: Language,
        theme: ThemeMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let server_id = server.as_ref().map(|server| server.id);
        let selected_authentication = server
            .as_ref()
            .map(|server| AuthenticationMode::from_label(&server.authentication))
            .unwrap_or(AuthenticationMode::ManualPassword);
        let name_value = server.as_ref().map(|server| server.name.clone());
        let host_value = server.as_ref().map(|server| server.host.clone());
        let port_value = server
            .as_ref()
            .map(|server| server.port.to_string())
            .unwrap_or_else(|| "22".to_string());
        let username_value = server.as_ref().map(|server| server.username.clone());
        let password_value = server.as_ref().map(|server| server.password.clone());

        let name_input = cx.new(|cx| {
            let input = InputState::new(window, cx).placeholder("Production Bastion");
            if let Some(value) = name_value {
                input.default_value(value)
            } else {
                input
            }
        });
        let host_input = cx.new(|cx| {
            let input = InputState::new(window, cx).placeholder("bastion.example.com");
            if let Some(value) = host_value {
                input.default_value(value)
            } else {
                input
            }
        });
        let port_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("22")
                .default_value(port_value)
        });
        let username_input = cx.new(|cx| {
            let input = InputState::new(window, cx).placeholder("root");
            if let Some(value) = username_value {
                input.default_value(value)
            } else {
                input
            }
        });
        let password_input = cx.new(|cx| {
            let input = InputState::new(window, cx)
                .placeholder("password")
                .masked(true);
            if let Some(value) = password_value {
                input.default_value(value)
            } else {
                input
            }
        });
        let auth_select = cx.new(|cx| {
            SelectState::new(
                language.auth_options(),
                Some(selected_authentication.selected_index()),
                window,
                cx,
            )
        });
        let _subscriptions = vec![cx.subscribe(&auth_select, Self::on_auth_select_event)];

        Self {
            parent,
            server_id,
            language,
            theme,
            name_input,
            host_input,
            port_input,
            username_input,
            password_input,
            password_revealed: false,
            auth_select,
            selected_authentication,
            status: language
                .tr(if server_id.is_some() {
                    TextKey::EditDialogReady
                } else {
                    TextKey::DialogReady
                })
                .into(),
            focus_handle: cx.focus_handle(),
            _subscriptions,
        }
    }

    fn read_draft(&self, cx: &mut Context<Self>) -> Result<ServerDraft> {
        let name = self
            .name_input
            .read(cx)
            .value()
            .to_string()
            .trim()
            .to_string();
        let host = self
            .host_input
            .read(cx)
            .value()
            .to_string()
            .trim()
            .to_string();
        let port_text = self
            .port_input
            .read(cx)
            .value()
            .to_string()
            .trim()
            .to_string();
        let username = self
            .username_input
            .read(cx)
            .value()
            .to_string()
            .trim()
            .to_string();
        let password = self
            .password_input
            .read(cx)
            .value()
            .to_string()
            .trim()
            .to_string();
        let selected_authentication = self
            .auth_select
            .read(cx)
            .selected_value()
            .map(|value| AuthenticationMode::from_label(value))
            .unwrap_or(self.selected_authentication);

        if name.is_empty() {
            return Err(anyhow!("{}", self.required_message(TextKey::Name)));
        }

        if host.is_empty() {
            return Err(anyhow!("{}", self.required_message(TextKey::Hostname)));
        }

        if username.is_empty() {
            return Err(anyhow!("{}", self.required_message(TextKey::Username)));
        }

        if selected_authentication == AuthenticationMode::ManualPassword && password.is_empty() {
            return Err(anyhow!("{}", self.required_message(TextKey::Password)));
        }

        let port = port_text
            .parse::<u16>()
            .with_context(|| self.port_number_message(&port_text))?;

        if port == 0 {
            return Err(anyhow!("{}", self.port_positive_message()));
        }

        Ok(ServerDraft {
            name,
            host,
            port,
            username,
            authentication: selected_authentication.storage_label().to_string(),
            password,
        })
    }

    fn required_message(&self, key: TextKey) -> String {
        match self.language {
            Language::Zh => format!("{} 不能为空", self.language.tr(key)),
            Language::En => format!("{} is required", self.language.tr(key)),
            Language::Ja => format!("{} は必須です", self.language.tr(key)),
        }
    }

    fn port_number_message(&self, port_text: &str) -> String {
        match self.language {
            Language::Zh => format!("Port 必须是 1 到 65535 的数字，当前值是 {port_text:?}"),
            Language::En => {
                format!("Port must be a number from 1 to 65535. Current value: {port_text:?}")
            }
            Language::Ja => {
                format!("Port は 1 から 65535 の数字で入力してください。現在の値: {port_text:?}")
            }
        }
    }

    fn port_positive_message(&self) -> &'static str {
        match self.language {
            Language::Zh => "Port 必须大于 0",
            Language::En => "Port must be greater than 0",
            Language::Ja => "Port は 0 より大きい必要があります",
        }
    }

    fn on_save(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let result = self.read_draft(cx).and_then(|draft| match self.server_id {
            Some(server_id) => self.parent.update(cx, |parent, cx| {
                parent.update_server_from_draft(server_id, draft, cx)
            }),
            None => self
                .parent
                .update(cx, |parent, cx| parent.add_server_from_draft(draft, cx)),
        });

        match result {
            Ok(()) => window.remove_window(),
            Err(error) => {
                self.status = match self.language {
                    Language::Zh => format!("保存失败：{error}"),
                    Language::En => format!("Save failed: {error}"),
                    Language::Ja => format!("保存に失敗しました: {error}"),
                }
                .into();
                cx.notify();
            }
        }
    }

    fn on_cancel(&mut self, _: &ClickEvent, window: &mut Window, _: &mut Context<Self>) {
        window.remove_window();
    }

    fn on_auth_select_event(
        &mut self,
        _: Entity<SelectState<Vec<&'static str>>>,
        event: &SelectEvent<Vec<&'static str>>,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(Some(value)) = event else {
            return;
        };

        self.selected_authentication = AuthenticationMode::from_label(value);
        cx.notify();
    }

    fn button(
        theme: ThemeMode,
        id: &'static str,
        text: &'static str,
        primary: bool,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &ClickEvent, &mut Window, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        let view = cx.entity();
        let palette = theme.palette();
        let button = Button::new(id)
            .label(text)
            .rounded_sm()
            .text_size(px(BASE_FONT_SIZE))
            .bg(if primary {
                rgb(palette.primary_bg)
            } else {
                rgb(palette.button_bg)
            })
            .border_color(if primary {
                rgb(palette.primary_bg)
            } else {
                rgb(palette.button_border)
            })
            .text_color(if primary {
                rgb(palette.primary_text)
            } else {
                rgb(palette.text)
            });

        let button = if primary { button.primary() } else { button };

        button.on_click(move |event, window, cx| {
            view.update(cx, |this, cx| on_click(this, event, window, cx));
        })
    }

    fn password_field(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .w_full()
            .child(Xssh::label(self.theme, self.language.tr(TextKey::Password)))
            .child(Self::password_input(
                self.theme,
                self.password_input.clone(),
                self.password_revealed,
                cx,
            ))
    }

    fn password_input(
        theme: ThemeMode,
        input_state: Entity<InputState>,
        password_revealed: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = theme.palette();
        let view = cx.entity();
        let input = Input::new(&input_state)
            .w_full()
            .rounded_sm()
            .bg(rgb(palette.input_inner_bg))
            .border_color(rgb(palette.input_border))
            .text_size(px(BASE_FONT_SIZE))
            .text_color(rgb(palette.text))
            .pr(px(44.));

        div().relative().w_full().child(input).child(
            div()
                .absolute()
                .right(px(8.))
                .top(px(0.))
                .bottom(px(0.))
                .flex()
                .items_center()
                .child(Self::password_eye_button(
                    theme,
                    input_state,
                    view,
                    password_revealed,
                )),
        )
    }

    fn password_eye_button(
        theme: ThemeMode,
        input_state: Entity<InputState>,
        view: Entity<Self>,
        password_revealed: bool,
    ) -> impl IntoElement {
        let palette = theme.palette();
        let icon = if password_revealed {
            icons::password_eye_off::icon(16., palette.muted).into_any_element()
        } else {
            icons::password_eye::icon(16., palette.muted).into_any_element()
        };

        div()
            .id("password-eye-toggle")
            .flex()
            .items_center()
            .justify_center()
            .size(px(28.))
            .rounded_sm()
            .child(icon)
            .hover(move |style| style.bg(rgb(palette.panel_hover)))
            .active(move |style| style.bg(rgb(palette.button_bg)))
            .on_mouse_down(MouseButton::Left, {
                let input_state = input_state.clone();
                let view = view.clone();
                move |_, window, cx| {
                    input_state.update(cx, |state, cx| {
                        state.set_masked(false, window, cx);
                    });
                    view.update(cx, |this, cx| {
                        this.password_revealed = true;
                        cx.notify();
                    });
                }
            })
            .on_mouse_up(MouseButton::Left, move |_, window, cx| {
                input_state.update(cx, |state, cx| {
                    state.set_masked(true, window, cx);
                });
                view.update(cx, |this, cx| {
                    this.password_revealed = false;
                    cx.notify();
                });
            })
    }
}

impl Focusable for CreateHostWindow {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CreateHostWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let language = self.language;
        let palette = self.theme.palette();
        let title = if self.server_id.is_some() {
            language.tr(TextKey::EditHost)
        } else {
            language.tr(TextKey::CreateHost)
        };

        div()
            .track_focus(&self.focus_handle(cx))
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(palette.panel_bg))
            .text_size(px(BASE_FONT_SIZE))
            .text_color(rgb(palette.text))
            .child(
                div()
                    .flex_none()
                    .px_4()
                    .pt_4()
                    .pb_3()
                    .text_size(px(BASE_FONT_SIZE))
                    .text_color(rgb(palette.text))
                    .child(title),
            )
            .child(
                div()
                    .id("create-host-form-scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_3()
                    .overflow_y_scroll()
                    .px_4()
                    .pb_3()
                    .child(Xssh::field(
                        self.theme,
                        language.tr(TextKey::Name),
                        self.name_input.clone(),
                        false,
                    ))
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .child(div().flex_1().child(Xssh::field(
                                self.theme,
                                language.tr(TextKey::Hostname),
                                self.host_input.clone(),
                                false,
                            )))
                            .child(div().w(px(120.)).child(Xssh::field(
                                self.theme,
                                language.tr(TextKey::Port),
                                self.port_input.clone(),
                                false,
                            ))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(Xssh::label(
                                self.theme,
                                language.tr(TextKey::Authentication),
                            ))
                            .child(
                                Select::new(&self.auth_select)
                                    .w_full()
                                    .rounded_sm()
                                    .bg(rgb(palette.input_inner_bg))
                                    .border_color(rgb(palette.input_border))
                                    .text_size(px(BASE_FONT_SIZE))
                                    .text_color(rgb(palette.text))
                                    .placeholder(language.tr(TextKey::SelectAuthentication)),
                            ),
                    )
                    .child(Xssh::field(
                        self.theme,
                        language.tr(TextKey::Username),
                        self.username_input.clone(),
                        false,
                    ))
                    .child(self.password_field(cx)),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_t_1()
                    .border_color(rgb(palette.separator))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(12.))
                            .text_color(rgb(palette.muted))
                            .child(self.status.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(Self::button(
                                self.theme,
                                "cancel-create-host-button",
                                language.tr(TextKey::Cancel),
                                false,
                                cx,
                                Self::on_cancel,
                            ))
                            .child(Self::button(
                                self.theme,
                                "save-host-button",
                                language.tr(TextKey::Save),
                                true,
                                cx,
                                Self::on_save,
                            )),
                    ),
            )
    }
}
