//! App-owned thumbnail cache keyed by path, file size and modification time.
use std::{collections::hash_map::DefaultHasher, fs, hash::{Hash, Hasher}, path::PathBuf, sync::atomic::{AtomicU64, Ordering}, time::UNIX_EPOCH};
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Source {pub path: PathBuf, length: u64, modified: u128}
impl Source {
    pub fn new(path: PathBuf) -> Option<Self> {
        let metadata = path.metadata().ok()?;
        if !metadata.is_file() {return None;}
        Some(Self {path, length: metadata.len(), modified: metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_nanos()})
    }
    fn destination(&self) -> PathBuf {
        let base = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache"));
        let mut hash = DefaultHasher::new();self.hash(&mut hash);
        base.join("veyra/thumbnails/v1").join(format!("{:016x}.png",hash.finish()))
    }
    pub fn generate(&self) -> Option<PathBuf> {
        let destination = self.destination();
        if valid(&destination) {return Some(destination);}
        fs::create_dir_all(destination.parent()?).ok()?;
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        let temporary = destination.with_extension(format!("{}-{}.tmp.png",std::process::id(),SERIAL.fetch_add(1,Ordering::Relaxed)));
        let mut success = native_image(&self.path,&temporary);
        if !success {
            for mime in mime_guess::from_path(&self.path).iter() {
                for provider in super::providers::providers().iter().filter(|p| p.supports(mime.essence_str())) {
                    let _ = fs::remove_file(&temporary);
                    if provider.generate(&self.path,&temporary) && valid(&temporary) {success=true;break;}
                }
                if success {break;}
            }
        }
        // Do not publish a stale result if the source changed during generation.
        let current = Source::new(self.path.clone());
        if success && current.as_ref() == Some(self) && fs::rename(&temporary,&destination).is_ok() {Some(destination)}
        else {let _ = fs::remove_file(temporary);None}
    }
}
fn valid(path: &std::path::Path) -> bool {image::image_dimensions(path).is_ok_and(|(w,h)| w>0 && h>0 && w<=1024 && h<=1024)}
fn native_image(input: &std::path::Path, output: &std::path::Path) -> bool {
    let Ok(reader) = image::ImageReader::open(input) else {return false;};
    let Ok(mut reader) = reader.with_guessed_format() else {return false;};
    if reader.format().is_none() {return false;}
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128*1024*1024);
    limits.max_image_width = Some(32768);limits.max_image_height = Some(32768);
    reader.limits(limits);
    let Ok(mut decoder) = reader.into_decoder() else {return false;};
    use image::ImageDecoder;
    let orientation = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let Ok(mut image) = image::DynamicImage::from_decoder(decoder) else {return false;};
    image.apply_orientation(orientation);
    image.thumbnail(256,256).save_with_format(output,image::ImageFormat::Png).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn images_and_gifs_are_resized_and_changes_invalidate_the_key() {
        let dir = std::env::temp_dir().join(format!("veyra-thumbnail-test-{}",std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        for extension in ["png","gif"] {
            let input = dir.join(format!("image with spaces.{extension}"));
            let output = dir.join(format!("preview-{extension}.png"));
            image::RgbaImage::from_pixel(640,320,image::Rgba([10,120,210,255])).save(&input).unwrap();
            let before = Source::new(input.clone()).unwrap();
            assert!(native_image(&input,&output));
            assert_eq!(image::image_dimensions(&output).unwrap(),(256,128));
            assert!(valid(&output));
            fs::write(&input,b"changed file").unwrap();
            let after = Source::new(input).unwrap();
            assert_ne!(before.destination(),after.destination());
        }
        let _ = fs::remove_dir_all(dir);
    }
}
