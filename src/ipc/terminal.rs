use std::{
    env,
    io::{ErrorKind, Read, Write},
    net::TcpStream,
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
    time::Duration,
};

use anyhow::{Context as AnyhowContext, Result, anyhow};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use ssh2::Session;

use super::{AuthenticationMode, ServerResource};

pub(crate) enum TerminalCommand {
    Input(Vec<u8>),
    Resize { cols: u16, rows: u16 },
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

pub(crate) fn open_ssh_terminal(server: ServerResource, cols: u16, rows: u16) -> TerminalHandle {
    let (input, input_rx) = mpsc::channel();
    let (event_tx, events) = mpsc::channel();

    thread::spawn(move || {
        if let Err(error) = run_ssh_terminal(server, cols, rows, input_rx, event_tx.clone()) {
            let _ = event_tx.send(TerminalEvent::Error(error.to_string()));
        }
    });

    TerminalHandle { input, events }
}

pub(crate) fn open_local_terminal(cols: u16, rows: u16) -> TerminalHandle {
    let (input, input_rx) = mpsc::channel();
    let (event_tx, events) = mpsc::channel();

    thread::spawn(move || {
        if let Err(error) = run_local_terminal(cols, rows, input_rx, event_tx.clone()) {
            let _ = event_tx.send(TerminalEvent::Error(error.to_string()));
        }
    });

    TerminalHandle { input, events }
}

fn run_local_terminal(
    cols: u16,
    rows: u16,
    input_rx: Receiver<TerminalCommand>,
    event_tx: Sender<TerminalEvent>,
) -> Result<()> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(pty_size(cols, rows))
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
    cols: u16,
    rows: u16,
    input_rx: Receiver<TerminalCommand>,
    event_tx: Sender<TerminalEvent>,
) -> Result<()> {
    validate_password_terminal(&server)?;

    let port = u16::try_from(server.port).context("SSH 端口不是有效端口")?;
    let tcp = TcpStream::connect((server.host.as_str(), port))
        .with_context(|| format!("连接 SSH 服务器失败: {}:{}", server.host, server.port))?;
    tcp.set_read_timeout(Some(Duration::from_secs(30))).ok();
    tcp.set_write_timeout(Some(Duration::from_secs(30))).ok();

    let mut session = Session::new().context("创建 SSH session 失败")?;
    session.set_tcp_stream(tcp);
    session.handshake().context("SSH 握手失败")?;
    session
        .userauth_password(&server.username, &server.password)
        .with_context(|| format!("SSH 密码认证失败: {}", server.username))?;

    if !session.authenticated() {
        return Err(anyhow!("SSH 认证未通过"));
    }

    let mut channel = session.channel_session().context("创建 SSH channel 失败")?;
    channel
        .request_pty(
            "xterm-256color",
            None,
            Some((cols.into(), rows.into(), 0, 0)),
        )
        .context("请求 SSH PTY 失败")?;
    channel.shell().context("启动 SSH shell 失败")?;
    session.set_blocking(false);

    let _ = event_tx.send(TerminalEvent::Connected);
    let mut pending_input = Vec::new();
    let mut buffer = [0_u8; 8192];

    loop {
        match drain_terminal_commands(&mut channel, &input_rx, &mut pending_input) {
            Ok(true) => return Ok(()),
            Ok(false) => {}
            Err(error) => {
                let _ = event_tx.send(TerminalEvent::Error(error.to_string()));
                return Err(error);
            }
        }

        if !pending_input.is_empty() {
            write_pending_input(&mut channel, &mut pending_input)?;
        }

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

fn drain_terminal_commands(
    channel: &mut ssh2::Channel,
    input_rx: &Receiver<TerminalCommand>,
    pending_input: &mut Vec<u8>,
) -> Result<bool> {
    loop {
        match input_rx.try_recv() {
            Ok(TerminalCommand::Input(bytes)) => pending_input.extend(bytes),
            Ok(TerminalCommand::Resize { cols, rows }) => resize_terminal(channel, cols, rows)?,
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
            Ok(TerminalCommand::Resize { cols, rows }) => {
                master.resize(pty_size(cols, rows))?;
            }
            Ok(TerminalCommand::Close) => return Ok(true),
            Err(TryRecvError::Empty) => return Ok(false),
            Err(TryRecvError::Disconnected) => return Ok(true),
        }
    }
}

fn resize_terminal(channel: &mut ssh2::Channel, cols: u16, rows: u16) -> Result<()> {
    match channel.request_pty_size(cols.into(), rows.into(), None, None) {
        Ok(()) => Ok(()),
        Err(error) => {
            let message = error.to_string();
            let io_error: std::io::Error = error.into();
            if io_error.kind() == ErrorKind::WouldBlock {
                return Ok(());
            }

            Err(anyhow!(message))
        }
    }
}

fn pty_size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
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
    if AuthenticationMode::from_label(&server.authentication) != AuthenticationMode::ManualPassword
    {
        return Err(anyhow!("Direct key 还没有配置 key 字段，暂时不能连接"));
    }

    if server.password.is_empty() {
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
}
