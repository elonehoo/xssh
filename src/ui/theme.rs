use gpui::{App, Hsla, Window, rgb};
use gpui_component::{
    Theme as ComponentTheme, ThemeColor, ThemeMode as ComponentThemeMode,
    ThemeTokens as ComponentThemeTokens,
};

use super::base46::{AppTheme, AppThemeId, AppThemeKind, TerminalThemePalette, app_theme_by_id};

pub(crate) const BASE_FONT_SIZE: f32 = 16.0;

#[derive(Clone, Copy)]
pub(crate) struct AppPalette {
    pub(crate) app_bg: u32,
    pub(crate) titlebar_bg: u32,
    pub(crate) panel_bg: u32,
    pub(crate) panel_hover: u32,
    pub(crate) text: u32,
    pub(crate) muted: u32,
    pub(crate) label: u32,
    pub(crate) border: u32,
    pub(crate) separator: u32,
    pub(crate) tab_active: u32,
    pub(crate) tab_inactive: u32,
    pub(crate) input_bg: u32,
    pub(crate) input_inner_bg: u32,
    pub(crate) input_border: u32,
    pub(crate) card_bg: u32,
    pub(crate) card_active: u32,
    pub(crate) card_border: u32,
    pub(crate) card_active_border: u32,
    pub(crate) icon_bg: u32,
    pub(crate) button_bg: u32,
    pub(crate) button_hover: u32,
    pub(crate) button_active: u32,
    pub(crate) button_border: u32,
    pub(crate) primary_bg: u32,
    pub(crate) primary_hover: u32,
    pub(crate) primary_active: u32,
    pub(crate) primary_text: u32,
    pub(crate) danger: u32,
}

impl AppThemeId {
    pub(crate) fn palette(self) -> AppPalette {
        app_theme_by_id(self).app_palette()
    }
}

impl AppTheme {
    fn app_palette(self) -> AppPalette {
        let accent = pick_accent(self.background, &self.ansi);
        let danger = self.ansi[1];
        let (primary_hover, primary_active) = primary_state_colors(accent);

        match self.kind {
            AppThemeKind::Dark => AppPalette {
                app_bg: shade(self.background, 0.10),
                titlebar_bg: tint(self.background, 0.04),
                panel_bg: self.background,
                panel_hover: tint(self.background, 0.10),
                text: self.foreground,
                muted: mix(self.foreground, self.background, 0.58),
                label: mix(self.foreground, self.background, 0.48),
                border: mix(self.foreground, self.background, 0.20),
                separator: mix(self.foreground, self.background, 0.14),
                tab_active: tint(self.background, 0.14),
                tab_inactive: tint(self.background, 0.06),
                input_bg: tint(self.background, 0.03),
                input_inner_bg: shade(self.background, 0.04),
                input_border: mix(self.foreground, self.background, 0.18),
                card_bg: tint(self.background, 0.03),
                card_active: tint(self.background, 0.09),
                card_border: mix(self.foreground, self.background, 0.13),
                card_active_border: mix(accent, self.foreground, 0.45),
                icon_bg: tint(self.background, 0.16),
                button_bg: tint(self.background, 0.10),
                button_hover: tint(self.background, 0.16),
                button_active: tint(self.background, 0.22),
                button_border: mix(self.foreground, self.background, 0.24),
                primary_bg: accent,
                primary_hover,
                primary_active,
                primary_text: readable_text(accent),
                danger,
            },
            AppThemeKind::Light => AppPalette {
                app_bg: shade(self.background, 0.04),
                titlebar_bg: shade(self.background, 0.06),
                panel_bg: self.background,
                panel_hover: shade(self.background, 0.08),
                text: self.foreground,
                muted: mix(self.foreground, self.background, 0.62),
                label: mix(self.foreground, self.background, 0.52),
                border: mix(self.foreground, self.background, 0.18),
                separator: mix(self.foreground, self.background, 0.12),
                tab_active: shade(self.background, 0.10),
                tab_inactive: shade(self.background, 0.03),
                input_bg: self.background,
                input_inner_bg: self.background,
                input_border: mix(self.foreground, self.background, 0.22),
                card_bg: self.background,
                card_active: shade(self.background, 0.06),
                card_border: mix(self.foreground, self.background, 0.14),
                card_active_border: mix(accent, self.foreground, 0.36),
                icon_bg: shade(self.background, 0.10),
                button_bg: self.background,
                button_hover: shade(self.background, 0.06),
                button_active: shade(self.background, 0.10),
                button_border: mix(self.foreground, self.background, 0.22),
                primary_bg: accent,
                primary_hover,
                primary_active,
                primary_text: readable_text(accent),
                danger,
            },
        }
    }
}

pub(crate) fn sync_component_theme(theme: AppThemeId, window: Option<&mut Window>, cx: &mut App) {
    let app_theme = *app_theme_by_id(theme);
    ComponentTheme::change(component_theme_mode(app_theme.kind), None, cx);

    let component_theme = ComponentTheme::global_mut(cx);
    apply_component_theme_colors(
        &mut component_theme.colors,
        app_theme.app_palette(),
        app_theme.terminal_palette(),
    );
    component_theme.tokens = ComponentThemeTokens::from(&component_theme.colors);

    if let Some(window) = window {
        window.refresh();
    }
}

fn component_theme_mode(kind: AppThemeKind) -> ComponentThemeMode {
    match kind {
        AppThemeKind::Dark => ComponentThemeMode::Dark,
        AppThemeKind::Light => ComponentThemeMode::Light,
    }
}

fn apply_component_theme_colors(
    colors: &mut ThemeColor,
    palette: AppPalette,
    terminal_palette: TerminalThemePalette,
) {
    let primary = ComponentStateColors {
        base: to_hsla(palette.primary_bg),
        hover: to_hsla(palette.primary_hover),
        active: to_hsla(palette.primary_active),
        foreground: to_hsla(palette.primary_text),
    };
    let danger = component_state_colors(palette.danger);
    let info = component_state_colors(terminal_palette.ansi[4]);
    let success = component_state_colors(terminal_palette.ansi[2]);
    let warning = component_state_colors(terminal_palette.ansi[3]);
    let text = to_hsla(palette.text);
    let muted = to_hsla(palette.muted);
    let panel_bg = to_hsla(palette.panel_bg);
    let panel_hover = to_hsla(palette.panel_hover);
    let button_bg = to_hsla(palette.button_bg);
    let button_hover = to_hsla(palette.button_hover);
    let button_active = to_hsla(palette.button_active);
    let border = to_hsla(palette.border);
    let card_bg = to_hsla(palette.card_bg);
    let card_active = to_hsla(palette.card_active);
    let blue = to_hsla(terminal_palette.ansi[4]);
    let blue_light = to_hsla(terminal_palette.ansi[12]);
    let green = to_hsla(terminal_palette.ansi[2]);
    let green_light = to_hsla(terminal_palette.ansi[10]);
    let yellow = to_hsla(terminal_palette.ansi[3]);
    let yellow_light = to_hsla(terminal_palette.ansi[11]);
    let magenta = to_hsla(terminal_palette.ansi[5]);
    let magenta_light = to_hsla(terminal_palette.ansi[13]);
    let cyan = to_hsla(terminal_palette.ansi[6]);
    let cyan_light = to_hsla(terminal_palette.ansi[14]);

    colors.accent = panel_hover;
    colors.accent_foreground = text;
    colors.accordion = card_bg;
    colors.accordion_hover = panel_hover;
    colors.background = panel_bg;
    colors.border = border;
    colors.button = button_bg;
    colors.button_active = button_active;
    colors.button_foreground = text;
    colors.button_hover = button_hover;
    colors.button_primary = primary.base;
    colors.button_primary_active = primary.active;
    colors.button_primary_foreground = primary.foreground;
    colors.button_primary_hover = primary.hover;
    colors.button_secondary = button_bg;
    colors.button_secondary_active = button_active;
    colors.button_secondary_foreground = text;
    colors.button_secondary_hover = button_hover;
    colors.button_danger = danger.base;
    colors.button_danger_active = danger.active;
    colors.button_danger_foreground = danger.foreground;
    colors.button_danger_hover = danger.hover;
    colors.button_info = info.base;
    colors.button_info_active = info.active;
    colors.button_info_foreground = info.foreground;
    colors.button_info_hover = info.hover;
    colors.button_success = success.base;
    colors.button_success_active = success.active;
    colors.button_success_foreground = success.foreground;
    colors.button_success_hover = success.hover;
    colors.button_warning = warning.base;
    colors.button_warning_active = warning.active;
    colors.button_warning_foreground = warning.foreground;
    colors.button_warning_hover = warning.hover;
    colors.group_box = panel_bg;
    colors.group_box_foreground = text;
    colors.caret = to_hsla(terminal_palette.cursor);
    colors.chart_1 = blue;
    colors.chart_2 = green;
    colors.chart_3 = yellow;
    colors.chart_4 = magenta;
    colors.chart_5 = cyan;
    colors.danger = danger.base;
    colors.danger_active = danger.active;
    colors.danger_foreground = danger.foreground;
    colors.danger_hover = danger.hover;
    colors.description_list_label = card_bg;
    colors.description_list_label_foreground = muted;
    colors.drag_border = to_hsla(palette.card_active_border);
    colors.drop_target = primary.base.opacity(0.18);
    colors.foreground = text;
    colors.info = info.base;
    colors.info_active = info.active;
    colors.info_foreground = info.foreground;
    colors.info_hover = info.hover;
    colors.input = to_hsla(palette.input_border);
    colors.link = primary.base;
    colors.link_active = primary.active;
    colors.link_hover = primary.hover;
    colors.list = card_bg;
    colors.list_active = card_active;
    colors.list_active_border = to_hsla(palette.card_active_border);
    colors.list_even = to_hsla(palette.input_bg);
    colors.list_head = to_hsla(palette.titlebar_bg);
    colors.list_hover = panel_hover;
    colors.muted = to_hsla(palette.button_bg);
    colors.muted_foreground = muted;
    colors.popover = panel_bg;
    colors.popover_foreground = text;
    colors.primary = primary.base;
    colors.primary_active = primary.active;
    colors.primary_foreground = primary.foreground;
    colors.primary_hover = primary.hover;
    colors.progress_bar = primary.base;
    colors.ring = to_hsla(palette.card_active_border);
    colors.scrollbar = panel_bg;
    colors.scrollbar_thumb = to_hsla(palette.button_border);
    colors.scrollbar_thumb_hover = muted;
    colors.secondary = button_bg;
    colors.secondary_active = button_active;
    colors.secondary_foreground = text;
    colors.secondary_hover = button_hover;
    colors.selection = primary.base.opacity(0.28);
    colors.sidebar = to_hsla(palette.app_bg);
    colors.sidebar_accent = card_active;
    colors.sidebar_accent_foreground = text;
    colors.sidebar_border = border;
    colors.sidebar_foreground = text;
    colors.sidebar_primary = primary.base;
    colors.sidebar_primary_foreground = primary.foreground;
    colors.skeleton = button_bg;
    colors.slider_bar = to_hsla(palette.button_border);
    colors.slider_thumb = primary.base;
    colors.success = success.base;
    colors.success_foreground = success.foreground;
    colors.success_hover = success.hover;
    colors.success_active = success.active;
    colors.chart_bullish = green;
    colors.chart_bearish = danger.base;
    colors.switch = to_hsla(palette.button_border);
    colors.switch_thumb = primary.base;
    colors.tab = to_hsla(palette.tab_inactive);
    colors.tab_active = to_hsla(palette.tab_active);
    colors.tab_active_foreground = text;
    colors.tab_bar = to_hsla(palette.titlebar_bg);
    colors.tab_bar_segmented = to_hsla(palette.tab_inactive);
    colors.tab_foreground = muted;
    colors.table = card_bg;
    colors.table_active = card_active;
    colors.table_active_border = to_hsla(palette.card_active_border);
    colors.table_even = to_hsla(palette.input_bg);
    colors.table_head = to_hsla(palette.titlebar_bg);
    colors.table_head_foreground = text;
    colors.table_hover = panel_hover;
    colors.table_row_border = border;
    colors.title_bar = to_hsla(palette.titlebar_bg);
    colors.title_bar_border = border;
    colors.tiles = panel_bg;
    colors.warning = warning.base;
    colors.warning_active = warning.active;
    colors.warning_hover = warning.hover;
    colors.warning_foreground = warning.foreground;
    colors.overlay = to_hsla(palette.app_bg).opacity(0.72);
    colors.window_border = border;
    colors.red = danger.base;
    colors.red_light = danger.hover;
    colors.green = green;
    colors.green_light = green_light;
    colors.blue = blue;
    colors.blue_light = blue_light;
    colors.yellow = yellow;
    colors.yellow_light = yellow_light;
    colors.magenta = magenta;
    colors.magenta_light = magenta_light;
    colors.cyan = cyan;
    colors.cyan_light = cyan_light;
}

#[derive(Clone, Copy)]
struct ComponentStateColors {
    base: Hsla,
    hover: Hsla,
    active: Hsla,
    foreground: Hsla,
}

fn component_state_colors(color: u32) -> ComponentStateColors {
    let (hover, active) = primary_state_colors(color);

    ComponentStateColors {
        base: to_hsla(color),
        hover: to_hsla(hover),
        active: to_hsla(active),
        foreground: to_hsla(readable_text(color)),
    }
}

fn pick_accent(background: u32, ansi: &[u32; 16]) -> u32 {
    [4, 5, 6, 2, 3, 1]
        .into_iter()
        .map(|index| ansi[index])
        .max_by_key(|color| color_distance(background, *color))
        .unwrap_or(ansi[4])
}

fn color_distance(a: u32, b: u32) -> u32 {
    let (ar, ag, ab) = rgb_parts(a);
    let (br, bg, bb) = rgb_parts(b);
    let dr = ar as i32 - br as i32;
    let dg = ag as i32 - bg as i32;
    let db = ab as i32 - bb as i32;

    (dr * dr + dg * dg + db * db) as u32
}

fn readable_text(background: u32) -> u32 {
    if luminance(background) > 0.48 {
        0x101010
    } else {
        0xffffff
    }
}

fn primary_state_colors(color: u32) -> (u32, u32) {
    if luminance(color) > 0.55 {
        (shade(color, 0.08), shade(color, 0.14))
    } else {
        (tint(color, 0.10), tint(color, 0.16))
    }
}

fn tint(color: u32, amount: f32) -> u32 {
    mix(0xffffff, color, amount)
}

fn shade(color: u32, amount: f32) -> u32 {
    mix(0x000000, color, amount)
}

fn mix(foreground: u32, background: u32, amount: f32) -> u32 {
    let amount = amount.clamp(0.0, 1.0);
    let (fr, fg, fb) = rgb_parts(foreground);
    let (br, bg, bb) = rgb_parts(background);

    let channel = |front: u8, back: u8| -> u32 {
        ((front as f32 * amount) + (back as f32 * (1.0 - amount))).round() as u32
    };

    (channel(fr, br) << 16) | (channel(fg, bg) << 8) | channel(fb, bb)
}

fn luminance(color: u32) -> f32 {
    let (r, g, b) = rgb_parts(color);
    (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0
}

fn rgb_parts(color: u32) -> (u8, u8, u8) {
    (
        ((color >> 16) & 0xff) as u8,
        ((color >> 8) & 0xff) as u8,
        (color & 0xff) as u8,
    )
}

fn to_hsla(color: u32) -> Hsla {
    rgb(color).into()
}
