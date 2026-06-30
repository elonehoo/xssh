use gpui::{IntoElement, prelude::*, px, rgb, svg};

pub(crate) mod add;
pub(crate) mod close;
pub(crate) mod connect;
pub(crate) mod connection_test;
pub(crate) mod delete;
pub(crate) mod edit;
pub(crate) mod notification_status;
pub(crate) mod password_eye;
pub(crate) mod password_eye_off;
pub(crate) mod server;
pub(crate) mod settings;
pub(crate) mod sidebar_toggle;
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
    close::ASSET,
    connect::ASSET,
    connection_test::ASSET,
    edit::ASSET,
    delete::ASSET,
    notification_status::INFO_ASSET,
    notification_status::CIRCLE_CHECK_ASSET,
    notification_status::CIRCLE_X_ASSET,
    notification_status::TRIANGLE_ALERT_ASSET,
    add::ASSET,
    settings::ASSET,
    sidebar_toggle::EXPANDED_ASSET,
    sidebar_toggle::COLLAPSED_ASSET,
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
