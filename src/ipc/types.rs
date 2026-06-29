use diesel::prelude::*;

use crate::schema::servers;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ActiveTab {
    Vault,
    Server(i32),
}
