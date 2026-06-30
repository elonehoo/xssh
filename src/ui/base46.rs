use gpui::SharedString;
use gpui_component::{IndexPath, select::SelectItem};

// Generated from https://github.com/NvChad/base46 theme tables.
// ANSI color order follows lua/base46/term.lua from that project.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TerminalThemeId(&'static str);

impl TerminalThemeId {
    pub(crate) const fn new(id: &'static str) -> Self {
        Self(id)
    }

    pub(crate) fn palette(self) -> TerminalThemePalette {
        terminal_theme_by_id(self).palette()
    }

    pub(crate) fn as_str(self) -> &'static str {
        self.0
    }

    pub(crate) fn from_setting_value(value: &str, kind: TerminalThemeKind) -> Self {
        TERMINAL_THEMES
            .iter()
            .find(|theme| theme.id == value && theme.kind == kind)
            .map(|theme| TerminalThemeId::new(theme.id))
            .unwrap_or_else(|| TerminalThemeId::new(default_terminal_theme(kind).id))
    }
}

pub(crate) const DEFAULT_DARK_TERMINAL_THEME_ID: TerminalThemeId =
    TerminalThemeId::new("default-dark");
pub(crate) const DEFAULT_LIGHT_TERMINAL_THEME_ID: TerminalThemeId =
    TerminalThemeId::new("default-light");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerminalThemeKind {
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TerminalTheme {
    id: &'static str,
    name: &'static str,
    kind: TerminalThemeKind,
    background: u32,
    foreground: u32,
    cursor: u32,
    ansi: [u32; 16],
}

impl TerminalTheme {
    pub(crate) fn palette(self) -> TerminalThemePalette {
        TerminalThemePalette {
            background: self.background,
            foreground: self.foreground,
            cursor: self.cursor,
            ansi: self.ansi,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TerminalThemePalette {
    pub(crate) background: u32,
    pub(crate) foreground: u32,
    pub(crate) cursor: u32,
    pub(crate) ansi: [u32; 16],
}

#[derive(Clone)]
pub(crate) struct TerminalThemeChoice {
    id: TerminalThemeId,
    title: SharedString,
}

impl TerminalThemeChoice {
    fn new(theme: &TerminalTheme) -> Self {
        Self {
            id: TerminalThemeId::new(theme.id),
            title: theme.name.into(),
        }
    }
}

impl SelectItem for TerminalThemeChoice {
    type Value = TerminalThemeId;

    fn title(&self) -> SharedString {
        self.title.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

pub(crate) fn terminal_theme_options(kind: TerminalThemeKind) -> Vec<TerminalThemeChoice> {
    TERMINAL_THEMES
        .iter()
        .filter(|theme| theme.kind == kind)
        .map(TerminalThemeChoice::new)
        .collect()
}

pub(crate) fn terminal_theme_selected_index(
    id: TerminalThemeId,
    kind: TerminalThemeKind,
) -> IndexPath {
    let row = TERMINAL_THEMES
        .iter()
        .filter(|theme| theme.kind == kind)
        .position(|theme| theme.id == id.0)
        .unwrap_or_else(|| default_terminal_theme_index(kind));

    IndexPath::default().row(row)
}

pub(crate) fn terminal_theme_by_id(id: TerminalThemeId) -> &'static TerminalTheme {
    TERMINAL_THEMES
        .iter()
        .find(|theme| theme.id == id.0)
        .unwrap_or_else(|| default_terminal_theme(TerminalThemeKind::Dark))
}

fn default_terminal_theme_index(kind: TerminalThemeKind) -> usize {
    let default_id = match kind {
        TerminalThemeKind::Dark => DEFAULT_DARK_TERMINAL_THEME_ID,
        TerminalThemeKind::Light => DEFAULT_LIGHT_TERMINAL_THEME_ID,
    };

    TERMINAL_THEMES
        .iter()
        .filter(|theme| theme.kind == kind)
        .position(|theme| theme.id == default_id.0)
        .unwrap_or(0)
}

fn default_terminal_theme(kind: TerminalThemeKind) -> &'static TerminalTheme {
    let default_id = match kind {
        TerminalThemeKind::Dark => DEFAULT_DARK_TERMINAL_THEME_ID,
        TerminalThemeKind::Light => DEFAULT_LIGHT_TERMINAL_THEME_ID,
    };

    TERMINAL_THEMES
        .iter()
        .find(|theme| theme.id == default_id.0)
        .unwrap_or(&TERMINAL_THEMES[0])
}

pub(crate) const TERMINAL_THEMES: [TerminalTheme; 94] = [
    TerminalTheme {
        id: "aquarium",
        name: "Aquarium",
        kind: TerminalThemeKind::Dark,
        background: 0x20202a,
        foreground: 0xbac0cb,
        cursor: 0xced4df,
        ansi: [
            0x2c2e3e, 0xebb9b9, 0xb1dba4, 0xe6dfb8, 0xa3b8ef, 0xf6bbe7, 0xb8dceb, 0xbac0cb,
            0x313449, 0xebb9b9, 0xb1dba4, 0xe6dfb8, 0xa3b8ef, 0xf6bbe7, 0xb8dceb, 0xced4df,
        ],
    },
    TerminalTheme {
        id: "ashes",
        name: "Ashes",
        kind: TerminalThemeKind::Dark,
        background: 0x1c2023,
        foreground: 0xc7ccd1,
        cursor: 0xc7ccd1,
        ansi: [
            0x272b2e, 0xc7ae95, 0x95c7ae, 0xaec795, 0xae95c7, 0xc795ae, 0x95aec7, 0xc7ccd1,
            0x44484b, 0xc7ae95, 0x95c7ae, 0xaec795, 0xae95c7, 0xc795ae, 0x95aec7, 0xf3f4f5,
        ],
    },
    TerminalTheme {
        id: "aylin",
        name: "Aylin",
        kind: TerminalThemeKind::Dark,
        background: 0x24262e,
        foreground: 0xebefff,
        cursor: 0xffffff,
        ansi: [
            0x2a2d36, 0xfd98b9, 0xecc48d, 0xacafb9, 0x7fdbca, 0x9fd4ff, 0x9fd4ff, 0xebefff,
            0x363b46, 0xfd98b9, 0xecc48d, 0xacafb9, 0x7fdbca, 0x9fd4ff, 0x9fd4ff, 0xabbbff,
        ],
    },
    TerminalTheme {
        id: "ayu_dark",
        name: "Ayu Dark",
        kind: TerminalThemeKind::Dark,
        background: 0x0b0e14,
        foreground: 0xc9c7be,
        cursor: 0xced4df,
        ansi: [
            0x1c1f25, 0xc9c7be, 0xaad84c, 0x56c3f9, 0xf07174, 0xffb454, 0xffb454, 0xc9c7be,
            0x2b2e34, 0xc9c7be, 0xaad84c, 0x56c3f9, 0xf07174, 0xffb454, 0xffb454, 0xd9d7ce,
        ],
    },
    TerminalTheme {
        id: "ayu_light",
        name: "Ayu Light",
        kind: TerminalThemeKind::Light,
        background: 0xfafafa,
        foreground: 0x5c6166,
        cursor: 0x26292f,
        ansi: [
            0xf0f0f0, 0xf07171, 0x86b300, 0x399ee6, 0x55b4d4, 0xfa8d3e, 0x4cbf99, 0x5c6166,
            0xdfdfdf, 0xf07171, 0x86b300, 0x399ee6, 0x55b4d4, 0xfa8d3e, 0x4cbf99, 0x484d52,
        ],
    },
    TerminalTheme {
        id: "bearded-arc",
        name: "Bearded Arc",
        kind: TerminalThemeKind::Dark,
        background: 0x1c2433,
        foreground: 0xc3cfd9,
        cursor: 0xabb7c1,
        ansi: [
            0x262e3d, 0xff738a, 0x3cec85, 0xeacd61, 0x69c3ff, 0x22ecdb, 0x77aed7, 0xc3cfd9,
            0x444c5b, 0xff738a, 0x3cec85, 0xeacd61, 0x69c3ff, 0x22ecdb, 0x77aed7, 0x08bdba,
        ],
    },
    TerminalTheme {
        id: "blossom_light",
        name: "Blossom Light",
        kind: TerminalThemeKind::Light,
        background: 0xe6dfdc,
        foreground: 0x746862,
        cursor: 0x695d57,
        ansi: [
            0xded7d4, 0x8779a8, 0x6c805c, 0x738199, 0xb3816a, 0x7e8e8e, 0x5e908e, 0x746862,
            0xd1cac7, 0x8779a8, 0x6c805c, 0x738199, 0xb3816a, 0x7e8e8e, 0x5e908e, 0x695d57,
        ],
    },
    TerminalTheme {
        id: "carbonfox",
        name: "Carbonfox",
        kind: TerminalThemeKind::Dark,
        background: 0x161616,
        foreground: 0xf2f4f8,
        cursor: 0xdfdfe0,
        ansi: [
            0x282828, 0x78a9ff, 0x25be6a, 0x3ddbd9, 0xff7eb6, 0xbe95ff, 0x33b1ff, 0xf2f4f8,
            0x3b3b3b, 0x78a9ff, 0x25be6a, 0x3ddbd9, 0xff7eb6, 0xbe95ff, 0x33b1ff, 0xffffff,
        ],
    },
    TerminalTheme {
        id: "catppuccin-latte",
        name: "Catppuccin Latte",
        kind: TerminalThemeKind::Light,
        background: 0xeff1f5,
        foreground: 0x4c4f69,
        cursor: 0x7287fd,
        ansi: [
            0xe5e8ef, 0xe64553, 0x40a02b, 0xdf8e1d, 0x1e66f5, 0x8839ef, 0x04a5e5, 0x4c4f69,
            0xc3c7d3, 0xe64553, 0x40a02b, 0xdf8e1d, 0x1e66f5, 0x8839ef, 0x04a5e5, 0x6c6f85,
        ],
    },
    TerminalTheme {
        id: "catppuccin",
        name: "Catppuccin",
        kind: TerminalThemeKind::Dark,
        background: 0x1e1d2d,
        foreground: 0xbfc6d4,
        cursor: 0xd9e0ee,
        ansi: [
            0x282737, 0xf38ba8, 0xabe9b3, 0xfae3b0, 0x89b4fa, 0xcba6f7, 0x89dceb, 0xbfc6d4,
            0x383747, 0xf38ba8, 0xabe9b3, 0xfae3b0, 0x89b4fa, 0xcba6f7, 0x89dceb, 0xd9e0ee,
        ],
    },
    TerminalTheme {
        id: "chadracula-evondev",
        name: "Chadracula Evondev",
        kind: TerminalThemeKind::Dark,
        background: 0x141423,
        foreground: 0xe9e9f4,
        cursor: 0xf8f8f2,
        ansi: [
            0x23233d, 0xc197fd, 0xe5c697, 0x62d6e8, 0x20e3b2, 0xff6bcb, 0x8be9fd, 0xe9e9f4,
            0x373760, 0xc197fd, 0xe5c697, 0x62d6e8, 0x20e3b2, 0xff6bcb, 0x8be9fd, 0xf7f7fb,
        ],
    },
    TerminalTheme {
        id: "chadracula",
        name: "Chadracula",
        kind: TerminalThemeKind::Dark,
        background: 0x282a36,
        foreground: 0xe9e9f4,
        cursor: 0xf8f8f2,
        ansi: [
            0x3a3c4e, 0xc197fd, 0xf1fa8c, 0x62d6e8, 0x50fa7b, 0xff86d3, 0x8be9fd, 0xe9e9f4,
            0x626483, 0xc197fd, 0xf1fa8c, 0x62d6e8, 0x50fa7b, 0xff86d3, 0x8be9fd, 0xf7f7fb,
        ],
    },
    TerminalTheme {
        id: "chadtain",
        name: "Chadtain",
        kind: TerminalThemeKind::Dark,
        background: 0x1a2026,
        foreground: 0xbebebe,
        cursor: 0xb0b0b0,
        ansi: [
            0x242a30, 0xac8a8c, 0x8aac8b, 0xaca98a, 0x7797b7, 0x948fb1, 0x8aabac, 0xbebebe,
            0x2e343a, 0xac8a8c, 0x8aac8b, 0xaca98a, 0x7797b7, 0x948fb1, 0x8aabac, 0xb0b0b0,
        ],
    },
    TerminalTheme {
        id: "chocolate",
        name: "Chocolate",
        kind: TerminalThemeKind::Dark,
        background: 0x252221,
        foreground: 0xc8baa4,
        cursor: 0xcdc0ad,
        ansi: [
            0x2b2827, 0xc65f5f, 0x8ca589, 0xd9b27c, 0x7d92a2, 0xc65f5f, 0x998396, 0xc8baa4,
            0x393635, 0xc65f5f, 0x8ca589, 0xd9b27c, 0x7d92a2, 0xc65f5f, 0x998396, 0xcdc0ad,
        ],
    },
    TerminalTheme {
        id: "darcula-dark",
        name: "Darcula Dark",
        kind: TerminalThemeKind::Dark,
        background: 0x2b2b2b,
        foreground: 0xabb2bf,
        cursor: 0xeeeeee,
        ansi: [
            0x393939, 0xc9d0d3, 0x6a8759, 0xdc9656, 0xad9e7d, 0xd3b987, 0xd3b987, 0xabb2bf,
            0x474747, 0xc9d0d3, 0x6a8759, 0xdc9656, 0xad9e7d, 0xd3b987, 0xd3b987, 0x99a2b1,
        ],
    },
    TerminalTheme {
        id: "dark_horizon",
        name: "Dark Horizon",
        kind: TerminalThemeKind::Dark,
        background: 0x0e0e0e,
        foreground: 0xc9c7be,
        cursor: 0xffffff,
        ansi: [
            0x181818, 0xdb627e, 0xe3a587, 0x169ac9, 0x32d5e3, 0x6be4e6, 0xf09483, 0xc9c7be,
            0x363636, 0xdb627e, 0xe3a587, 0x169ac9, 0x32d5e3, 0x6be4e6, 0xf09483, 0xd9d7ce,
        ],
    },
    TerminalTheme {
        id: "decay",
        name: "Decay",
        kind: TerminalThemeKind::Dark,
        background: 0x171b20,
        foreground: 0xb6beca,
        cursor: 0xdee1e6,
        ansi: [
            0x21262e, 0x70a5eb, 0x78dba9, 0xf1cf8a, 0x86aaec, 0xc68aee, 0xe26c7c, 0xb6beca,
            0x485263, 0x70a5eb, 0x78dba9, 0xf1cf8a, 0x86aaec, 0xc68aee, 0xe26c7c, 0xdee1e6,
        ],
    },
    TerminalTheme {
        id: "default-dark",
        name: "Default Dark",
        kind: TerminalThemeKind::Dark,
        background: 0x181818,
        foreground: 0xd8d8d8,
        cursor: 0xf8f8f8,
        ansi: [
            0x282828, 0xab4642, 0xa1b56c, 0xf7ca88, 0x7cafc2, 0xba8baf, 0x86c1b9, 0xd8d8d8,
            0x585858, 0xab4642, 0xa1b56c, 0xf7ca88, 0x7cafc2, 0xba8baf, 0x86c1b9, 0xf8f8f8,
        ],
    },
    TerminalTheme {
        id: "default-light",
        name: "Default Light",
        kind: TerminalThemeKind::Light,
        background: 0xf8f8f8,
        foreground: 0x383838,
        cursor: 0x181818,
        ansi: [
            0xe8e8e8, 0xab4642, 0x9aaf61, 0xf1a02e, 0x71a8bd, 0xb481a8, 0x7bbbb3, 0x383838,
            0xb8b8b8, 0xab4642, 0x9aaf61, 0xf1a02e, 0x71a8bd, 0xb481a8, 0x7bbbb3, 0x181818,
        ],
    },
    TerminalTheme {
        id: "doomchad",
        name: "Doomchad",
        kind: TerminalThemeKind::Dark,
        background: 0x282c34,
        foreground: 0xa7aebb,
        cursor: 0xbbc2cf,
        ansi: [
            0x32363e, 0xff6c6b, 0x98be65, 0xecbe7b, 0xdc8ef3, 0x48a6e6, 0x66c4ff, 0xa7aebb,
            0x4e525a, 0xff6c6b, 0x98be65, 0xecbe7b, 0xdc8ef3, 0x48a6e6, 0x66c4ff, 0xbbc2cf,
        ],
    },
    TerminalTheme {
        id: "eldritch",
        name: "Eldritch",
        kind: TerminalThemeKind::Dark,
        background: 0x171928,
        foreground: 0xabb4da,
        cursor: 0xebfafa,
        ansi: [
            0x21253a, 0xf16c75, 0xf1fc79, 0xf7c67f, 0x7081d0, 0xa48cf2, 0x04d1f9, 0xabb4da,
            0x3b4261, 0xf16c75, 0xf1fc79, 0xf7c67f, 0x7081d0, 0xa48cf2, 0x04d1f9, 0xffffff,
        ],
    },
    TerminalTheme {
        id: "embark",
        name: "Embark",
        kind: TerminalThemeKind::Dark,
        background: 0x1e1c31,
        foreground: 0xcbe3e7,
        cursor: 0xcbe3e7,
        ansi: [
            0x282643, 0xa1efd3, 0xffe9aa, 0xa1efd3, 0x91ddff, 0xa1efd3, 0xaaffe4, 0xcbe3e7,
            0x3e3859, 0xa1efd3, 0xffe9aa, 0xa1efd3, 0x91ddff, 0xa1efd3, 0xaaffe4, 0xffffff,
        ],
    },
    TerminalTheme {
        id: "everblush",
        name: "Everblush",
        kind: TerminalThemeKind::Dark,
        background: 0x141b1e,
        foreground: 0xdadada,
        cursor: 0xdadada,
        ansi: [
            0x1e2528, 0xe57474, 0x8ccf7e, 0xe5c76b, 0x67b0e8, 0xc47fd5, 0x6cbfbf, 0xdadada,
            0x2d3437, 0xe57474, 0x8ccf7e, 0xe5c76b, 0x67b0e8, 0xc47fd5, 0x6cbfbf, 0xdadada,
        ],
    },
    TerminalTheme {
        id: "everforest",
        name: "Everforest",
        kind: TerminalThemeKind::Dark,
        background: 0x2b3339,
        foreground: 0xd3c6aa,
        cursor: 0xd3c6aa,
        ansi: [
            0x323c41, 0x7fbbb3, 0xdbbc7f, 0x83c092, 0xa7c080, 0xe67e80, 0xe69875, 0xd3c6aa,
            0x424a50, 0x7fbbb3, 0xdbbc7f, 0x83c092, 0xa7c080, 0xe67e80, 0xe69875, 0xe7dabe,
        ],
    },
    TerminalTheme {
        id: "everforest_light",
        name: "Everforest Light",
        kind: TerminalThemeKind::Light,
        background: 0xfff9e8,
        foreground: 0x495157,
        cursor: 0x272f35,
        ansi: [
            0xf6f0df, 0x5f9b93, 0xd59600, 0x8da101, 0x87a060, 0xc85552, 0xef615e, 0x495157,
            0xe5dfce, 0x5f9b93, 0xd59600, 0x8da101, 0x87a060, 0xc85552, 0xef615e, 0x272f35,
        ],
    },
    TerminalTheme {
        id: "falcon",
        name: "Falcon",
        kind: TerminalThemeKind::Dark,
        background: 0x020222,
        foreground: 0xeeeef5,
        cursor: 0xf8f8ff,
        ansi: [
            0x0b0b2b, 0xbfdaff, 0xc8d0e3, 0xffc552, 0xffc552, 0x8bccbf, 0xb4b4b9, 0xeeeef5,
            0x202040, 0xbfdaff, 0xc8d0e3, 0xffc552, 0xffc552, 0x8bccbf, 0xb4b4b9, 0xf8f8ff,
        ],
    },
    TerminalTheme {
        id: "flex-light",
        name: "Flex Light",
        kind: TerminalThemeKind::Light,
        background: 0xfffcf0,
        foreground: 0x2a2929,
        cursor: 0x2a2929,
        ansi: [
            0xf2efe4, 0xd14d41, 0x879a39, 0x8b7ec8, 0x4385be, 0xd0a215, 0x3aa99f, 0x2a2929,
            0xb8b5ad, 0xd14d41, 0x879a39, 0x8b7ec8, 0x4385be, 0xd0a215, 0x3aa99f, 0xc8ccd4,
        ],
    },
    TerminalTheme {
        id: "flexoki-light",
        name: "Flexoki Light",
        kind: TerminalThemeKind::Light,
        background: 0xfffcf0,
        foreground: 0x2a2929,
        cursor: 0x2a2929,
        ansi: [
            0xf2efe4, 0xaf3029, 0x66800b, 0x5e409d, 0x205ea6, 0xad8301, 0x24837b, 0x2a2929,
            0xb8b5ad, 0xaf3029, 0x66800b, 0x5e409d, 0x205ea6, 0xad8301, 0x24837b, 0xc8ccd4,
        ],
    },
    TerminalTheme {
        id: "flexoki",
        name: "Flexoki",
        kind: TerminalThemeKind::Dark,
        background: 0x100f0f,
        foreground: 0xcecdc3,
        cursor: 0xcecdc3,
        ansi: [
            0x1c1b1b, 0xd14d41, 0x879a39, 0x8b7ec8, 0x4385be, 0xd0a215, 0x3aa99f, 0xcecdc3,
            0x393636, 0xd14d41, 0x879a39, 0x8b7ec8, 0x4385be, 0xd0a215, 0x3aa99f, 0xc8ccd4,
        ],
    },
    TerminalTheme {
        id: "flouromachine",
        name: "Flouromachine",
        kind: TerminalThemeKind::Dark,
        background: 0x262335,
        foreground: 0x61e2ff,
        cursor: 0xffffff,
        ansi: [
            0x322f47, 0x61e2ff, 0xb77bf9, 0xfc199a, 0xffcc00, 0xfc199a, 0xff8b39, 0x61e2ff,
            0x4a476b, 0x61e2ff, 0xb77bf9, 0xfc199a, 0xffcc00, 0xfc199a, 0xff8b39, 0xffffff,
        ],
    },
    TerminalTheme {
        id: "gatekeeper",
        name: "Gatekeeper",
        kind: TerminalThemeKind::Dark,
        background: 0x101010,
        foreground: 0xd8d9dd,
        cursor: 0xcccdd1,
        ansi: [
            0x171717, 0xffb20f, 0x00e756, 0xbe620a, 0xc54bcf, 0xff4394, 0x29adff, 0xd8d9dd,
            0x252525, 0xffb20f, 0x00e756, 0xbe620a, 0xc54bcf, 0xff4394, 0x29adff, 0xcccdd1,
        ],
    },
    TerminalTheme {
        id: "github_dark",
        name: "GitHub Dark",
        kind: TerminalThemeKind::Dark,
        background: 0x24292e,
        foreground: 0xc9d1d9,
        cursor: 0xd3dbe3,
        ansi: [
            0x33383d, 0xb392e9, 0xa5d6ff, 0xffdf5d, 0x6ab1f0, 0xff7f8d, 0x83caff, 0xc9d1d9,
            0x42474c, 0xb392e9, 0xa5d6ff, 0xffdf5d, 0x6ab1f0, 0xff7f8d, 0x83caff, 0xdde5ed,
        ],
    },
    TerminalTheme {
        id: "github_light",
        name: "GitHub Light",
        kind: TerminalThemeKind::Light,
        background: 0xffffff,
        foreground: 0x383d42,
        cursor: 0x24292e,
        ansi: [
            0xedeff1, 0x5a32a3, 0x4c2889, 0xb08800, 0x005cc5, 0xde2c2e, 0x8263eb, 0x383d42,
            0xd7d9db, 0x5a32a3, 0x4c2889, 0xb08800, 0x005cc5, 0xde2c2e, 0x8263eb, 0x24292e,
        ],
    },
    TerminalTheme {
        id: "gruvbox",
        name: "Gruvbox",
        kind: TerminalThemeKind::Dark,
        background: 0x282828,
        foreground: 0xd5c4a1,
        cursor: 0xebdbb2,
        ansi: [
            0x3c3836, 0xfb4934, 0xb8bb26, 0xfabd2f, 0x83a598, 0xd3869b, 0x8ec07c, 0xd5c4a1,
            0x484442, 0xfb4934, 0xb8bb26, 0xfabd2f, 0x83a598, 0xd3869b, 0x8ec07c, 0xfbf1c7,
        ],
    },
    TerminalTheme {
        id: "gruvbox_light",
        name: "Gruvbox Light",
        kind: TerminalThemeKind::Light,
        background: 0xf2e5bc,
        foreground: 0x504945,
        cursor: 0x504945,
        ansi: [
            0xe3d6ad, 0x9d0006, 0x79740e, 0xb57614, 0x076678, 0x8f3f71, 0x427b58, 0x504945,
            0xd8cba2, 0x9d0006, 0x79740e, 0xb57614, 0x076678, 0x8f3f71, 0x427b58, 0x282828,
        ],
    },
    TerminalTheme {
        id: "gruvchad",
        name: "Gruvchad",
        kind: TerminalThemeKind::Dark,
        background: 0x1e2122,
        foreground: 0xc0b196,
        cursor: 0xc7b89d,
        ansi: [
            0x2c2f30, 0xec6b64, 0xa9b665, 0xe0c080, 0x7daea3, 0xd3869b, 0x86b17f, 0xc0b196,
            0x404344, 0xec6b64, 0xa9b665, 0xe0c080, 0x7daea3, 0xd3869b, 0x86b17f, 0xc7b89d,
        ],
    },
    TerminalTheme {
        id: "hiberbee",
        name: "Hiberbee",
        kind: TerminalThemeKind::Dark,
        background: 0x121110,
        foreground: 0xbbbab9,
        cursor: 0xfffefd,
        ansi: [
            0x2a2625, 0xf25022, 0xffb900, 0x7fdbca, 0x7fdbca, 0xee7762, 0x409cff, 0xbbbab9,
            0x3a3433, 0xf25022, 0xffb900, 0x7fdbca, 0x7fdbca, 0xee7762, 0x409cff, 0xdfdedd,
        ],
    },
    TerminalTheme {
        id: "horizon",
        name: "Horizon",
        kind: TerminalThemeKind::Dark,
        background: 0x1d1f27,
        foreground: 0xd5d8da,
        cursor: 0xd5d8da,
        ansi: [
            0x4b4c53, 0xe95678, 0x21bfc2, 0xfabd2f, 0x59c2ff, 0xb877db, 0xb877db, 0xd5d8da,
            0x4b4c53, 0xe95678, 0x21bfc2, 0xfabd2f, 0x59c2ff, 0xb877db, 0xb877db, 0x6c6f93,
        ],
    },
    TerminalTheme {
        id: "jabuti",
        name: "Jabuti",
        kind: TerminalThemeKind::Dark,
        background: 0x292a37,
        foreground: 0xc0cbe3,
        cursor: 0xd9e0ee,
        ansi: [
            0x343545, 0xec6a88, 0x3fdaa4, 0xe1c697, 0x3fc6de, 0xbe95ff, 0xff7eb6, 0xc0cbe3,
            0x45475d, 0xec6a88, 0x3fdaa4, 0xe1c697, 0x3fc6de, 0xbe95ff, 0xff7eb6, 0xffffff,
        ],
    },
    TerminalTheme {
        id: "jellybeans",
        name: "Jellybeans",
        kind: TerminalThemeKind::Dark,
        background: 0x151515,
        foreground: 0xd9d9c4,
        cursor: 0xe8e8d3,
        ansi: [
            0x2e2e2e, 0xc6b5da, 0x99ad6a, 0xe1b655, 0x8fa5cd, 0xe18be1, 0x99ad6a, 0xd9d9c4,
            0x424242, 0xc6b5da, 0x99ad6a, 0xe1b655, 0x8fa5cd, 0xe18be1, 0x99ad6a, 0xf1f1e5,
        ],
    },
    TerminalTheme {
        id: "kanagawa-dragon",
        name: "Kanagawa Dragon",
        kind: TerminalThemeKind::Dark,
        background: 0x181616,
        foreground: 0xadada4,
        cursor: 0xadada4,
        ansi: [
            0x1f1d1d, 0xc4b28a, 0x87a987, 0x8ea4a2, 0x8ba4b0, 0xa292a3, 0x8ea4a2, 0xadada4,
            0x2d2b2b, 0xc4b28a, 0x87a987, 0x8ea4a2, 0x8ba4b0, 0xa292a3, 0x8ea4a2, 0x737c73,
        ],
    },
    TerminalTheme {
        id: "kanagawa",
        name: "Kanagawa",
        kind: TerminalThemeKind::Dark,
        background: 0x1f1f28,
        foreground: 0xc8c3a6,
        cursor: 0xdcd7ba,
        ansi: [
            0x2a2a37, 0xd8616b, 0x98bb6c, 0xdca561, 0x7e9cd8, 0x9c86bf, 0x7fb4ca, 0xc8c3a6,
            0x363646, 0xd8616b, 0x98bb6c, 0xdca561, 0x7e9cd8, 0x9c86bf, 0x7fb4ca, 0xdcd7ba,
        ],
    },
    TerminalTheme {
        id: "material-darker",
        name: "Material Darker",
        kind: TerminalThemeKind::Dark,
        background: 0x212121,
        foreground: 0xdff0f0,
        cursor: 0xeeffff,
        ansi: [
            0x292929, 0xb0bec5, 0xc3e88d, 0xffcb6b, 0x82aaff, 0xc792ea, 0xc3e88d, 0xdff0f0,
            0x383838, 0xb0bec5, 0xc3e88d, 0xffcb6b, 0x82aaff, 0xc792ea, 0xc3e88d, 0xeeffff,
        ],
    },
    TerminalTheme {
        id: "material-deep-ocean",
        name: "Material Deep Ocean",
        kind: TerminalThemeKind::Dark,
        background: 0x0f111a,
        foreground: 0xeeffff,
        cursor: 0xeeffff,
        ansi: [
            0x23293e, 0xf07178, 0xc3e88d, 0xffcb6b, 0x82aaff, 0xc792ea, 0x89ddff, 0xeeffff,
            0x374162, 0xf07178, 0xc3e88d, 0xffcb6b, 0x82aaff, 0xc792ea, 0x89ddff, 0xb5b8c1,
        ],
    },
    TerminalTheme {
        id: "material-lighter",
        name: "Material Lighter",
        kind: TerminalThemeKind::Light,
        background: 0xfafafa,
        foreground: 0x435862,
        cursor: 0x435862,
        ansi: [
            0xeeeeee, 0xf59717, 0x91b859, 0x00bcd4, 0x6182b8, 0x7c4dff, 0x39adb5, 0x435862,
            0xd2d4d5, 0xf59717, 0x91b859, 0x00bcd4, 0x6182b8, 0x7c4dff, 0x39adb5, 0x546e7a,
        ],
    },
    TerminalTheme {
        id: "melange",
        name: "Melange",
        kind: TerminalThemeKind::Dark,
        background: 0x2a2520,
        foreground: 0xece1d7,
        cursor: 0xece1d7,
        ansi: [
            0x39342f, 0xece1d7, 0x9aacce, 0x99d59d, 0xebc06d, 0xe49b5d, 0xebc06d, 0xece1d7,
            0x4d4843, 0xece1d7, 0x9aacce, 0x99d59d, 0xebc06d, 0xe49b5d, 0xebc06d, 0xd8cdc3,
        ],
    },
    TerminalTheme {
        id: "midnight_breeze",
        name: "Midnight Breeze",
        kind: TerminalThemeKind::Dark,
        background: 0x0d1117,
        foreground: 0xc9d1d9,
        cursor: 0xc9d1d9,
        ansi: [
            0x161b22, 0xfb6f6f, 0x56d364, 0xffdf5d, 0x58a6ff, 0xbc8cff, 0x39c5cf, 0xc9d1d9,
            0x313641, 0xfb6f6f, 0x56d364, 0xffdf5d, 0x58a6ff, 0xbc8cff, 0x39c5cf, 0xdde5ed,
        ],
    },
    TerminalTheme {
        id: "mito-laser",
        name: "Mito Laser",
        kind: TerminalThemeKind::Dark,
        background: 0x201947,
        foreground: 0xeee8d5,
        cursor: 0xeee8d5,
        ansi: [
            0x271e56, 0xff047d, 0x859900, 0xb58900, 0x268bd2, 0x6c71c4, 0x2aa198, 0xeee8d5,
            0x352975, 0xff047d, 0x859900, 0xb58900, 0x268bd2, 0x6c71c4, 0x2aa198, 0xfdf6e3,
        ],
    },
    TerminalTheme {
        id: "monekai",
        name: "Monekai",
        kind: TerminalThemeKind::Dark,
        background: 0x272822,
        foreground: 0xf8f8f2,
        cursor: 0xf5f4f1,
        ansi: [
            0x383830, 0xfd971f, 0xa6e22e, 0xf4bf75, 0x66d9ef, 0xf92672, 0xa1efe4, 0xf8f8f2,
            0x75715e, 0xfd971f, 0xa6e22e, 0xf4bf75, 0x66d9ef, 0xf92672, 0xa1efe4, 0xf9f8f5,
        ],
    },
    TerminalTheme {
        id: "monochrome",
        name: "Monochrome",
        kind: TerminalThemeKind::Dark,
        background: 0x101010,
        foreground: 0xbfc5d0,
        cursor: 0xd8dee9,
        ansi: [
            0x1f1f1f, 0xeee8d5, 0x7b9198, 0x859ba2, 0xced4df, 0xdad4c3, 0xdfdfda, 0xbfc5d0,
            0x383838, 0xeee8d5, 0x7b9198, 0x859ba2, 0xced4df, 0xdad4c3, 0xdfdfda, 0xced4df,
        ],
    },
    TerminalTheme {
        id: "mountain",
        name: "Mountain",
        kind: TerminalThemeKind::Dark,
        background: 0x0f0f0f,
        foreground: 0xd8d8d8,
        cursor: 0xf0f0f0,
        ansi: [
            0x151515, 0xb18f91, 0x8aac8b, 0xb1ae8f, 0xa5a0c2, 0xac8aac, 0x91b2b3, 0xd8d8d8,
            0x222222, 0xb18f91, 0x8aac8b, 0xb1ae8f, 0xa5a0c2, 0xac8aac, 0x91b2b3, 0xf0f0f0,
        ],
    },
    TerminalTheme {
        id: "nano-light",
        name: "Nano Light",
        kind: TerminalThemeKind::Light,
        background: 0xffffff,
        foreground: 0x37474f,
        cursor: 0x37474f,
        ansi: [
            0xeceff1, 0x673ab7, 0x8497a0, 0x673ab7, 0x263238, 0x37474f, 0x673ab7, 0x37474f,
            0xc4c4c4, 0x673ab7, 0x8497a0, 0x673ab7, 0x263238, 0x37474f, 0x673ab7, 0x263238,
        ],
    },
    TerminalTheme {
        id: "neofusion",
        name: "Neofusion",
        kind: TerminalThemeKind::Dark,
        background: 0x06101e,
        foreground: 0xe8e5b5,
        cursor: 0x66def9,
        ansi: [
            0x0a1c36, 0x66def9, 0x01eca7, 0xfd5e3a, 0x35b5ff, 0x66def9, 0xfd5e3a, 0xe8e5b5,
            0x102e5a, 0x66def9, 0x01eca7, 0xfd5e3a, 0x35b5ff, 0x66def9, 0xfd5e3a, 0x66def9,
        ],
    },
    TerminalTheme {
        id: "nightfox",
        name: "Nightfox",
        kind: TerminalThemeKind::Dark,
        background: 0x192330,
        foreground: 0xc0c8d5,
        cursor: 0xcdcecf,
        ansi: [
            0x252f3c, 0xe26886, 0x8ebaa4, 0xdbc074, 0x86abdc, 0x9d79d6, 0x7ad4d6, 0xc0c8d5,
            0x3d4754, 0xe26886, 0x8ebaa4, 0xdbc074, 0x86abdc, 0x9d79d6, 0x7ad4d6, 0xced6e3,
        ],
    },
    TerminalTheme {
        id: "nightlamp",
        name: "Nightlamp",
        kind: TerminalThemeKind::Dark,
        background: 0x18191f,
        foreground: 0xb8af9e,
        cursor: 0xe0d6bd,
        ansi: [
            0x222329, 0xb8aad9, 0x8aa387, 0xccb89c, 0xb58385, 0x8e9cb4, 0x7aacaa, 0xb8af9e,
            0x3c3d43, 0xb8aad9, 0x8aa387, 0xccb89c, 0xb58385, 0x8e9cb4, 0x7aacaa, 0xe0d6bd,
        ],
    },
    TerminalTheme {
        id: "nightowl",
        name: "Nightowl",
        kind: TerminalThemeKind::Dark,
        background: 0x011627,
        foreground: 0xced6e3,
        cursor: 0xd6deeb,
        ansi: [
            0x0c2132, 0xecc48d, 0x29e68e, 0xc792ea, 0x82aaff, 0xc792ea, 0xaad2ff, 0xced6e3,
            0x223748, 0xecc48d, 0x29e68e, 0xc792ea, 0x82aaff, 0xc792ea, 0xaad2ff, 0xfeffff,
        ],
    },
    TerminalTheme {
        id: "nord",
        name: "Nord",
        kind: TerminalThemeKind::Dark,
        background: 0x2e3440,
        foreground: 0xe5e9f0,
        cursor: 0xabb2bf,
        ansi: [
            0x3b4252, 0x88c0d0, 0xa3be8c, 0x88c0d0, 0x81a1c1, 0x81a1c1, 0x81a1c1, 0xe5e9f0,
            0x4c566a, 0x88c0d0, 0xa3be8c, 0x88c0d0, 0x81a1c1, 0x81a1c1, 0x81a1c1, 0x8fbcbb,
        ],
    },
    TerminalTheme {
        id: "obsidian-ember",
        name: "Obsidian Ember",
        kind: TerminalThemeKind::Dark,
        background: 0x1e1e1e,
        foreground: 0xaaaaaa,
        cursor: 0xd3d3d3,
        ansi: [
            0x2c2c2c, 0xff8548, 0x848484, 0xff8548, 0xeeeeee, 0xffffff, 0xeeeeee, 0xaaaaaa,
            0x3a3a3a, 0xff8548, 0x848484, 0xff8548, 0xeeeeee, 0xffffff, 0xeeeeee, 0x8fbcbb,
        ],
    },
    TerminalTheme {
        id: "oceanic-light",
        name: "Oceanic Light",
        kind: TerminalThemeKind::Light,
        background: 0xd8dee9,
        foreground: 0x343d46,
        cursor: 0x26292f,
        ansi: [
            0xcdd3de, 0xb40b11, 0x869235, 0xa48c32, 0x526f93, 0x896a98, 0x5b9c90, 0x343d46,
            0xa7adba, 0xb40b11, 0x869235, 0xa48c32, 0x526f93, 0x896a98, 0x5b9c90, 0x1b2b34,
        ],
    },
    TerminalTheme {
        id: "oceanic-next",
        name: "Oceanic Next",
        kind: TerminalThemeKind::Dark,
        background: 0x1b2b34,
        foreground: 0xc0c5ce,
        cursor: 0xd8dee9,
        ansi: [
            0x343d46, 0x6cbdbc, 0x99c794, 0xf99157, 0x6699cc, 0xc594c5, 0x5aaeae, 0xc0c5ce,
            0x65737e, 0x6cbdbc, 0x99c794, 0xf99157, 0x6699cc, 0xc594c5, 0x5aaeae, 0xd8dee9,
        ],
    },
    TerminalTheme {
        id: "one_light",
        name: "One Light",
        kind: TerminalThemeKind::Light,
        background: 0xfafafa,
        foreground: 0x383a42,
        cursor: 0x54555b,
        ansi: [
            0xf4f4f4, 0xd84a3d, 0x50a14f, 0xc18401, 0x4078f2, 0xa626a4, 0x0070a8, 0x383a42,
            0xdfdfe0, 0xd84a3d, 0x50a14f, 0xc18401, 0x4078f2, 0xa626a4, 0x0070a8, 0x090a0b,
        ],
    },
    TerminalTheme {
        id: "onedark",
        name: "Onedark",
        kind: TerminalThemeKind::Dark,
        background: 0x1e222a,
        foreground: 0xabb2bf,
        cursor: 0xabb2bf,
        ansi: [
            0x353b45, 0xe06c75, 0x98c379, 0xe5c07b, 0x61afef, 0xc678dd, 0x56b6c2, 0xabb2bf,
            0x545862, 0xe06c75, 0x98c379, 0xe5c07b, 0x61afef, 0xc678dd, 0x56b6c2, 0xc8ccd4,
        ],
    },
    TerminalTheme {
        id: "onenord",
        name: "Onenord",
        kind: TerminalThemeKind::Dark,
        background: 0x2a303c,
        foreground: 0xbfc5d0,
        cursor: 0xd8dee9,
        ansi: [
            0x3b4252, 0xd57780, 0xa3be8c, 0xebcb8b, 0x81a1c1, 0xb48ead, 0x97b7d7, 0xbfc5d0,
            0x4c566a, 0xd57780, 0xa3be8c, 0xebcb8b, 0x81a1c1, 0xb48ead, 0x97b7d7, 0xced4df,
        ],
    },
    TerminalTheme {
        id: "onenord_light",
        name: "Onenord Light",
        kind: TerminalThemeKind::Light,
        background: 0xd8dee9,
        foreground: 0x3e4450,
        cursor: 0x2a303c,
        ansi: [
            0xf4f4f4, 0xa3454e, 0x75905e, 0xb88339, 0x3f5f7f, 0x8d6786, 0x5b7b9b, 0x3e4450,
            0xdfdfe0, 0xa3454e, 0x75905e, 0xb88339, 0x3f5f7f, 0x8d6786, 0x5b7b9b, 0x2a303c,
        ],
    },
    TerminalTheme {
        id: "oxocarbon",
        name: "Oxocarbon",
        kind: TerminalThemeKind::Dark,
        background: 0x161616,
        foreground: 0xf2f4f8,
        cursor: 0xf2f4f8,
        ansi: [
            0x262626, 0x3ddbd9, 0x33b1ff, 0xee5396, 0x42be65, 0xbe95ff, 0xff7eb6, 0xf2f4f8,
            0x525252, 0x3ddbd9, 0x33b1ff, 0xee5396, 0x42be65, 0xbe95ff, 0xff7eb6, 0x08bdba,
        ],
    },
    TerminalTheme {
        id: "palenight",
        name: "Palenight",
        kind: TerminalThemeKind::Dark,
        background: 0x292d3e,
        foreground: 0xd3d3d3,
        cursor: 0xffffff,
        ansi: [
            0x444267, 0xf07178, 0xc3e88d, 0xffcb6b, 0x82aaff, 0xc792ea, 0x89ddff, 0xd3d3d3,
            0x676e95, 0xf07178, 0xc3e88d, 0xffcb6b, 0x82aaff, 0xc792ea, 0x89ddff, 0xffffff,
        ],
    },
    TerminalTheme {
        id: "pastelDark",
        name: "PastelDark",
        kind: TerminalThemeKind::Dark,
        background: 0x131a21,
        foreground: 0xced4df,
        cursor: 0xb5bcc9,
        ansi: [
            0x2c333a, 0xef8891, 0x9ce5c0, 0xf5d595, 0xa3b8ef, 0xc2a2e3, 0xabb9e0, 0xced4df,
            0x40474e, 0xef8891, 0x9ce5c0, 0xf5d595, 0xa3b8ef, 0xc2a2e3, 0xabb9e0, 0xb5bcc9,
        ],
    },
    TerminalTheme {
        id: "pastelbeans",
        name: "Pastelbeans",
        kind: TerminalThemeKind::Dark,
        background: 0x151515,
        foreground: 0xd0d0d0,
        cursor: 0xe8e8d3,
        ansi: [
            0x202020, 0xffdab9, 0xd1f1a9, 0xebbbff, 0xbbdaff, 0xff9da4, 0xc0e9ff, 0xd0d0d0,
            0x505050, 0xffdab9, 0xd1f1a9, 0xebbbff, 0xbbdaff, 0xff9da4, 0xc0e9ff, 0xf5f5f5,
        ],
    },
    TerminalTheme {
        id: "penumbra_dark",
        name: "Penumbra Dark",
        kind: TerminalThemeKind::Dark,
        background: 0x303338,
        foreground: 0xcecece,
        cursor: 0xfffdfb,
        ansi: [
            0x3a3d42, 0x999999, 0x4ec093, 0xca7081, 0x7a9bec, 0xbe85d1, 0xd68b47, 0xcecece,
            0x484b50, 0x999999, 0x4ec093, 0xca7081, 0x7a9bec, 0xbe85d1, 0xd68b47, 0xfff7ed,
        ],
    },
    TerminalTheme {
        id: "penumbra_light",
        name: "Penumbra Light",
        kind: TerminalThemeKind::Light,
        background: 0xfff7ed,
        foreground: 0x636363,
        cursor: 0x3e4044,
        ansi: [
            0xfff7ed, 0xca7081, 0x3ea57b, 0xba823a, 0x4380bc, 0xac78bd, 0x22839b, 0x636363,
            0xcecece, 0xca7081, 0x3ea57b, 0xba823a, 0x4380bc, 0xac78bd, 0x22839b, 0x24272b,
        ],
    },
    TerminalTheme {
        id: "poimandres",
        name: "Poimandres",
        kind: TerminalThemeKind::Dark,
        background: 0x1b1e28,
        foreground: 0xa6accd,
        cursor: 0xe4f0fb,
        ansi: [
            0x2b3040, 0xa6accd, 0x5de4c7, 0x5de4c7, 0xadd7ff, 0x91b4d5, 0x89ddff, 0xa6accd,
            0x3b4258, 0xa6accd, 0x5de4c7, 0x5de4c7, 0xadd7ff, 0x91b4d5, 0x89ddff, 0xffffff,
        ],
    },
    TerminalTheme {
        id: "radium",
        name: "Radium",
        kind: TerminalThemeKind::Dark,
        background: 0x101317,
        foreground: 0xc5c5c6,
        cursor: 0xd4d4d5,
        ansi: [
            0x1a1d21, 0x37d99e, 0xe87979, 0xe5d487, 0x5fb0fc, 0xc397d8, 0x37d99e, 0xc5c5c6,
            0x2b2e32, 0x37d99e, 0xe87979, 0xe5d487, 0x5fb0fc, 0xc397d8, 0x37d99e, 0xd4d4d5,
        ],
    },
    TerminalTheme {
        id: "rosepine-dawn",
        name: "Rosepine Dawn",
        kind: TerminalThemeKind::Light,
        background: 0xfaf4ed,
        foreground: 0x575279,
        cursor: 0x575279,
        ansi: [
            0xfffaf3, 0xb4637a, 0x56949f, 0xd7827e, 0x907aa9, 0xea9d34, 0x286983, 0x575279,
            0x9893a5, 0xb4637a, 0x56949f, 0xd7827e, 0x907aa9, 0xea9d34, 0x286983, 0xdfdad9,
        ],
    },
    TerminalTheme {
        id: "rosepine",
        name: "Rosepine",
        kind: TerminalThemeKind::Dark,
        background: 0x191724,
        foreground: 0xe0def4,
        cursor: 0xe0def4,
        ansi: [
            0x1f1d2e, 0xeb6f92, 0x31748f, 0xebbcba, 0xc4a7e7, 0xf6c177, 0x9ccfd8, 0xe0def4,
            0x6e6a86, 0xeb6f92, 0x31748f, 0xebbcba, 0xc4a7e7, 0xf6c177, 0x9ccfd8, 0x524f67,
        ],
    },
    TerminalTheme {
        id: "rxyhn",
        name: "Rxyhn",
        kind: TerminalThemeKind::Dark,
        background: 0x061115,
        foreground: 0xd9d7d6,
        cursor: 0xd9d7d6,
        ansi: [
            0x0c171b, 0xf26e74, 0x82c29c, 0xe9967e, 0x79aaeb, 0xc488ec, 0x6791c9, 0xd9d7d6,
            0x192428, 0xf26e74, 0x82c29c, 0xe9967e, 0x79aaeb, 0xc488ec, 0x6791c9, 0xedebea,
        ],
    },
    TerminalTheme {
        id: "scaryforest",
        name: "Scaryforest",
        kind: TerminalThemeKind::Dark,
        background: 0x121f1d,
        foreground: 0xdde5e0,
        cursor: 0xdde5e0,
        ansi: [
            0x1d2b28, 0x9d6d6d, 0x83aa7c, 0xc0b283, 0x77beb4, 0x8c9f87, 0x7ebdae, 0xdde5e0,
            0x2e403b, 0x9d6d6d, 0x83aa7c, 0xc0b283, 0x77beb4, 0x8c9f87, 0x7ebdae, 0xecf4ef,
        ],
    },
    TerminalTheme {
        id: "seoul256_dark",
        name: "Seoul256 Dark",
        kind: TerminalThemeKind::Dark,
        background: 0x4a4a4a,
        foreground: 0xd8d8d8,
        cursor: 0xdfe0e0,
        ansi: [
            0x515151, 0xdf9a98, 0x97bb98, 0xe0bb71, 0x96bbdc, 0xdfbdbc, 0x97bcbc, 0xd8d8d8,
            0x5f5f5f, 0xdf9a98, 0x97bb98, 0xe0bb71, 0x96bbdc, 0xdfbdbc, 0x97bcbc, 0xdfe0e0,
        ],
    },
    TerminalTheme {
        id: "seoul256_light",
        name: "Seoul256 Light",
        kind: TerminalThemeKind::Light,
        background: 0xe0e0e0,
        foreground: 0x4e4e4e,
        cursor: 0x4e4e4e,
        ansi: [
            0xd0d0d0, 0x6a6a6a, 0x5f885f, 0xaf8760, 0x5f87ae, 0x875f87, 0x67a9aa, 0x4e4e4e,
            0xc0c0c0, 0x6a6a6a, 0x5f885f, 0xaf8760, 0x5f87ae, 0x875f87, 0x67a9aa, 0x5c5c5c,
        ],
    },
    TerminalTheme {
        id: "solarized_dark",
        name: "Solarized Dark",
        kind: TerminalThemeKind::Dark,
        background: 0x002b36,
        foreground: 0x93a1a1,
        cursor: 0xabb2bf,
        ansi: [
            0x06313c, 0xdc322f, 0x859900, 0xb58900, 0x268bd2, 0x6c71c4, 0x2aa198, 0x93a1a1,
            0x133e49, 0xdc322f, 0x859900, 0xb58900, 0x268bd2, 0x6c71c4, 0x2aa198, 0xfdf6e3,
        ],
    },
    TerminalTheme {
        id: "solarized_light",
        name: "Solarized Light",
        kind: TerminalThemeKind::Light,
        background: 0xfdf6e3,
        foreground: 0x586e75,
        cursor: 0x002b36,
        ansi: [
            0xeee8d5, 0xdc322f, 0x859900, 0xb58900, 0x268bd2, 0x6c71c4, 0x2aa198, 0x586e75,
            0x93a1a1, 0xdc322f, 0x859900, 0xb58900, 0x268bd2, 0x6c71c4, 0x2aa198, 0x002b36,
        ],
    },
    TerminalTheme {
        id: "solarized_osaka",
        name: "Solarized Osaka",
        kind: TerminalThemeKind::Dark,
        background: 0x011219,
        foreground: 0x9eabac,
        cursor: 0x9eabac,
        ansi: [
            0x022736, 0x268bd2, 0x29a298, 0xb28500, 0x268bd2, 0x849900, 0xc94c16, 0x9eabac,
            0x044a67, 0x268bd2, 0x29a298, 0xb28500, 0x268bd2, 0x849900, 0xc94c16, 0xfdf6e3,
        ],
    },
    TerminalTheme {
        id: "starlight",
        name: "Starlight",
        kind: TerminalThemeKind::Dark,
        background: 0x242424,
        foreground: 0xe6e6e6,
        cursor: 0xe6e6e6,
        ansi: [
            0x323232, 0xe3c401, 0x47b413, 0x13c299, 0x24acd4, 0xf2affd, 0xff4d51, 0xe6e6e6,
            0x474747, 0xe3c401, 0x47b413, 0x13c299, 0x24acd4, 0xf2affd, 0xff4d51, 0xffffff,
        ],
    },
    TerminalTheme {
        id: "sunrise_breeze",
        name: "Sunrise Breeze",
        kind: TerminalThemeKind::Light,
        background: 0xf5f5f5,
        foreground: 0x1b1f23,
        cursor: 0x1b1f23,
        ansi: [
            0xececec, 0xd64545, 0x238636, 0xbb8009, 0x0969da, 0x8250df, 0x2c9ab7, 0x1b1f23,
            0x9ea7b1, 0xd64545, 0x238636, 0xbb8009, 0x0969da, 0x8250df, 0x2c9ab7, 0x3b4045,
        ],
    },
    TerminalTheme {
        id: "sweetpastel",
        name: "Sweetpastel",
        kind: TerminalThemeKind::Dark,
        background: 0x1b1f23,
        foreground: 0xfde5e6,
        cursor: 0xffdede,
        ansi: [
            0x25292d, 0xe5a3a1, 0xb4e3ad, 0xece3b1, 0xa3cbe7, 0xceace8, 0xf8b3cc, 0xfde5e6,
            0x393d41, 0xe5a3a1, 0xb4e3ad, 0xece3b1, 0xa3cbe7, 0xceace8, 0xf8b3cc, 0xf8f9fa,
        ],
    },
    TerminalTheme {
        id: "tokyodark",
        name: "Tokyodark",
        kind: TerminalThemeKind::Dark,
        background: 0x11121d,
        foreground: 0xabb2bf,
        cursor: 0xa0a8cd,
        ansi: [
            0x1b1c27, 0xee6d85, 0xdfae67, 0x7199ee, 0x95c561, 0xa485dd, 0xa485dd, 0xabb2bf,
            0x282934, 0xee6d85, 0xdfae67, 0x7199ee, 0x95c561, 0xa485dd, 0xa485dd, 0xa0a8cd,
        ],
    },
    TerminalTheme {
        id: "tokyonight",
        name: "Tokyonight",
        kind: TerminalThemeKind::Dark,
        background: 0x1a1b26,
        foreground: 0xa9b1d6,
        cursor: 0xc0caf5,
        ansi: [
            0x16161e, 0x73daca, 0x9ece6a, 0x0db9d7, 0x2ac3de, 0xbb9af7, 0xb4f9f8, 0xa9b1d6,
            0x444b6a, 0x73daca, 0x9ece6a, 0x0db9d7, 0x2ac3de, 0xbb9af7, 0xb4f9f8, 0xd5d6db,
        ],
    },
    TerminalTheme {
        id: "tomorrow_night",
        name: "Tomorrow Night",
        kind: TerminalThemeKind::Dark,
        background: 0x1d1f21,
        foreground: 0xc5c8c6,
        cursor: 0xc5c8c2,
        ansi: [
            0x282a2e, 0xcc6666, 0xb5bd68, 0xf0c674, 0x81a2be, 0xb294bb, 0x8abeb7, 0xc5c8c6,
            0x969896, 0xcc6666, 0xb5bd68, 0xf0c674, 0x81a2be, 0xb294bb, 0x8abeb7, 0xffffff,
        ],
    },
    TerminalTheme {
        id: "tundra",
        name: "Tundra",
        kind: TerminalThemeKind::Dark,
        background: 0x111827,
        foreground: 0xf3f4f6,
        cursor: 0xffffff,
        ansi: [
            0x1e2534, 0xddd6fe, 0xb5e8b0, 0xfbc19d, 0xbae6fd, 0xfca5a5, 0xbae6fd, 0xf3f4f6,
            0x323948, 0xddd6fe, 0xb5e8b0, 0xfbc19d, 0xbae6fd, 0xfca5a5, 0xbae6fd, 0xd1d5db,
        ],
    },
    TerminalTheme {
        id: "vesper",
        name: "Vesper",
        kind: TerminalThemeKind::Dark,
        background: 0x101010,
        foreground: 0xffffff,
        cursor: 0xffffff,
        ansi: [
            0x1c1c1c, 0xffc799, 0x99ffe4, 0xfbadff, 0xffc799, 0xfbadff, 0x838383, 0xffffff,
            0x595959, 0xffc799, 0x99ffe4, 0xfbadff, 0xffc799, 0xfbadff, 0x838383, 0xffffff,
        ],
    },
    TerminalTheme {
        id: "vscode_dark",
        name: "VS Code Dark",
        kind: TerminalThemeKind::Dark,
        background: 0x1e1e1e,
        foreground: 0xd4d4d4,
        cursor: 0xdee1e6,
        ansi: [
            0x262626, 0xd16969, 0xbd8d78, 0xd7ba7d, 0xdcdcaa, 0xc586c0, 0x9cdcfe, 0xd4d4d4,
            0x3c3c3c, 0xd16969, 0xbd8d78, 0xd7ba7d, 0xdcdcaa, 0xc586c0, 0x9cdcfe, 0xffffff,
        ],
    },
    TerminalTheme {
        id: "vscode_light",
        name: "VS Code Light",
        kind: TerminalThemeKind::Light,
        background: 0xffffff,
        foreground: 0x343434,
        cursor: 0x343434,
        ansi: [
            0xefefef, 0x007acc, 0xc72e0f, 0xaf00db, 0x0000ff, 0x0064c1, 0x007acc, 0x343434,
            0xd7d7d7, 0x007acc, 0xc72e0f, 0xaf00db, 0x0000ff, 0x0064c1, 0x007acc, 0x424242,
        ],
    },
    TerminalTheme {
        id: "wombat",
        name: "Wombat",
        kind: TerminalThemeKind::Dark,
        background: 0x222222,
        foreground: 0xd6d2c9,
        cursor: 0xe4e0d7,
        ansi: [
            0x303030, 0xffcc66, 0xaee474, 0xefdeab, 0x88b8f6, 0xff8f7e, 0x7eb6bc, 0xd6d2c9,
            0x3e3e3e, 0xffcc66, 0xaee474, 0xefdeab, 0x88b8f6, 0xff8f7e, 0x7eb6bc, 0xe4e0d7,
        ],
    },
    TerminalTheme {
        id: "yoru",
        name: "Yoru",
        kind: TerminalThemeKind::Dark,
        background: 0x0c0e0f,
        foreground: 0xedeff0,
        cursor: 0xedeff0,
        ansi: [
            0x121415, 0xf26e74, 0x82c29c, 0xe79881, 0x709ad2, 0xc58cec, 0x6791c9, 0xedeff0,
            0x1f2122, 0xf26e74, 0x82c29c, 0xe79881, 0x709ad2, 0xc58cec, 0x6791c9, 0xf2f4f5,
        ],
    },
    TerminalTheme {
        id: "zenburn",
        name: "Zenburn",
        kind: TerminalThemeKind::Dark,
        background: 0x383838,
        foreground: 0xdcdccc,
        cursor: 0xffffff,
        ansi: [
            0x3f3f3f, 0xbc98ec, 0xca7b7b, 0xe0cf9f, 0x7cb8bb, 0xdc8cc3, 0xe0cf9f, 0xdcdccc,
            0x545454, 0xbc98ec, 0xca7b7b, 0xe0cf9f, 0x7cb8bb, 0xdc8cc3, 0xe0cf9f, 0xffffff,
        ],
    },
];
