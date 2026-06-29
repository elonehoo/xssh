use gpui::{Context, Entity, IntoElement, SharedString, div, prelude::*, px, relative, rgb};
use gpui_component::input::Input;

use crate::{
    ipc::{ActiveTab, ServerResource},
    ui::{BASE_FONT_SIZE, TextKey, icons},
};

use super::XsshDemo;

impl XsshDemo {
    pub(in crate::pages::index) fn host_card(
        &self,
        server: ServerResource,
        active: bool,
        view: Entity<Self>,
    ) -> impl IntoElement {
        let server_for_click = server.clone();
        let server_for_link = server.clone();
        let server_for_edit = server.clone();
        let server_for_delete = server.clone();
        let view_for_click = view.clone();
        let view_for_link = view.clone();
        let view_for_edit = view.clone();
        let view_for_delete = view;
        let language = self.language;
        let theme = self.theme;
        let group_name = SharedString::from(format!("host-card-actions-{}", server.id));
        let palette = self.theme.palette();

        div()
            .id(SharedString::from(format!("host-card-{}", server.id)))
            .group(group_name.clone())
            .relative()
            .flex()
            .items_center()
            .gap_3()
            .flex_grow()
            .flex_shrink()
            .flex_basis(relative(0.48))
            .min_w(px(300.))
            .min_h(px(76.))
            .px_4()
            .py_3()
            .rounded_lg()
            .bg(if active {
                rgb(palette.card_active)
            } else {
                rgb(palette.card_bg)
            })
            .border_1()
            .border_color(if active {
                rgb(palette.card_active_border)
            } else {
                rgb(palette.card_border)
            })
            .hover(move |style| {
                style
                    .bg(rgb(palette.panel_hover))
                    .border_color(rgb(palette.card_active_border))
            })
            .on_click(move |_, _, cx| {
                let server = server_for_click.clone();
                view_for_click.update(cx, |this, cx| {
                    this.open_server_tab(server, cx);
                });
            })
            .child(Self::server_icon(self.theme))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_hidden()
                    .gap_1()
                    .pr(px(100.))
                    .child(
                        div()
                            .text_size(px(BASE_FONT_SIZE))
                            .text_color(rgb(palette.text))
                            .truncate()
                            .child(server.name),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(rgb(palette.muted))
                            .truncate()
                            .child(format!(
                                "{}@{}:{}",
                                server.username, server.host, server.port
                            )),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top_3()
                    .right_3()
                    .flex()
                    .items_center()
                    .gap_1()
                    .invisible()
                    .group_hover(group_name, |style| style.visible())
                    .child(Self::host_action_button(
                        self.theme,
                        SharedString::from(format!("connect-host-{}", server.id)),
                        icons::connect::icon(15., palette.text),
                        move |_, _, cx| {
                            cx.stop_propagation();
                            let server = server_for_link.clone();
                            view_for_link.update(cx, |this, cx| {
                                this.open_server_tab(server, cx);
                            });
                        },
                    ))
                    .child(Self::host_action_button(
                        self.theme,
                        SharedString::from(format!("edit-host-{}", server.id)),
                        icons::edit::icon(15., palette.text),
                        move |_, _, cx| {
                            cx.stop_propagation();
                            let server = server_for_edit.clone();
                            view_for_edit.update(cx, |this, cx| {
                                this.open_edit_host_window(server, cx);
                            });
                        },
                    ))
                    .child(Self::host_action_button(
                        self.theme,
                        SharedString::from(format!("delete-host-{}", server.id)),
                        icons::delete::icon(15., palette.danger),
                        move |_, window, cx| {
                            cx.stop_propagation();
                            let server = server_for_delete.clone();
                            Self::open_delete_host_dialog(
                                language,
                                theme,
                                server,
                                view_for_delete.clone(),
                                window,
                                cx,
                            );
                        },
                    )),
            )
    }

    pub(in crate::pages::index) fn vault_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let language = self.language;
        let palette = self.theme.palette();
        let query = self
            .search_input
            .read(cx)
            .value()
            .to_string()
            .trim()
            .to_lowercase();
        let servers = self
            .servers
            .iter()
            .filter(|server| {
                query.is_empty()
                    || server.name.to_lowercase().contains(&query)
                    || server.host.to_lowercase().contains(&query)
                    || server.username.to_lowercase().contains(&query)
            })
            .cloned()
            .collect::<Vec<_>>();
        let view = cx.entity();
        let host_cards = servers
            .clone()
            .into_iter()
            .map(|server| {
                let active = self.active_tab == ActiveTab::Server(server.id);
                self.host_card(server, active, view.clone())
            })
            .collect::<Vec<_>>();

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .gap_4()
            .bg(rgb(palette.app_bg))
            .child(
                Input::new(&self.search_input)
                    .w_full()
                    .rounded_md()
                    .bg(rgb(palette.input_bg))
                    .border_color(rgb(palette.input_border))
                    .text_size(px(BASE_FONT_SIZE))
                    .text_color(rgb(palette.text)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Self::primary_button(
                                self.theme,
                                "create-host-top-button",
                                language.tr(TextKey::CreateHost),
                                cx,
                                Self::on_open_create_host_window,
                            ))
                            .child(Self::secondary_button(
                                self.theme,
                                "terminal-button",
                                language.tr(TextKey::Terminal),
                                cx,
                                Self::on_open_first_server,
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_size(px(16.))
                            .text_color(rgb(palette.muted))
                            .child(language.tr(TextKey::SortNewest))
                            .child(icons::sort_newest::icon(14., palette.muted)),
                    ),
            )
            .child(
                div()
                    .id("server-list-scroll")
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_start()
                    .content_start()
                    .gap_3()
                    .overflow_y_scroll()
                    .size_full()
                    .when(servers.is_empty(), |list| {
                        list.child(
                            div()
                                .p_4()
                                .text_color(rgb(palette.muted))
                                .child(language.tr(TextKey::EmptyHosts)),
                        )
                    })
                    .children(host_cards),
            )
    }
}
