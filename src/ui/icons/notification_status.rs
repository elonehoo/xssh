use super::IconAsset;

pub(crate) const INFO_PATH: &str = "icons/info.svg";
pub(crate) const CIRCLE_CHECK_PATH: &str = "icons/circle-check.svg";
pub(crate) const CIRCLE_X_PATH: &str = "icons/circle-x.svg";
pub(crate) const TRIANGLE_ALERT_PATH: &str = "icons/triangle-alert.svg";

pub(crate) const INFO_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>"#;
pub(crate) const CIRCLE_CHECK_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/></svg>"#;
pub(crate) const CIRCLE_X_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/></svg>"#;
pub(crate) const TRIANGLE_ALERT_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.46 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/></svg>"#;

pub(crate) const INFO_ASSET: IconAsset = IconAsset {
    path: INFO_PATH,
    svg: INFO_SVG,
};
pub(crate) const CIRCLE_CHECK_ASSET: IconAsset = IconAsset {
    path: CIRCLE_CHECK_PATH,
    svg: CIRCLE_CHECK_SVG,
};
pub(crate) const CIRCLE_X_ASSET: IconAsset = IconAsset {
    path: CIRCLE_X_PATH,
    svg: CIRCLE_X_SVG,
};
pub(crate) const TRIANGLE_ALERT_ASSET: IconAsset = IconAsset {
    path: TRIANGLE_ALERT_PATH,
    svg: TRIANGLE_ALERT_SVG,
};
