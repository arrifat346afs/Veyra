//! Asynchronous, cached previews using image-rs and installed desktop thumbnailers.
mod cache;
mod providers;
use gpui_kit::*;
use std::{path::PathBuf, sync::Arc};
use cache::Source;
struct ThumbnailAsset;
impl Asset for ThumbnailAsset {
    type Source = Source;
    type Output = Option<PathBuf>;
    fn load(source: Source, cx: &mut App) -> impl std::future::Future<Output=Self::Output> + Send + 'static {
        let executor = cx.background_executor().clone();
        async move {
            // Bound expensive decoder work without blocking the UI or executor threads.
            static LIMIT: async_lock::Semaphore = async_lock::Semaphore::new(2);
            let _permit = LIMIT.acquire().await;
            executor.spawn(async move {source.generate()}).await
        }
    }
}
/// Show the themed icon while loading and whenever a provider cannot preview the file.
pub fn preview(path: &std::path::Path, icon: PathBuf, enabled: bool, size: f32) -> AnyElement {
    let source = enabled.then(|| Source::new(path.to_path_buf())).flatten();
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
