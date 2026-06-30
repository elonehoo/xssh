mod authentication;
mod database;
mod migrations;
mod servers;
mod settings;
mod terminal;
mod types;

pub(crate) use authentication::AuthenticationMode;
pub(crate) use database::open_database;
pub(crate) use migrations::applied_migration_count;
#[cfg(test)]
pub(crate) use migrations::migrate_database;
pub(crate) use servers::{delete_server, insert_server, load_servers, update_server};
pub(crate) use settings::{load_app_settings, save_app_settings};
pub(crate) use terminal::{
    TerminalCommand, TerminalEvent, TerminalSize, UploadTaskEvent, UploadTaskEventKind,
    open_local_terminal, open_ssh_terminal, spawn_ssh_connection_test,
};
pub(crate) use types::{
    AppSettingsData, ServerConnectionDraft, ServerDraft, ServerResource, SshConnectionTestResult,
};
