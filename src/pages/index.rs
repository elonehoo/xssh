mod actions;
mod chrome;
mod connection;
mod controls;
mod hosts;
mod render;
mod tabs;
mod terminal;
mod windows;

use std::{collections::HashMap, sync::mpsc::Receiver};

use diesel::sqlite::SqliteConnection;
use gpui::{Context, Entity, FocusHandle, Subscription, Window, WindowHandle, prelude::*};
use gpui_component::{
    Root, Theme as ComponentTheme, ThemeMode as ComponentThemeMode, input::InputState,
};

use crate::{
    ipc::{
        AppSettingsData, ServerResource, SshConnectionTestResult, load_app_settings, load_servers,
        open_database, save_app_settings,
    },
    ui::{Language, TerminalThemeId, TerminalThemeKind, TextKey, ThemeMode},
};

use tabs::{ActiveTab, OpenTab, TerminalId};
use terminal::TerminalSession;

pub(in crate::pages::index) struct HostConnectionTestTarget {
    pub(in crate::pages::index) server_id: i32,
    pub(in crate::pages::index) server_name: String,
}

pub(crate) struct Xssh {
    connection: SqliteConnection,
    servers: Vec<ServerResource>,
    open_tabs: Vec<OpenTab>,
    terminal_sessions: HashMap<TerminalId, TerminalSession>,
    active_tab: ActiveTab,
    host_connection_test_receivers:
        HashMap<i32, Receiver<SshConnectionTestResult<HostConnectionTestTarget>>>,
    language: Language,
    theme: ThemeMode,
    dark_terminal_theme: TerminalThemeId,
    light_terminal_theme: TerminalThemeId,
    search_input: Entity<InputState>,
    create_host_window: Option<WindowHandle<Root>>,
    edit_host_window: Option<WindowHandle<Root>>,
    settings_window: Option<WindowHandle<Root>>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl Xssh {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (_database_path, mut connection) = open_database().expect("SQLite 初始化失败");
        let servers = load_servers(&mut connection).expect("服务器资源读取失败");
        let app_settings = load_app_settings(&mut connection).expect("应用设置读取失败");
        let language = Language::from_setting_value(&app_settings.language);
        let theme = ThemeMode::from_setting_value(&app_settings.theme);
        ComponentTheme::change(Self::component_theme_mode(theme), Some(window), cx);
        let dark_terminal_theme = TerminalThemeId::from_setting_value(
            &app_settings.dark_terminal_theme,
            TerminalThemeKind::Dark,
        );
        let light_terminal_theme = TerminalThemeId::from_setting_value(
            &app_settings.light_terminal_theme,
            TerminalThemeKind::Light,
        );
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(language.tr(TextKey::SearchHosts)));
        let _subscriptions = Vec::new();

        let mut this = Self {
            connection,
            servers,
            open_tabs: Vec::new(),
            terminal_sessions: HashMap::new(),
            active_tab: ActiveTab::Vault,
            host_connection_test_receivers: HashMap::new(),
            language,
            theme,
            dark_terminal_theme,
            light_terminal_theme,
            search_input,
            create_host_window: None,
            edit_host_window: None,
            settings_window: None,
            focus_handle: cx.focus_handle(),
            _subscriptions,
        };

        if this.app_settings_data() != app_settings {
            this.persist_app_settings();
        }

        this
    }

    pub(in crate::pages::index) fn active_terminal_theme(&self) -> TerminalThemeId {
        match self.theme {
            ThemeMode::Dark => self.dark_terminal_theme,
            ThemeMode::Light => self.light_terminal_theme,
        }
    }

    pub(in crate::pages::index) fn component_theme_mode(theme: ThemeMode) -> ComponentThemeMode {
        match theme {
            ThemeMode::Dark => ComponentThemeMode::Dark,
            ThemeMode::Light => ComponentThemeMode::Light,
        }
    }

    fn app_settings_data(&self) -> AppSettingsData {
        AppSettingsData {
            language: self.language.setting_value().to_string(),
            theme: self.theme.setting_value().to_string(),
            dark_terminal_theme: self.dark_terminal_theme.as_str().to_string(),
            light_terminal_theme: self.light_terminal_theme.as_str().to_string(),
        }
    }

    fn persist_app_settings(&mut self) {
        let app_settings = self.app_settings_data();

        if let Err(error) = save_app_settings(&mut self.connection, &app_settings) {
            eprintln!("保存应用设置失败: {error}");
        }
    }
}
