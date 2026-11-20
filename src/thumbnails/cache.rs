//! App-owned thumbnail cache keyed by path, file size and modification time.
use std::{collections::hash_map::DefaultHasher, fs, hash::{Hash, Hasher}, path::PathBuf, sync::atomic::{AtomicU64, Ordering}, time::UNIX_EPOCH};
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Source {pub path: PathBuf, generation: u64}
#[derive(Hash, PartialEq, Eq)]
struct Snapshot {path: PathBuf, length: u64, modified: u128}
impl Snapshot {
    fn read(path: &std::path::Path) -> Option<Self> {
        let metadata = path.metadata().ok()?;
        if !metadata.is_file() {return None;}
        Some(Self {path: path.to_path_buf(),length: metadata.len(),modified: metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_nanos()})
    }
    fn destination(&self) -> PathBuf {
        let base = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache"));
        let mut hash = DefaultHasher::new();self.hash(&mut hash);
        base.join("veyra/thumbnails/v1").join(format!("{:016x}.png",hash.finish()))
    }
}
impl Source {
    /// Constructing an asset key never touches the filesystem. Its generation
    /// changes on navigation, so a revisited folder checks files again.
    pub fn new(path: PathBuf, generation: u64) -> Self {Self {path,generation}}
    pub fn generate(&self) -> Option<PathBuf> {
        let snapshot = Snapshot::read(&self.path)?;
        let destination = snapshot.destination();
        if valid(&destination) {return Some(destination);}
        fs::create_dir_all(destination.parent()?).ok()?;
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        let temporary = destination.with_extension(format!("{}-{}.tmp.png",std::process::id(),SERIAL.fetch_add(1,Ordering::Relaxed)));
        let is_appimage = self.path.extension().and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("appimage"));
        let mut success = if is_appimage {
            super::appimage::generate(&self.path, &temporary)
        } else {
            native_image(&self.path,&temporary)
        };
        if !success {
            for mime in detected_mimes(&self.path) {
                let _ = fs::remove_file(&temporary);
                if super::builtin::generate(&mime, &self.path, &temporary) && valid(&temporary) {
                    success = true;
                    break;
                }
                let _ = fs::remove_file(&temporary);
                if super::office::generate(&mime, &self.path, &temporary) && valid(&temporary) {
                    success = true;
                    break;
                }
                for provider in super::providers::providers().iter().filter(|p| p.supports(&mime)) {
                    let _ = fs::remove_file(&temporary);
                    if provider.generate(&self.path,&temporary) && valid(&temporary) {success=true;break;}
                }
                if success {break;}
            }
        }
        // Do not publish a stale result if the source changed during generation.
        let current = Snapshot::read(&self.path);
        if success && current.as_ref() == Some(&snapshot) && fs::rename(&temporary,&destination).is_ok() {Some(destination)}
        else {let _ = fs::remove_file(temporary);None}
    }
}
fn detected_mimes(path: &std::path::Path) -> Vec<String> {
    use std::io::Read;
    let mut sample = [0; 16 * 1024];
    let bytes = fs::File::open(path).ok().and_then(|mut file| file.read(&mut sample).ok()).unwrap_or(0);
    let (content_type, _) = gio::content_type_guess(path.file_name(), Some(&sample[..bytes]));
    let mut mimes = gio::content_type_get_mime_type(content_type.as_str())
        .map(|mime| vec![mime.to_string()]).unwrap_or_default();
    for mime in mime_guess::from_path(path).iter().map(|mime| mime.essence_str().to_owned()) {
        if !mimes.contains(&mime) { mimes.push(mime); }
    }
    mimes
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
            let before = Snapshot::read(&input).unwrap();
            assert!(native_image(&input,&output));
            assert_eq!(image::image_dimensions(&output).unwrap(),(256,128));
            assert!(valid(&output));
            fs::write(&input,b"changed file").unwrap();
            let after = Snapshot::read(&input).unwrap();
            assert_ne!(before.destination(),after.destination());
        }
        let _ = fs::remove_dir_all(dir);
    }
}
