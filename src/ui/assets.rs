use std::borrow::Cow;

use anyhow::Result;
use gpui::{AssetSource, SharedString};

use super::icons;

pub(crate) struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(icons::asset_for_path(path).map(|asset| Cow::Borrowed(asset.svg.as_bytes())))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let path = path.trim_matches('/');

        Ok(icons::ASSETS
            .iter()
            .map(|asset| asset.path)
            .filter(|icon| {
                path.is_empty()
                    || icon
                        .strip_prefix(path)
                        .is_some_and(|rest| rest.starts_with('/'))
            })
            .map(SharedString::from)
            .collect())
    }
}
