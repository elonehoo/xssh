use anyhow::{Context as AnyhowContext, Result, anyhow};
use diesel::{prelude::*, sqlite::SqliteConnection};

use crate::schema::servers;

use super::types::{NewServer, ServerDraft, ServerResource};

pub(crate) fn load_servers(connection: &mut SqliteConnection) -> Result<Vec<ServerResource>> {
    servers::table
        .select(ServerResource::as_select())
        .order((servers::created_at.desc(), servers::id.desc()))
        .load(connection)
        .context("读取服务器资源失败")
}

pub(crate) fn insert_server(
    connection: &mut SqliteConnection,
    draft: &ServerDraft,
) -> Result<ServerResource> {
    let new_server = NewServer {
        name: &draft.name,
        host: &draft.host,
        port: i32::from(draft.port),
        username: &draft.username,
        authentication: &draft.authentication,
        password: &draft.password,
    };

    diesel::insert_into(servers::table)
        .values(&new_server)
        .returning(ServerResource::as_returning())
        .get_result(connection)
        .context("保存服务器资源失败")
}

pub(crate) fn update_server(
    connection: &mut SqliteConnection,
    server_id: i32,
    draft: &ServerDraft,
) -> Result<ServerResource> {
    diesel::update(servers::table.filter(servers::id.eq(server_id)))
        .set((
            servers::name.eq(&draft.name),
            servers::host.eq(&draft.host),
            servers::port.eq(i32::from(draft.port)),
            servers::username.eq(&draft.username),
            servers::authentication.eq(&draft.authentication),
            servers::password.eq(&draft.password),
            servers::updated_at.eq(diesel::dsl::sql::<diesel::sql_types::Text>(
                "CURRENT_TIMESTAMP",
            )),
        ))
        .returning(ServerResource::as_returning())
        .get_result(connection)
        .with_context(|| format!("更新服务器资源失败: id={server_id}"))
}

pub(crate) fn delete_server(connection: &mut SqliteConnection, server_id: i32) -> Result<()> {
    let deleted = diesel::delete(servers::table.filter(servers::id.eq(server_id)))
        .execute(connection)
        .with_context(|| format!("删除服务器资源失败: id={server_id}"))?;

    if deleted == 0 {
        return Err(anyhow!("服务器资源不存在: id={server_id}"));
    }

    Ok(())
}
