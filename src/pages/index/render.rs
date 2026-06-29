use gpui::{
    App, Context, FocusHandle, Focusable, IntoElement, Render, Window, div, prelude::*, px, rgb,
};
use gpui_component::Root;

use crate::{ipc::ActiveTab, ui::BASE_FONT_SIZE};

use super::XsshDemo;

impl Focusable for XsshDemo {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for XsshDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_tab = self.active_tab.clone();
        let palette = self.theme.palette();

        div()
            .track_focus(&self.focus_handle(cx))
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(palette.app_bg))
            .text_size(px(BASE_FONT_SIZE))
            .text_color(rgb(palette.text))
            .child(self.titlebar(cx))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .size_full()
                    .child(self.sidebar(cx))
                    .child(match active_tab {
                        ActiveTab::Vault => self.vault_view(cx).into_any_element(),
                        ActiveTab::Server(server_id) => {
                            self.server_view(server_id).into_any_element()
                        }
                    }),
            )
            .children(Root::render_dialog_layer(window, cx))
    }
}
