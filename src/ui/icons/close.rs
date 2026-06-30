use gpui::IntoElement;

use super::{IconAsset, render};

pub(crate) const PATH: &str = "xssh/icons/close.svg";
pub(crate) const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24"><!-- Icon from Huge Icons by Hugeicons - undefined --><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M18 6L6 18m12 0L6 6"/></svg>"#;
pub(crate) const ASSET: IconAsset = IconAsset {
    path: PATH,
    svg: SVG,
};

pub(crate) fn icon(size: f32, color: u32) -> impl IntoElement {
    render(PATH, size, color)
}
