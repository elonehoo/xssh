use std::{rc::Rc, sync::mpsc::Receiver, time::Duration};

use gpui::{
    Context, FontStyle, FontWeight, HighlightStyle, KeyDownEvent, Keystroke, Pixels,
    ScrollStrategy, Size, StyledText, UnderlineStyle, Window, px, rgb, size,
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

use super::{Xssh, tabs::TerminalId};

pub(super) const TERMINAL_ROWS: u16 = 30;
pub(super) const TERMINAL_COLS: u16 = 100;
pub(super) const TERMINAL_LINE_HEIGHT: f32 = 18.0;
const TERMINAL_MAX_DISPLAY_ROWS: usize = 10_000;
const TERMINAL_SCROLLBACK_ROWS: usize = TERMINAL_MAX_DISPLAY_ROWS;

pub(super) struct TerminalSession {
    pub(super) scroll_handle: VirtualListScrollHandle,
    size: TerminalSize,
    pub(super) input: Option<std::sync::mpsc::Sender<TerminalCommand>>,
    display: TerminalDisplay,
    parser: Parser,
    state: TerminalConnectionState,
    pending_scroll_to_bottom: bool,
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

impl TerminalSession {
    pub(super) fn new() -> Self {
        let size = TerminalSize::new(TERMINAL_COLS, TERMINAL_ROWS);
        let mut parser = Parser::new(size.rows, size.cols, TERMINAL_SCROLLBACK_ROWS);
        let display = TerminalDisplay::from_parser(&mut parser);

        Self {
            scroll_handle: VirtualListScrollHandle::new(),
            size,
            input: None,
            display,
            parser,
            state: TerminalConnectionState::Idle,
            pending_scroll_to_bottom: false,
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

    pub(super) fn styled_text(&self, palette: TerminalThemePalette) -> StyledText {
        if self.is_plain() {
            return StyledText::new(self.text.clone());
        }

        let highlights = self
            .runs
            .iter()
            .scan(0, |offset, run| {
                let start = *offset;
                let end = start + run.len;
                *offset = end;
                Some((start..end, run.style.highlight(palette)))
            })
            .collect::<Vec<_>>();

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
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(bytes) = terminal_bytes_from_key(&event.keystroke) else {
            return;
        };

        if self.send_terminal_input(terminal_id, bytes) {
            cx.stop_propagation();
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
                while let Ok(event) = events.try_recv() {
                    pending.push(event);
                }

                if pending.is_empty() {
                    continue;
                }

                let keep_polling = this
                    .update(cx, |this, cx| {
                        this.apply_terminal_events(terminal_id, pending, cx)
                    })
                    .unwrap_or(false);

                if !keep_polling {
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
        let Some(session) = self.terminal_sessions.get_mut(&terminal_id) else {
            return false;
        };

        let mut pending_output = Vec::new();

        for event in events {
            match event {
                TerminalEvent::Connected => {
                    flush_terminal_output(session, &mut pending_output);
                    session.mark_connected();
                }
                TerminalEvent::Output(bytes) => {
                    pending_output.extend_from_slice(&bytes);
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
        cx.notify();
        self.terminal_sessions
            .get(&terminal_id)
            .map(TerminalSession::should_poll)
            .unwrap_or(false)
    }
}

fn flush_terminal_output(session: &mut TerminalSession, pending_output: &mut Vec<u8>) {
    if pending_output.is_empty() {
        return;
    }

    session.process_output(pending_output);
    pending_output.clear();
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
            let text = keystroke.key_char.as_ref().unwrap_or(&keystroke.key);
            if text.chars().count() != 1 {
                return None;
            }
            bytes.extend_from_slice(text.as_bytes());
        }
    }

    Some(bytes)
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
}
