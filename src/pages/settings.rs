use gpui::{
    App, Context, Entity, FocusHandle, Focusable, IntoElement, Render, Subscription, Window, div,
    prelude::*, px, rgb,
};
use gpui_component::select::{Select, SelectEvent, SelectState};

use crate::ui::{BASE_FONT_SIZE, Language, LanguageChoice, TextKey, ThemeChoice, ThemeMode, icons};

use super::Xssh;

pub(super) struct SettingsWindow {
    parent: Entity<Xssh>,
    language: Language,
    theme: ThemeMode,
    language_select: Entity<SelectState<Vec<LanguageChoice>>>,
    theme_select: Entity<SelectState<Vec<ThemeChoice>>>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl SettingsWindow {
    pub(super) fn new(
        parent: Entity<Xssh>,
        language: Language,
        theme: ThemeMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let language_select = cx.new(|cx| {
            SelectState::new(
                vec![
                    LanguageChoice::new(Language::Zh),
                    LanguageChoice::new(Language::En),
                    LanguageChoice::new(Language::Ja),
                ],
                Some(language.selected_index()),
                window,
                cx,
            )
        });
        let theme_select = cx.new(|cx| {
            SelectState::new(
                ThemeMode::options(language),
                Some(theme.selected_index()),
                window,
                cx,
            )
        });
        let _subscriptions = vec![
            cx.subscribe_in(&language_select, window, Self::on_language_select_event),
            cx.subscribe_in(&theme_select, window, Self::on_theme_select_event),
        ];

        Self {
            parent,
            language,
            theme,
            language_select,
            theme_select,
            focus_handle: cx.focus_handle(),
            _subscriptions,
        }
    }

    fn on_language_select_event(
        &mut self,
        _: &Entity<SelectState<Vec<LanguageChoice>>>,
        event: &SelectEvent<Vec<LanguageChoice>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(Some(language)) = event else {
            return;
        };

        self.language = *language;
        self.theme_select.update(cx, |select, cx| {
            select.set_items(ThemeMode::options(self.language), window, cx);
            select.set_selected_index(Some(self.theme.selected_index()), window, cx);
        });
        self.parent.update(cx, |parent, cx| {
            parent.set_language(self.language, window, cx);
        });
        cx.notify();
    }

    fn on_theme_select_event(
        &mut self,
        _: &Entity<SelectState<Vec<ThemeChoice>>>,
        event: &SelectEvent<Vec<ThemeChoice>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(Some(theme)) = event else {
            return;
        };

        self.theme = *theme;
        self.parent.update(cx, |parent, cx| {
            parent.set_theme(self.theme, cx);
        });
        cx.notify();
    }

    fn setting_row(
        theme: ThemeMode,
        label: &'static str,
        control: impl IntoElement,
    ) -> impl IntoElement {
        let palette = theme.palette();

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .child(
                div()
                    .text_size(px(BASE_FONT_SIZE))
                    .text_color(rgb(palette.text))
                    .child(label),
            )
            .child(div().w(px(190.)).child(control))
    }
}

impl Focusable for SettingsWindow {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme.palette();
        let language = self.language;

        div()
            .track_focus(&self.focus_handle(cx))
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(palette.panel_bg))
            .text_size(px(BASE_FONT_SIZE))
            .text_color(rgb(palette.text))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .pt_4()
                    .pb_3()
                    .border_b_1()
                    .border_color(rgb(palette.separator))
                    .child(icons::settings::icon(18., palette.text))
                    .child(language.tr(TextKey::Settings)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_4()
                    .child(Self::setting_row(
                        self.theme,
                        language.tr(TextKey::Theme),
                        Select::new(&self.theme_select)
                            .w_full()
                            .rounded_sm()
                            .bg(rgb(palette.input_bg))
                            .border_color(rgb(palette.input_border))
                            .text_color(rgb(palette.text)),
                    ))
                    .child(Self::setting_row(
                        self.theme,
                        language.tr(TextKey::Language),
                        Select::new(&self.language_select)
                            .w_full()
                            .rounded_sm()
                            .bg(rgb(palette.input_bg))
                            .border_color(rgb(palette.input_border))
                            .text_color(rgb(palette.text)),
                    )),
            )
    }
}
