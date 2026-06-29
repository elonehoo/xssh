use anyhow::Result;
use gpui::{ClickEvent, Context, Window, WindowHandle};
use gpui_component::Root;

use crate::{
    ipc::{ServerDraft, ServerResource, delete_server, insert_server, update_server},
    ui::{Language, TextKey, ThemeMode},
};

use super::{
    Xssh,
    tabs::{ActiveTab, OpenTab, TerminalId},
};

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
        cx.notify();
    }

    pub(in crate::pages) fn set_theme(&mut self, theme: ThemeMode, cx: &mut Context<Self>) {
        self.theme = theme;
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
        window.focus(&self.focus_handle);
        self.start_terminal_connection(server, cx);
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
        window.focus(&self.focus_handle);
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

    fn next_available_tab(&self) -> ActiveTab {
        self.open_tabs
            .last()
            .map(OpenTab::active_tab)
            .unwrap_or(ActiveTab::Vault)
    }
}
