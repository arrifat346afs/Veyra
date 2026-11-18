//! Asynchronous, cached previews using image-rs and installed desktop thumbnailers.
mod cache;
mod appimage;
mod providers;
mod workers;
use gpui_kit::*;
use std::{path::PathBuf, sync::Arc};
use cache::Source;
struct ThumbnailAsset;
impl Asset for ThumbnailAsset {
    type Source = Source;
    type Output = Option<PathBuf>;
    fn load(source: Source, _cx: &mut App) -> impl std::future::Future<Output=Self::Output> + Send + 'static {
        async move { workers::generate(source).await }
    }
}
/// Show the themed icon while loading and whenever a provider cannot preview the file.
pub fn preview(path: &std::path::Path, icon: PathBuf, enabled: bool, size: f32, generation: u64) -> AnyElement {
    let source = enabled.then(|| Source::new(path.to_path_buf(), generation));
    let fallback: Resource = icon.into();
    let image = ImageSource::Custom(Arc::new(move |window,cx| {
        if let Some(source)=&source {
            if let Some(Some(path)) = window.use_asset::<ThumbnailAsset>(source,cx) {
                if let Some(Ok(image)) = window.use_asset::<ImgResourceLoader>(&Resource::from(path),cx) {return Some(Ok(image));}
            }
        }
        window.use_asset::<ImgResourceLoader>(&fallback,cx)
    }));
    img(image).size(px(size)).object_fit(ObjectFit::Contain).into_any_element()
}
