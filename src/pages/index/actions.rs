use std::{sync::mpsc::TryRecvError, time::Duration};

use anyhow::Result;
use gpui::{ClickEvent, Context, Window, WindowHandle};
use gpui_component::{Root, Theme as ComponentTheme, WindowExt, notification::NotificationType};

use crate::{
    ipc::{
        ServerConnectionDraft, ServerDraft, ServerResource, delete_server, insert_server,
        spawn_ssh_connection_test, update_server,
    },
    ui::{Language, TerminalThemeId, TextKey, ThemeMode, status_notification},
};

use super::{
    HostConnectionTestTarget, Xssh,
    tabs::{ActiveTab, OpenTab, TerminalId},
};

struct HostConnectionTestNotification;

impl Xssh {
    pub(in crate::pages::index) fn activate_singleton_window(
        handle: &mut Option<WindowHandle<Root>>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(window_handle) = *handle else {
            return false;
        };

        if window_handle
            .update(cx, |_, window, _| {
                window.activate_window();
            })
            .is_ok()
        {
            return true;
        }

        *handle = None;
        false
    }

    pub(in crate::pages) fn add_server_from_draft(
        &mut self,
        draft: ServerDraft,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let server = insert_server(&mut self.connection, &draft)?;
        self.servers.insert(0, server);
        cx.notify();
        Ok(())
    }

    pub(in crate::pages) fn update_server_from_draft(
        &mut self,
        server_id: i32,
        draft: ServerDraft,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let updated = update_server(&mut self.connection, server_id, &draft)?;

        if let Some(server) = self
            .servers
            .iter_mut()
            .find(|server| server.id == server_id)
        {
            *server = updated.clone();
        } else {
            self.servers.insert(0, updated.clone());
        }

        for tab in &mut self.open_tabs {
            if let OpenTab::Server(server) = tab
                && server.id == server_id
            {
                *server = updated.clone();
            }
        }

        cx.notify();
        Ok(())
    }

    pub(in crate::pages::index) fn delete_server_by_id(
        &mut self,
        server_id: i32,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        delete_server(&mut self.connection, server_id)?;

        self.host_connection_test_receivers.remove(&server_id);
        self.remove_terminal_session(TerminalId::Server(server_id));
        self.servers.retain(|server| server.id != server_id);
        self.open_tabs
            .retain(|tab| tab.server_id() != Some(server_id));

        if self.active_tab == ActiveTab::Server(server_id) {
            self.active_tab = self.next_available_tab();
        }

        cx.notify();
        Ok(())
    }

    pub(in crate::pages) fn set_language(
        &mut self,
        language: Language,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.language = language;
        self.search_input.update(cx, |input, cx| {
            input.set_placeholder(self.language.tr(TextKey::SearchHosts), window, cx);
        });
        self.persist_app_settings();
        cx.notify();
    }

    pub(in crate::pages) fn set_theme(
        &mut self,
        theme: ThemeMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.theme = theme;
        ComponentTheme::change(Self::component_theme_mode(theme), Some(window), cx);
        self.persist_app_settings();
        cx.notify();
    }

    pub(in crate::pages) fn set_dark_terminal_theme(
        &mut self,
        terminal_theme: TerminalThemeId,
        cx: &mut Context<Self>,
    ) {
        self.dark_terminal_theme = terminal_theme;
        self.persist_app_settings();
        cx.notify();
    }

    pub(in crate::pages) fn set_light_terminal_theme(
        &mut self,
        terminal_theme: TerminalThemeId,
        cx: &mut Context<Self>,
    ) {
        self.light_terminal_theme = terminal_theme;
        self.persist_app_settings();
        cx.notify();
    }

    pub(in crate::pages::index) fn on_vault_tab(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_tab = ActiveTab::Vault;
        cx.notify();
    }

    pub(in crate::pages::index) fn on_open_local_terminal(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_local_terminal_tab(window, cx);
    }

    pub(in crate::pages::index) fn connect_server(
        &mut self,
        server: ServerResource,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_server_tab(server.clone());
        self.ensure_terminal_session(TerminalId::Server(server.id));
        window.focus(&self.focus_handle, cx);
        self.start_terminal_connection(server, cx);
    }

    pub(in crate::pages::index) fn test_server_connection(
        &mut self,
        server: ServerResource,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let draft = match ServerConnectionDraft::try_from(&server) {
            Ok(draft) => draft,
            Err(error) => {
                window.push_notification(
                    status_notification(
                        self.host_connection_test_failed_message(&server.name, &error.to_string()),
                        NotificationType::Error,
                        cx,
                    )
                    .id1::<HostConnectionTestNotification>(server.id),
                    cx,
                );
                return;
            }
        };

        let server_id = server.id;
        self.host_connection_test_receivers.insert(
            server_id,
            spawn_ssh_connection_test(
                draft,
                HostConnectionTestTarget {
                    server_id,
                    server_name: server.name.clone(),
                },
            ),
        );
        window.push_notification(
            status_notification(
                self.host_connection_test_running_message(&server),
                NotificationType::Info,
                cx,
            )
            .id1::<HostConnectionTestNotification>(server.id),
            cx,
        );

        self.spawn_host_connection_test_poller(window, cx);
    }

    pub(in crate::pages::index) fn open_local_terminal_tab(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self
            .open_tabs
            .iter()
            .any(|tab| matches!(tab, OpenTab::LocalTerminal))
        {
            self.open_tabs.push(OpenTab::LocalTerminal);
        }

        self.active_tab = ActiveTab::LocalTerminal;
        self.ensure_terminal_session(TerminalId::Local);
        window.focus(&self.focus_handle, cx);
        self.start_local_terminal_connection(cx);
        cx.notify();
    }

    pub(in crate::pages::index) fn close_local_terminal_tab(&mut self, cx: &mut Context<Self>) {
        self.remove_terminal_session(TerminalId::Local);
        self.open_tabs
            .retain(|tab| !matches!(tab, OpenTab::LocalTerminal));

        if self.active_tab == ActiveTab::LocalTerminal {
            self.active_tab = self.next_available_tab();
        }

        cx.notify();
    }

    pub(in crate::pages::index) fn close_server_tab(
        &mut self,
        server_id: i32,
        cx: &mut Context<Self>,
    ) {
        self.remove_terminal_session(TerminalId::Server(server_id));
        self.open_tabs
            .retain(|tab| tab.server_id() != Some(server_id));

        if self.active_tab == ActiveTab::Server(server_id) {
            self.active_tab = self.next_available_tab();
        }

        cx.notify();
    }

    pub(in crate::pages::index) fn open_server_tab(&mut self, server: ServerResource) {
        if let Some(OpenTab::Server(existing)) = self
            .open_tabs
            .iter_mut()
            .find(|tab| tab.server_id() == Some(server.id))
        {
            *existing = server.clone();
        } else {
            self.open_tabs.push(OpenTab::Server(server.clone()));
        }

        self.active_tab = ActiveTab::Server(server.id);
    }

    fn spawn_host_connection_test_poller(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, window| {
            loop {
                window
                    .background_executor()
                    .timer(Duration::from_millis(100))
                    .await;

                let keep_polling = this
                    .update_in(window, |this, window, cx| {
                        this.poll_host_connection_test_results(window, cx)
                    })
                    .unwrap_or(false);

                if !keep_polling {
                    break;
                }
            }
        })
        .detach();
    }

    fn poll_host_connection_test_results(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut pending = false;
        let mut completed = Vec::new();
        let mut disconnected = Vec::new();

        for (server_id, rx) in &self.host_connection_test_receivers {
            match rx.try_recv() {
                Ok(result) => completed.push((*server_id, result)),
                Err(TryRecvError::Empty) => pending = true,
                Err(TryRecvError::Disconnected) => disconnected.push(*server_id),
            }
        }

        for (server_id, result) in completed {
            self.host_connection_test_receivers.remove(&server_id);
            self.push_host_connection_test_result(result, window, cx);
        }

        for server_id in disconnected {
            self.host_connection_test_receivers.remove(&server_id);
            window.push_notification(
                status_notification(
                    self.host_connection_test_no_result_message(),
                    NotificationType::Error,
                    cx,
                )
                .id1::<HostConnectionTestNotification>(server_id),
                cx,
            );
        }

        pending
    }

    fn push_host_connection_test_result(
        &self,
        result: crate::ipc::SshConnectionTestResult<HostConnectionTestTarget>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = result.context;
        let notification = match result.result {
            Ok(()) => status_notification(
                self.host_connection_test_succeeded_message(&target.server_name),
                NotificationType::Success,
                cx,
            ),
            Err(error) => status_notification(
                self.host_connection_test_failed_message(&target.server_name, &error),
                NotificationType::Error,
                cx,
            ),
        }
        .id1::<HostConnectionTestNotification>(target.server_id);

        window.push_notification(notification, cx);
    }

    fn host_connection_test_running_message(&self, server: &ServerResource) -> String {
        match self.language {
            Language::Zh => format!(
                "正在测试连接：{}@{}:{}",
                server.username, server.host, server.port
            ),
            Language::En => format!(
                "Testing connection: {}@{}:{}",
                server.username, server.host, server.port
            ),
            Language::Ja => format!(
                "接続をテスト中: {}@{}:{}",
                server.username, server.host, server.port
            ),
        }
    }

    fn host_connection_test_succeeded_message(&self, server_name: &str) -> String {
        match self.language {
            Language::Zh => format!("{server_name} 连接测试成功。"),
            Language::En => format!("{server_name} connection test succeeded."),
            Language::Ja => format!("{server_name} の接続テストに成功しました。"),
        }
    }

    fn host_connection_test_failed_message(&self, server_name: &str, error: &str) -> String {
        match self.language {
            Language::Zh => format!("{server_name} 连接测试失败：{error}"),
            Language::En => format!("{server_name} connection test failed: {error}"),
            Language::Ja => format!("{server_name} の接続テストに失敗しました: {error}"),
        }
    }

    fn host_connection_test_no_result_message(&self) -> &'static str {
        match self.language {
            Language::Zh => "连接测试失败：没有返回结果",
            Language::En => "Connection test failed: no result returned",
            Language::Ja => "接続テストに失敗しました: 結果が返りませんでした",
        }
    }

    fn next_available_tab(&self) -> ActiveTab {
        self.open_tabs
            .last()
            .map(OpenTab::active_tab)
            .unwrap_or(ActiveTab::Vault)
    }
}
