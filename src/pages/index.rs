mod actions;
mod chrome;
mod connection;
mod controls;
mod hosts;
mod render;
mod sidebar;
mod tabs;
mod terminal;
mod uploads;
mod windows;

use std::{collections::HashMap, sync::mpsc::Receiver};

use diesel::sqlite::SqliteConnection;
use gpui::{
    Context, Entity, FocusHandle, KeystrokeEvent, Subscription, Window, WindowHandle, prelude::*,
};
use gpui_component::{Root, input::InputState};

use crate::{
    ipc::{
        AppSettingsData, ServerResource, SshConnectionTestResult, load_app_settings, load_servers,
        open_database, save_app_settings,
    },
    ui::{AppThemeId, Language, TextKey, sync_component_theme},
};

use tabs::{ActiveTab, OpenTab, TerminalId};
use terminal::TerminalSession;
use uploads::UploadTask;

pub(in crate::pages::index) struct HostConnectionTestTarget {
    pub(in crate::pages::index) server_id: i32,
    pub(in crate::pages::index) server_name: String,
}

pub(crate) struct Xssh {
    connection: SqliteConnection,
    servers: Vec<ServerResource>,
    open_tabs: Vec<OpenTab>,
    terminal_sessions: HashMap<TerminalId, TerminalSession>,
    upload_tasks: Vec<UploadTask>,
    upload_log_open: bool,
    active_tab: ActiveTab,
    host_connection_test_receivers:
        HashMap<i32, Receiver<SshConnectionTestResult<HostConnectionTestTarget>>>,
    language: Language,
    theme: AppThemeId,
    search_input: Entity<InputState>,
    sidebar_collapsed: bool,
    create_host_window: Option<WindowHandle<Root>>,
    edit_host_window: Option<WindowHandle<Root>>,
    settings_window: Option<WindowHandle<Root>>,
    focus_handle: FocusHandle,
    terminal_ime_buffer: String,
    terminal_ime_marked_range: Option<std::ops::Range<usize>>,
    _subscriptions: Vec<Subscription>,
}

impl Xssh {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (_database_path, mut connection) = open_database().expect("SQLite 初始化失败");
        let servers = load_servers(&mut connection).expect("服务器资源读取失败");
        let app_settings = load_app_settings(&mut connection).expect("应用设置读取失败");
        let language = Language::from_setting_value(&app_settings.language);
        let theme = AppThemeId::from_setting_value(&app_settings.app_theme);
        sync_component_theme(theme, Some(window), cx);
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(language.tr(TextKey::SearchHosts)));
        let view = cx.weak_entity();
        let _subscriptions =
            vec![
                cx.intercept_keystrokes(move |event: &KeystrokeEvent, window, cx| {
                    let Some(view) = view.upgrade() else {
                        return;
                    };

                    let _ = view.update(cx, |this, cx| {
                        this.intercept_terminal_tab(event, window, cx);
                    });
                }),
            ];

        let mut this = Self {
            connection,
            servers,
            open_tabs: Vec::new(),
            terminal_sessions: HashMap::new(),
            upload_tasks: Vec::new(),
            upload_log_open: false,
            active_tab: ActiveTab::Vault,
            host_connection_test_receivers: HashMap::new(),
            language,
            theme,
            search_input,
            sidebar_collapsed: false,
            create_host_window: None,
            edit_host_window: None,
            settings_window: None,
            focus_handle: cx.focus_handle(),
            terminal_ime_buffer: String::new(),
            terminal_ime_marked_range: None,
            _subscriptions,
        };

        if this.app_settings_data() != app_settings {
            this.persist_app_settings();
        }

        this
    }

    pub(in crate::pages::index) fn active_terminal_palette(
        &self,
    ) -> crate::ui::TerminalThemePalette {
        self.theme.terminal_palette()
    }

    fn app_settings_data(&self) -> AppSettingsData {
        AppSettingsData {
            language: self.language.setting_value().to_string(),
            app_theme: self.theme.as_str().to_string(),
        }
    }

    fn persist_app_settings(&mut self) {
        let app_settings = self.app_settings_data();

        if let Err(error) = save_app_settings(&mut self.connection, &app_settings) {
            eprintln!("保存应用设置失败: {error}");
        }
    }
}
