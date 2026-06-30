use std::ops::Range;

use gpui::{
    AnyElement, Bounds, Context, IntoElement, Pixels, SharedString, canvas, div, prelude::*, px,
    rgb,
};
use gpui_component::v_virtual_list;

use crate::{ipc::TerminalSize, ui::TextKey};

use super::{
    Xssh,
    tabs::TerminalId,
    terminal::{TERMINAL_COLS, TERMINAL_LINE_HEIGHT, TERMINAL_ROWS},
};

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
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let terminal_palette = self.active_terminal_theme().palette();
        self.scroll_terminal_to_bottom_if_needed(TerminalId::Local);

        div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(rgb(terminal_palette.background))
            .child(self.terminal_output(TerminalId::Local, cx))
    }

    pub(in crate::pages::index) fn server_view(
        &mut self,
        server_id: i32,
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
                self.scroll_terminal_to_bottom_if_needed(terminal_id);

                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .overflow_hidden()
                    .bg(rgb(self.active_terminal_theme().palette().background))
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
        let terminal_palette = self.active_terminal_theme().palette();
        let focus_handle = self.focus_handle.clone();
        let resize_probe = terminal_resize_probe(cx.entity(), terminal_id);
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
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(rgb(terminal_palette.background))
            .font_family("Menlo")
            .text_size(px(13.))
            .text_color(rgb(terminal_palette.foreground))
            .on_click(move |_, window, _| {
                window.focus(&focus_handle);
            })
            .on_key_down(cx.listener(move |this, event, window, cx| {
                this.on_terminal_key_down(terminal_id, event, window, cx);
            }))
            .child(resize_probe)
            .when_some(terminal_list, |this, list| this.child(list))
            .when(!self.terminal_sessions.contains_key(&terminal_id), |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size_full()
                        .text_color(rgb(terminal_palette.foreground))
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
                let terminal_palette = self.active_terminal_theme().palette();
                visible_range
                    .filter_map(|row| session.display_line(row))
                    .map(|line| {
                        div()
                            .h(px(TERMINAL_LINE_HEIGHT))
                            .w_full()
                            .line_height(px(TERMINAL_LINE_HEIGHT))
                            .whitespace_nowrap()
                            .child(line.styled_text(terminal_palette))
                            .into_any_element()
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn terminal_resize_probe(view: gpui::Entity<Xssh>, terminal_id: TerminalId) -> impl IntoElement {
    canvas(
        move |bounds, _, cx| {
            let size = terminal_dimensions_from_bounds(bounds);
            view.update(cx, |this, cx| {
                if this.resize_terminal_session(terminal_id, size) {
                    cx.notify();
                }
            });
        },
        |_, _, _, _| {},
    )
    .absolute()
    .size_full()
}

fn terminal_dimensions_from_bounds(bounds: Bounds<Pixels>) -> TerminalSize {
    terminal_dimensions_from_pixels(
        bounds.size.width.to_f64() as f32,
        bounds.size.height.to_f64() as f32,
    )
}

fn terminal_dimensions_from_pixels(width: f32, height: f32) -> TerminalSize {
    let terminal_width = (width - TERMINAL_PADDING_X).max(0.0);
    let terminal_height = (height - TERMINAL_PADDING_Y).max(0.0);

    let cols = (terminal_width / TERMINAL_CELL_WIDTH).floor() as u16;
    let rows = (terminal_height / TERMINAL_LINE_HEIGHT).floor() as u16;

    TerminalSize::new(
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
    fn computes_terminal_dimensions_from_terminal_area() {
        assert_eq!(
            terminal_dimensions_from_pixels(2048., 859.),
            TerminalSize::new(258, 45)
        );
    }

    #[test]
    fn keeps_terminal_dimensions_usable_for_small_windows() {
        assert_eq!(
            terminal_dimensions_from_pixels(120., 100.),
            TerminalSize::new(24, 6)
        );
    }
}
