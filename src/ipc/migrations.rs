use anyhow::{Context as AnyhowContext, Result, anyhow};
use diesel::{prelude::*, sqlite::SqliteConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub(crate) fn migrate_database(connection: &mut SqliteConnection) -> Result<()> {
    connection
        .run_pending_migrations(MIGRATIONS)
        .map(|_| ())
        .map_err(|error| anyhow!("执行数据库 migration 失败: {error}"))
}

#[derive(QueryableByName)]
struct RowCount {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

pub(crate) fn applied_migration_count(connection: &mut SqliteConnection) -> Result<i64> {
    diesel::sql_query("SELECT COUNT(*) AS count FROM __diesel_schema_migrations")
        .get_result::<RowCount>(connection)
        .map(|row| row.count)
        .context("读取 migration 记录失败")
}
