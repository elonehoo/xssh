use gpui::{
    App, Context, Entity, FocusHandle, Focusable, IntoElement, Render, SharedString, Subscription,
    Window, div, prelude::*, px, rgb,
};
use gpui_component::{
    select::{Select, SelectDelegate, SelectEvent, SelectState},
    setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings},
};

use crate::ui::{
    BASE_FONT_SIZE, Language, LanguageChoice, TerminalThemeChoice, TerminalThemeId,
    TerminalThemeKind, TextKey, ThemeChoice, ThemeMode, terminal_theme_options,
    terminal_theme_selected_index,
};

use super::Xssh;

pub(super) struct SettingsWindow {
    parent: Entity<Xssh>,
    language: Language,
    theme: ThemeMode,
    dark_terminal_theme: TerminalThemeId,
    light_terminal_theme: TerminalThemeId,
    language_select: Entity<SelectState<Vec<LanguageChoice>>>,
    theme_select: Entity<SelectState<Vec<ThemeChoice>>>,
    dark_terminal_theme_select: Entity<SelectState<Vec<TerminalThemeChoice>>>,
    light_terminal_theme_select: Entity<SelectState<Vec<TerminalThemeChoice>>>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl SettingsWindow {
    pub(super) fn new(
        parent: Entity<Xssh>,
        language: Language,
        theme: ThemeMode,
        dark_terminal_theme: TerminalThemeId,
        light_terminal_theme: TerminalThemeId,
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
        let dark_terminal_theme_select = cx.new(|cx| {
            SelectState::new(
                terminal_theme_options(TerminalThemeKind::Dark),
                Some(terminal_theme_selected_index(
                    dark_terminal_theme,
                    TerminalThemeKind::Dark,
                )),
                window,
                cx,
            )
            .searchable(true)
        });
        let light_terminal_theme_select = cx.new(|cx| {
            SelectState::new(
                terminal_theme_options(TerminalThemeKind::Light),
                Some(terminal_theme_selected_index(
                    light_terminal_theme,
                    TerminalThemeKind::Light,
                )),
                window,
                cx,
            )
            .searchable(true)
        });
        let _subscriptions = vec![
            cx.subscribe_in(&language_select, window, Self::on_language_select_event),
            cx.subscribe_in(&theme_select, window, Self::on_theme_select_event),
            cx.subscribe_in(
                &dark_terminal_theme_select,
                window,
                Self::on_dark_terminal_theme_select_event,
            ),
            cx.subscribe_in(
                &light_terminal_theme_select,
                window,
                Self::on_light_terminal_theme_select_event,
            ),
        ];

        Self {
            parent,
            language,
            theme,
            dark_terminal_theme,
            light_terminal_theme,
            language_select,
            theme_select,
            dark_terminal_theme_select,
            light_terminal_theme_select,
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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(Some(theme)) = event else {
            return;
        };

        self.theme = *theme;
        self.parent.update(cx, |parent, cx| {
            parent.set_theme(self.theme, window, cx);
        });
        cx.notify();
    }

    fn on_dark_terminal_theme_select_event(
        &mut self,
        _: &Entity<SelectState<Vec<TerminalThemeChoice>>>,
        event: &SelectEvent<Vec<TerminalThemeChoice>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(Some(terminal_theme)) = event else {
            return;
        };

        self.dark_terminal_theme = *terminal_theme;
        self.parent.update(cx, |parent, cx| {
            parent.set_dark_terminal_theme(self.dark_terminal_theme, cx);
        });
        cx.notify();
    }

    fn on_light_terminal_theme_select_event(
        &mut self,
        _: &Entity<SelectState<Vec<TerminalThemeChoice>>>,
        event: &SelectEvent<Vec<TerminalThemeChoice>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(Some(terminal_theme)) = event else {
            return;
        };

        self.light_terminal_theme = *terminal_theme;
        self.parent.update(cx, |parent, cx| {
            parent.set_light_terminal_theme(self.light_terminal_theme, cx);
        });
        cx.notify();
    }

    fn select_field<D>(
        select: Entity<SelectState<D>>,
        theme: ThemeMode,
    ) -> SettingField<SharedString>
    where
        D: SelectDelegate + 'static,
    {
        SettingField::render(move |_, _, _| {
            let palette = theme.palette();

            Select::new(&select)
                .w(px(280.))
                .rounded_sm()
                .bg(rgb(palette.input_bg))
                .border_color(rgb(palette.input_border))
                .text_color(rgb(palette.text))
        })
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
            .size_full()
            .bg(rgb(palette.panel_bg))
            .text_size(px(BASE_FONT_SIZE))
            .text_color(rgb(palette.text))
            .child(
                Settings::new("xssh-settings").sidebar_width(px(210.)).page(
                    SettingPage::new(language.tr(TextKey::Settings))
                        .default_open(true)
                        .resettable(false)
                        .group(
                            SettingGroup::new()
                                .title(language.tr(TextKey::Appearance))
                                .item(SettingItem::new(
                                    language.tr(TextKey::Theme),
                                    Self::select_field(self.theme_select.clone(), self.theme),
                                )),
                        )
                        .group(
                            SettingGroup::new()
                                .title(language.tr(TextKey::TerminalTheme))
                                .item(SettingItem::new(
                                    language.tr(TextKey::DarkTerminalTheme),
                                    Self::select_field(
                                        self.dark_terminal_theme_select.clone(),
                                        self.theme,
                                    ),
                                ))
                                .item(SettingItem::new(
                                    language.tr(TextKey::LightTerminalTheme),
                                    Self::select_field(
                                        self.light_terminal_theme_select.clone(),
                                        self.theme,
                                    ),
                                )),
                        )
                        .group(
                            SettingGroup::new()
                                .title(language.tr(TextKey::Language))
                                .item(SettingItem::new(
                                    language.tr(TextKey::Language),
                                    Self::select_field(self.language_select.clone(), self.theme),
                                )),
                        ),
                ),
            )
    }
}
