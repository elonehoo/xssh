use gpui::SharedString;
use gpui_component::{IndexPath, select::SelectItem};

use super::i18n::{Language, TextKey};

pub(crate) const BASE_FONT_SIZE: f32 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    pub(crate) fn from_setting_value(value: &str) -> Self {
        match value {
            "light" => Self::Light,
            _ => Self::Dark,
        }
    }

    pub(crate) fn setting_value(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub(crate) fn selected_index(self) -> IndexPath {
        let row = match self {
            Self::Dark => 0,
            Self::Light => 1,
        };
        IndexPath::default().row(row)
    }

    pub(crate) fn options(language: Language) -> Vec<ThemeChoice> {
        vec![
            ThemeChoice::new(Self::Dark, language.tr(TextKey::DarkTheme)),
            ThemeChoice::new(Self::Light, language.tr(TextKey::LightTheme)),
        ]
    }

    pub(crate) fn palette(self) -> AppPalette {
        match self {
            Self::Dark => AppPalette {
                app_bg: 0x050505,
                titlebar_bg: 0x1f1f1f,
                panel_bg: 0x0f0f0f,
                panel_hover: 0x202020,
                text: 0xe8e8e8,
                muted: 0x8a8a8a,
                label: 0x57606a,
                border: 0x343434,
                separator: 0x2b2b2b,
                tab_active: 0x3c3c3c,
                tab_inactive: 0x262626,
                input_bg: 0x101010,
                input_inner_bg: 0x121212,
                input_border: 0x303030,
                card_bg: 0x141414,
                card_active: 0x202020,
                card_border: 0x242424,
                card_active_border: 0x4a4a4a,
                icon_bg: 0x333333,
                button_bg: 0x242424,
                button_border: 0x3a3a3a,
                primary_bg: 0xd8d8d8,
                primary_text: 0x181818,
                danger: 0xff7a7a,
            },
            Self::Light => AppPalette {
                app_bg: 0xf5f5f5,
                titlebar_bg: 0xf0f0f0,
                panel_bg: 0xffffff,
                panel_hover: 0xeeeeee,
                text: 0x202020,
                muted: 0x6f6f6f,
                label: 0x5d6673,
                border: 0xd7d7d7,
                separator: 0xdfdfdf,
                tab_active: 0xe1e1e1,
                tab_inactive: 0xf4f4f4,
                input_bg: 0xffffff,
                input_inner_bg: 0xffffff,
                input_border: 0xd0d0d0,
                card_bg: 0xffffff,
                card_active: 0xf1f1f1,
                card_border: 0xe0e0e0,
                card_active_border: 0xbdbdbd,
                icon_bg: 0xe8e8e8,
                button_bg: 0xffffff,
                button_border: 0xd0d0d0,
                primary_bg: 0x202020,
                primary_text: 0xffffff,
                danger: 0xc83f3f,
            },
        }
    }
}

#[derive(Clone)]
pub(crate) struct ThemeChoice {
    mode: ThemeMode,
    title: SharedString,
}

impl ThemeChoice {
    fn new(mode: ThemeMode, title: &'static str) -> Self {
        Self {
            mode,
            title: title.into(),
        }
    }
}

impl SelectItem for ThemeChoice {
    type Value = ThemeMode;

    fn title(&self) -> SharedString {
        self.title.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.mode
    }
}

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
    pub(crate) button_border: u32,
    pub(crate) primary_bg: u32,
    pub(crate) primary_text: u32,
    pub(crate) danger: u32,
}
