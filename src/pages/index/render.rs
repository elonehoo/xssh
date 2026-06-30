use gpui::{
    App, Context, FocusHandle, Focusable, IntoElement, Render, Window, div, prelude::*, px, rgb,
};
use gpui_component::Root;

use crate::ui::BASE_FONT_SIZE;

use super::{Xssh, tabs::ActiveTab};

impl Focusable for Xssh {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Xssh {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_tab = self.active_tab;
        let palette = self.theme.palette();

        div()
            .track_focus(&self.focus_handle(cx))
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(rgb(palette.app_bg))
            .text_size(px(BASE_FONT_SIZE))
            .text_color(rgb(palette.text))
            .child(self.titlebar(window, cx))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_hidden()
                    .child(match active_tab {
                        ActiveTab::Vault => div()
                            .flex()
                            .flex_row()
                            .size_full()
                            .child(self.sidebar(cx))
                            .child(self.vault_view(cx))
                            .into_any_element(),
                        ActiveTab::LocalTerminal => self.local_terminal_view(cx).into_any_element(),
                        ActiveTab::Server(server_id) => {
                            self.server_view(server_id, cx).into_any_element()
                        }
                    }),
            )
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
