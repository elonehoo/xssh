use anyhow::Result;
use gpui::{ClickEvent, Context, Window, WindowHandle};
use gpui_component::Root;

use crate::{
    ipc::{ActiveTab, ServerDraft, ServerResource, delete_server, insert_server, update_server},
    ui::{Language, TextKey, ThemeMode},
};

use super::XsshDemo;

impl XsshDemo {
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

        for tab in self
            .open_tabs
            .iter_mut()
            .filter(|server| server.id == server_id)
        {
            *tab = updated.clone();
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

        self.servers.retain(|server| server.id != server_id);
        self.open_tabs.retain(|server| server.id != server_id);

        if self.active_tab == ActiveTab::Server(server_id) {
            self.active_tab = ActiveTab::Vault;
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

    pub(in crate::pages::index) fn on_open_first_server(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(server) = self.servers.first().cloned() {
            self.open_server_tab(server, cx);
        } else {
            cx.notify();
        }
    }

    pub(in crate::pages::index) fn open_server_tab(
        &mut self,
        server: ServerResource,
        cx: &mut Context<Self>,
    ) {
        if !self.open_tabs.iter().any(|tab| tab.id == server.id) {
            self.open_tabs.push(server.clone());
        }

        self.active_tab = ActiveTab::Server(server.id);
        cx.notify();
    }
}
