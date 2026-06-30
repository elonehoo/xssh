use std::{
    env,
    io::{ErrorKind, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
    time::Duration,
};

use anyhow::{Context as AnyhowContext, Result, anyhow};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use ssh2::Session;

use super::{AuthenticationMode, ServerConnectionDraft, ServerResource, SshConnectionTestResult};

const SSH_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

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
    Close,
}

pub(crate) enum TerminalEvent {
    Connected,
    Output(Vec<u8>),
    Disconnected,
    Error(String),
}

pub(crate) struct TerminalHandle {
    pub(crate) input: Sender<TerminalCommand>,
    pub(crate) events: Receiver<TerminalEvent>,
}

#[derive(Default)]
struct PendingSshTerminalCommands {
    input: Vec<u8>,
    resize: Option<TerminalSize>,
}

impl PendingSshTerminalCommands {
    fn push_input(&mut self, bytes: Vec<u8>) {
        self.input.extend(bytes);
    }

    fn set_resize(&mut self, size: TerminalSize) {
        self.resize = Some(size);
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
    let mut pending_commands = PendingSshTerminalCommands::default();
    let mut buffer = [0_u8; 8192];

    loop {
        match drain_ssh_terminal_commands(&mut channel, &input_rx, &mut pending_commands) {
            Ok(true) => return Ok(()),
            Ok(false) => {}
            Err(error) => {
                let _ = event_tx.send(TerminalEvent::Error(error.to_string()));
                return Err(error);
            }
        }

        pending_commands.flush_resize(&mut channel)?;
        pending_commands.flush_input(&mut channel)?;

        loop {
            match channel.read(&mut buffer) {
                Ok(0) => break,
                Ok(size) => {
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
) -> Result<bool> {
    loop {
        match input_rx.try_recv() {
            Ok(TerminalCommand::Input(bytes)) => pending_commands.push_input(bytes),
            Ok(TerminalCommand::Resize(size)) => pending_commands.set_resize(size),
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
            Ok(TerminalCommand::Close) => return Ok(true),
            Err(TryRecvError::Empty) => return Ok(false),
            Err(TryRecvError::Disconnected) => return Ok(true),
        }
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
}
