use diesel::prelude::*;

use crate::schema::servers;

use anyhow::{Context as AnyhowContext, Result, anyhow};

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = servers)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct ServerResource {
    pub(crate) id: i32,
    pub(crate) name: String,
    pub(crate) host: String,
    pub(crate) port: i32,
    pub(crate) username: String,
    pub(crate) authentication: String,
    pub(crate) password: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ServerDraft {
    pub(crate) name: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) username: String,
    pub(crate) authentication: String,
    pub(crate) password: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ServerConnectionDraft {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) username: String,
    pub(crate) authentication: String,
    pub(crate) password: String,
}

impl TryFrom<&ServerResource> for ServerConnectionDraft {
    type Error = anyhow::Error;

    fn try_from(server: &ServerResource) -> Result<Self> {
        let port = u16::try_from(server.port).context("SSH 端口不是有效端口")?;

        if port == 0 {
            return Err(anyhow!("SSH 端口不是有效端口"));
        }

        Ok(Self {
            host: server.host.clone(),
            port,
            username: server.username.clone(),
            authentication: server.authentication.clone(),
            password: server.password.clone(),
        })
    }
}

pub(crate) struct SshConnectionTestResult<T> {
    pub(crate) context: T,
    pub(crate) result: std::result::Result<(), String>,
}

#[derive(Insertable)]
#[diesel(table_name = servers)]
pub(super) struct NewServer<'a> {
    pub(super) name: &'a str,
    pub(super) host: &'a str,
    pub(super) port: i32,
    pub(super) username: &'a str,
    pub(super) authentication: &'a str,
    pub(super) password: &'a str,
}
