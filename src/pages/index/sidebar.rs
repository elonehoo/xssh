use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, Context, ElementId, Entity, IntoElement, RenderOnce, SharedString,
    Window, div, prelude::*, px, rgb,
};
use gpui_component::{
    ActiveTheme, Collapsible, Icon, Selectable, Side, Sizable,
    button::{Button, ButtonVariants},
    sidebar::{Sidebar, SidebarItem, SidebarMenuItem},
};

use crate::ui::{TextKey, icons};

use super::{Xssh, tabs::ActiveTab};

const SIDEBAR_COLLAPSED_WIDTH: f32 = 48.;
const SIDEBAR_WIDTH: f32 = 180.;

type SidebarActionClick = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(Clone, IntoElement)]
struct SidebarAction {
    id: SharedString,
    icon_path: &'static str,
    label: SharedString,
    tooltip: SharedString,
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
        active: bool,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            icon_path,
            label: label.into(),
            tooltip: tooltip.into(),
            active,
            collapsed: false,
            on_click: Rc::new(on_click),
        }
    }

    fn render_action(self, id: ElementId, window: &mut Window, cx: &mut App) -> AnyElement {
        let handler = self.on_click.clone();

        if self.collapsed {
            return Button::new(id)
                .ghost()
                .with_size(px(32.))
                .rounded(cx.theme().radius)
                .selected(self.active)
                .tooltip(self.tooltip)
                .icon(Icon::empty().path(self.icon_path))
                .on_click(move |event, window, cx| handler(event, window, cx))
                .into_any_element();
        }

        SidebarItem::render(
            SidebarMenuItem::new(self.label)
                .icon(Icon::empty().path(self.icon_path).small())
                .active(self.active)
                .on_click(move |event, window, cx| handler(event, window, cx)),
            id,
            window,
            cx,
        )
        .into_any_element()
    }
}

impl Collapsible for SidebarAction {
    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    fn is_collapsed(&self) -> bool {
        self.collapsed
    }
}

impl RenderOnce for SidebarAction {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = ElementId::from(self.id.clone());
        self.render_action(id, window, cx)
    }
}

impl SidebarItem for SidebarAction {
    fn render(
        self,
        id: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        self.render_action(id.into(), window, cx)
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
            .ghost()
            .small()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.tab_inactive))
            .text_color(rgb(palette.muted))
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

        Sidebar::new("main-sidebar")
            .side(Side::Left)
            .w(width)
            .collapsible(true)
            .collapsed(collapsed)
            .child(SidebarAction::new(
                "hosts-sidebar-button",
                icons::server::PATH,
                self.language.tr(TextKey::Hosts),
                self.language.tr(TextKey::Hosts),
                self.active_tab == ActiveTab::Vault,
                move |event, window, cx| {
                    view.update(cx, |this, cx| {
                        this.on_vault_tab(event, window, cx);
                    });
                },
            ))
            .footer(
                div().id("settings-sidebar-button").w_full().child(
                    SidebarAction::new(
                        "settings-sidebar-item",
                        icons::settings::PATH,
                        self.language.tr(TextKey::Settings),
                        self.language.tr(TextKey::Settings),
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
    }
}
