use gpui::{
    App, ClickEvent, Context, Entity, IntoElement, MouseButton, SharedString, Window, div,
    prelude::*, px, rgb,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    input::{Input, InputState},
    tooltip::Tooltip,
};

use crate::ui::{BASE_FONT_SIZE, ThemeMode, icons};

use super::Xssh;

impl Xssh {
    pub(in crate::pages) fn label(theme: ThemeMode, text: &str) -> impl IntoElement {
        let palette = theme.palette();

        div()
            .text_size(px(12.))
            .text_color(rgb(palette.label))
            .child(text.to_string())
    }

    pub(in crate::pages) fn input(
        theme: ThemeMode,
        input_state: Entity<InputState>,
        masked: bool,
    ) -> impl IntoElement {
        let palette = theme.palette();
        let input = Input::new(&input_state)
            .w_full()
            .rounded_sm()
            .bg(rgb(palette.input_inner_bg))
            .border_color(rgb(palette.input_border))
            .text_size(px(BASE_FONT_SIZE))
            .text_color(rgb(palette.text))
            .when(masked, |this| this.pr(px(44.)));

        div().relative().w_full().child(input).when(masked, |this| {
            this.child(
                div()
                    .absolute()
                    .right(px(8.))
                    .top(px(0.))
                    .bottom(px(0.))
                    .flex()
                    .items_center()
                    .child(Self::password_eye_button(theme, input_state)),
            )
        })
    }

    pub(in crate::pages) fn password_eye_button(
        theme: ThemeMode,
        input_state: Entity<InputState>,
    ) -> impl IntoElement {
        let palette = theme.palette();

        div()
            .id("password-eye-toggle")
            .flex()
            .items_center()
            .justify_center()
            .size(px(28.))
            .rounded_sm()
            .child(icons::password_eye::icon(16., palette.muted))
            .hover(move |style| style.bg(rgb(palette.panel_hover)))
            .active(move |style| style.bg(rgb(palette.button_bg)))
            .on_mouse_down(MouseButton::Left, {
                let input_state = input_state.clone();
                move |_, window, cx| {
                    input_state.update(cx, |state, cx| {
                        state.set_masked(false, window, cx);
                    });
                }
            })
            .on_mouse_up(MouseButton::Left, move |_, window, cx| {
                input_state.update(cx, |state, cx| {
                    state.set_masked(true, window, cx);
                });
            })
    }

    pub(in crate::pages) fn field(
        theme: ThemeMode,
        label: &str,
        input: Entity<InputState>,
        masked: bool,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .w_full()
            .child(Self::label(theme, label))
            .child(Self::input(theme, input, masked))
    }

    pub(in crate::pages::index) fn primary_button(
        theme: ThemeMode,
        id: &'static str,
        text: &'static str,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &ClickEvent, &mut Window, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        let view = cx.entity();
        let palette = theme.palette();

        Button::new(id)
            .primary()
            .icon(icons::add::button_icon(palette.primary_text))
            .label(text)
            .bg(rgb(palette.primary_bg))
            .text_color(rgb(palette.primary_text))
            .rounded_sm()
            .text_size(px(BASE_FONT_SIZE))
            .on_click(move |event, window, cx| {
                view.update(cx, |this, cx| on_click(this, event, window, cx));
            })
    }

    pub(in crate::pages::index) fn secondary_button(
        theme: ThemeMode,
        id: &'static str,
        text: &'static str,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &ClickEvent, &mut Window, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        let view = cx.entity();
        let palette = theme.palette();

        Button::new(id)
            .label(text)
            .bg(rgb(palette.button_bg))
            .border_color(rgb(palette.button_border))
            .text_color(rgb(palette.text))
            .rounded_sm()
            .text_size(px(BASE_FONT_SIZE))
            .on_click(move |event, window, cx| {
                view.update(cx, |this, cx| on_click(this, event, window, cx));
            })
    }

    pub(in crate::pages::index) fn server_icon(theme: ThemeMode) -> impl IntoElement {
        let palette = theme.palette();

        div()
            .flex()
            .items_center()
            .justify_center()
            .size(px(22.))
            .rounded_sm()
            .bg(rgb(palette.icon_bg))
            .child(icons::server::icon(17., palette.text))
    }

    pub(in crate::pages::index) fn host_action_button(
        theme: ThemeMode,
        id: SharedString,
        tooltip: &'static str,
        icon: impl IntoElement,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        let palette = theme.palette();
        let tooltip = SharedString::from(tooltip);

        div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .size(px(28.))
            .rounded_sm()
            .border_1()
            .border_color(rgb(palette.button_border))
            .bg(rgb(palette.button_bg))
            .child(icon)
            .hover(move |style| {
                style
                    .bg(rgb(palette.panel_hover))
                    .border_color(rgb(palette.card_active_border))
            })
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            .on_click(on_click)
    }
}
