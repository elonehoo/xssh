use std::{
    env,
    fs::File as LocalFile,
    io::{ErrorKind, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context as AnyhowContext, Result, anyhow};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use ssh2::{OpenFlags, OpenType, Session};

use super::{AuthenticationMode, ServerConnectionDraft, ServerResource, SshConnectionTestResult};

const SSH_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const XSSH_CWD_OSC_PREFIX: &[u8] = b"\x1b]777;xssh-cwd=";
const STANDARD_CWD_OSC_PREFIX: &[u8] = b"\x1b]7;file://";
const HIDDEN_CWD_QUERY_COMMAND: &str =
    " d=$(pwd -P 2>/dev/null) && printf '\\033]777;xssh-cwd=%s\\007' \"$d\"\n";
const HIDDEN_CWD_QUERY_TIMEOUT: Duration = Duration::from_millis(1200);
static NEXT_UPLOAD_TASK_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TerminalSize {
    pub(crate) cols: u16,
    pub(crate) rows: u16,
}

impl TerminalSize {
    pub(crate) const fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }
}

pub(crate) enum TerminalCommand {
    Input(Vec<u8>),
    Resize(TerminalSize),
    UploadFiles { paths: Vec<PathBuf> },
    Close,
}

pub(crate) enum TerminalEvent {
    Connected,
    Output(Vec<u8>),
    UploadTask(UploadTaskEvent),
    Disconnected,
    Error(String),
}

#[derive(Clone, Debug)]
pub(crate) struct UploadTaskEvent {
    pub(crate) task_id: u64,
    pub(crate) server_name: String,
    pub(crate) kind: UploadTaskEventKind,
}

#[derive(Clone, Debug)]
pub(crate) enum UploadTaskEventKind {
    Started {
        remote_directory: Option<String>,
        file_count: usize,
    },
    FileSucceeded {
        local_path: String,
        remote_path: String,
    },
    FileFailed {
        local_path: String,
        error: String,
    },
    Finished {
        succeeded: usize,
        failed: usize,
    },
    Failed {
        error: String,
    },
}

pub(crate) struct TerminalHandle {
    pub(crate) input: Sender<TerminalCommand>,
    pub(crate) events: Receiver<TerminalEvent>,
}

#[derive(Default)]
struct PendingSshTerminalCommands {
    input: Vec<u8>,
    resize: Option<TerminalSize>,
    upload_files: Vec<PendingUploadFiles>,
    submitted_lines: Vec<String>,
}

struct PendingUploadFiles {
    paths: Vec<PathBuf>,
}

impl PendingSshTerminalCommands {
    fn push_input(&mut self, bytes: Vec<u8>, input_tracker: &mut TerminalInputTracker) {
        self.submitted_lines.extend(input_tracker.push(&bytes));
        self.input.extend(bytes);
    }

    fn set_resize(&mut self, size: TerminalSize) {
        self.resize = Some(size);
    }

    fn push_upload_files(&mut self, paths: Vec<PathBuf>) {
        self.upload_files.push(PendingUploadFiles { paths });
    }

    fn flush_resize(&mut self, channel: &mut ssh2::Channel) -> Result<()> {
        if let Some(size) = self.resize
            && try_resize_terminal(channel, size)?
        {
            self.resize = None;
        }

        Ok(())
    }

    fn flush_input(&mut self, channel: &mut ssh2::Channel) -> Result<()> {
        if !self.input.is_empty() {
            write_pending_input(channel, &mut self.input)?;
        }

        Ok(())
    }

    fn has_input(&self) -> bool {
        !self.input.is_empty()
    }

    fn take_upload_files(&mut self) -> Vec<PendingUploadFiles> {
        std::mem::take(&mut self.upload_files)
    }

    fn take_submitted_lines(&mut self) -> Vec<String> {
        std::mem::take(&mut self.submitted_lines)
    }
}

struct TerminalInputTracker {
    line: Vec<u8>,
    reliable: bool,
}

impl Default for TerminalInputTracker {
    fn default() -> Self {
        Self {
            line: Vec::new(),
            reliable: true,
        }
    }
}

impl TerminalInputTracker {
    fn has_pending_line_input(&self) -> bool {
        !self.line.is_empty()
    }

    fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        let mut submitted = Vec::new();

        for &byte in bytes {
            match byte {
                b'\r' | b'\n' => {
                    if self.reliable
                        && let Ok(line) = String::from_utf8(self.line.clone())
                    {
                        submitted.push(line);
                    }

                    self.line.clear();
                    self.reliable = true;
                }
                0x7f | 0x08 => {
                    self.line.pop();
                }
                0x15 => {
                    self.line.clear();
                }
                byte if byte == b'\t' || byte == b' ' || byte >= 0x20 => {
                    self.line.push(byte);
                }
                _ => {
                    self.reliable = false;
                }
            }
        }

        submitted
    }
}

#[derive(Debug)]
struct RemoteDirectoryTracker {
    confirmed: Option<String>,
}

#[derive(Default)]
struct RemotePwdOutputTracker {
    pending: bool,
    buffer: String,
}

#[derive(Default)]
struct RemoteDirectoryOutputFilter {
    pending: Vec<u8>,
    suppress_until_user_input: bool,
}

#[derive(Default)]
struct RemoteDirectoryResolver {
    directory: RemoteDirectoryTracker,
    pwd_output_tracker: RemotePwdOutputTracker,
    output_filter: RemoteDirectoryOutputFilter,
}

#[derive(Default)]
struct UploadSummary {
    succeeded: usize,
    failed: usize,
}

impl Default for RemoteDirectoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl RemoteDirectoryTracker {
    fn new() -> Self {
        Self { confirmed: None }
    }

    fn confirmed_current(&self) -> Option<&str> {
        self.confirmed.as_deref()
    }

    fn mark_unconfirmed(&mut self) {
        self.confirmed = None;
    }

    fn apply_resolved(&mut self, directory: String) {
        self.confirmed = Some(directory);
    }
}

impl RemotePwdOutputTracker {
    fn observe_submitted_lines(&mut self, lines: &[String]) {
        for line in lines {
            if is_pwd_command(line) {
                self.pending = true;
                self.buffer.clear();
            } else if self.pending && !line.trim().is_empty() {
                self.pending = false;
                self.buffer.clear();
            }
        }
    }

    fn push_output(&mut self, bytes: &[u8]) -> Option<String> {
        if !self.pending {
            return None;
        }

        self.buffer.push_str(&String::from_utf8_lossy(bytes));
        if self.buffer.len() > 16 * 1024 {
            let keep_from = self.buffer.len().saturating_sub(8 * 1024);
            self.buffer.drain(..keep_from);
        }

        let Some(directory) = extract_pwd_output_directory(&self.buffer) else {
            return None;
        };

        self.pending = false;
        self.buffer.clear();
        Some(directory)
    }
}

#[derive(Clone, Copy)]
enum RemoteDirectoryOutputEvent {
    XsshCwdOsc,
    StandardCwdOsc,
}

struct OscTerminator {
    content_end: usize,
    sequence_end: usize,
}

impl RemoteDirectoryOutputFilter {
    fn before_user_input(&mut self) {
        self.pending.clear();
        self.suppress_until_user_input = false;
    }

    fn begin_hidden_directory_query(&mut self) {
        self.pending.clear();
        self.suppress_until_user_input = true;
    }

    fn push_output(&mut self, bytes: &[u8]) -> (Vec<u8>, Vec<String>) {
        self.pending.extend_from_slice(bytes);

        let mut output = Vec::new();
        let mut directories = Vec::new();

        loop {
            let Some((event_index, event)) = self.next_event() else {
                let tail_len = partial_remote_directory_event_tail_len(&self.pending);
                let emit_len = self.pending.len().saturating_sub(tail_len);
                self.drain_pending_prefix(emit_len, &mut output);
                break;
            };

            self.drain_pending_prefix(event_index, &mut output);

            match event {
                RemoteDirectoryOutputEvent::XsshCwdOsc => {
                    let Some(terminator) =
                        find_osc_terminator(&self.pending, XSSH_CWD_OSC_PREFIX.len())
                    else {
                        break;
                    };

                    if let Some(directory) = parse_xssh_cwd_osc_path(
                        &self.pending[XSSH_CWD_OSC_PREFIX.len()..terminator.content_end],
                    ) {
                        directories.push(directory);
                    }

                    self.pending.drain(..terminator.sequence_end);
                }
                RemoteDirectoryOutputEvent::StandardCwdOsc => {
                    let Some(terminator) =
                        find_osc_terminator(&self.pending, STANDARD_CWD_OSC_PREFIX.len())
                    else {
                        break;
                    };

                    if let Some(directory) = parse_standard_cwd_osc_path(
                        &self.pending[STANDARD_CWD_OSC_PREFIX.len()..terminator.content_end],
                    ) {
                        directories.push(directory);
                    }

                    self.pending.drain(..terminator.sequence_end);
                }
            }
        }

        (output, directories)
    }

    fn next_event(&self) -> Option<(usize, RemoteDirectoryOutputEvent)> {
        let mut next = None;
        next = nearest_event(
            next,
            find_subslice(&self.pending, XSSH_CWD_OSC_PREFIX),
            RemoteDirectoryOutputEvent::XsshCwdOsc,
        );
        nearest_event(
            next,
            find_subslice(&self.pending, STANDARD_CWD_OSC_PREFIX),
            RemoteDirectoryOutputEvent::StandardCwdOsc,
        )
    }

    fn drain_pending_prefix(&mut self, len: usize, output: &mut Vec<u8>) {
        if len == 0 {
            return;
        }

        let drained = self.pending.drain(..len).collect::<Vec<_>>();
        if !self.suppress_until_user_input {
            output.extend(drained);
        }
    }
}

impl RemoteDirectoryResolver {
    fn before_user_input(&mut self) {
        self.output_filter.before_user_input();
    }

    fn observe_submitted_lines(&mut self, lines: &[String]) {
        self.pwd_output_tracker.observe_submitted_lines(lines);

        if lines.iter().any(|line| !line.trim().is_empty()) {
            self.directory.mark_unconfirmed();
        }
    }

    fn confirmed_directory_for_upload(
        &mut self,
        channel: &mut ssh2::Channel,
        event_tx: &Sender<TerminalEvent>,
        buffer: &mut [u8],
        allow_hidden_query: bool,
    ) -> Result<Option<String>> {
        if let Some(directory) = self.directory.confirmed_current() {
            return Ok(Some(directory.to_string()));
        }

        if allow_hidden_query {
            let _ = self.refresh_from_shell(channel, event_tx, buffer)?;
        }

        Ok(self.directory.confirmed_current().map(ToString::to_string))
    }

    fn refresh_from_shell(
        &mut self,
        channel: &mut ssh2::Channel,
        event_tx: &Sender<TerminalEvent>,
        buffer: &mut [u8],
    ) -> Result<bool> {
        let mut pending_input = HIDDEN_CWD_QUERY_COMMAND.as_bytes().to_vec();
        let started_at = Instant::now();
        self.output_filter.begin_hidden_directory_query();

        while started_at.elapsed() < HIDDEN_CWD_QUERY_TIMEOUT {
            if !pending_input.is_empty() {
                write_pending_input(channel, &mut pending_input)?;
            }

            let mut resolved = false;
            loop {
                match channel.read(buffer) {
                    Ok(0) => break,
                    Ok(size) => {
                        resolved |= self.process_output(&buffer[..size], event_tx);
                    }
                    Err(error) => {
                        let message = error.to_string();
                        if error.kind() == ErrorKind::WouldBlock {
                            break;
                        }

                        return Err(anyhow!(message));
                    }
                }
            }

            if resolved {
                return Ok(true);
            }

            if channel.eof() {
                return Ok(false);
            }

            thread::sleep(Duration::from_millis(8));
        }

        Ok(false)
    }

    fn process_output(&mut self, bytes: &[u8], event_tx: &Sender<TerminalEvent>) -> bool {
        let (filtered_output, directories) = self.output_filter.push_output(bytes);
        let mut resolved = false;

        for directory in directories {
            self.directory.apply_resolved(directory);
            resolved = true;
        }

        if let Some(directory) = self.pwd_output_tracker.push_output(&filtered_output) {
            self.directory.apply_resolved(directory);
            resolved = true;
        }

        if !filtered_output.is_empty() {
            let _ = event_tx.send(TerminalEvent::Output(filtered_output));
        }

        resolved
    }
}

pub(crate) fn open_ssh_terminal(server: ServerResource, size: TerminalSize) -> TerminalHandle {
    let (input, input_rx) = mpsc::channel();
    let (event_tx, events) = mpsc::channel();

    thread::spawn(move || {
        if let Err(error) = run_ssh_terminal(server, size, input_rx, event_tx.clone()) {
            let _ = event_tx.send(TerminalEvent::Error(error.to_string()));
        }
    });

    TerminalHandle { input, events }
}

pub(crate) fn open_local_terminal(size: TerminalSize) -> TerminalHandle {
    let (input, input_rx) = mpsc::channel();
    let (event_tx, events) = mpsc::channel();

    thread::spawn(move || {
        if let Err(error) = run_local_terminal(size, input_rx, event_tx.clone()) {
            let _ = event_tx.send(TerminalEvent::Error(error.to_string()));
        }
    });

    TerminalHandle { input, events }
}

pub(crate) fn test_ssh_connection(connection: &ServerConnectionDraft) -> Result<()> {
    validate_password_connection(&connection.authentication, &connection.password)?;
    let _session = authenticated_ssh_session(
        &connection.host,
        connection.port,
        &connection.username,
        &connection.password,
        Some(SSH_CONNECT_TIMEOUT),
    )?;

    Ok(())
}

pub(crate) fn spawn_ssh_connection_test<T: Send + 'static>(
    connection: ServerConnectionDraft,
    context: T,
) -> Receiver<SshConnectionTestResult<T>> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = test_ssh_connection(&connection).map_err(|error| error.to_string());
        let _ = tx.send(SshConnectionTestResult { context, result });
    });

    rx
}

fn run_local_terminal(
    size: TerminalSize,
    input_rx: Receiver<TerminalCommand>,
    event_tx: Sender<TerminalEvent>,
) -> Result<()> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(pty_size(size))
        .context("创建本机 PTY 失败")?;
    let shell = env::var("SHELL")
        .ok()
        .filter(|shell| !shell.is_empty())
        .unwrap_or_else(|| "/bin/zsh".to_string());
    let mut command = CommandBuilder::new(shell);
    command.env("TERM", "xterm-256color");

    if let Some(home) = env::var_os("HOME") {
        command.cwd(home);
    }

    let mut child = pair
        .slave
        .spawn_command(command)
        .context("启动本机 shell 失败")?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().context("读取 PTY 失败")?;
    let mut writer = pair.master.take_writer().context("写入 PTY 失败")?;
    let reader_events = event_tx.clone();

    thread::spawn(move || {
        let mut buffer = [0_u8; 8192];

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    let _ = reader_events.send(TerminalEvent::Disconnected);
                    return;
                }
                Ok(size) => {
                    let _ = reader_events.send(TerminalEvent::Output(buffer[..size].to_vec()));
                }
                Err(error) => {
                    let _ = reader_events.send(TerminalEvent::Error(error.to_string()));
                    return;
                }
            }
        }
    });

    let _ = event_tx.send(TerminalEvent::Connected);

    loop {
        if drain_local_terminal_commands(pair.master.as_ref(), &mut writer, &input_rx)? {
            let _ = child.kill();
            return Ok(());
        }

        if child.try_wait()?.is_some() {
            let _ = event_tx.send(TerminalEvent::Disconnected);
            return Ok(());
        }

        thread::sleep(Duration::from_millis(8));
    }
}

fn run_ssh_terminal(
    server: ServerResource,
    size: TerminalSize,
    input_rx: Receiver<TerminalCommand>,
    event_tx: Sender<TerminalEvent>,
) -> Result<()> {
    validate_password_terminal(&server)?;

    let port = u16::try_from(server.port).context("SSH 端口不是有效端口")?;
    let session =
        authenticated_ssh_session(&server.host, port, &server.username, &server.password, None)?;

    let mut channel = session.channel_session().context("创建 SSH channel 失败")?;
    channel
        .request_pty(
            "xterm-256color",
            None,
            Some((size.cols.into(), size.rows.into(), 0, 0)),
        )
        .context("请求 SSH PTY 失败")?;
    channel.shell().context("启动 SSH shell 失败")?;
    session.set_blocking(false);

    let _ = event_tx.send(TerminalEvent::Connected);
    let mut remote_directory = RemoteDirectoryResolver::default();
    let mut pending_commands = PendingSshTerminalCommands::default();
    let mut input_tracker = TerminalInputTracker::default();
    let mut buffer = [0_u8; 8192];

    loop {
        match drain_ssh_terminal_commands(
            &mut channel,
            &input_rx,
            &mut pending_commands,
            &mut input_tracker,
        ) {
            Ok(true) => return Ok(()),
            Ok(false) => {}
            Err(error) => {
                let _ = event_tx.send(TerminalEvent::Error(error.to_string()));
                return Err(error);
            }
        }

        pending_commands.flush_resize(&mut channel)?;
        if pending_commands.has_input() {
            remote_directory.before_user_input();
        }
        pending_commands.flush_input(&mut channel)?;
        let submitted_lines = pending_commands.take_submitted_lines();
        remote_directory.observe_submitted_lines(&submitted_lines);
        for upload in pending_commands.take_upload_files() {
            let upload_directory = remote_directory.confirmed_directory_for_upload(
                &mut channel,
                &event_tx,
                &mut buffer,
                !input_tracker.has_pending_line_input(),
            )?;

            spawn_sftp_uploads(&server, upload_directory, upload.paths, &event_tx);
        }

        loop {
            match channel.read(&mut buffer) {
                Ok(0) => break,
                Ok(size) => {
                    remote_directory.process_output(&buffer[..size], &event_tx);
                }
                Err(error) => {
                    let message = error.to_string();
                    if error.kind() == ErrorKind::WouldBlock {
                        break;
                    }

                    return Err(anyhow!(message));
                }
            }
        }

        if channel.eof() {
            let _ = event_tx.send(TerminalEvent::Disconnected);
            return Ok(());
        }

        thread::sleep(Duration::from_millis(8));
    }
}

fn drain_ssh_terminal_commands(
    channel: &mut ssh2::Channel,
    input_rx: &Receiver<TerminalCommand>,
    pending_commands: &mut PendingSshTerminalCommands,
    input_tracker: &mut TerminalInputTracker,
) -> Result<bool> {
    loop {
        match input_rx.try_recv() {
            Ok(TerminalCommand::Input(bytes)) => {
                pending_commands.push_input(bytes, input_tracker);
            }
            Ok(TerminalCommand::Resize(size)) => pending_commands.set_resize(size),
            Ok(TerminalCommand::UploadFiles { paths }) => pending_commands.push_upload_files(paths),
            Ok(TerminalCommand::Close) => {
                let _ = channel.close();
                return Ok(true);
            }
            Err(TryRecvError::Empty) => return Ok(false),
            Err(TryRecvError::Disconnected) => {
                let _ = channel.close();
                return Ok(true);
            }
        }
    }
}

fn drain_local_terminal_commands(
    master: &dyn MasterPty,
    writer: &mut dyn Write,
    input_rx: &Receiver<TerminalCommand>,
) -> Result<bool> {
    loop {
        match input_rx.try_recv() {
            Ok(TerminalCommand::Input(bytes)) => writer.write_all(&bytes)?,
            Ok(TerminalCommand::Resize(size)) => {
                master.resize(pty_size(size))?;
            }
            Ok(TerminalCommand::UploadFiles { .. }) => {}
            Ok(TerminalCommand::Close) => return Ok(true),
            Err(TryRecvError::Empty) => return Ok(false),
            Err(TryRecvError::Disconnected) => return Ok(true),
        }
    }
}

fn spawn_sftp_uploads(
    server: &ServerResource,
    remote_directory: Option<String>,
    local_paths: Vec<PathBuf>,
    event_tx: &Sender<TerminalEvent>,
) {
    if local_paths.is_empty() {
        return;
    }

    let task_id = next_upload_task_id();
    let Some(remote_directory) = remote_directory else {
        send_upload_task_event(
            event_tx,
            server,
            task_id,
            UploadTaskEventKind::Started {
                remote_directory: None,
                file_count: local_paths.len(),
            },
        );
        send_upload_task_event(
            event_tx,
            server,
            task_id,
            UploadTaskEventKind::Failed {
                error: "无法自动确定远端当前目录，请回到 shell 提示符后重试拖拽上传。".to_string(),
            },
        );
        return;
    };

    let server = server.clone();
    let event_tx = event_tx.clone();

    thread::spawn(move || {
        upload_files_with_status(server, task_id, remote_directory, local_paths, event_tx);
    });
}

fn upload_files_with_status(
    server: ServerResource,
    task_id: u64,
    remote_directory: String,
    local_paths: Vec<PathBuf>,
    event_tx: Sender<TerminalEvent>,
) {
    send_upload_task_event(
        &event_tx,
        &server,
        task_id,
        UploadTaskEventKind::Started {
            remote_directory: Some(remote_directory.clone()),
            file_count: local_paths.len(),
        },
    );

    match upload_files_to_remote(&server, task_id, &remote_directory, &local_paths, &event_tx) {
        Ok(summary) => {
            send_upload_task_event(
                &event_tx,
                &server,
                task_id,
                UploadTaskEventKind::Finished {
                    succeeded: summary.succeeded,
                    failed: summary.failed,
                },
            );
        }
        Err(error) => {
            send_upload_task_event(
                &event_tx,
                &server,
                task_id,
                UploadTaskEventKind::Failed {
                    error: error.to_string(),
                },
            );
        }
    }
}

fn upload_files_to_remote(
    server: &ServerResource,
    task_id: u64,
    remote_directory: &str,
    local_paths: &[PathBuf],
    event_tx: &Sender<TerminalEvent>,
) -> Result<UploadSummary> {
    validate_password_terminal(server)?;

    let port = u16::try_from(server.port).context("SSH 端口不是有效端口")?;
    let session =
        authenticated_ssh_session(&server.host, port, &server.username, &server.password, None)?;
    let sftp = session.sftp().context("创建 SFTP 会话失败")?;
    let mut summary = UploadSummary::default();

    for local_path in local_paths {
        let display_name = local_path.display().to_string();
        match upload_one_file(&sftp, remote_directory, local_path) {
            Ok(remote_path) => {
                summary.succeeded += 1;
                send_upload_task_event(
                    event_tx,
                    server,
                    task_id,
                    UploadTaskEventKind::FileSucceeded {
                        local_path: display_name,
                        remote_path,
                    },
                );
            }
            Err(error) => {
                summary.failed += 1;
                send_upload_task_event(
                    event_tx,
                    server,
                    task_id,
                    UploadTaskEventKind::FileFailed {
                        local_path: display_name,
                        error: error.to_string(),
                    },
                );
            }
        }
    }

    Ok(summary)
}

fn next_upload_task_id() -> u64 {
    NEXT_UPLOAD_TASK_ID.fetch_add(1, Ordering::Relaxed)
}

fn send_upload_task_event(
    event_tx: &Sender<TerminalEvent>,
    server: &ServerResource,
    task_id: u64,
    kind: UploadTaskEventKind,
) {
    let _ = event_tx.send(TerminalEvent::UploadTask(UploadTaskEvent {
        task_id,
        server_name: server.name.clone(),
        kind,
    }));
}

fn upload_one_file(sftp: &ssh2::Sftp, remote_directory: &str, local_path: &Path) -> Result<String> {
    let metadata = local_path
        .metadata()
        .with_context(|| format!("无法读取本地文件: {}", local_path.display()))?;

    if !metadata.is_file() {
        return Err(anyhow!("暂不支持上传文件夹"));
    }

    let file_name = local_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("本地文件名无效"))?;
    let remote_path = remote_child_path(remote_directory, file_name);
    let mut local_file = LocalFile::open(local_path)
        .with_context(|| format!("无法打开本地文件: {}", local_path.display()))?;
    let mut remote_file = sftp
        .open_mode(
            Path::new(&remote_path),
            OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
            0o644,
            OpenType::File,
        )
        .with_context(|| format!("无法打开远端文件: {remote_path}"))?;

    std::io::copy(&mut local_file, &mut remote_file)
        .with_context(|| format!("写入远端文件失败: {remote_path}"))?;

    Ok(remote_path)
}

fn is_pwd_command(line: &str) -> bool {
    let Some(words) = split_simple_shell_words(line) else {
        return false;
    };

    if words.first().map(String::as_str) != Some("pwd") {
        return false;
    }

    words[1..].iter().all(|arg| {
        arg.starts_with('-')
            && arg
                .trim_start_matches('-')
                .chars()
                .all(|flag| flag == 'L' || flag == 'P')
    })
}

fn extract_pwd_output_directory(output: &str) -> Option<String> {
    clean_terminal_text(output)
        .lines()
        .map(str::trim)
        .find(|line| is_absolute_remote_path(line))
        .map(ToString::to_string)
}

fn is_absolute_remote_path(path: &str) -> bool {
    path.starts_with('/') && !path.contains('\0')
}

fn nearest_event(
    current: Option<(usize, RemoteDirectoryOutputEvent)>,
    next_index: Option<usize>,
    event: RemoteDirectoryOutputEvent,
) -> Option<(usize, RemoteDirectoryOutputEvent)> {
    match (current, next_index) {
        (None, Some(index)) => Some((index, event)),
        (Some((current_index, _)), Some(index)) if index < current_index => Some((index, event)),
        (Some(current), _) => Some(current),
        (None, None) => None,
    }
}

fn partial_remote_directory_event_tail_len(bytes: &[u8]) -> usize {
    partial_match_tail_len(bytes, XSSH_CWD_OSC_PREFIX)
        .max(partial_match_tail_len(bytes, STANDARD_CWD_OSC_PREFIX))
}

fn partial_match_tail_len(bytes: &[u8], needle: &[u8]) -> usize {
    let max_len = bytes.len().min(needle.len().saturating_sub(1));

    (1..=max_len)
        .rev()
        .find(|&len| bytes[bytes.len() - len..] == needle[..len])
        .unwrap_or(0)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn find_osc_terminator(bytes: &[u8], content_start: usize) -> Option<OscTerminator> {
    let mut index = content_start;

    while index < bytes.len() {
        if bytes[index] == b'\x07' {
            return Some(OscTerminator {
                content_end: index,
                sequence_end: index + 1,
            });
        }

        if bytes[index] == b'\x1b' && bytes.get(index + 1) == Some(&b'\\') {
            return Some(OscTerminator {
                content_end: index,
                sequence_end: index + 2,
            });
        }

        index += 1;
    }

    None
}

fn parse_xssh_cwd_osc_path(bytes: &[u8]) -> Option<String> {
    let path = String::from_utf8(bytes.to_vec()).ok()?;

    if is_absolute_remote_path(&path) {
        Some(path)
    } else {
        None
    }
}

fn parse_standard_cwd_osc_path(bytes: &[u8]) -> Option<String> {
    let path_start = bytes.iter().position(|&byte| byte == b'/')?;
    let path = String::from_utf8(percent_decode_bytes(&bytes[path_start..])).ok()?;

    if is_absolute_remote_path(&path) {
        Some(path)
    } else {
        None
    }
}

fn percent_decode_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%'
            && let (Some(high), Some(low)) = (bytes.get(index + 1), bytes.get(index + 2))
            && let (Some(high), Some(low)) = (hex_value(*high), hex_value(*low))
        {
            decoded.push((high << 4) | low);
            index += 3;
            continue;
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    decoded
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn clean_terminal_text(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut cleaned = String::new();
    let mut chars = normalized.chars().peekable();

    while let Some(char) = chars.next() {
        if char == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for char in chars.by_ref() {
                    if ('@'..='~').contains(&char) {
                        break;
                    }
                }
            } else {
                chars.next();
            }
            continue;
        }

        if char == '\n' || char == '\t' || !char.is_control() {
            cleaned.push(char);
        }
    }

    cleaned
}

fn split_simple_shell_words(line: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(char) = chars.next() {
        match char {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '\\' if !in_single_quote => {
                let escaped = chars.next()?;
                current.push(escaped);
            }
            '#' if !in_single_quote && !in_double_quote && current.is_empty() => break,
            char if char.is_whitespace() && !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            ';' | '&' | '|' | '<' | '>' | '(' | ')' if !in_single_quote && !in_double_quote => {
                return None;
            }
            _ => current.push(char),
        }
    }

    if in_single_quote || in_double_quote {
        return None;
    }

    if !current.is_empty() {
        words.push(current);
    }

    Some(words)
}

fn remote_child_path(remote_directory: &str, file_name: &str) -> String {
    if remote_directory == "/" {
        format!("/{file_name}")
    } else {
        format!("{}/{file_name}", remote_directory.trim_end_matches('/'))
    }
}

fn try_resize_terminal(channel: &mut ssh2::Channel, size: TerminalSize) -> Result<bool> {
    match channel.request_pty_size(size.cols.into(), size.rows.into(), None, None) {
        Ok(()) => Ok(true),
        Err(error) => {
            let message = error.to_string();
            let io_error: std::io::Error = error.into();
            if io_error.kind() == ErrorKind::WouldBlock {
                return Ok(false);
            }

            Err(anyhow!(message))
        }
    }
}

fn pty_size(size: TerminalSize) -> PtySize {
    PtySize {
        rows: size.rows,
        cols: size.cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn authenticated_ssh_session(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    connect_timeout: Option<Duration>,
) -> Result<Session> {
    let tcp = connect_ssh_tcp(host, port, connect_timeout)?;
    tcp.set_read_timeout(Some(Duration::from_secs(30))).ok();
    tcp.set_write_timeout(Some(Duration::from_secs(30))).ok();

    let mut session = Session::new().context("创建 SSH session 失败")?;
    session.set_tcp_stream(tcp);
    session.handshake().context("SSH 握手失败")?;
    session
        .userauth_password(username, password)
        .with_context(|| format!("SSH 密码认证失败: {username}"))?;

    if !session.authenticated() {
        return Err(anyhow!("SSH 认证未通过"));
    }

    Ok(session)
}

fn connect_ssh_tcp(host: &str, port: u16, connect_timeout: Option<Duration>) -> Result<TcpStream> {
    let Some(connect_timeout) = connect_timeout else {
        return TcpStream::connect((host, port))
            .with_context(|| format!("连接 SSH 服务器失败: {host}:{port}"));
    };

    let addresses = (host, port)
        .to_socket_addrs()
        .with_context(|| format!("解析 SSH 地址失败: {host}:{port}"))?
        .collect::<Vec<_>>();

    if addresses.is_empty() {
        return Err(anyhow!("没有可用的 SSH 地址: {host}:{port}"));
    }

    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, connect_timeout) {
            Ok(tcp) => return Ok(tcp),
            Err(error) => last_error = Some(error),
        }
    }

    Err(anyhow!(
        "连接 SSH 服务器失败: {host}:{port}: {}",
        last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "未知错误".to_string())
    ))
}

fn write_pending_input(channel: &mut ssh2::Channel, pending_input: &mut Vec<u8>) -> Result<()> {
    match channel.write(pending_input) {
        Ok(0) => {}
        Ok(size) => {
            pending_input.drain(..size);
        }
        Err(error) => {
            let message = error.to_string();
            if error.kind() != ErrorKind::WouldBlock {
                return Err(anyhow!(message));
            }
        }
    }

    Ok(())
}

fn validate_password_terminal(server: &ServerResource) -> Result<()> {
    validate_password_connection(&server.authentication, &server.password)
}

fn validate_password_connection(authentication: &str, password: &str) -> Result<()> {
    if AuthenticationMode::from_label(authentication) != AuthenticationMode::ManualPassword {
        return Err(anyhow!("Direct key 还没有配置 key 字段，暂时不能连接"));
    }

    if password.is_empty() {
        return Err(anyhow!("Host 没有保存密码，无法连接"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_direct_key_until_key_fields_exist() {
        let server = ServerResource {
            id: 1,
            name: "Production".to_string(),
            host: "bastion.example.com".to_string(),
            port: 22,
            username: "root".to_string(),
            authentication: "Direct key".to_string(),
            password: String::new(),
        };

        let error = validate_password_terminal(&server).unwrap_err();
        assert!(error.to_string().contains("Direct key"));
    }

    #[test]
    fn rejects_empty_password() {
        let server = ServerResource {
            id: 1,
            name: "Production".to_string(),
            host: "bastion.example.com".to_string(),
            port: 22,
            username: "root".to_string(),
            authentication: "Manual Password".to_string(),
            password: String::new(),
        };

        let error = validate_password_terminal(&server).unwrap_err();
        assert!(error.to_string().contains("密码"));
    }

    #[test]
    fn connection_test_rejects_empty_password_before_network() {
        let connection = ServerConnectionDraft {
            host: "bastion.example.com".to_string(),
            port: 22,
            username: "root".to_string(),
            authentication: "Manual Password".to_string(),
            password: String::new(),
        };

        let error = test_ssh_connection(&connection).unwrap_err();
        assert!(error.to_string().contains("密码"));
    }

    #[test]
    fn terminal_input_tracker_collects_submitted_lines() {
        let mut tracker = TerminalInputTracker::default();

        assert!(tracker.push(b"cd /var/www").is_empty());
        assert!(tracker.has_pending_line_input());
        assert_eq!(tracker.push(b"\r"), vec!["cd /var/www".to_string()]);
        assert!(!tracker.has_pending_line_input());
    }

    #[test]
    fn terminal_input_tracker_handles_backspace_and_ctrl_u() {
        let mut tracker = TerminalInputTracker::default();

        tracker.push(b"cd /tmpx");
        tracker.push(&[0x7f]);
        assert_eq!(tracker.push(b"\r"), vec!["cd /tmp".to_string()]);

        tracker.push(b"cd /wrong");
        tracker.push(&[0x15]);
        tracker.push(b"cd /right");
        assert_eq!(tracker.push(b"\r"), vec!["cd /right".to_string()]);
    }

    #[test]
    fn detects_supported_pwd_commands() {
        assert!(is_pwd_command("pwd"));
        assert!(is_pwd_command(" pwd -P "));
        assert!(is_pwd_command("pwd -LP"));
        assert!(!is_pwd_command("echo pwd"));
        assert!(!is_pwd_command("pwd /tmp"));
        assert!(!is_pwd_command("pwd --help"));
    }

    #[test]
    fn extracts_pwd_output_directory() {
        assert_eq!(
            extract_pwd_output_directory("pwd\r\n/app/pubinfo-bot\r\n[root@localhost app]# "),
            Some("/app/pubinfo-bot".to_string())
        );
        assert_eq!(
            extract_pwd_output_directory("\x1b[32m/app/pubinfo-bot\x1b[0m\r\n"),
            Some("/app/pubinfo-bot".to_string())
        );
    }

    #[test]
    fn pwd_output_tracker_updates_after_pwd_command() {
        let mut tracker = RemotePwdOutputTracker::default();

        tracker.observe_submitted_lines(&["pwd".to_string()]);

        assert_eq!(tracker.push_output(b"[root@localhost app]# pwd\r\n"), None);
        assert_eq!(
            tracker.push_output(b"/app/pubinfo-bot\r\n[root@localhost app]# "),
            Some("/app/pubinfo-bot".to_string())
        );
        assert_eq!(tracker.push_output(b"/tmp\r\n"), None);
    }

    #[test]
    fn pwd_output_tracker_ignores_unrelated_output() {
        let mut tracker = RemotePwdOutputTracker::default();

        tracker.observe_submitted_lines(&["echo pwd".to_string()]);

        assert_eq!(tracker.push_output(b"/tmp\r\n"), None);
    }

    #[test]
    fn remote_directory_tracker_requires_confirmed_path() {
        let mut tracker = RemoteDirectoryTracker::new();
        assert_eq!(tracker.confirmed_current(), None);

        tracker.apply_resolved("/app".to_string());
        assert_eq!(tracker.confirmed_current(), Some("/app"));

        tracker.mark_unconfirmed();
        assert_eq!(tracker.confirmed_current(), None);
    }

    #[test]
    fn remote_directory_output_filter_reads_hidden_xssh_cwd() {
        let mut filter = RemoteDirectoryOutputFilter::default();

        let (output, directories) =
            filter.push_output(b"before\x1b]777;xssh-cwd=/app/pubinfo-bot\x07after");

        assert_eq!(output, b"beforeafter");
        assert_eq!(directories, vec!["/app/pubinfo-bot".to_string()]);
    }

    #[test]
    fn remote_directory_output_filter_handles_split_osc() {
        let mut filter = RemoteDirectoryOutputFilter::default();

        let (output, directories) = filter.push_output(b"before\x1b]777;xssh-cwd=/app");

        assert_eq!(output, b"before");
        assert!(directories.is_empty());

        let (output, directories) = filter.push_output(b"\x1b\\after");

        assert_eq!(output, b"after");
        assert_eq!(directories, vec!["/app".to_string()]);
    }

    #[test]
    fn remote_directory_output_filter_reads_standard_osc7() {
        let mut filter = RemoteDirectoryOutputFilter::default();

        let (output, directories) =
            filter.push_output(b"\x1b]7;file://localhost/app/pubinfo-bot\x07$ ");

        assert_eq!(output, b"$ ");
        assert_eq!(directories, vec!["/app/pubinfo-bot".to_string()]);
    }

    #[test]
    fn remote_directory_output_filter_decodes_standard_osc7_path() {
        let mut filter = RemoteDirectoryOutputFilter::default();

        let (output, directories) =
            filter.push_output(b"\x1b]7;file://localhost/app/release%20build\x07");

        assert!(output.is_empty());
        assert_eq!(directories, vec!["/app/release build".to_string()]);
    }

    #[test]
    fn remote_directory_output_filter_hides_explicit_directory_query() {
        let mut filter = RemoteDirectoryOutputFilter::default();

        filter.begin_hidden_directory_query();
        let (output, directories) = filter.push_output(
            b"$ d=$(pwd -P 2>/dev/null) && printf hidden\r\n\x1b]777;xssh-cwd=/app\x07$ ",
        );

        assert!(output.is_empty());
        assert_eq!(directories, vec!["/app".to_string()]);

        filter.before_user_input();
        let (output, directories) = filter.push_output(b"pwd\r\n");
        assert_eq!(output, b"pwd\r\n");
        assert!(directories.is_empty());
    }

    #[test]
    fn upload_task_fails_without_known_remote_directory() {
        let server = ServerResource {
            id: 1,
            name: "Production".to_string(),
            host: "bastion.example.com".to_string(),
            port: 22,
            username: "root".to_string(),
            authentication: "Manual Password".to_string(),
            password: "secret".to_string(),
        };
        let (event_tx, event_rx) = mpsc::channel();

        spawn_sftp_uploads(
            &server,
            None,
            vec![PathBuf::from("/tmp/example.txt")],
            &event_tx,
        );

        match event_rx.recv_timeout(Duration::from_secs(1)).unwrap() {
            TerminalEvent::UploadTask(event) => {
                assert_eq!(event.server_name, "Production");
                assert!(matches!(
                    event.kind,
                    UploadTaskEventKind::Started {
                        remote_directory: None,
                        file_count: 1
                    }
                ));
            }
            _ => panic!("expected upload task event"),
        }

        match event_rx.recv_timeout(Duration::from_secs(1)).unwrap() {
            TerminalEvent::UploadTask(event) => match event.kind {
                UploadTaskEventKind::Failed { error } => {
                    assert!(error.contains("无法自动确定远端当前目录"));
                }
                _ => panic!("expected failed upload task event"),
            },
            _ => panic!("expected upload task event"),
        }
    }

    #[test]
    fn builds_remote_child_paths() {
        assert_eq!(remote_child_path("/", "app.tar.gz"), "/app.tar.gz");
        assert_eq!(
            remote_child_path("/srv/app/", "app.tar.gz"),
            "/srv/app/app.tar.gz"
        );
    }
}
