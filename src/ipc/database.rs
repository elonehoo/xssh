use std::path::PathBuf;

use anyhow::{Context as AnyhowContext, Result, anyhow};
use diesel::{Connection, sqlite::SqliteConnection};

use super::migrations::migrate_database;

fn database_path() -> Result<PathBuf> {
    Ok(std::env::current_dir()
        .context("读取当前目录失败")?
        .join("xssh.sqlite3"))
}

pub(crate) fn open_database() -> Result<(PathBuf, SqliteConnection)> {
    let path = database_path()?;
    let database_url = path
        .to_str()
        .ok_or_else(|| anyhow!("SQLite 数据库路径不是有效 UTF-8: {}", path.display()))?;
    let mut connection = SqliteConnection::establish(database_url)
        .with_context(|| format!("打开 SQLite 数据库失败: {}", path.display()))?;
    migrate_database(&mut connection)?;
    Ok((path, connection))
}
