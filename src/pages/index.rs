mod actions;
mod chrome;
mod connection;
mod controls;
mod hosts;
mod render;
mod windows;

use diesel::sqlite::SqliteConnection;
use gpui::{Context, Entity, FocusHandle, Subscription, Window, WindowHandle, prelude::*};
use gpui_component::{Root, input::InputState};

use crate::{
    ipc::{ActiveTab, ServerResource, load_servers, open_database},
    ui::{Language, TextKey, ThemeMode},
};

pub(crate) struct XsshDemo {
    connection: SqliteConnection,
    servers: Vec<ServerResource>,
    open_tabs: Vec<ServerResource>,
    active_tab: ActiveTab,
    language: Language,
    theme: ThemeMode,
    search_input: Entity<InputState>,
    create_host_window: Option<WindowHandle<Root>>,
    edit_host_window: Option<WindowHandle<Root>>,
    settings_window: Option<WindowHandle<Root>>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl XsshDemo {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (_database_path, mut connection) = open_database().expect("SQLite 初始化失败");
        let servers = load_servers(&mut connection).expect("服务器资源读取失败");
        let language = Language::Zh;
        let theme = ThemeMode::Dark;
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(language.tr(TextKey::SearchHosts)));
        let _subscriptions = Vec::new();

        Self {
            connection,
            servers,
            open_tabs: Vec::new(),
            active_tab: ActiveTab::Vault,
            language,
            theme,
            search_input,
            create_host_window: None,
            edit_host_window: None,
            settings_window: None,
            focus_handle: cx.focus_handle(),
            _subscriptions,
        }
    }
}
