use gpui::{
    App, Context, Entity, IntoElement, MouseButton, MouseMoveEvent, SharedString, Window,
    WindowControlArea, div, prelude::*, px, rgb,
};

use crate::ui::{AppThemeId, TextKey, icons};

use super::{
    Xssh,
    tabs::{ActiveTab, OpenTab},
};

#[derive(Default)]
struct TitlebarDragState {
    should_move_window: bool,
}

impl Xssh {
    fn titlebar_drag_space(
        width: Option<f32>,
        drag_state: Entity<TitlebarDragState>,
        window: &mut Window,
    ) -> impl IntoElement {
        div()
            .h_full()
            .window_control_area(WindowControlArea::Drag)
            .when_some(width, |this, width| this.flex_none().w(px(width)))
            .when(width.is_none(), |this| this.flex_1())
            .on_mouse_down(
                MouseButton::Left,
                window.listener_for(&drag_state, |state, _, _, _| {
                    state.should_move_window = true;
                }),
            )
    }

    fn tab_close_icon(
        theme: AppThemeId,
        id: SharedString,
        group_name: SharedString,
        on_close: impl Fn(&mut App) + 'static,
    ) -> impl IntoElement {
        let palette = theme.palette();
        let server_icon_group = group_name.clone();

        div()
            .id(id)
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .size(px(22.))
            .rounded_sm()
            .bg(rgb(palette.icon_bg))
            .hover(move |style| style.bg(rgb(palette.button_hover)))
            .child(
                div()
                    .absolute()
                    .top(px(0.))
                    .right(px(0.))
                    .bottom(px(0.))
                    .left(px(0.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .group_hover(server_icon_group, |style| style.invisible())
                    .child(icons::server::icon(17., palette.text)),
            )
            .child(
                div()
                    .absolute()
                    .top(px(0.))
                    .right(px(0.))
                    .bottom(px(0.))
                    .left(px(0.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .invisible()
                    .group_hover(group_name, |style| style.visible())
                    .child(icons::close::icon(12., palette.muted)),
            )
            .on_click(move |_, _, cx| {
                cx.stop_propagation();
                on_close(cx);
            })
    }

    pub(in crate::pages::index) fn local_terminal_tab(
        &self,
        active: bool,
        view: Entity<Self>,
    ) -> impl IntoElement {
        let palette = self.theme.palette();
        let group_name = SharedString::from("title-tab-local-terminal");
        let view_for_close = view.clone();
        let label = self.language.tr(TextKey::Terminal);

        div()
            .id("tab-local-terminal")
            .group(group_name.clone())
            .relative()
            .flex()
            .items_center()
            .gap_2()
            .h(px(28.))
            .max_w(px(220.))
            .px_2()
            .rounded_sm()
            .bg(if active {
                rgb(palette.tab_active)
            } else {
                rgb(palette.tab_inactive)
            })
            .border_1()
            .border_color(if active {
                rgb(palette.card_active_border)
            } else {
                rgb(palette.border)
            })
            .text_size(px(13.))
            .text_color(if active {
                rgb(palette.text)
            } else {
                rgb(palette.muted)
            })
            .child(Self::tab_close_icon(
                self.theme,
                SharedString::from("close-tab-local-terminal"),
                group_name,
                move |cx| {
                    view_for_close.update(cx, |this, cx| {
                        this.close_local_terminal_tab(cx);
                    });
                },
            ))
            .child(div().min_w(px(0.)).max_w(px(150.)).truncate().child(label))
            .hover(move |style| {
                style
                    .bg(rgb(palette.panel_hover))
                    .border_color(rgb(palette.card_active_border))
            })
            .on_click(move |_, _, cx| {
                view.update(cx, |this, cx| {
                    this.active_tab = ActiveTab::LocalTerminal;
                    cx.notify();
                });
            })
    }

    pub(in crate::pages::index) fn title_tab(
        &self,
        server_id: i32,
        label: String,
        active: bool,
        view: Entity<Self>,
    ) -> impl IntoElement {
        let palette = self.theme.palette();
        let group_name = SharedString::from(format!("title-tab-{server_id}"));
        let view_for_close = view.clone();

        div()
            .id(SharedString::from(format!("tab-{server_id}")))
            .group(group_name.clone())
            .relative()
            .flex()
            .items_center()
            .gap_2()
            .h(px(28.))
            .max_w(px(280.))
            .px_2()
            .rounded_sm()
            .bg(if active {
                rgb(palette.tab_active)
            } else {
                rgb(palette.tab_inactive)
            })
            .border_1()
            .border_color(if active {
                rgb(palette.card_active_border)
            } else {
                rgb(palette.border)
            })
            .text_size(px(13.))
            .text_color(if active {
                rgb(palette.text)
            } else {
                rgb(palette.muted)
            })
            .child(Self::tab_close_icon(
                self.theme,
                SharedString::from(format!("close-tab-{server_id}")),
                group_name,
                move |cx| {
                    view_for_close.update(cx, |this, cx| {
                        this.close_server_tab(server_id, cx);
                    });
                },
            ))
            .child(div().min_w(px(0.)).max_w(px(210.)).truncate().child(label))
            .hover(move |style| {
                style
                    .bg(rgb(palette.panel_hover))
                    .border_color(rgb(palette.card_active_border))
            })
            .on_click(move |_, _, cx| {
                view.update(cx, |this, cx| {
                    this.active_tab = ActiveTab::Server(server_id);
                    cx.notify();
                });
            })
    }

    pub(in crate::pages::index) fn titlebar(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity();
        let language = self.language;
        let palette = self.theme.palette();
        let drag_state = window.use_state(cx, |_, _| TitlebarDragState::default());
        let mut open_tabs = Vec::new();
        for tab in self.open_tabs.clone() {
            open_tabs.push(
                Self::titlebar_drag_space(Some(8.), drag_state.clone(), window).into_any_element(),
            );
            open_tabs.push(match tab {
                OpenTab::LocalTerminal => self
                    .local_terminal_tab(self.active_tab == ActiveTab::LocalTerminal, view.clone())
                    .into_any_element(),
                OpenTab::Server(server) => {
                    let active = self.active_tab == ActiveTab::Server(server.id);
                    self.title_tab(server.id, server.name, active, view.clone())
                        .into_any_element()
                }
            });
        }

        div()
            .flex()
            .flex_none()
            .relative()
            .items_center()
            .h(px(36.))
            .w_full()
            .bg(rgb(palette.titlebar_bg))
            .border_b_1()
            .border_color(rgb(palette.border))
            .on_mouse_down_out(window.listener_for(&drag_state, |state, _, _, _| {
                state.should_move_window = false;
            }))
            .on_mouse_up(
                MouseButton::Left,
                window.listener_for(&drag_state, |state, _, _, _| {
                    state.should_move_window = false;
                }),
            )
            .on_mouse_move(window.listener_for(
                &drag_state,
                |state, event: &MouseMoveEvent, window, _| {
                    if state.should_move_window && event.dragging() {
                        state.should_move_window = false;
                        window.start_window_move();
                    }
                },
            ))
            .child(Self::titlebar_drag_space(
                Some(92.),
                drag_state.clone(),
                window,
            ))
            .child(self.sidebar_toggle(view.clone()))
            .child(Self::titlebar_drag_space(
                Some(8.),
                drag_state.clone(),
                window,
            ))
            .child(
                div()
                    .id("vault-tab")
                    .flex()
                    .items_center()
                    .h(px(28.))
                    .px_2()
                    .rounded_sm()
                    .border_1()
                    .border_color(if self.active_tab == ActiveTab::Vault {
                        rgb(palette.card_active_border)
                    } else {
                        rgb(palette.border)
                    })
                    .bg(if self.active_tab == ActiveTab::Vault {
                        rgb(palette.tab_active)
                    } else {
                        rgb(palette.tab_inactive)
                    })
                    .gap_1()
                    .text_size(px(13.))
                    .text_color(rgb(palette.text))
                    .child(icons::vault::icon(15., palette.text))
                    .child(language.tr(TextKey::Vault))
                    .hover(move |style| style.bg(rgb(palette.tab_active)))
                    .on_click(cx.listener(Self::on_vault_tab)),
            )
            .child(Self::titlebar_drag_space(
                Some(8.),
                drag_state.clone(),
                window,
            ))
            .child(
                div()
                    .h(px(18.))
                    .w(px(1.))
                    .bg(rgb(palette.border))
                    .opacity(0.65),
            )
            .child(Self::titlebar_drag_space(
                Some(8.),
                drag_state.clone(),
                window,
            ))
            .children(open_tabs)
            .child(Self::titlebar_drag_space(None, drag_state.clone(), window))
            .child(self.upload_log_menu(view.clone()))
            .child(Self::titlebar_drag_space(Some(8.), drag_state, window))
    }
}
