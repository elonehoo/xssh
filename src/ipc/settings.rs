use anyhow::{Context as AnyhowContext, Result};
use diesel::{dsl::sql, prelude::*, sql_types::Text, sqlite::SqliteConnection};

use crate::schema::app_settings;

use super::types::{AppSettingsData, AppSettingsRow};

const DEFAULT_APP_SETTINGS_ID: &str = "default";

pub(crate) fn load_app_settings(connection: &mut SqliteConnection) -> Result<AppSettingsData> {
    let row = app_settings::table
        .filter(app_settings::id.eq(DEFAULT_APP_SETTINGS_ID))
        .select(AppSettingsRow::as_select())
        .first::<AppSettingsRow>(connection)
        .optional()
        .context("读取应用设置失败")?;

    let Some(row) = row else {
        let settings = AppSettingsData::default();
        save_app_settings(connection, &settings)?;
        return Ok(settings);
    };

    serde_json::from_str(&row.settings_data).context("解析应用设置失败")
}

pub(crate) fn save_app_settings(
    connection: &mut SqliteConnection,
    settings: &AppSettingsData,
) -> Result<()> {
    let settings_data = serde_json::to_string(settings).context("序列化应用设置失败")?;

    diesel::insert_into(app_settings::table)
        .values((
            app_settings::id.eq(DEFAULT_APP_SETTINGS_ID),
            app_settings::settings_data.eq(&settings_data),
            app_settings::created_at.eq(sql::<Text>("CURRENT_TIMESTAMP")),
            app_settings::updated_at.eq(sql::<Text>("CURRENT_TIMESTAMP")),
        ))
        .on_conflict(app_settings::id)
        .do_update()
        .set((
            app_settings::settings_data.eq(&settings_data),
            app_settings::updated_at.eq(sql::<Text>("CURRENT_TIMESTAMP")),
        ))
        .execute(connection)
        .context("保存应用设置失败")?;

    Ok(())
}
