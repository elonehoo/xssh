use gpui::{
    Context, Entity, IntoElement, SharedString, WindowControlArea, div, prelude::*, px, rgb,
};

use crate::ui::{BASE_FONT_SIZE, TextKey, ThemeMode, icons};

use super::{
    Xssh,
    tabs::{ActiveTab, OpenTab},
};

impl Xssh {
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
            .child(Self::server_icon(self.theme).into_any_element())
            .child(div().min_w(px(0.)).max_w(px(150.)).truncate().child(label))
            .child(
                div()
                    .id("close-tab-local-terminal")
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(18.))
                    .rounded_sm()
                    .invisible()
                    .group_hover(group_name, |style| style.visible())
                    .hover(move |style| style.bg(rgb(palette.button_bg)))
                    .child(icons::close::icon(12., palette.muted))
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        view_for_close.update(cx, |this, cx| {
                            this.close_local_terminal_tab(cx);
                        });
                    }),
            )
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
            .child(Self::server_icon(self.theme).into_any_element())
            .child(div().min_w(px(0.)).max_w(px(210.)).truncate().child(label))
            .child(
                div()
                    .id(SharedString::from(format!("close-tab-{server_id}")))
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(18.))
                    .rounded_sm()
                    .invisible()
                    .group_hover(group_name, |style| style.visible())
                    .hover(move |style| style.bg(rgb(palette.button_bg)))
                    .child(icons::close::icon(12., palette.muted))
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        view_for_close.update(cx, |this, cx| {
                            this.close_server_tab(server_id, cx);
                        });
                    }),
            )
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

    pub(in crate::pages::index) fn titlebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let language = self.language;
        let palette = self.theme.palette();
        let open_tabs = self
            .open_tabs
            .clone()
            .into_iter()
            .map(|tab| match tab {
                OpenTab::LocalTerminal => self
                    .local_terminal_tab(self.active_tab == ActiveTab::LocalTerminal, view.clone())
                    .into_any_element(),
                OpenTab::Server(server) => {
                    let active = self.active_tab == ActiveTab::Server(server.id);
                    self.title_tab(server.id, server.name, active, view.clone())
                        .into_any_element()
                }
            })
            .collect::<Vec<_>>();

        div()
            .flex()
            .flex_none()
            .items_center()
            .h(px(36.))
            .w_full()
            .pl(px(92.))
            .pr_3()
            .gap_2()
            .bg(rgb(palette.titlebar_bg))
            .border_b_1()
            .border_color(rgb(palette.border))
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
            .child(
                div()
                    .h(px(18.))
                    .w(px(1.))
                    .bg(rgb(palette.border))
                    .opacity(0.65),
            )
            .children(open_tabs)
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .window_control_area(WindowControlArea::Drag),
            )
    }

    pub(in crate::pages::index) fn sidebar_item(
        theme: ThemeMode,
        label: &'static str,
        active: bool,
    ) -> impl IntoElement {
        let palette = theme.palette();

        div()
            .flex()
            .items_center()
            .gap_3()
            .h(px(32.))
            .px_4()
            .rounded_md()
            .bg(if active {
                rgb(palette.panel_hover)
            } else {
                rgb(palette.sidebar_bg)
            })
            .text_color(if active {
                rgb(palette.text)
            } else {
                rgb(palette.muted)
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(20.))
                    .child(icons::server::icon(
                        18.,
                        if active { palette.text } else { palette.muted },
                    )),
            )
            .child(div().text_size(px(BASE_FONT_SIZE)).child(label))
    }

    pub(in crate::pages::index) fn settings_button(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme.palette();

        div()
            .id("settings-sidebar-button")
            .flex()
            .items_center()
            .gap_3()
            .h(px(32.))
            .px_3()
            .rounded_md()
            .text_size(px(BASE_FONT_SIZE))
            .text_color(rgb(palette.muted))
            .child(icons::settings::icon(18., palette.muted))
            .child(self.language.tr(TextKey::Settings))
            .hover(move |style| {
                style
                    .bg(rgb(palette.panel_hover))
                    .text_color(rgb(palette.text))
            })
            .on_click(cx.listener(Self::on_open_settings_window))
    }

    pub(in crate::pages::index) fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme.palette();

        div()
            .flex()
            .flex_col()
            .w(px(280.))
            .h_full()
            .p_3()
            .bg(rgb(palette.sidebar_bg))
            .border_r_1()
            .border_color(rgb(palette.border))
            .child(Self::sidebar_item(
                self.theme,
                self.language.tr(TextKey::Hosts),
                true,
            ))
            .child(div().flex_1())
            .child(div().h(px(1.)).w_full().bg(rgb(palette.separator)))
            .child(div().pt_3().child(self.settings_button(cx)))
    }
}
