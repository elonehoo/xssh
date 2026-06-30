use std::{ops::Range, path::PathBuf, rc::Rc, sync::mpsc::Receiver, time::Duration};

use gpui::{
    Bounds, ClipboardItem, Context, EntityInputHandler, ExternalPaths, FontStyle, FontWeight,
    HighlightStyle, KeyDownEvent, Keystroke, KeystrokeEvent, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, ScrollStrategy, Size, StyledText, UTF16Selection, UnderlineStyle,
    Window, px, rgb, size,
};
use gpui_component::VirtualListScrollHandle;
use vt100::{Color, Parser};

use crate::{
    ipc::{
        ServerResource, TerminalCommand, TerminalEvent, TerminalSize, open_local_terminal,
        open_ssh_terminal,
    },
    ui::{TerminalThemePalette, TextKey},
};

use super::{
    Xssh,
    tabs::{ActiveTab, TerminalId},
};

pub(super) const TERMINAL_ROWS: u16 = 30;
pub(super) const TERMINAL_COLS: u16 = 100;
pub(super) const TERMINAL_LINE_HEIGHT: f32 = 18.0;
const TERMINAL_MAX_DISPLAY_ROWS: usize = 10_000;
const TERMINAL_SCROLLBACK_ROWS: usize = TERMINAL_MAX_DISPLAY_ROWS;
const TERMINAL_CONTENT_PADDING_X: f64 = 16.0;
const TERMINAL_CONTENT_PADDING_Y: f64 = 16.0;
const TERMINAL_CELL_WIDTH_PX: f64 = 7.8;

pub(super) struct TerminalSession {
    pub(super) scroll_handle: VirtualListScrollHandle,
    size: TerminalSize,
    pub(super) input: Option<std::sync::mpsc::Sender<TerminalCommand>>,
    display: TerminalDisplay,
    parser: Parser,
    state: TerminalConnectionState,
    pending_scroll_to_bottom: bool,
    bounds: Option<Bounds<Pixels>>,
    visible_range: Range<usize>,
    selection: Option<TerminalSelection>,
}

struct TerminalDisplay {
    cols: u16,
    scrollback_len: usize,
    line_sizes: Rc<Vec<Size<Pixels>>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TerminalLine {
    text: String,
    runs: Vec<TerminalLineRun>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TerminalLineRun {
    len: usize,
    style: TerminalTextStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TerminalTextStyle {
    foreground: Color,
    background: Color,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    inverse: bool,
    cursor: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerminalConnectionState {
    Idle,
    Connecting,
    Connected,
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TerminalSelectionPoint {
    row: usize,
    col: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TerminalSelection {
    anchor: TerminalSelectionPoint,
    head: TerminalSelectionPoint,
    dragging: bool,
}

impl TerminalSelection {
    fn new(point: TerminalSelectionPoint) -> Self {
        Self {
            anchor: point,
            head: point,
            dragging: true,
        }
    }

    fn normalized(&self) -> (TerminalSelectionPoint, TerminalSelectionPoint) {
        if self.anchor <= self.head {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }

    fn is_empty(&self) -> bool {
        self.anchor == self.head
    }
}

impl TerminalSession {
    pub(super) fn new() -> Self {
        let size = TerminalSize::new(TERMINAL_COLS, TERMINAL_ROWS);
        let mut parser = Parser::new(size.rows, size.cols, TERMINAL_SCROLLBACK_ROWS);
        let display = TerminalDisplay::from_parser(&mut parser);
        let display_len = display.line_count();

        Self {
            scroll_handle: VirtualListScrollHandle::new(),
            size,
            input: None,
            display,
            parser,
            state: TerminalConnectionState::Idle,
            pending_scroll_to_bottom: false,
            bounds: None,
            visible_range: 0..display_len,
            selection: None,
        }
    }

    pub(super) fn resize(&mut self, size: TerminalSize) -> bool {
        if self.size == size {
            return false;
        }

        self.size = size;
        self.parser.screen_mut().set_size(size.rows, size.cols);
        self.rebuild_display();
        true
    }

    fn process_output(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
        self.rebuild_display();
        self.pending_scroll_to_bottom = true;
    }

    fn write_status(&mut self, text: impl AsRef<str>) {
        let line = format!("\r\n{}\r\n", text.as_ref());
        self.process_output(line.as_bytes());
    }

    fn rebuild_display(&mut self) {
        self.display = TerminalDisplay::from_parser(&mut self.parser);
    }

    pub(super) fn display_line_sizes(&self) -> Rc<Vec<Size<Pixels>>> {
        self.display.line_sizes.clone()
    }

    pub(super) fn display_line(&self, row: usize) -> Option<TerminalLine> {
        self.display.line(&self.parser, row)
    }

    #[cfg(test)]
    fn display_line_text(&self, row: usize) -> Option<String> {
        self.display_line(row).map(|line| line.text().to_string())
    }

    fn display_len(&self) -> usize {
        self.display.line_count()
    }

    fn set_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.bounds = Some(bounds);
    }

    fn set_visible_range(&mut self, visible_range: Range<usize>) {
        self.visible_range = visible_range;
    }

    fn copy_text(&self) -> String {
        let mut lines = (0..self.display_len())
            .filter_map(|row| self.display_line(row))
            .map(|line| line.copy_text())
            .collect::<Vec<_>>();

        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }

        lines.join("\n")
    }

    fn selected_text(&self) -> Option<String> {
        let selection = self
            .selection
            .as_ref()
            .filter(|selection| !selection.is_empty())?;
        let (start, end) = selection.normalized();
        let mut lines = Vec::new();

        for row in start.row..=end.row {
            let line = self.display_line(row)?.copy_text();
            let line_len = line.chars().count();
            let start_col = if row == start.row {
                start.col.min(line_len)
            } else {
                0
            };
            let end_col = if row == end.row {
                end.col.min(line_len)
            } else {
                line_len
            };

            lines.push(slice_chars(&line, start_col..end_col));
        }

        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }

        (!lines.is_empty()).then(|| lines.join("\n"))
    }

    fn remote_directory_hint_from_output(&self) -> Option<String> {
        let mut expecting_pwd_output = false;
        let mut latest_directory = None;

        for row in 0..self.display_len() {
            let Some(line) = self.display_line(row) else {
                continue;
            };
            let text = line.copy_text();
            let trimmed = text.trim();

            if expecting_pwd_output {
                if terminal_line_is_absolute_path(trimmed) {
                    latest_directory = Some(trimmed.to_string());
                    expecting_pwd_output = false;
                    continue;
                }

                if !trimmed.is_empty() && !terminal_line_contains_pwd_command(trimmed) {
                    expecting_pwd_output = false;
                }
            }

            if terminal_line_contains_pwd_command(trimmed) {
                expecting_pwd_output = true;
            }
        }

        latest_directory
    }

    pub(super) fn selection_range_for_line(
        &self,
        row: usize,
        line: &TerminalLine,
    ) -> Option<Range<usize>> {
        let selection = self
            .selection
            .as_ref()
            .filter(|selection| !selection.is_empty())?;
        let (start, end) = selection.normalized();
        if row < start.row || row > end.row {
            return None;
        }

        let line_len = line.text().chars().count();
        let start_col = if row == start.row {
            start.col.min(line_len)
        } else {
            0
        };
        let end_col = if row == end.row {
            end.col.min(line_len)
        } else {
            line_len
        };

        (start_col < end_col).then_some(start_col..end_col)
    }

    fn selection_point_from_position(&self, position: Point<Pixels>) -> TerminalSelectionPoint {
        let Some(bounds) = self.bounds else {
            return TerminalSelectionPoint { row: 0, col: 0 };
        };

        let x = position.x.to_f64() - bounds.origin.x.to_f64() - TERMINAL_CONTENT_PADDING_X;
        let y = position.y.to_f64() - bounds.origin.y.to_f64() - TERMINAL_CONTENT_PADDING_Y;
        let col = (x / TERMINAL_CELL_WIDTH_PX).floor().max(0.0) as usize;
        let row_offset = (y / f64::from(TERMINAL_LINE_HEIGHT)).floor().max(0.0) as usize;
        let row = self
            .visible_range
            .start
            .saturating_add(row_offset)
            .min(self.display_len().saturating_sub(1));

        TerminalSelectionPoint { row, col }
    }

    fn begin_selection(&mut self, point: TerminalSelectionPoint) {
        self.selection = Some(TerminalSelection::new(point));
    }

    fn update_selection(&mut self, point: TerminalSelectionPoint) -> bool {
        let Some(selection) = self
            .selection
            .as_mut()
            .filter(|selection| selection.dragging)
        else {
            return false;
        };

        if selection.head == point {
            return false;
        }

        selection.head = point;
        true
    }

    fn end_selection(&mut self, point: TerminalSelectionPoint) {
        let Some(selection) = self.selection.as_mut() else {
            return;
        };

        selection.head = point;
        selection.dragging = false;

        if selection.is_empty() {
            self.selection = None;
        }
    }

    fn stop_selecting(&mut self) {
        if let Some(selection) = self.selection.as_mut() {
            selection.dragging = false;
        }
    }

    fn take_pending_scroll_target(&mut self) -> Option<usize> {
        if !self.pending_scroll_to_bottom {
            return None;
        }

        self.pending_scroll_to_bottom = false;
        Some(self.display_len().saturating_sub(1))
    }

    fn is_running(&self) -> bool {
        matches!(
            self.state,
            TerminalConnectionState::Connecting | TerminalConnectionState::Connected
        )
    }

    fn mark_connecting(&mut self) {
        self.state = TerminalConnectionState::Connecting;
    }

    fn mark_connected(&mut self) {
        self.state = TerminalConnectionState::Connected;
    }

    fn mark_closed(&mut self) -> bool {
        if self.state == TerminalConnectionState::Closed {
            return false;
        }

        self.state = TerminalConnectionState::Closed;
        true
    }

    fn should_poll(&self) -> bool {
        self.state != TerminalConnectionState::Closed
    }
}

impl TerminalDisplay {
    fn from_parser(parser: &mut Parser) -> Self {
        let (rows, cols) = parser.screen().size();
        let scrollback_len = parser_scrollback_len(parser);
        let line_count = scrollback_len + usize::from(rows);

        Self {
            cols,
            scrollback_len,
            line_sizes: terminal_line_sizes(line_count.max(1)),
        }
    }

    fn line_count(&self) -> usize {
        self.line_sizes.len()
    }

    fn line(&self, parser: &Parser, row: usize) -> Option<TerminalLine> {
        if row >= self.line_count() {
            return None;
        }

        let mut screen = parser.screen().clone();
        let cursor = if row >= self.scrollback_len && !screen.hide_cursor() {
            Some(screen.cursor_position())
        } else {
            None
        };

        let visible_row = if row < self.scrollback_len {
            screen.set_scrollback(self.scrollback_len - row);
            0
        } else {
            screen.set_scrollback(0);
            u16::try_from(row - self.scrollback_len).ok()?
        };

        Some(terminal_line_from_screen(
            &screen,
            visible_row,
            self.cols,
            cursor,
        ))
    }
}

fn terminal_line_sizes(count: usize) -> Rc<Vec<Size<Pixels>>> {
    Rc::new(
        (0..count)
            .map(|_| size(px(0.), px(TERMINAL_LINE_HEIGHT)))
            .collect(),
    )
}

fn parser_scrollback_len(parser: &mut Parser) -> usize {
    let previous_scrollback = parser.screen().scrollback();
    parser.screen_mut().set_scrollback(usize::MAX);
    let scrollback_len = parser.screen().scrollback();
    parser.screen_mut().set_scrollback(previous_scrollback);

    scrollback_len
}

impl TerminalLine {
    fn new() -> Self {
        Self {
            text: String::new(),
            runs: Vec::new(),
        }
    }

    fn plain(text: impl Into<String>) -> Self {
        let mut line = Self::new();
        let text = text.into();
        line.push(&text, TerminalTextStyle::default());
        line
    }

    fn push(&mut self, text: &str, style: TerminalTextStyle) {
        if text.is_empty() {
            return;
        }

        self.text.push_str(text);

        if let Some(last_run) = self.runs.last_mut()
            && last_run.style == style
        {
            last_run.len += text.len();
            return;
        }

        self.runs.push(TerminalLineRun {
            len: text.len(),
            style,
        });
    }

    pub(super) fn text(&self) -> &str {
        &self.text
    }

    fn copy_text(&self) -> String {
        let mut text = String::new();
        let mut offset = 0;

        for run in &self.runs {
            let end = offset + run.len;
            if !run.style.cursor {
                text.push_str(&self.text[offset..end]);
            }
            offset = end;
        }

        text.trim_end().to_string()
    }

    pub(super) fn styled_text(
        &self,
        palette: TerminalThemePalette,
        selection: Option<Range<usize>>,
        selection_background: u32,
    ) -> StyledText {
        if self.is_plain() && selection.is_none() {
            return StyledText::new(self.text.clone());
        }

        let mut highlights = Vec::new();
        if !self.is_plain() {
            highlights.extend(self.runs.iter().scan(0, |offset, run| {
                let start = *offset;
                let end = start + run.len;
                *offset = end;
                Some((start..end, run.style.highlight(palette)))
            }));
        }

        if let Some(selection) = selection {
            let byte_range = char_range_to_byte_range(&self.text, selection);
            if byte_range.start < byte_range.end {
                highlights.push((
                    byte_range,
                    HighlightStyle {
                        background_color: Some(rgb(selection_background).opacity(0.35).into()),
                        ..Default::default()
                    },
                ));
            }
        }

        StyledText::new(self.text.clone()).with_highlights(highlights)
    }

    fn is_plain(&self) -> bool {
        self.runs
            .iter()
            .all(|run| run.style == TerminalTextStyle::default())
    }
}

impl Default for TerminalTextStyle {
    fn default() -> Self {
        Self {
            foreground: Color::Default,
            background: Color::Default,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            inverse: false,
            cursor: false,
        }
    }
}

impl TerminalTextStyle {
    fn from_cell(cell: &vt100::Cell) -> Self {
        Self {
            foreground: cell.fgcolor(),
            background: cell.bgcolor(),
            bold: cell.bold(),
            dim: cell.dim(),
            italic: cell.italic(),
            underline: cell.underline(),
            inverse: cell.inverse(),
            cursor: false,
        }
    }

    fn with_cursor(mut self) -> Self {
        self.cursor = true;
        self
    }

    fn has_visible_background(self) -> bool {
        self.background != Color::Default || self.inverse || self.cursor
    }

    fn highlight(self, palette: TerminalThemePalette) -> HighlightStyle {
        let foreground = if self.cursor {
            palette.cursor
        } else {
            self.resolved_foreground(palette)
        };
        let background = self.resolved_background(palette);
        let foreground_color = rgb(foreground).into();

        HighlightStyle {
            color: Some(foreground_color),
            font_weight: self.bold.then_some(FontWeight::BOLD),
            font_style: self.italic.then_some(FontStyle::Italic),
            background_color: background.map(|color| rgb(color).into()),
            underline: self.underline.then_some(UnderlineStyle {
                color: Some(foreground_color),
                thickness: px(1.0),
                wavy: false,
            }),
            fade_out: self.dim.then_some(0.45),
            ..Default::default()
        }
    }

    fn resolved_foreground(self, palette: TerminalThemePalette) -> u32 {
        if self.inverse {
            terminal_color(self.background, palette).unwrap_or(palette.background)
        } else {
            terminal_color(self.foreground, palette).unwrap_or(palette.foreground)
        }
    }

    fn resolved_background(self, palette: TerminalThemePalette) -> Option<u32> {
        if self.cursor {
            return None;
        }

        if self.inverse {
            Some(terminal_color(self.foreground, palette).unwrap_or(palette.foreground))
        } else {
            terminal_color(self.background, palette)
        }
    }
}

fn terminal_line_from_screen(
    screen: &vt100::Screen,
    row: u16,
    cols: u16,
    cursor: Option<(u16, u16)>,
) -> TerminalLine {
    let last_col = (0..cols).rfind(|col| {
        let is_cursor = cursor == Some((row, *col));
        let Some(cell) = screen.cell(row, *col) else {
            return is_cursor;
        };

        is_cursor
            || cell.has_contents()
            || TerminalTextStyle::from_cell(cell).has_visible_background()
    });

    let Some(last_col) = last_col else {
        return TerminalLine::plain(" ");
    };

    let mut line = TerminalLine::new();

    for col in 0..=last_col {
        let is_cursor = cursor == Some((row, col));
        let Some(cell) = screen.cell(row, col) else {
            line.push(" ", TerminalTextStyle::default());
            continue;
        };

        if cell.is_wide_continuation() && !is_cursor {
            continue;
        }

        let style = if is_cursor {
            TerminalTextStyle::from_cell(cell).with_cursor()
        } else {
            TerminalTextStyle::from_cell(cell)
        };
        let text = if is_cursor {
            "█"
        } else if cell.has_contents() {
            cell.contents()
        } else {
            " "
        };

        line.push(text, style);
    }

    if line.text().is_empty() {
        TerminalLine::plain(" ")
    } else {
        line
    }
}

fn terminal_color(color: Color, palette: TerminalThemePalette) -> Option<u32> {
    match color {
        Color::Default => None,
        Color::Idx(index) if index < 16 => Some(palette.ansi[usize::from(index)]),
        Color::Idx(index) if (16..=231).contains(&index) => Some(indexed_256_color(index)),
        Color::Idx(index) if index >= 232 => Some(indexed_grayscale_color(index)),
        Color::Idx(_) => None,
        Color::Rgb(red, green, blue) => {
            Some((u32::from(red) << 16) | (u32::from(green) << 8) | u32::from(blue))
        }
    }
}

fn indexed_256_color(index: u8) -> u32 {
    const LEVELS: [u32; 6] = [0, 95, 135, 175, 215, 255];
    let index = u32::from(index - 16);
    let red = LEVELS[(index / 36) as usize];
    let green = LEVELS[((index / 6) % 6) as usize];
    let blue = LEVELS[(index % 6) as usize];

    (red << 16) | (green << 8) | blue
}

fn indexed_grayscale_color(index: u8) -> u32 {
    let value = u32::from(8 + (index - 232) * 10);

    (value << 16) | (value << 8) | value
}

impl Xssh {
    pub(in crate::pages::index) fn ensure_terminal_session(&mut self, terminal_id: TerminalId) {
        self.terminal_sessions
            .entry(terminal_id)
            .or_insert_with(TerminalSession::new);
    }

    pub(in crate::pages::index) fn start_terminal_connection(
        &mut self,
        server: ServerResource,
        cx: &mut Context<Self>,
    ) {
        let terminal_id = TerminalId::Server(server.id);
        let Some(session) = self.terminal_sessions.get_mut(&terminal_id) else {
            return;
        };

        if session.is_running() {
            return;
        }

        session.mark_connecting();
        session.write_status(format!(
            "Connecting to {}@{}:{}...",
            server.username, server.host, server.port
        ));

        let handle = open_ssh_terminal(server, session.size);
        session.input = Some(handle.input);
        self.spawn_terminal_event_poller(terminal_id, handle.events, cx);
        cx.notify();
    }

    pub(in crate::pages::index) fn start_local_terminal_connection(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.terminal_sessions.get_mut(&TerminalId::Local) else {
            return;
        };

        if session.is_running() {
            return;
        }

        session.mark_connecting();

        let handle = open_local_terminal(session.size);
        session.input = Some(handle.input);
        self.spawn_terminal_event_poller(TerminalId::Local, handle.events, cx);
        cx.notify();
    }

    pub(in crate::pages::index) fn remove_terminal_session(&mut self, terminal_id: TerminalId) {
        if let Some(session) = self.terminal_sessions.get_mut(&terminal_id)
            && let Some(input) = session.input.take()
        {
            let _ = input.send(TerminalCommand::Close);
        }

        self.terminal_sessions.remove(&terminal_id);
    }

    pub(in crate::pages::index) fn resize_terminal_session(
        &mut self,
        terminal_id: TerminalId,
        size: TerminalSize,
    ) -> bool {
        let Some(session) = self.terminal_sessions.get_mut(&terminal_id) else {
            return false;
        };

        if !session.resize(size) {
            return false;
        }

        if let Some(input) = &session.input {
            let _ = input.send(TerminalCommand::Resize(size));
        }

        true
    }

    pub(in crate::pages::index) fn update_terminal_bounds(
        &mut self,
        terminal_id: TerminalId,
        bounds: Bounds<Pixels>,
    ) {
        if let Some(session) = self.terminal_sessions.get_mut(&terminal_id) {
            session.set_bounds(bounds);
        }
    }

    pub(in crate::pages::index) fn update_terminal_visible_range(
        &mut self,
        terminal_id: TerminalId,
        visible_range: Range<usize>,
    ) {
        if let Some(session) = self.terminal_sessions.get_mut(&terminal_id) {
            session.set_visible_range(visible_range);
        }
    }

    pub(in crate::pages::index) fn scroll_terminal_to_bottom_if_needed(
        &mut self,
        terminal_id: TerminalId,
    ) {
        let Some(session) = self.terminal_sessions.get_mut(&terminal_id) else {
            return;
        };

        let Some(target) = session.take_pending_scroll_target() else {
            return;
        };

        session
            .scroll_handle
            .scroll_to_item(target, ScrollStrategy::Top);
    }

    pub(in crate::pages::index) fn on_terminal_key_down(
        &mut self,
        terminal_id: TerminalId,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.handle_terminal_platform_shortcut(terminal_id, event, cx) {
            window.prevent_default();
            cx.stop_propagation();
            return;
        }

        let Some(bytes) = terminal_bytes_from_key(&event.keystroke) else {
            return;
        };

        if self.send_terminal_input(terminal_id, bytes) {
            window.prevent_default();
            cx.stop_propagation();
        }
    }

    pub(in crate::pages::index) fn intercept_terminal_tab(
        &mut self,
        event: &KeystrokeEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(bytes) = terminal_tab_bytes(&event.keystroke) else {
            return;
        };
        let Some(terminal_id) = self.active_terminal_id() else {
            return;
        };
        if !self.focus_handle.is_focused(window) {
            return;
        }

        let _ = self.send_terminal_input(terminal_id, bytes);
        window.prevent_default();
        cx.stop_propagation();
    }

    fn terminal_selection_point(
        &self,
        terminal_id: TerminalId,
        position: Point<Pixels>,
    ) -> TerminalSelectionPoint {
        self.terminal_sessions
            .get(&terminal_id)
            .map(|session| session.selection_point_from_position(position))
            .unwrap_or(TerminalSelectionPoint { row: 0, col: 0 })
    }

    pub(in crate::pages::index) fn on_terminal_selection_mouse_down(
        &mut self,
        terminal_id: TerminalId,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let point = self.terminal_selection_point(terminal_id, event.position);
        if let Some(session) = self.terminal_sessions.get_mut(&terminal_id) {
            session.begin_selection(point);
            cx.stop_propagation();
            cx.notify();
        }
    }

    pub(in crate::pages::index) fn on_terminal_selection_mouse_move(
        &mut self,
        terminal_id: TerminalId,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !event.dragging() {
            return;
        }

        let point = self.terminal_selection_point(terminal_id, event.position);
        if let Some(session) = self.terminal_sessions.get_mut(&terminal_id)
            && session.update_selection(point)
        {
            cx.stop_propagation();
            cx.notify();
        }
    }

    pub(in crate::pages::index) fn on_terminal_selection_mouse_up(
        &mut self,
        terminal_id: TerminalId,
        event: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let point = self.terminal_selection_point(terminal_id, event.position);
        if let Some(session) = self.terminal_sessions.get_mut(&terminal_id) {
            session.end_selection(point);
            cx.stop_propagation();
            cx.notify();
        }
    }

    pub(in crate::pages::index) fn stop_terminal_selection(
        &mut self,
        terminal_id: TerminalId,
        cx: &mut Context<Self>,
    ) {
        if let Some(session) = self.terminal_sessions.get_mut(&terminal_id) {
            session.stop_selecting();
            cx.notify();
        }
    }

    pub(in crate::pages::index) fn on_terminal_file_drop(
        &mut self,
        terminal_id: TerminalId,
        paths: &ExternalPaths,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let local_paths = paths.paths().iter().cloned().collect::<Vec<_>>();
        if local_paths.is_empty() {
            return;
        }

        let TerminalId::Server(server_id) = terminal_id else {
            self.add_failed_upload_task(
                "Local".to_string(),
                local_paths.len(),
                "只有 SSH 服务器终端支持拖拽上传。",
            );
            cx.stop_propagation();
            cx.notify();
            return;
        };

        let file_count = local_paths.len();
        if self.send_terminal_upload_files(terminal_id, local_paths) {
            cx.stop_propagation();
        } else {
            self.add_failed_upload_task(
                self.server_name(server_id),
                file_count,
                "SSH 未连接，无法上传文件。",
            );
            cx.stop_propagation();
        }

        cx.notify();
    }

    fn handle_terminal_platform_shortcut(
        &mut self,
        terminal_id: TerminalId,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if !event.keystroke.modifiers.platform
            || event.keystroke.modifiers.control
            || event.keystroke.modifiers.alt
        {
            return false;
        }

        match event.keystroke.key.as_str() {
            key if key.eq_ignore_ascii_case("c") => self.copy_terminal_buffer(terminal_id, cx),
            key if key.eq_ignore_ascii_case("v") => self.paste_terminal_clipboard(terminal_id, cx),
            _ => false,
        }
    }

    fn copy_terminal_buffer(&self, terminal_id: TerminalId, cx: &mut Context<Self>) -> bool {
        let Some(session) = self.terminal_sessions.get(&terminal_id) else {
            return false;
        };

        let text = session
            .selected_text()
            .unwrap_or_else(|| session.copy_text());
        if text.is_empty() {
            return false;
        }

        cx.write_to_clipboard(ClipboardItem::new_string(text));
        true
    }

    fn paste_terminal_clipboard(
        &mut self,
        terminal_id: TerminalId,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return false;
        };

        self.send_terminal_input(terminal_id, terminal_paste_bytes(&text))
    }

    fn active_terminal_id(&self) -> Option<TerminalId> {
        match self.active_tab {
            ActiveTab::Vault => None,
            ActiveTab::LocalTerminal => Some(TerminalId::Local),
            ActiveTab::Server(server_id) => Some(TerminalId::Server(server_id)),
        }
    }

    fn send_terminal_input(&mut self, terminal_id: TerminalId, bytes: Vec<u8>) -> bool {
        let Some(session) = self.terminal_sessions.get(&terminal_id) else {
            return false;
        };

        let Some(input) = &session.input else {
            return false;
        };

        input.send(TerminalCommand::Input(bytes)).is_ok()
    }

    fn send_terminal_upload_files(&mut self, terminal_id: TerminalId, paths: Vec<PathBuf>) -> bool {
        let Some(session) = self.terminal_sessions.get(&terminal_id) else {
            return false;
        };

        let Some(input) = &session.input else {
            return false;
        };

        input
            .send(TerminalCommand::UploadFiles {
                paths,
                remote_directory_hint: session.remote_directory_hint_from_output(),
            })
            .is_ok()
    }

    fn server_name(&self, server_id: i32) -> String {
        self.open_tabs
            .iter()
            .find_map(|tab| match tab {
                super::tabs::OpenTab::Server(server) if server.id == server_id => {
                    Some(server.name.clone())
                }
                _ => None,
            })
            .or_else(|| {
                self.servers
                    .iter()
                    .find(|server| server.id == server_id)
                    .map(|server| server.name.clone())
            })
            .unwrap_or_else(|| format!("Server {server_id}"))
    }

    fn spawn_terminal_event_poller(
        &mut self,
        terminal_id: TerminalId,
        events: Receiver<TerminalEvent>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;

                let mut pending = Vec::new();
                let mut event_channel_disconnected = false;
                loop {
                    match events.try_recv() {
                        Ok(event) => pending.push(event),
                        Err(std::sync::mpsc::TryRecvError::Empty) => break,
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            event_channel_disconnected = true;
                            break;
                        }
                    }
                }

                if pending.is_empty() {
                    if event_channel_disconnected {
                        break;
                    }
                    continue;
                }

                let keep_polling = this
                    .update(cx, |this, cx| {
                        this.apply_terminal_events(terminal_id, pending, cx)
                    })
                    .unwrap_or(false);

                if !keep_polling && event_channel_disconnected {
                    break;
                }
            }
        })
        .detach();
    }

    fn apply_terminal_events(
        &mut self,
        terminal_id: TerminalId,
        events: Vec<TerminalEvent>,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut pending_output = Vec::new();
        let mut upload_events = Vec::new();
        let keep_polling = {
            let Some(session) = self.terminal_sessions.get_mut(&terminal_id) else {
                for event in events {
                    if let TerminalEvent::UploadTask(event) = event {
                        upload_events.push(event);
                    }
                }

                for event in upload_events {
                    self.apply_upload_task_event(event);
                }

                cx.notify();
                return true;
            };

            for event in events {
                match event {
                    TerminalEvent::Connected => {
                        flush_terminal_output(session, &mut pending_output);
                        session.mark_connected();
                    }
                    TerminalEvent::Output(bytes) => {
                        pending_output.extend_from_slice(&bytes);
                    }
                    TerminalEvent::UploadTask(event) => {
                        upload_events.push(event);
                    }
                    TerminalEvent::Disconnected => {
                        flush_terminal_output(session, &mut pending_output);
                        if !session.mark_closed() {
                            continue;
                        }

                        session.input = None;
                        session.write_status(self.language.tr(TextKey::TerminalDisconnected));
                    }
                    TerminalEvent::Error(error) => {
                        flush_terminal_output(session, &mut pending_output);
                        if !session.mark_closed() {
                            continue;
                        }

                        session.input = None;
                        session.write_status(error);
                    }
                }
            }

            flush_terminal_output(session, &mut pending_output);
            session.should_poll()
        };

        for event in upload_events {
            self.apply_upload_task_event(event);
        }

        cx.notify();
        keep_polling
    }
}

impl EntityInputHandler for Xssh {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let text_len = utf16_len(&self.terminal_ime_buffer);
        let start = range_utf16.start.min(text_len);
        let end = range_utf16.end.min(text_len).max(start);
        let range = start..end;
        let byte_range = utf16_range_to_byte_range(&self.terminal_ime_buffer, range.clone());

        actual_range.replace(range);
        Some(self.terminal_ime_buffer[byte_range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let cursor = utf16_len(&self.terminal_ime_buffer);
        Some(UTF16Selection {
            range: cursor..cursor,
            reversed: false,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.terminal_ime_marked_range.clone()
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.terminal_ime_buffer.clear();
        self.terminal_ime_marked_range = None;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal_ime_buffer.clear();
        self.terminal_ime_marked_range = None;

        if !text.is_empty()
            && let Some(terminal_id) = self.active_terminal_id()
        {
            self.send_terminal_input(terminal_id, terminal_paste_bytes(text));
        }

        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        replace_utf16_text_range(
            &mut self.terminal_ime_buffer,
            range_utf16.or_else(|| self.terminal_ime_marked_range.clone()),
            new_text,
        );

        self.terminal_ime_marked_range = if self.terminal_ime_buffer.is_empty() {
            None
        } else {
            Some(0..utf16_len(&self.terminal_ime_buffer))
        };

        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(bounds)
    }

    fn character_index_for_point(
        &mut self,
        _: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        Some(utf16_len(&self.terminal_ime_buffer))
    }
}

fn flush_terminal_output(session: &mut TerminalSession, pending_output: &mut Vec<u8>) {
    if pending_output.is_empty() {
        return;
    }

    session.process_output(pending_output);
    pending_output.clear();
}

fn terminal_paste_bytes(text: &str) -> Vec<u8> {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "\r")
        .into_bytes()
}

fn terminal_text_bytes(text: &str) -> Option<Vec<u8>> {
    if text.chars().count() != 1 {
        return None;
    }

    Some(text.as_bytes().to_vec())
}

fn terminal_line_contains_pwd_command(line: &str) -> bool {
    let trimmed = line.trim();
    if terminal_line_is_pwd_command(trimmed) {
        return true;
    }

    let Some(pwd_index) = trimmed.rfind("pwd") else {
        return false;
    };
    let prefix = trimmed[..pwd_index].trim_end();
    let suffix = trimmed[pwd_index + "pwd".len()..].trim_start();

    if !prefix.ends_with(['#', '$', '>', '%']) {
        return false;
    }

    terminal_pwd_suffix_is_supported(suffix)
}

fn terminal_line_is_pwd_command(line: &str) -> bool {
    let Some(suffix) = line.strip_prefix("pwd") else {
        return false;
    };

    suffix.is_empty()
        || suffix
            .chars()
            .next()
            .is_some_and(|character| character.is_whitespace())
            && terminal_pwd_suffix_is_supported(suffix.trim_start())
}

fn terminal_pwd_suffix_is_supported(suffix: &str) -> bool {
    suffix.split_whitespace().all(|arg| {
        arg.starts_with('-')
            && arg
                .trim_start_matches('-')
                .chars()
                .all(|flag| flag == 'L' || flag == 'P')
    })
}

fn terminal_line_is_absolute_path(line: &str) -> bool {
    line.starts_with('/') && !line.contains('\0')
}

fn terminal_tab_bytes(keystroke: &Keystroke) -> Option<Vec<u8>> {
    if keystroke.key != "tab"
        || keystroke.modifiers.platform
        || keystroke.modifiers.control
        || keystroke.modifiers.alt
        || keystroke.modifiers.function
    {
        return None;
    }

    if keystroke.modifiers.shift {
        Some(b"\x1b[Z".to_vec())
    } else {
        Some(vec![b'\t'])
    }
}

fn slice_chars(text: &str, range: Range<usize>) -> String {
    let byte_range = char_range_to_byte_range(text, range);
    text[byte_range].to_string()
}

fn char_range_to_byte_range(text: &str, range: Range<usize>) -> Range<usize> {
    let len = text.chars().count();
    let start = range.start.min(len);
    let end = range.end.min(len).max(start);

    char_offset_to_byte_index(text, start)..char_offset_to_byte_index(text, end)
}

fn char_offset_to_byte_index(text: &str, offset: usize) -> usize {
    if offset == 0 {
        return 0;
    }

    text.char_indices()
        .nth(offset)
        .map(|(byte_index, _)| byte_index)
        .unwrap_or(text.len())
}

fn terminal_bytes_from_key(keystroke: &Keystroke) -> Option<Vec<u8>> {
    if keystroke.modifiers.platform {
        return None;
    }

    if keystroke.modifiers.control {
        return control_key_bytes(&keystroke.key);
    }

    let mut bytes = Vec::new();
    if keystroke.modifiers.alt {
        bytes.push(0x1b);
    }

    match keystroke.key.as_str() {
        "enter" => bytes.push(b'\r'),
        "backspace" => bytes.push(0x7f),
        "tab" => bytes.push(b'\t'),
        "escape" => bytes.push(0x1b),
        "up" => bytes.extend_from_slice(b"\x1b[A"),
        "down" => bytes.extend_from_slice(b"\x1b[B"),
        "right" => bytes.extend_from_slice(b"\x1b[C"),
        "left" => bytes.extend_from_slice(b"\x1b[D"),
        "home" => bytes.extend_from_slice(b"\x1b[H"),
        "end" => bytes.extend_from_slice(b"\x1b[F"),
        "pageup" => bytes.extend_from_slice(b"\x1b[5~"),
        "pagedown" => bytes.extend_from_slice(b"\x1b[6~"),
        "delete" => bytes.extend_from_slice(b"\x1b[3~"),
        _ => {
            if keystroke.is_ime_in_progress() {
                return None;
            }

            let text = keystroke.key_char.as_ref().unwrap_or(&keystroke.key);
            bytes.extend_from_slice(&terminal_text_bytes(text)?);
        }
    }

    Some(bytes)
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn replace_utf16_text_range(text: &mut String, range_utf16: Option<Range<usize>>, new_text: &str) {
    let range_utf16 = range_utf16.unwrap_or_else(|| {
        let len = utf16_len(text);
        len..len
    });
    let byte_range = utf16_range_to_byte_range(text, range_utf16);

    text.replace_range(byte_range, new_text);
}

fn utf16_range_to_byte_range(text: &str, range_utf16: Range<usize>) -> Range<usize> {
    let len = utf16_len(text);
    let start = range_utf16.start.min(len);
    let end = range_utf16.end.min(len).max(start);

    utf16_offset_to_byte_index(text, start)..utf16_offset_to_byte_index(text, end)
}

fn utf16_offset_to_byte_index(text: &str, offset: usize) -> usize {
    if offset == 0 {
        return 0;
    }

    let mut current = 0;
    for (byte_index, character) in text.char_indices() {
        if current >= offset {
            return byte_index;
        }

        current += character.len_utf16();
        if current > offset {
            return byte_index;
        }
    }

    text.len()
}

fn control_key_bytes(key: &str) -> Option<Vec<u8>> {
    let byte = match key {
        "@" | "space" => 0x00,
        "[" | "escape" => 0x1b,
        "\\" => 0x1c,
        "]" => 0x1d,
        "^" => 0x1e,
        "_" => 0x1f,
        "?" | "backspace" => 0x7f,
        key if key.len() == 1 => {
            let byte = key.as_bytes()[0].to_ascii_lowercase();
            if byte.is_ascii_lowercase() {
                byte - b'a' + 1
            } else {
                return None;
            }
        }
        _ => return None,
    };

    Some(vec![byte])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_lines_include_output_beyond_viewport_rows() {
        let mut session = TerminalSession::new();
        session.resize(TerminalSize::new(80, 6));
        let output = (0..40)
            .map(|row| format!("line {row}\r\n"))
            .collect::<String>();

        session.process_output(output.as_bytes());

        assert!(session.display_len() > usize::from(session.size.rows));
        assert!(
            (0..session.display_len())
                .filter_map(|row| session.display_line_text(row))
                .any(|line| line.contains("line 39"))
        );
    }

    #[test]
    fn display_keeps_fixed_viewport_rows() {
        let mut session = TerminalSession::new();
        session.resize(TerminalSize::new(80, 30));

        session.process_output(b"prompt");

        assert_eq!(session.display_len(), usize::from(session.size.rows));
        assert!(session.display_line_text(0).unwrap().contains("prompt"));
    }

    #[test]
    fn keeps_ansi_cell_styles() {
        let mut session = TerminalSession::new();
        session.process_output(b"\x1b[31mred\x1b[0m \x1b[44;1;4mblue-bg\x1b[0m\r\n");

        let line = session.display_line(0).unwrap();

        assert!(line.text().contains("red"));
        assert_eq!(line.runs[0].style.foreground, Color::Idx(1));
        assert_eq!(line.runs[2].style.background, Color::Idx(4));
        assert!(line.runs[2].style.bold);
        assert!(line.runs[2].style.underline);
    }

    #[test]
    fn resolves_ansi_256_and_rgb_colors() {
        let palette = TerminalThemePalette {
            background: 0x000000,
            foreground: 0xffffff,
            cursor: 0xffffff,
            ansi: [
                0x000000, 0x111111, 0x222222, 0x333333, 0x444444, 0x555555, 0x666666, 0x777777,
                0x888888, 0x999999, 0xaaaaaa, 0xbbbbbb, 0xcccccc, 0xdddddd, 0xeeeeee, 0xffffff,
            ],
        };

        assert_eq!(terminal_color(Color::Idx(4), palette), Some(0x444444));
        assert_eq!(terminal_color(Color::Idx(196), palette), Some(0xff0000));
        assert_eq!(terminal_color(Color::Idx(232), palette), Some(0x080808));
        assert_eq!(terminal_color(Color::Rgb(1, 2, 3), palette), Some(0x010203));
    }

    #[test]
    fn output_requests_one_scroll_to_latest_line() {
        let mut session = TerminalSession::new();
        session.process_output(b"first\r\nsecond\r\n");

        assert_eq!(
            session.take_pending_scroll_target(),
            Some(session.display_len().saturating_sub(1))
        );
        assert_eq!(session.take_pending_scroll_target(), None);
    }

    #[test]
    fn reads_remote_directory_hint_from_pwd_output() {
        let mut session = TerminalSession::new();

        session.process_output(b"[root@localhost pubinfo-bot]# pwd\r\n/app/pubinfo-bot\r\n[root@localhost pubinfo-bot]# ");

        assert_eq!(
            session.remote_directory_hint_from_output(),
            Some("/app/pubinfo-bot".to_string())
        );
    }

    #[test]
    fn uses_latest_remote_directory_hint_from_pwd_output() {
        let mut session = TerminalSession::new();

        session.process_output(b"$ pwd\r\n/old\r\n$ pwd -P\r\n/new\r\n$ ");

        assert_eq!(
            session.remote_directory_hint_from_output(),
            Some("/new".to_string())
        );
    }

    #[test]
    fn detects_pwd_commands_in_terminal_lines() {
        assert!(terminal_line_contains_pwd_command("pwd"));
        assert!(terminal_line_contains_pwd_command("pwd -P"));
        assert!(terminal_line_contains_pwd_command(
            "[root@localhost app]# pwd"
        ));
        assert!(terminal_line_contains_pwd_command(
            "user@host:/srv$ pwd -LP"
        ));
        assert!(!terminal_line_contains_pwd_command("echo pwd"));
        assert!(!terminal_line_contains_pwd_command("pwd /tmp"));
    }

    #[test]
    fn copies_terminal_buffer_without_empty_viewport_tail() {
        let mut session = TerminalSession::new();
        session.resize(TerminalSize::new(80, 6));
        session.process_output(b"hello\r\nworld\r\n");

        assert_eq!(session.copy_text(), "hello\nworld");
    }

    #[test]
    fn copies_selected_terminal_text_across_rows() {
        let mut session = TerminalSession::new();
        session.resize(TerminalSize::new(80, 6));
        session.process_output(b"hello\r\nworld\r\n");

        session.begin_selection(TerminalSelectionPoint { row: 0, col: 1 });
        session.end_selection(TerminalSelectionPoint { row: 1, col: 3 });

        assert_eq!(session.selected_text(), Some("ello\nwor".to_string()));
    }

    #[test]
    fn clears_empty_terminal_selection() {
        let mut session = TerminalSession::new();

        session.begin_selection(TerminalSelectionPoint { row: 0, col: 2 });
        session.end_selection(TerminalSelectionPoint { row: 0, col: 2 });

        assert_eq!(session.selected_text(), None);
    }

    #[test]
    fn slices_selected_text_by_characters() {
        assert_eq!(slice_chars("a你好b", 1..3), "你好");
    }

    #[test]
    fn maps_terminal_selection_point_from_visible_area() {
        let mut session = TerminalSession::new();
        session.resize(TerminalSize::new(80, 30));
        session.set_bounds(Bounds::new(
            gpui::point(gpui::px(0.), gpui::px(0.)),
            gpui::size(gpui::px(800.), gpui::px(600.)),
        ));
        session.set_visible_range(10..40);

        let point = session.selection_point_from_position(gpui::point(
            gpui::px(TERMINAL_CONTENT_PADDING_X as f32 + TERMINAL_CELL_WIDTH_PX as f32 * 4.0),
            gpui::px(TERMINAL_CONTENT_PADDING_Y as f32 + TERMINAL_LINE_HEIGHT * 2.0),
        ));

        assert_eq!(point, TerminalSelectionPoint { row: 12, col: 4 });
    }

    #[test]
    fn maps_common_terminal_keys() {
        assert_eq!(
            terminal_bytes_from_key(&Keystroke::parse("enter").unwrap()),
            Some(vec![b'\r'])
        );
        assert_eq!(
            terminal_bytes_from_key(&Keystroke::parse("backspace").unwrap()),
            Some(vec![0x7f])
        );
        assert_eq!(
            terminal_bytes_from_key(&Keystroke::parse("tab").unwrap()),
            Some(vec![b'\t'])
        );
        assert_eq!(
            terminal_bytes_from_key(&Keystroke::parse("up").unwrap()),
            Some(b"\x1b[A".to_vec())
        );
    }

    #[test]
    fn maps_control_keys() {
        assert_eq!(
            terminal_bytes_from_key(&Keystroke::parse("ctrl-c").unwrap()),
            Some(vec![3])
        );
        assert_eq!(
            terminal_bytes_from_key(&Keystroke::parse("ctrl-d").unwrap()),
            Some(vec![4])
        );
    }

    #[test]
    fn maps_tab_before_root_focus_navigation() {
        assert_eq!(
            terminal_tab_bytes(&Keystroke::parse("tab").unwrap()),
            Some(vec![b'\t'])
        );
        assert_eq!(
            terminal_tab_bytes(&Keystroke::parse("shift-tab").unwrap()),
            Some(b"\x1b[Z".to_vec())
        );
        assert_eq!(
            terminal_tab_bytes(&Keystroke::parse("cmd-tab").unwrap()),
            None
        );
    }

    #[test]
    fn maps_utf8_text_input() {
        assert_eq!(
            terminal_bytes_from_key(&Keystroke::parse("a->中").unwrap()),
            Some("中".as_bytes().to_vec())
        );
    }

    #[test]
    fn ignores_uncommitted_ime_keystrokes() {
        assert_eq!(
            terminal_bytes_from_key(&Keystroke::parse("a").unwrap()),
            None
        );
    }

    #[test]
    fn normalizes_paste_newlines_for_terminal_input() {
        assert_eq!(
            terminal_paste_bytes("echo 你好\r\npwd\nwhoami\rdate"),
            b"echo \xe4\xbd\xa0\xe5\xa5\xbd\rpwd\rwhoami\rdate".to_vec()
        );
    }

    #[test]
    fn replaces_ime_text_by_utf16_range() {
        let mut text = "你a好".to_string();

        replace_utf16_text_range(&mut text, Some(1..2), "中");

        assert_eq!(text, "你中好");
    }
}
