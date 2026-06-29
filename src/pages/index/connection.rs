use gpui::{IntoElement, div, prelude::*, px, rgb};

use crate::{
    ipc::AuthenticationMode,
    ui::{BASE_FONT_SIZE, TextKey},
};

use super::Xssh;

impl Xssh {
    pub(in crate::pages::index) fn server_view(&self, server_id: i32) -> impl IntoElement {
        let language = self.language;
        let palette = self.theme.palette();
        let server = self
            .open_tabs
            .iter()
            .find(|server| server.id == server_id)
            .or_else(|| self.servers.iter().find(|server| server.id == server_id));

        match server {
            Some(server) => div()
                .flex()
                .flex_col()
                .size_full()
                .bg(rgb(palette.app_bg))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .h(px(56.))
                        .px_5()
                        .border_b_1()
                        .border_color(rgb(palette.separator))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(Self::server_icon(self.theme))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_size(px(BASE_FONT_SIZE))
                                                .text_color(rgb(palette.text))
                                                .child(server.name.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(rgb(palette.muted))
                                                .child(format!(
                                                    "{}@{}:{}",
                                                    server.username, server.host, server.port
                                                )),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(rgb(0x65b95f))
                                .child(language.tr(TextKey::TabReady)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_5()
                        .text_size(px(14.))
                        .text_color(rgb(palette.text))
                        .child(format!(
                            "$ ssh {}@{} -p {}",
                            server.username, server.host, server.port
                        ))
                        .child(format!(
                            "{}: {}{}",
                            language.tr(TextKey::AuthenticationInfo),
                            AuthenticationMode::from_label(&server.authentication)
                                .display_label(language),
                            if server.password.is_empty() {
                                String::new()
                            } else {
                                format!(" · {}", language.tr(TextKey::PasswordSaved))
                            }
                        ))
                        .child(language.tr(TextKey::ConnectionIntro)),
                )
                .into_any_element(),
            None => div()
                .flex()
                .items_center()
                .justify_center()
                .size_full()
                .bg(rgb(palette.app_bg))
                .text_color(rgb(palette.muted))
                .child(language.tr(TextKey::MissingTab))
                .into_any_element(),
        }
    }
}
