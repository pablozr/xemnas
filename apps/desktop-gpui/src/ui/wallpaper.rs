//! The background image behind the window's surfaces.
//!
//! GPUI on this revision cannot blur what lies behind a panel. The image is
//! therefore blurred once, here: decoded, scaled down (a blurred image needs
//! few pixels; the GPU scales it back up smoothly), blurred and darkened, off
//! the UI thread, and kept until the choice changes. The surfaces painted
//! over it are translucent, so the result reads as frosted glass without a
//! single blur at paint time.
//!
//! Darkening also guards legibility: on top of the chosen level, a bright
//! image is pulled down until its average luminance stays under a ceiling,
//! so text keeps its contrast whatever the person picks.

use std::path::Path;
use std::sync::Arc;

use gpui::{App, Global, Image, ImageFormat};

use crate::i18n::common as t;
use crate::ui::appearance::{self, Level, Wallpaper};

/// A shipped background: id, title and the full and thumbnail images.
pub struct Builtin {
    /// Stable id saved in the preference.
    pub id: &'static str,
    /// Name in English. The picker shows [`Builtin::title`], which follows
    /// the interface language.
    pub title: &'static str,
    /// The 2560×1440 JPEG.
    full: &'static [u8],
    /// The 320×180 JPEG for the picker.
    thumb: &'static [u8],
}

macro_rules! builtin {
    ($id:literal, $title:literal) => {
        Builtin {
            id: $id,
            title: $title,
            full: include_bytes!(concat!("../../assets/wallpapers/", $id, ".jpg")),
            thumb: include_bytes!(concat!("../../assets/wallpapers/", $id, "-thumb.jpg")),
        }
    };
}

/// The backgrounds shipped with the app, inspired by the Organization's
/// world (original images rendered by `tools/wallpapers/render.py`).
pub const BUILTINS: [Builtin; 4] = [
    builtin!("never", "The city that never existed"),
    builtin!("castle", "The castle"),
    builtin!("thirteen", "Where nothing gathers"),
    builtin!("chain", "Chain"),
];

impl Builtin {
    /// Name shown in the picker, in the interface language.
    pub fn title(&self) -> &'static str {
        match self.id {
            "never" => t::wallpaper_never(),
            "castle" => t::wallpaper_castle(),
            "thirteen" => t::wallpaper_thirteen(),
            _ => t::wallpaper_chain(),
        }
    }

    /// The picker thumbnail.
    pub fn thumbnail(&self) -> Arc<Image> {
        Arc::new(Image::from_bytes(ImageFormat::Jpeg, self.thumb.to_vec()))
    }
}

/// A 256 px tile of soft white grain, laid over the background image only:
/// a matte, printed feel instead of flat digital color.
pub fn grain() -> Arc<Image> {
    static GRAIN: std::sync::LazyLock<Arc<Image>> = std::sync::LazyLock::new(|| {
        Arc::new(Image::from_bytes(
            ImageFormat::Png,
            include_bytes!("../../assets/texture/grain.png").to_vec(),
        ))
    });
    GRAIN.clone()
}

/// Side of a grain tile.
pub const GRAIN_TILE: f32 = 256.0;

/// What a prepared background depends on.
#[derive(Clone, Debug, PartialEq)]
struct Key {
    wallpaper: Wallpaper,
    blur: Level,
    dim: Level,
}

/// The prepared background, and the one being prepared.
#[derive(Default)]
struct State {
    ready: Option<(Key, Arc<Image>)>,
    pending: Option<Key>,
    failed: Option<Key>,
}

impl Global for State {}

/// The prepared background for the current choice, if it is ready. Starts
/// preparing it when the choice changed.
pub fn current(cx: &mut App) -> Option<Arc<Image>> {
    let appearance = appearance::current(cx);
    if appearance.wallpaper == Wallpaper::None {
        return None;
    }
    let key = Key {
        wallpaper: appearance.wallpaper,
        blur: appearance.blur,
        dim: appearance.dim,
    };
    let state = cx.default_global::<State>();
    if let Some((ready, image)) = &state.ready {
        if *ready == key {
            return Some(image.clone());
        }
    }
    // While the new one is prepared, the previous stays up: no flash.
    let shown = state.ready.as_ref().map(|(_, image)| image.clone());
    if state.pending.as_ref() != Some(&key) && state.failed.as_ref() != Some(&key) {
        state.pending = Some(key.clone());
        prepare(key, cx);
    }
    shown
}

/// Whether the background in effect failed to load (a moved custom file).
pub fn failed(cx: &App) -> bool {
    let appearance = appearance::current(cx);
    cx.try_global::<State>().is_some_and(|state| {
        state
            .failed
            .as_ref()
            .is_some_and(|key| key.wallpaper == appearance.wallpaper)
    })
}

fn prepare(key: Key, cx: &mut App) {
    let source = key.wallpaper.clone();
    let (blur, dim) = (key.blur, key.dim);
    let task = cx
        .background_executor()
        .spawn(async move { process(&source, blur, dim) });
    cx.spawn(async move |cx| {
        let result = task.await;
        cx.update(|cx| {
            let state = cx.default_global::<State>();
            if state.pending.as_ref() != Some(&key) {
                return;
            }
            state.pending = None;
            match result {
                Ok(bytes) => {
                    state.failed = None;
                    state.ready = Some((key, Arc::new(Image::from_bytes(ImageFormat::Png, bytes))));
                }
                Err(error) => {
                    tracing::warn!(error = %error, operation = "prepare_wallpaper", "failed");
                    state.failed = Some(key);
                }
            }
            cx.refresh_windows();
        });
    })
    .detach();
}

/// Width the image is reduced to before blurring, and the blur radius.
fn reduction(blur: Level) -> (u32, f32) {
    match blur {
        Level::Low => (1600, 1.5),
        Level::Medium => (900, 3.0),
        Level::High => (480, 4.0),
    }
}

/// How much the image is darkened at each level (0 keeps it, 1 is black).
fn darkening(dim: Level) -> f32 {
    match dim {
        Level::Low => 0.16,
        Level::Medium => 0.32,
        Level::High => 0.50,
    }
}

/// The average luminance the darkened image may keep, whatever its source:
/// light text stays over a dark field.
const LUMINANCE_CEILING: f32 = 0.22;

fn process(source: &Wallpaper, blur: Level, dim: Level) -> Result<Vec<u8>, String> {
    let decoded = match source {
        Wallpaper::None => return Err("no wallpaper".into()),
        Wallpaper::Builtin(id) => {
            let builtin = BUILTINS
                .iter()
                .find(|builtin| builtin.id == id)
                .ok_or_else(|| format!("unknown builtin {id}"))?;
            image::load_from_memory(builtin.full).map_err(|error| error.to_string())?
        }
        Wallpaper::Custom(path) => image::open(path).map_err(|error| error.to_string())?,
    };
    let (width, sigma) = reduction(blur);
    let width = width.min(decoded.width().max(1));
    let height = ((decoded.height() as f32) * width as f32 / decoded.width().max(1) as f32)
        .round()
        .max(1.0) as u32;
    let small = decoded
        .resize_exact(width, height, image::imageops::FilterType::Triangle)
        .to_rgb8();
    let mut blurred = image::imageops::fast_blur(&small, sigma);
    let average = average_luminance(&blurred);
    let mut keep = 1.0 - darkening(dim);
    if average * keep > LUMINANCE_CEILING {
        keep = LUMINANCE_CEILING / average.max(0.001);
    }
    for pixel in blurred.pixels_mut() {
        for channel in pixel.0.iter_mut() {
            *channel = (*channel as f32 * keep).round() as u8;
        }
    }
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(blurred)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

/// Mean relative luminance (0–1) over a sample of pixels.
fn average_luminance(image: &image::RgbImage) -> f32 {
    let mut total = 0.0;
    let mut count = 0.0;
    for (index, pixel) in image.pixels().enumerate() {
        if index % 7 != 0 {
            continue;
        }
        let [r, g, b] = pixel.0;
        total += (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0;
        count += 1.0;
    }
    if count == 0.0 {
        0.0
    } else {
        total / count
    }
}

/// The custom image's file name, for the picker.
pub fn custom_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_prepares_and_stays_under_the_luminance_ceiling() {
        for builtin in &BUILTINS {
            let bytes = process(
                &Wallpaper::Builtin(builtin.id.into()),
                Level::High,
                Level::Low,
            )
            .expect(builtin.id);
            let image = image::load_from_memory(&bytes).expect("png").to_rgb8();
            assert!(
                average_luminance(&image) <= LUMINANCE_CEILING + 0.01,
                "{}",
                builtin.id
            );
        }
    }

    #[test]
    fn a_bright_image_is_pulled_down() {
        let white = image::RgbImage::from_pixel(64, 36, image::Rgb([250, 250, 250]));
        let path = std::env::temp_dir().join(format!("xemnas-white-{}.png", std::process::id()));
        white.save(&path).expect("save");
        let bytes =
            process(&Wallpaper::Custom(path.clone()), Level::Medium, Level::Low).expect("process");
        let image = image::load_from_memory(&bytes).expect("png").to_rgb8();
        assert!(average_luminance(&image) <= LUMINANCE_CEILING + 0.01);
        let _ = std::fs::remove_file(path);
        assert!(process(
            &Wallpaper::Custom("missing.png".into()),
            Level::Low,
            Level::Low
        )
        .is_err());
    }
}
