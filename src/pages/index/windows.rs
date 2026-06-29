use gpui::{
    App, Bounds, ClickEvent, Context, Entity, Focusable, Window, WindowBounds, WindowKind,
    WindowOptions, div, prelude::*, px, rgb, size,
};
use gpui_component::{
    Root, StyledExt, WindowExt,
    button::ButtonVariant,
    dialog::{Dialog, DialogButtonProps},
};

use crate::{
    ipc::ServerResource,
    ui::{BASE_FONT_SIZE, Language, TextKey, ThemeMode},
};

use super::super::{post_host::CreateHostWindow, settings::SettingsWindow};
use super::XsshDemo;

impl XsshDemo {
    pub(in crate::pages::index) fn on_open_create_host_window(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if Self::activate_singleton_window(&mut self.create_host_window, cx) {
            return;
        }

        let parent = cx.entity();
        let language = self.language;
        let theme = self.theme;
        let bounds = Bounds::centered(None, size(px(560.), px(560.)), cx);
        let create_window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(520.), px(520.))),
                    kind: WindowKind::Normal,
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some(language.tr(TextKey::CreateHost).into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    let view =
                        cx.new(|cx| CreateHostWindow::new(parent, language, theme, window, cx));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .unwrap();

        create_window
            .update(cx, |root, window, cx| {
                if let Ok(view) = root.view().clone().downcast::<CreateHostWindow>() {
                    window.focus(&view.read(cx).name_focus_handle(cx));
                }
            })
            .ok();

        self.create_host_window = Some(create_window);
        cx.notify();
    }

    pub(in crate::pages::index) fn open_edit_host_window(
        &mut self,
        server: ServerResource,
        cx: &mut Context<Self>,
    ) {
        if Self::activate_singleton_window(&mut self.edit_host_window, cx) {
            return;
        }

        let parent = cx.entity();
        let language = self.language;
        let theme = self.theme;
        let bounds = Bounds::centered(None, size(px(560.), px(560.)), cx);
        let edit_window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(520.), px(520.))),
                    kind: WindowKind::Normal,
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some(language.tr(TextKey::EditHost).into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                move |window, cx| {
                    let view = cx.new(|cx| {
                        CreateHostWindow::edit(parent, language, theme, server, window, cx)
                    });
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .unwrap();

        edit_window
            .update(cx, |root, window, cx| {
                if let Ok(view) = root.view().clone().downcast::<CreateHostWindow>() {
                    window.focus(&view.read(cx).name_focus_handle(cx));
                }
            })
            .ok();

        self.edit_host_window = Some(edit_window);
    }

    pub(in crate::pages::index) fn open_delete_host_dialog(
        language: Language,
        theme: ThemeMode,
        server: ServerResource,
        view: Entity<Self>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let palette = theme.palette();
        let server_id = server.id;
        let server_name = server.name.clone();
        let server_detail = format!("{}@{}:{}", server.username, server.host, server.port);
        let title = language.tr(TextKey::DeleteHost).to_string();
        let message = language.delete_host_message(&server_name);
        let delete_text = language.tr(TextKey::Delete).to_string();
        let cancel_text = language.tr(TextKey::Cancel).to_string();

        window.open_dialog(cx, move |dialog: Dialog, _, _| {
            dialog
                .title(
                    div()
                        .text_size(px(BASE_FONT_SIZE))
                        .font_semibold()
                        .child(title.clone()),
                )
                .w(px(420.))
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(delete_text.clone())
                        .ok_variant(ButtonVariant::Danger)
                        .cancel_text(cancel_text.clone()),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .text_size(px(14.))
                        .text_color(rgb(palette.text))
                        .child(message.clone())
                        .child(
                            div()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.input_inner_bg))
                                .p_3()
                                .child(server_detail.clone()),
                        ),
                )
                .on_ok({
                    let view = view.clone();
                    move |_, _, cx| match view
                        .update(cx, |this, cx| this.delete_server_by_id(server_id, cx))
                    {
                        Ok(()) => true,
                        Err(error) => {
                            eprintln!("删除 Host 失败: {error}");
                            false
                        }
                    }
                })
        });
    }

    pub(in crate::pages::index) fn on_open_settings_window(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if Self::activate_singleton_window(&mut self.settings_window, cx) {
            return;
        }

        let parent = cx.entity();
        let language = self.language;
        let theme = self.theme;
        let bounds = Bounds::centered(None, size(px(420.), px(260.)), cx);
        let settings_window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(380.), px(240.))),
                    kind: WindowKind::Normal,
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some(language.tr(TextKey::Settings).into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    let view =
                        cx.new(|cx| SettingsWindow::new(parent, language, theme, window, cx));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .unwrap();

        settings_window
            .update(cx, |root, window, cx| {
                if let Ok(view) = root.view().clone().downcast::<SettingsWindow>() {
                    window.focus(&view.read(cx).focus_handle(cx));
                }
            })
            .ok();

        self.settings_window = Some(settings_window);
    }
}
