use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, Context, ElementId, Entity, IntoElement, RenderOnce, SharedString,
    Window, deferred, div, prelude::*, px, rgb,
};
use gpui_component::{
    Icon, Sizable,
    button::{Button, ButtonVariants},
};

use crate::ui::{AppThemeId, TextKey, icons};

use super::{Xssh, tabs::ActiveTab};

const SIDEBAR_COLLAPSED_WIDTH: f32 = 48.;
const SIDEBAR_WIDTH: f32 = 180.;
const SIDEBAR_ACTION_SIZE: f32 = 32.;
const SIDEBAR_ACTION_HEIGHT: f32 = 36.;
const SIDEBAR_TOOLTIP_OFFSET_X: f32 = 40.;
const SIDEBAR_TOOLTIP_OFFSET_Y: f32 = 2.;
const SIDEBAR_TOOLTIP_HEIGHT: f32 = 28.;

type SidebarActionClick = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(Clone, IntoElement)]
struct SidebarAction {
    id: SharedString,
    icon_path: &'static str,
    label: SharedString,
    tooltip: SharedString,
    theme: AppThemeId,
    active: bool,
    collapsed: bool,
    on_click: SidebarActionClick,
}

impl SidebarAction {
    fn new(
        id: impl Into<SharedString>,
        icon_path: &'static str,
        label: impl Into<SharedString>,
        tooltip: impl Into<SharedString>,
        theme: AppThemeId,
        active: bool,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            icon_path,
            label: label.into(),
            tooltip: tooltip.into(),
            theme,
            active,
            collapsed: false,
            on_click: Rc::new(on_click),
        }
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    fn render_action(self, id: ElementId) -> AnyElement {
        if self.collapsed {
            self.render_collapsed(id)
        } else {
            self.render_expanded(id)
        }
    }

    fn foreground(&self) -> u32 {
        let palette = self.theme.palette();

        if self.active {
            palette.text
        } else {
            palette.muted
        }
    }

    fn icon(&self, foreground: u32, small: bool) -> Icon {
        let icon = Icon::empty()
            .path(self.icon_path)
            .text_color(rgb(foreground));

        if small { icon.small() } else { icon }
    }

    fn render_collapsed(self, id: ElementId) -> AnyElement {
        let handler = self.on_click.clone();
        let palette = self.theme.palette();
        let foreground = self.foreground();
        let group_name = self.id.clone();
        let border = if self.active {
            palette.card_active_border
        } else {
            palette.titlebar_bg
        };

        div()
            .id(id)
            .group(group_name.clone())
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .w(px(SIDEBAR_ACTION_SIZE))
            .h(px(SIDEBAR_ACTION_SIZE))
            .rounded_md()
            .border_1()
            .border_color(rgb(border))
            .when(self.active, |this| this.bg(rgb(palette.card_active)))
            .hover(move |style| {
                style
                    .bg(rgb(palette.button_hover))
                    .border_color(rgb(palette.card_active_border))
            })
            .active(move |style| style.bg(rgb(palette.button_active)))
            .child(self.icon(foreground, false))
            .child(Self::right_tooltip(self.theme, group_name, self.tooltip))
            .on_click(move |event, window, cx| handler(event, window, cx))
            .into_any_element()
    }

    fn right_tooltip(
        theme: AppThemeId,
        group_name: SharedString,
        tooltip: SharedString,
    ) -> impl IntoElement {
        let palette = theme.palette();
        let tooltip_group = group_name.clone();

        deferred(
            div()
                .group(tooltip_group)
                .absolute()
                .left(px(SIDEBAR_TOOLTIP_OFFSET_X))
                .top(px(SIDEBAR_TOOLTIP_OFFSET_Y))
                .flex()
                .h(px(SIDEBAR_TOOLTIP_HEIGHT))
                .items_center()
                .rounded_md()
                .border_1()
                .border_color(rgb(palette.border))
                .bg(rgb(palette.panel_bg))
                .px_2()
                .text_size(px(12.))
                .text_color(rgb(palette.text))
                .whitespace_nowrap()
                .shadow_md()
                .invisible()
                .group_hover(group_name, |style| style.visible())
                .child(tooltip),
        )
        .with_priority(2)
    }

    fn render_expanded(self, id: ElementId) -> AnyElement {
        let handler = self.on_click.clone();
        let palette = self.theme.palette();
        let foreground = self.foreground();

        div()
            .id(id)
            .flex()
            .items_center()
            .gap_2()
            .h(px(SIDEBAR_ACTION_HEIGHT))
            .w_full()
            .px_2()
            .rounded_md()
            .text_size(px(14.))
            .text_color(rgb(foreground))
            .when(self.active, |this| {
                this.bg(rgb(palette.card_active))
                    .text_color(rgb(palette.text))
                    .border_1()
                    .border_color(rgb(palette.card_active_border))
            })
            .hover(move |style| {
                style
                    .bg(rgb(palette.button_hover))
                    .text_color(rgb(palette.text))
            })
            .child(self.icon(foreground, true))
            .child(div().min_w(px(0.)).truncate().child(self.label))
            .on_click(move |event, window, cx| handler(event, window, cx))
            .into_any_element()
    }
}

impl RenderOnce for SidebarAction {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let id = ElementId::from(self.id.clone());
        self.render_action(id)
    }
}

impl Xssh {
    pub(in crate::pages::index) fn sidebar_toggle(&self, view: Entity<Self>) -> impl IntoElement {
        let palette = self.theme.palette();
        let icon_path = if self.sidebar_collapsed {
            icons::sidebar_toggle::COLLAPSED_PATH
        } else {
            icons::sidebar_toggle::EXPANDED_PATH
        };
        let tooltip = SharedString::from(if self.sidebar_collapsed {
            self.language.tr(TextKey::ExpandSidebar)
        } else {
            self.language.tr(TextKey::CollapseSidebar)
        });

        Button::new("sidebar-toggle-button")
            .secondary()
            .small()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .tooltip(tooltip)
            .icon(Icon::empty().path(icon_path).small())
            .on_click(move |_, _, cx| {
                view.update(cx, |this, cx| {
                    this.sidebar_collapsed = !this.sidebar_collapsed;
                    cx.notify();
                });
            })
    }

    pub(in crate::pages::index) fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let settings_view = view.clone();
        let collapsed = self.sidebar_collapsed;
        let width = if collapsed {
            px(SIDEBAR_COLLAPSED_WIDTH)
        } else {
            px(SIDEBAR_WIDTH)
        };
        let palette = self.theme.palette();

        div()
            .id("main-sidebar")
            .relative()
            .flex()
            .flex_col()
            .flex_none()
            .h_full()
            .w(width)
            .bg(rgb(palette.titlebar_bg))
            .text_color(rgb(palette.text))
            .child(
                div()
                    .id("main-sidebar-content")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h(px(0.))
                    .gap_2()
                    .px_3()
                    .py_3()
                    .when(collapsed, |this| this.items_center().p_2())
                    .child(
                        SidebarAction::new(
                            "hosts-sidebar-button",
                            icons::server::PATH,
                            self.language.tr(TextKey::Hosts),
                            self.language.tr(TextKey::Hosts),
                            self.theme,
                            self.active_tab == ActiveTab::Vault,
                            move |event, window, cx| {
                                view.update(cx, |this, cx| {
                                    this.on_vault_tab(event, window, cx);
                                });
                            },
                        )
                        .collapsed(collapsed),
                    ),
            )
            .child(
                div()
                    .id("settings-sidebar-button")
                    .flex_none()
                    .flex()
                    .w_full()
                    .px_3()
                    .pb_3()
                    .when(collapsed, |this| this.justify_center().px_2())
                    .child(
                        SidebarAction::new(
                            "settings-sidebar-item",
                            icons::settings::PATH,
                            self.language.tr(TextKey::Settings),
                            self.language.tr(TextKey::Settings),
                            self.theme,
                            false,
                            move |event, window, cx| {
                                settings_view.update(cx, |this, cx| {
                                    this.on_open_settings_window(event, window, cx);
                                });
                            },
                        )
                        .collapsed(collapsed),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top(px(0.))
                    .right(px(0.))
                    .bottom(px(0.))
                    .w(px(1.))
                    .bg(rgb(palette.border)),
            )
    }
}
