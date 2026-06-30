use gpui::{
    App, Context, Entity, FocusHandle, Focusable, IntoElement, Render, SharedString,
    StyleRefinement, Subscription, Window, WindowControlArea, div, prelude::*, px, rgb,
};
use gpui_component::{
    StyledExt,
    select::{Select, SelectDelegate, SelectEvent, SelectState},
    setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings},
};

use crate::ui::{
    AppThemeChoice, AppThemeId, BASE_FONT_SIZE, Language, LanguageChoice, TextKey,
    app_theme_options, app_theme_selected_index,
};

use super::Xssh;

pub(super) struct SettingsWindow {
    parent: Entity<Xssh>,
    language: Language,
    theme: AppThemeId,
    language_select: Entity<SelectState<Vec<LanguageChoice>>>,
    theme_select: Entity<SelectState<Vec<AppThemeChoice>>>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl SettingsWindow {
    pub(super) fn new(
        parent: Entity<Xssh>,
        language: Language,
        theme: AppThemeId,
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
                app_theme_options(),
                Some(app_theme_selected_index(theme)),
                window,
                cx,
            )
            .searchable(true)
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
        self.parent.update(cx, |parent, cx| {
            parent.set_language(self.language, window, cx);
        });
        cx.notify();
    }

    fn on_theme_select_event(
        &mut self,
        _: &Entity<SelectState<Vec<AppThemeChoice>>>,
        event: &SelectEvent<Vec<AppThemeChoice>>,
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

    fn select_field<D>(
        select: Entity<SelectState<D>>,
        theme: AppThemeId,
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

    fn sidebar_style(theme: AppThemeId) -> StyleRefinement {
        let palette = theme.palette();

        StyleRefinement::default()
            .bg(rgb(palette.app_bg))
            .text_color(rgb(palette.text))
            .border_color(rgb(palette.border))
    }

    fn header(theme: AppThemeId, language: Language) -> impl IntoElement {
        let palette = theme.palette();

        div()
            .flex()
            .flex_none()
            .items_center()
            .h(px(36.))
            .w_full()
            .bg(rgb(palette.titlebar_bg))
            .border_b_1()
            .border_color(rgb(palette.border))
            .window_control_area(WindowControlArea::Drag)
            .child(div().flex_none().w(px(92.)))
            .child(
                div()
                    .text_size(px(13.))
                    .font_semibold()
                    .text_color(rgb(palette.text))
                    .child(language.tr(TextKey::Settings)),
            )
            .child(div().flex_1())
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
        let sidebar_style = Self::sidebar_style(self.theme);

        div()
            .track_focus(&self.focus_handle(cx))
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(palette.panel_bg))
            .text_size(px(BASE_FONT_SIZE))
            .text_color(rgb(palette.text))
            .child(Self::header(self.theme, language))
            .child(
                div().flex_1().min_h(px(0.)).child(
                    Settings::new("xssh-settings")
                        .sidebar_width(px(210.))
                        .sidebar_style(&sidebar_style)
                        .page(
                            SettingPage::new(language.tr(TextKey::Settings))
                                .default_open(true)
                                .resettable(false)
                                .group(
                                    SettingGroup::new()
                                        .title(language.tr(TextKey::Appearance))
                                        .item(SettingItem::new(
                                            language.tr(TextKey::Theme),
                                            Self::select_field(
                                                self.theme_select.clone(),
                                                self.theme,
                                            ),
                                        )),
                                )
                                .group(
                                    SettingGroup::new()
                                        .title(language.tr(TextKey::Language))
                                        .item(SettingItem::new(
                                            language.tr(TextKey::Language),
                                            Self::select_field(
                                                self.language_select.clone(),
                                                self.theme,
                                            ),
                                        )),
                                ),
                        ),
                ),
            )
    }
}
