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
    time::Duration,
};

use anyhow::{Context as AnyhowContext, Result, anyhow};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use ssh2::{OpenFlags, OpenType, Session};

use super::{AuthenticationMode, ServerConnectionDraft, ServerResource, SshConnectionTestResult};

const SSH_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
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
    UploadFiles {
        paths: Vec<PathBuf>,
        remote_directory_hint: Option<String>,
    },
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
    remote_directory_hint: Option<String>,
}

impl PendingSshTerminalCommands {
    fn push_input(&mut self, bytes: Vec<u8>, input_tracker: &mut TerminalInputTracker) {
        self.submitted_lines.extend(input_tracker.push(&bytes));
        self.input.extend(bytes);
    }

    fn set_resize(&mut self, size: TerminalSize) {
        self.resize = Some(size);
    }

    fn push_upload_files(&mut self, paths: Vec<PathBuf>, remote_directory_hint: Option<String>) {
        self.upload_files.push(PendingUploadFiles {
            paths,
            remote_directory_hint,
        });
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

#[derive(Debug, PartialEq, Eq)]
enum RemoteDirectoryEffect {
    Unchanged,
    Change(RemoteDirectoryTarget),
    Unknown,
}

#[derive(Debug, PartialEq, Eq)]
enum RemoteDirectoryTarget {
    Home,
    Previous,
    Path(String),
}

#[derive(Debug)]
struct RemoteDirectoryTracker {
    current: Option<String>,
    previous: Option<String>,
}

#[derive(Default)]
struct RemotePwdOutputTracker {
    pending: bool,
    buffer: String,
}

#[derive(Default)]
struct UploadSummary {
    succeeded: usize,
    failed: usize,
}

impl RemoteDirectoryTracker {
    fn new(current: Option<String>) -> Self {
        Self {
            current,
            previous: None,
        }
    }

    fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    fn mark_unknown(&mut self) {
        self.current = None;
    }

    fn apply_resolved(&mut self, directory: String) {
        if self.current.as_deref() == Some(directory.as_str()) {
            return;
        }

        self.previous = self.current.replace(directory);
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
    let initial_remote_directory = remote_pwd(&session).ok();

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
    let mut remote_directory = RemoteDirectoryTracker::new(initial_remote_directory);
    let mut pwd_output_tracker = RemotePwdOutputTracker::default();
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
        pending_commands.flush_input(&mut channel)?;
        let submitted_lines = pending_commands.take_submitted_lines();
        pwd_output_tracker.observe_submitted_lines(&submitted_lines);
        apply_submitted_lines_to_remote_directory(&server, &mut remote_directory, submitted_lines);
        for upload in pending_commands.take_upload_files() {
            spawn_sftp_uploads(
                &server,
                &remote_directory,
                upload.paths,
                upload.remote_directory_hint,
                &event_tx,
            );
        }

        loop {
            match channel.read(&mut buffer) {
                Ok(0) => break,
                Ok(size) => {
                    if let Some(directory) = pwd_output_tracker.push_output(&buffer[..size]) {
                        remote_directory.apply_resolved(directory);
                    }
                    let _ = event_tx.send(TerminalEvent::Output(buffer[..size].to_vec()));
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
            Ok(TerminalCommand::UploadFiles {
                paths,
                remote_directory_hint,
            }) => pending_commands.push_upload_files(paths, remote_directory_hint),
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

fn apply_submitted_lines_to_remote_directory(
    server: &ServerResource,
    remote_directory: &mut RemoteDirectoryTracker,
    submitted_lines: Vec<String>,
) {
    for line in submitted_lines {
        match remote_directory_effect(&line) {
            RemoteDirectoryEffect::Unchanged => {}
            RemoteDirectoryEffect::Unknown => remote_directory.mark_unknown(),
            RemoteDirectoryEffect::Change(target) => {
                match resolve_remote_directory_target(server, remote_directory, target) {
                    Ok(directory) => remote_directory.apply_resolved(directory),
                    Err(_) => remote_directory.mark_unknown(),
                }
            }
        }
    }
}

fn spawn_sftp_uploads(
    server: &ServerResource,
    remote_directory: &RemoteDirectoryTracker,
    local_paths: Vec<PathBuf>,
    remote_directory_hint: Option<String>,
    event_tx: &Sender<TerminalEvent>,
) {
    if local_paths.is_empty() {
        return;
    }

    let task_id = next_upload_task_id();
    let Some(remote_directory) = remote_directory
        .current()
        .map(ToString::to_string)
        .or_else(|| remote_directory_hint.filter(|directory| is_absolute_remote_path(directory)))
    else {
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
                error: "无法确定远端当前目录，请先执行 pwd 或使用普通 cd 命令进入目标目录后再拖拽上传。"
                    .to_string(),
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

fn resolve_remote_directory_target(
    server: &ServerResource,
    remote_directory: &RemoteDirectoryTracker,
    target: RemoteDirectoryTarget,
) -> Result<String> {
    validate_password_terminal(server)?;

    let port = u16::try_from(server.port).context("SSH 端口不是有效端口")?;
    let session =
        authenticated_ssh_session(&server.host, port, &server.username, &server.password, None)?;
    let command = remote_directory_command(remote_directory, target)?;

    remote_pwd_command(&session, &command)
}

fn remote_pwd(session: &Session) -> Result<String> {
    remote_pwd_command(session, "pwd -P")
}

fn remote_pwd_command(session: &Session, command: &str) -> Result<String> {
    let mut channel = session.channel_session().context("创建 SSH channel 失败")?;
    channel.exec(command).context("执行远端目录命令失败")?;

    let mut output = String::new();
    channel
        .read_to_string(&mut output)
        .context("读取远端目录失败")?;
    let mut stderr = String::new();
    let _ = channel.stderr().read_to_string(&mut stderr);
    channel.wait_close().context("关闭远端目录命令失败")?;

    if channel.exit_status().unwrap_or(1) != 0 {
        let message = stderr.trim();
        return Err(anyhow!(
            "{}",
            if message.is_empty() {
                "远端目录命令失败"
            } else {
                message
            }
        ));
    }

    output
        .lines()
        .last()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| anyhow!("远端目录为空"))
}

fn remote_directory_command(
    remote_directory: &RemoteDirectoryTracker,
    target: RemoteDirectoryTarget,
) -> Result<String> {
    match target {
        RemoteDirectoryTarget::Home => Ok("cd && pwd -P".to_string()),
        RemoteDirectoryTarget::Previous => {
            let previous = remote_directory
                .previous
                .as_deref()
                .ok_or_else(|| anyhow!("没有可用的上一个远端目录"))?;

            Ok(format!("cd -- {} && pwd -P", shell_quote(previous)))
        }
        RemoteDirectoryTarget::Path(path) if path == "~" => Ok("cd && pwd -P".to_string()),
        RemoteDirectoryTarget::Path(path) if path.starts_with("~/") => {
            let relative_home_path = path.trim_start_matches("~/");

            Ok(format!(
                "cd && cd -- {} && pwd -P",
                shell_quote(relative_home_path)
            ))
        }
        RemoteDirectoryTarget::Path(path) if path.starts_with('/') => {
            Ok(format!("cd -- {} && pwd -P", shell_quote(&path)))
        }
        RemoteDirectoryTarget::Path(path) => {
            let current = remote_directory
                .current()
                .ok_or_else(|| anyhow!("无法确定当前远端目录"))?;

            Ok(format!(
                "cd -- {} && cd -- {} && pwd -P",
                shell_quote(current),
                shell_quote(&path)
            ))
        }
    }
}

fn remote_directory_effect(line: &str) -> RemoteDirectoryEffect {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return RemoteDirectoryEffect::Unchanged;
    }

    let Some(command) = first_shell_word(trimmed) else {
        return RemoteDirectoryEffect::Unknown;
    };

    match command.as_str() {
        "cd" => cd_target_from_command(trimmed),
        "pushd" | "popd" => RemoteDirectoryEffect::Unknown,
        _ => RemoteDirectoryEffect::Unchanged,
    }
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

fn cd_target_from_command(line: &str) -> RemoteDirectoryEffect {
    let Some(words) = split_simple_shell_words(line) else {
        return RemoteDirectoryEffect::Unknown;
    };

    if words.first().map(String::as_str) != Some("cd") {
        return RemoteDirectoryEffect::Unchanged;
    }

    let args = cd_args(&words[1..]);
    match args.as_slice() {
        [] => RemoteDirectoryEffect::Change(RemoteDirectoryTarget::Home),
        [target] if target == "-" => RemoteDirectoryEffect::Change(RemoteDirectoryTarget::Previous),
        [target] if path_uses_shell_expansion(target) => RemoteDirectoryEffect::Unknown,
        [target] => RemoteDirectoryEffect::Change(RemoteDirectoryTarget::Path(target.clone())),
        _ => RemoteDirectoryEffect::Unknown,
    }
}

fn cd_args(args: &[String]) -> Vec<String> {
    if args.first().map(String::as_str) == Some("--") {
        args[1..].to_vec()
    } else {
        args.to_vec()
    }
}

fn first_shell_word(line: &str) -> Option<String> {
    let mut word = String::new();
    let chars = line.trim_start().chars();

    for char in chars {
        match char {
            char if char.is_whitespace() => break,
            '\'' | '"' | '\\' | ';' | '&' | '|' | '<' | '>' | '(' | ')' => return None,
            _ => word.push(char),
        }
    }

    if word.is_empty() { None } else { Some(word) }
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

fn path_uses_shell_expansion(path: &str) -> bool {
    path.contains('$')
        || path.contains('`')
        || path.contains('*')
        || path.contains('?')
        || (path.starts_with('~') && path != "~" && !path.starts_with("~/"))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
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
        assert_eq!(tracker.push(b"\r"), vec!["cd /var/www".to_string()]);
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
    fn parses_simple_cd_commands() {
        assert_eq!(
            remote_directory_effect("cd /srv/app"),
            RemoteDirectoryEffect::Change(RemoteDirectoryTarget::Path("/srv/app".to_string()))
        );
        assert_eq!(
            remote_directory_effect("cd \"release build\""),
            RemoteDirectoryEffect::Change(RemoteDirectoryTarget::Path("release build".to_string()))
        );
        assert_eq!(
            remote_directory_effect("cd"),
            RemoteDirectoryEffect::Change(RemoteDirectoryTarget::Home)
        );
        assert_eq!(
            remote_directory_effect("cd -"),
            RemoteDirectoryEffect::Change(RemoteDirectoryTarget::Previous)
        );
    }

    #[test]
    fn marks_complex_directory_changes_unknown() {
        assert_eq!(
            remote_directory_effect("cd $PROJECT"),
            RemoteDirectoryEffect::Unknown
        );
        assert_eq!(
            remote_directory_effect("cd ~deploy"),
            RemoteDirectoryEffect::Unknown
        );
        assert_eq!(
            remote_directory_effect("cd /tmp && cd other"),
            RemoteDirectoryEffect::Unknown
        );
        assert_eq!(
            remote_directory_effect("pushd /tmp"),
            RemoteDirectoryEffect::Unknown
        );
    }

    #[test]
    fn leaves_unrelated_commands_unchanged() {
        assert_eq!(
            remote_directory_effect("echo hello && pwd"),
            RemoteDirectoryEffect::Unchanged
        );
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
        let remote_directory = RemoteDirectoryTracker::new(None);
        let (event_tx, event_rx) = mpsc::channel();

        spawn_sftp_uploads(
            &server,
            &remote_directory,
            vec![PathBuf::from("/tmp/example.txt")],
            None,
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
                    assert!(error.contains("无法确定远端当前目录"));
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

    #[test]
    fn quotes_remote_shell_paths() {
        assert_eq!(shell_quote("/tmp/a b"), "'/tmp/a b'");
        assert_eq!(shell_quote("/tmp/it's"), "'/tmp/it'\\''s'");
    }
}
