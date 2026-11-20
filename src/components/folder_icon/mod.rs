//! Asynchronous, theme-keyed folder SVGs. System icon files are never modified.
mod palette;
use gpui_kit::component::{ActiveTheme as _, Icon};
use gpui_kit::*;
use gpui_kit_assets::IconName;
use std::{path::{Path, PathBuf}, sync::Arc};

#[derive(Clone, Hash, PartialEq, Eq)]
struct Source {
    path: PathBuf,
    accent: [u8; 3],
}
struct FolderAsset;
impl Asset for FolderAsset {
    type Source = Source;
    type Output = Option<Arc<Image>>;
    fn load(
        source: Source,
        _cx: &mut App,
    ) -> impl std::future::Future<Output = Self::Output> + Send + 'static {
        async move {
            let svg = std::fs::read_to_string(source.path).ok()?;
            let themed = palette::recolor(&svg, source.accent);
            Some(Arc::new(Image::from_bytes(
                ImageFormat::Svg,
                themed.into_bytes(),
            )))
        }
    }
}
pub fn view(path: PathBuf, size: f32, cx: &mut App) -> AnyElement {
    let color = cx.theme().primary;
    if path.as_os_str().is_empty() {
        return Icon::new(IconName::Folder)
            .size(px(size))
            .text_color(color)
            .into_any_element();
    }
    let rgb = color.to_rgb();
    let source = Source {
        path: path.clone(),
        accent: [rgb.r, rgb.g, rgb.b].map(|c| (c * 255.).round() as u8),
    };
    let fallback: Resource = path.into();
    let image = ImageSource::Custom(Arc::new(move |window, cx| {
        match window.use_asset::<FolderAsset>(&source, cx) {
            Some(Some(image)) => image.clone().use_render_image(window, cx).map(Ok),
            Some(None) => window.use_asset::<ImgResourceLoader>(&fallback, cx),
            None => None,
        }
    }));
    img(image)
        .size(px(size))
        .object_fit(ObjectFit::Contain)
        .into_any_element()
}

pub fn view_for_folder(icon: PathBuf, folder: &Path, size: f32, cx: &mut App) -> AnyElement {
    let Some(custom) = crate::services::folder_icons::get(folder) else { return view(icon, size, cx); };
    let custom: Resource = custom.into();
    let fallback: Resource = icon.into();
    let image = ImageSource::Custom(Arc::new(move |window, cx| {
        window.use_asset::<ImgResourceLoader>(&custom, cx)
            .or_else(|| window.use_asset::<ImgResourceLoader>(&fallback, cx))
    }));
    img(image).size(px(size)).object_fit(ObjectFit::Contain).into_any_element()
}
