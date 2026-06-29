use std::{rc::Rc, sync::mpsc::Receiver, time::Duration};

use gpui::{
    Context, KeyDownEvent, Keystroke, Pixels, ScrollStrategy, Size, Timer, Window, px, size,
};
use gpui_component::VirtualListScrollHandle;
use vt100::Parser;

use crate::{
    ipc::{ServerResource, TerminalCommand, TerminalEvent, open_local_terminal, open_ssh_terminal},
    ui::TextKey,
};

use super::{Xssh, tabs::TerminalId};

pub(super) const TERMINAL_ROWS: u16 = 30;
pub(super) const TERMINAL_COLS: u16 = 100;
pub(super) const TERMINAL_LINE_HEIGHT: f32 = 18.0;
const TERMINAL_MAX_DISPLAY_ROWS: usize = 10_000;
const TERMINAL_HISTORY_LIMIT: usize = 4 * 1024 * 1024;

pub(super) struct TerminalSession {
    pub(super) scroll_handle: VirtualListScrollHandle,
    pub(super) rows: u16,
    pub(super) cols: u16,
    pub(super) input: Option<std::sync::mpsc::Sender<TerminalCommand>>,
    display: TerminalDisplay,
    state: TerminalConnectionState,
    pending_scroll_to_bottom: bool,
    history: Vec<u8>,
}

struct TerminalDisplay {
    lines: Vec<String>,
    line_sizes: Rc<Vec<Size<Pixels>>>,
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
        Self {
            scroll_handle: VirtualListScrollHandle::new(),
            rows: TERMINAL_ROWS,
            cols: TERMINAL_COLS,
            input: None,
            display: TerminalDisplay::from_history(&[], TERMINAL_COLS, TERMINAL_ROWS),
            state: TerminalConnectionState::Idle,
            pending_scroll_to_bottom: false,
            history: Vec::new(),
        }
    }

    pub(super) fn resize(&mut self, cols: u16, rows: u16) -> bool {
        if self.cols == cols && self.rows == rows {
            return false;
        }

        self.cols = cols;
        self.rows = rows;
        self.rebuild_display();
        true
    }

    fn process_output(&mut self, bytes: &[u8]) {
        self.remember_output(bytes);
        self.rebuild_display();
        self.pending_scroll_to_bottom = true;
    }

    fn write_status(&mut self, text: impl AsRef<str>) {
        let line = format!("\r\n{}\r\n", text.as_ref());
        self.process_output(line.as_bytes());
    }

    fn remember_output(&mut self, bytes: &[u8]) {
        self.history.extend_from_slice(bytes);

        if self.history.len() <= TERMINAL_HISTORY_LIMIT {
            return;
        }

        let overflow = self.history.len() - TERMINAL_HISTORY_LIMIT;
        let drain_end = self
            .history
            .get(overflow..)
            .and_then(|tail| tail.iter().position(|byte| *byte == b'\n'))
            .map(|position| overflow + position + 1)
            .unwrap_or(overflow);

        self.history.drain(..drain_end);
    }

    fn rebuild_display(&mut self) {
        self.display = TerminalDisplay::from_history(&self.history, self.cols, self.rows);
    }

    pub(super) fn display_line_sizes(&self) -> Rc<Vec<Size<Pixels>>> {
        self.display.line_sizes.clone()
    }

    pub(super) fn display_line(&self, row: usize) -> Option<&str> {
        self.display.lines.get(row).map(String::as_str)
    }

    fn display_len(&self) -> usize {
        self.display.lines.len()
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
    fn from_history(history: &[u8], cols: u16, viewport_rows: u16) -> Self {
        let display_rows = terminal_display_rows(history, cols, viewport_rows);
        let mut parser = Parser::new(display_rows, cols, 0);
        parser.process(history);

        let screen = parser.screen();
        let (_, cols) = screen.size();
        let (cursor_row, cursor_col) = screen.cursor_position();
        let hide_cursor = screen.hide_cursor();
        let raw_lines = screen.rows(0, cols).collect::<Vec<_>>();
        let last_visible_row = last_visible_terminal_row(&raw_lines, cursor_row);
        let lines = raw_lines
            .into_iter()
            .take(last_visible_row + 1)
            .enumerate()
            .map(|(row, line)| {
                if !hide_cursor && row == usize::from(cursor_row) {
                    line_with_cursor(line, usize::from(cursor_col))
                } else if line.is_empty() {
                    " ".to_string()
                } else {
                    line
                }
            })
            .collect::<Vec<_>>();

        Self {
            line_sizes: terminal_line_sizes(lines.len().max(1)),
            lines,
        }
    }
}

fn terminal_line_sizes(count: usize) -> Rc<Vec<Size<Pixels>>> {
    Rc::new(
        (0..count)
            .map(|_| size(px(0.), px(TERMINAL_LINE_HEIGHT)))
            .collect(),
    )
}

fn terminal_display_rows(history: &[u8], cols: u16, viewport_rows: u16) -> u16 {
    let cols = usize::from(cols.max(1));
    let mut rows = 1_usize;
    let mut current_col = 0_usize;
    let mut in_escape = false;

    for byte in history {
        match *byte {
            0x1b => in_escape = true,
            b'[' | b']' | b'(' | b')' if in_escape => {}
            0x40..=0x7e if in_escape => in_escape = false,
            _ if in_escape => {}
            b'\n' => {
                rows += 1;
                current_col = 0;
            }
            b'\r' => current_col = 0,
            b'\t' => current_col += 4,
            0x20..=0x7e | 0x80..=0xff => current_col += 1,
            _ => {}
        }

        if current_col >= cols {
            rows += current_col / cols;
            current_col %= cols;
        }
    }

    rows.clamp(usize::from(viewport_rows.max(1)), TERMINAL_MAX_DISPLAY_ROWS) as u16
}

fn line_with_cursor(line: String, cursor_col: usize) -> String {
    let mut chars = line.chars().collect::<Vec<_>>();

    if chars.len() <= cursor_col {
        chars.resize(cursor_col, ' ');
        chars.push('█');
    } else {
        chars[cursor_col] = '█';
    }

    chars.into_iter().collect()
}

fn last_visible_terminal_row(lines: &[String], cursor_row: u16) -> usize {
    let last_content_row = lines
        .iter()
        .rposition(|line| !line.trim_end().is_empty())
        .unwrap_or(0);
    let cursor_row = usize::from(cursor_row).min(lines.len().saturating_sub(1));

    last_content_row.max(cursor_row)
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

        let handle = open_ssh_terminal(server, session.cols, session.rows);
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

        let handle = open_local_terminal(session.cols, session.rows);
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
        cols: u16,
        rows: u16,
    ) {
        let Some(session) = self.terminal_sessions.get_mut(&terminal_id) else {
            return;
        };

        if !session.resize(cols, rows) {
            return;
        }

        if let Some(input) = &session.input {
            let _ = input.send(TerminalCommand::Resize { cols, rows });
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
                Timer::after(Duration::from_millis(16)).await;

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

        for event in events {
            match event {
                TerminalEvent::Connected => {
                    session.mark_connected();
                }
                TerminalEvent::Output(bytes) => {
                    session.process_output(&bytes);
                }
                TerminalEvent::Disconnected => {
                    if !session.mark_closed() {
                        continue;
                    }

                    session.input = None;
                    session.write_status(self.language.tr(TextKey::TerminalDisconnected));
                }
                TerminalEvent::Error(error) => {
                    if !session.mark_closed() {
                        continue;
                    }

                    session.input = None;
                    session.write_status(error);
                }
            }
        }

        cx.notify();
        self.terminal_sessions
            .get(&terminal_id)
            .map(TerminalSession::should_poll)
            .unwrap_or(false)
    }
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
    fn trims_trailing_blank_terminal_rows() {
        let lines = vec![
            "prompt".to_string(),
            String::new(),
            String::new(),
            String::new(),
        ];

        assert_eq!(last_visible_terminal_row(&lines, 0), 0);
    }

    #[test]
    fn keeps_cursor_row_visible() {
        let lines = vec![
            "prompt".to_string(),
            String::new(),
            String::new(),
            String::new(),
        ];

        assert_eq!(last_visible_terminal_row(&lines, 2), 2);
    }

    #[test]
    fn display_lines_include_output_beyond_viewport_rows() {
        let mut session = TerminalSession::new();
        session.resize(80, 6);
        let output = (0..40)
            .map(|row| format!("line {row}\r\n"))
            .collect::<String>();

        session.process_output(output.as_bytes());

        assert!(session.display_len() > usize::from(session.rows));
        assert!(
            (0..session.display_len())
                .filter_map(|row| session.display_line(row))
                .any(|line| line.contains("line 39"))
        );
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
