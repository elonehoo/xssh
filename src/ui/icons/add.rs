use gpui::{prelude::*, rgb};
use gpui_component::Icon;

use super::IconAsset;

pub(crate) const PATH: &str = "xssh/icons/add.svg";
pub(crate) const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12.001 5v14.002m7.001-7H5"/></svg>"#;
pub(crate) const ASSET: IconAsset = IconAsset {
    path: PATH,
    svg: SVG,
};

pub(crate) fn button_icon(color: u32) -> Icon {
    Icon::empty().path(PATH).text_color(rgb(color))
}
