use crate::ipc::ServerResource;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ActiveTab {
    Vault,
    LocalTerminal,
    Server(i32),
}

#[derive(Clone, Debug)]
pub(super) enum OpenTab {
    LocalTerminal,
    Server(ServerResource),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum TerminalId {
    Local,
    Server(i32),
}

impl OpenTab {
    pub(super) fn active_tab(&self) -> ActiveTab {
        match self {
            Self::LocalTerminal => ActiveTab::LocalTerminal,
            Self::Server(server) => ActiveTab::Server(server.id),
        }
    }

    pub(super) fn server_id(&self) -> Option<i32> {
        match self {
            Self::LocalTerminal => None,
            Self::Server(server) => Some(server.id),
        }
    }
}

impl TerminalId {
    pub(super) fn element_suffix(self) -> String {
        match self {
            Self::Local => "local".to_string(),
            Self::Server(server_id) => format!("server-{server_id}"),
        }
    }
}
