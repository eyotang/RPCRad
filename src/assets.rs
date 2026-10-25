use std::borrow::Cow;

use gpui_kit::{AssetSource, Result, SharedString};

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path == "brand/logo.png" {
            return Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/app-logo.png"
            ))));
        }
        gpui_kit::assets::AllAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::AllAssets.list(path)?;
        if "brand/logo.png".starts_with(path) {
            paths.push("brand/logo.png".into());
        }
        Ok(paths)
    }
}
