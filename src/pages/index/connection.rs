use std::ops::Range;

use gpui::{AnyElement, Context, IntoElement, SharedString, Window, div, prelude::*, px, rgb};
use gpui_component::v_virtual_list;

use crate::ui::TextKey;

use super::{
    Xssh,
    tabs::TerminalId,
    terminal::{TERMINAL_COLS, TERMINAL_LINE_HEIGHT, TERMINAL_ROWS},
};

const TITLEBAR_HEIGHT: f32 = 36.0;
const TERMINAL_PADDING_X: f32 = 32.0;
const TERMINAL_PADDING_Y: f32 = 32.0;
const TERMINAL_CELL_WIDTH: f32 = 7.8;
const MIN_TERMINAL_COLS: u16 = 24;
const MIN_TERMINAL_ROWS: u16 = 6;
const MAX_TERMINAL_COLS: u16 = 360;
const MAX_TERMINAL_ROWS: u16 = 180;

impl Xssh {
    pub(in crate::pages::index) fn local_terminal_view(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme.palette();
        let (cols, rows) = terminal_dimensions_for_window(window);
        self.resize_terminal_session(TerminalId::Local, cols, rows);
        self.scroll_terminal_to_bottom_if_needed(TerminalId::Local);

        div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(rgb(palette.input_inner_bg))
            .child(self.terminal_output(TerminalId::Local, cx))
    }

    pub(in crate::pages::index) fn server_view(
        &mut self,
        server_id: i32,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let language = self.language;
        let palette = self.theme.palette();
        let terminal_id = TerminalId::Server(server_id);
        let tab_exists = self
            .open_tabs
            .iter()
            .any(|tab| tab.server_id() == Some(server_id));

        match tab_exists {
            true => {
                let (cols, rows) = terminal_dimensions_for_window(window);
                self.resize_terminal_session(terminal_id, cols, rows);
                self.scroll_terminal_to_bottom_if_needed(terminal_id);

                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .overflow_hidden()
                    .bg(rgb(palette.input_inner_bg))
                    .child(self.terminal_output(terminal_id, cx))
                    .into_any_element()
            }
            false => div()
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

    fn terminal_output(&self, terminal_id: TerminalId, cx: &mut Context<Self>) -> impl IntoElement {
        let language = self.language;
        let palette = self.theme.palette();
        let focus_handle = self.focus_handle.clone();
        let terminal_list = self.terminal_sessions.get(&terminal_id).map(|session| {
            let line_sizes = session.display_line_sizes();
            let scroll_handle = session.scroll_handle.clone();
            let list_id =
                SharedString::from(format!("terminal-lines-{}", terminal_id.element_suffix()));

            v_virtual_list(
                cx.entity(),
                list_id,
                line_sizes,
                move |this, visible_range, _, _| {
                    this.terminal_line_elements(terminal_id, visible_range)
                },
            )
            .track_scroll(&scroll_handle)
            .size_full()
            .p_4()
        });

        div()
            .id(SharedString::from(format!(
                "terminal-output-{}",
                terminal_id.element_suffix()
            )))
            .track_focus(&self.focus_handle)
            .focusable()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(rgb(palette.input_inner_bg))
            .font_family("Menlo")
            .text_size(px(13.))
            .text_color(rgb(palette.text))
            .on_click(move |_, window, _| {
                window.focus(&focus_handle);
            })
            .on_key_down(cx.listener(move |this, event, window, cx| {
                this.on_terminal_key_down(terminal_id, event, window, cx);
            }))
            .when_some(terminal_list, |this, list| this.child(list))
            .when(!self.terminal_sessions.contains_key(&terminal_id), |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size_full()
                        .text_color(rgb(palette.muted))
                        .child(language.tr(TextKey::TerminalEmpty)),
                )
            })
    }

    fn terminal_line_elements(
        &self,
        terminal_id: TerminalId,
        visible_range: Range<usize>,
    ) -> Vec<AnyElement> {
        self.terminal_sessions
            .get(&terminal_id)
            .map(|session| {
                visible_range
                    .filter_map(|row| session.display_line(row))
                    .map(|line| {
                        div()
                            .h(px(TERMINAL_LINE_HEIGHT))
                            .line_height(px(TERMINAL_LINE_HEIGHT))
                            .whitespace_nowrap()
                            .child(line.to_string())
                            .into_any_element()
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn terminal_dimensions_for_window(window: &Window) -> (u16, u16) {
    let size = window.viewport_size();

    terminal_dimensions_from_pixels(size.width.to_f64() as f32, size.height.to_f64() as f32)
}

fn terminal_dimensions_from_pixels(width: f32, height: f32) -> (u16, u16) {
    let terminal_width = (width - TERMINAL_PADDING_X).max(0.0);
    let terminal_height = (height - TITLEBAR_HEIGHT - TERMINAL_PADDING_Y).max(0.0);

    let cols = (terminal_width / TERMINAL_CELL_WIDTH).floor() as u16;
    let rows = (terminal_height / TERMINAL_LINE_HEIGHT).floor() as u16;

    (
        cols.clamp(MIN_TERMINAL_COLS, MAX_TERMINAL_COLS)
            .max(TERMINAL_COLS.min(MIN_TERMINAL_COLS)),
        rows.clamp(MIN_TERMINAL_ROWS, MAX_TERMINAL_ROWS)
            .max(TERMINAL_ROWS.min(MIN_TERMINAL_ROWS)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_terminal_dimensions_from_window_size() {
        assert_eq!(terminal_dimensions_from_pixels(2048., 895.), (258, 45));
    }

    #[test]
    fn keeps_terminal_dimensions_usable_for_small_windows() {
        assert_eq!(terminal_dimensions_from_pixels(120., 100.), (24, 6));
    }
}
