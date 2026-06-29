use gpui::{IntoElement, prelude::*, px, rgb, svg};

pub(crate) mod add;
pub(crate) mod connect;
pub(crate) mod delete;
pub(crate) mod edit;
pub(crate) mod password_eye;
pub(crate) mod password_eye_off;
pub(crate) mod server;
pub(crate) mod settings;
pub(crate) mod sort_newest;
pub(crate) mod vault;

pub(crate) const GPUI_CHEVRON_DOWN_ICON_PATH: &str = "icons/chevron-down.svg";
pub(crate) const GPUI_EYE_ICON_PATH: &str = "icons/eye.svg";

#[derive(Clone, Copy)]
pub(crate) struct IconAsset {
    pub(crate) path: &'static str,
    pub(crate) svg: &'static str,
}

pub(crate) const ASSETS: &[IconAsset] = &[
    vault::ASSET,
    sort_newest::ASSET,
    server::ASSET,
    connect::ASSET,
    edit::ASSET,
    delete::ASSET,
    add::ASSET,
    settings::ASSET,
    password_eye::ASSET,
    password_eye_off::ASSET,
    IconAsset {
        path: GPUI_EYE_ICON_PATH,
        svg: password_eye::SVG,
    },
    IconAsset {
        path: GPUI_CHEVRON_DOWN_ICON_PATH,
        svg: sort_newest::SVG,
    },
];

pub(crate) fn asset_for_path(path: &str) -> Option<IconAsset> {
    ASSETS.iter().copied().find(|asset| asset.path == path)
}

pub(crate) fn render(path: &'static str, size: f32, color: u32) -> impl IntoElement {
    svg()
        .size(px(size))
        .text_color(rgb(color))
        .path(path)
        .flex_shrink_0()
}
