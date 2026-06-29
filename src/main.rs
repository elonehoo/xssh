mod ipc;
mod pages;
mod schema;
mod ui;

use gpui::{
    App, AppContext, Application, Bounds, Focusable, WindowBounds, WindowOptions, point, px, size,
};
use gpui_component::Root;

use crate::{
    ipc::{applied_migration_count, open_database},
    pages::XsshDemo,
    ui::AppAssets,
};

fn main() {
    if std::env::args().any(|arg| arg == "--migrate-only") {
        let (database_path, mut connection) = open_database().expect("SQLite 初始化失败");
        let migration_count =
            applied_migration_count(&mut connection).expect("读取 migration 记录失败");
        println!(
            "SQLite ready: {}\nApplied migrations: {}",
            database_path.display(),
            migration_count
        );
        return;
    }

    Application::new()
        .with_assets(AppAssets)
        .run(|cx: &mut App| {
            gpui_component::init(cx);

            let bounds = Bounds::centered(None, size(px(980.), px(640.)), cx);
            let window = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        window_min_size: Some(size(px(760.), px(500.))),
                        titlebar: Some(gpui::TitlebarOptions {
                            title: Some("XSSH Demo".into()),
                            appears_transparent: true,
                            traffic_light_position: Some(point(px(12.), px(11.))),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    |window, cx| {
                        let view = cx.new(|cx| XsshDemo::new(window, cx));
                        cx.new(|cx| Root::new(view, window, cx))
                    },
                )
                .unwrap();

            window
                .update(cx, |root, window, cx| {
                    if let Ok(view) = root.view().clone().downcast::<XsshDemo>() {
                        window.focus(&view.read(cx).focus_handle(cx));
                    }
                    cx.activate(true);
                })
                .ok();
        });
}

#[cfg(test)]
mod tests {
    use diesel::Connection;
    use diesel::sqlite::SqliteConnection;
    use gpui::AssetSource;

    use crate::{
        ipc::{
            AuthenticationMode, ServerDraft, applied_migration_count, delete_server, insert_server,
            load_servers, migrate_database, update_server,
        },
        ui::{AppAssets, icons},
    };

    #[test]
    fn creates_and_reads_server_resources() {
        let mut connection = SqliteConnection::establish(":memory:").unwrap();
        migrate_database(&mut connection).unwrap();

        let migration_count = applied_migration_count(&mut connection).unwrap();
        assert!(migration_count >= 1);

        let server = insert_server(
            &mut connection,
            &ServerDraft {
                name: "Demo".to_string(),
                host: "127.0.0.1".to_string(),
                port: 22,
                username: "root".to_string(),
                authentication: AuthenticationMode::ManualPassword
                    .storage_label()
                    .to_string(),
                password: "secret".to_string(),
            },
        )
        .unwrap();

        assert_eq!(server.name, "Demo");
        assert_eq!(server.host, "127.0.0.1");
        assert_eq!(server.port, 22);
        assert_eq!(server.username, "root");
        assert_eq!(
            server.authentication,
            AuthenticationMode::ManualPassword.storage_label()
        );
        assert_eq!(server.password, "secret");

        let servers = load_servers(&mut connection).unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].id, server.id);

        let updated = update_server(
            &mut connection,
            server.id,
            &ServerDraft {
                name: "Production".to_string(),
                host: "10.0.0.1".to_string(),
                port: 2222,
                username: "admin".to_string(),
                authentication: AuthenticationMode::DirectKey.storage_label().to_string(),
                password: String::new(),
            },
        )
        .unwrap();

        assert_eq!(updated.id, server.id);
        assert_eq!(updated.name, "Production");
        assert_eq!(updated.host, "10.0.0.1");
        assert_eq!(updated.port, 2222);
        assert_eq!(updated.username, "admin");
        assert_eq!(
            updated.authentication,
            AuthenticationMode::DirectKey.storage_label()
        );
        assert_eq!(updated.password, "");

        delete_server(&mut connection, server.id).unwrap();
        let servers = load_servers(&mut connection).unwrap();
        assert!(servers.is_empty());
    }

    #[test]
    fn loads_app_svg_assets() {
        let assets = AppAssets;

        for path in [
            icons::vault::PATH,
            icons::sort_newest::PATH,
            icons::server::PATH,
            icons::connect::PATH,
            icons::edit::PATH,
            icons::delete::PATH,
            icons::add::PATH,
            icons::settings::PATH,
            icons::password_eye::PATH,
            icons::password_eye_off::PATH,
            icons::GPUI_EYE_ICON_PATH,
            icons::GPUI_CHEVRON_DOWN_ICON_PATH,
        ] {
            let bytes = assets.load(path).unwrap().unwrap();
            let svg = std::str::from_utf8(bytes.as_ref()).unwrap();
            assert!(svg.contains("<svg"));
        }

        assert!(assets.load("missing.svg").unwrap().is_none());
    }
}
