use std::path::PathBuf;

use anyhow::{Context as AnyhowContext, Result, anyhow};
use diesel::{Connection, sqlite::SqliteConnection};

use super::migrations::migrate_database;

fn database_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("XSSH_DATABASE_PATH") {
        return Ok(PathBuf::from(path));
    }

    let home = std::env::var_os("HOME").ok_or_else(|| anyhow!("读取 HOME 目录失败"))?;
    let directory = PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("XSSH");
    std::fs::create_dir_all(&directory)
        .with_context(|| format!("创建 SQLite 数据目录失败: {}", directory.display()))?;

    Ok(directory.join("xssh.sqlite3"))
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
