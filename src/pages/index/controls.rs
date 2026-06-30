use gpui::{
    App, ClickEvent, Context, Entity, IntoElement, MouseButton, SharedString, Window, div,
    prelude::*, px, rgb,
};
use gpui_component::{
    Icon, Sizable,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
};

use crate::ui::{AppThemeId, BASE_FONT_SIZE, icons};

use super::Xssh;

impl Xssh {
    pub(in crate::pages) fn label(theme: AppThemeId, text: &str) -> impl IntoElement {
        let palette = theme.palette();

        div()
            .text_size(px(12.))
            .text_color(rgb(palette.label))
            .child(text.to_string())
    }

    pub(in crate::pages) fn input(
        theme: AppThemeId,
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
        theme: AppThemeId,
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
            .hover(move |style| style.bg(rgb(palette.button_hover)))
            .active(move |style| style.bg(rgb(palette.button_active)))
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
        theme: AppThemeId,
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
        theme: AppThemeId,
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
            .rounded_sm()
            .text_size(px(BASE_FONT_SIZE))
            .on_click(move |event, window, cx| {
                view.update(cx, |this, cx| on_click(this, event, window, cx));
            })
    }

    pub(in crate::pages::index) fn secondary_button(
        id: &'static str,
        text: &'static str,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &ClickEvent, &mut Window, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        let view = cx.entity();

        Button::new(id)
            .secondary()
            .label(text)
            .rounded_sm()
            .text_size(px(BASE_FONT_SIZE))
            .on_click(move |event, window, cx| {
                view.update(cx, |this, cx| on_click(this, event, window, cx));
            })
    }

    pub(in crate::pages::index) fn server_icon(theme: AppThemeId) -> impl IntoElement {
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
        id: SharedString,
        tooltip: &'static str,
        icon_path: &'static str,
        icon_color: u32,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        Button::new(id)
            .small()
            .rounded_sm()
            .icon(
                Icon::empty()
                    .path(icon_path)
                    .size_4()
                    .text_color(rgb(icon_color)),
            )
            .tooltip(tooltip)
            .on_click(on_click)
    }
}
