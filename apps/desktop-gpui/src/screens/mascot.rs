//! The mascot's flipbook player.
//!
//! `assets/mascot/sheet.png` holds 32 close frames of the toon figure: 27
//! turns of the head, 4 blink steps and one with the eyes lit. Motion is the
//! clock choosing a frame, never the layout moving an image, because the
//! layout snaps whole images to physical pixels and a 3 px float then moves
//! in visible steps.
//!
//! Each frame is uploaded with a margin around it and painted through a fixed,
//! pixel-aligned window while the image underneath slides by any fraction of a
//! pixel: the renderer picks the visible part of the frame in source texels,
//! about half a pixel at the dock's size, so the float is continuous without
//! drawing anything at more than the window's own rate. Frames are decoded
//! once, on first use.

use std::sync::{Arc, LazyLock};

use gpui::prelude::*;
use gpui::{canvas, div, point, px, size, AnyElement, Bounds, Corners, RenderImage};
use image::{Frame, ImageFormat, RgbaImage};

const SHEET: &[u8] = include_bytes!("../../assets/mascot/sheet.png");
/// Side of a cell of the sheet, in texels.
const CELL: u32 = 160;
const COLUMNS: u32 = 8;
/// Margin added around each cell so the image can slide under its window.
const PAD: u32 = 12;
const FRAMES: usize = 32;

/// Frame of the head at rest, looking at the person.
pub const REST: usize = 13;
/// The head turned fully one way and the other.
pub const LOOK_RIGHT: usize = 2;
pub const LOOK_LEFT: usize = 24;
/// First of the four blink frames (75, 50, 25 and 2 percent open).
pub const BLINK_FIRST: usize = 27;
/// Highest blink step: the eyes shut.
pub const BLINK_STEPS: usize = 4;
/// Frame with the eyes lit.
pub const GLOW: usize = 31;

static SHEET_FRAMES: LazyLock<Vec<Arc<RenderImage>>> = LazyLock::new(|| {
    let sheet = image::load_from_memory_with_format(SHEET, ImageFormat::Png)
        .expect("the mascot sheet is a valid PNG")
        .into_rgba8();
    let side = CELL + 2 * PAD;
    (0..FRAMES as u32)
        .map(|index| {
            let (column, row) = (index % COLUMNS, index / COLUMNS);
            let mut padded = RgbaImage::new(side, side);
            for y in 0..CELL {
                for x in 0..CELL {
                    let [r, g, b, a] = sheet.get_pixel(column * CELL + x, row * CELL + y).0;
                    // The renderer takes BGRA.
                    padded.put_pixel(PAD + x, PAD + y, image::Rgba([b, g, r, a]));
                }
            }
            Arc::new(RenderImage::new(vec![Frame::new(padded)]))
        })
        .collect()
});

/// What to draw this frame.
#[derive(Clone, Copy)]
pub struct Pose {
    /// Which of the 32 frames.
    pub frame: usize,
    /// How far up the figure floats, in pixels.
    pub lift: f32,
    /// How lit the eyes are, 0 to 1, over the frame.
    pub glow: f32,
    /// Whether the next frame will differ: the canvas then asks for it.
    pub animating: bool,
}

fn paint_cell(frame: usize, lift: f32, animating: bool) -> impl IntoElement {
    let image = SHEET_FRAMES[frame.min(FRAMES - 1)].clone();
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            // The window stays where the layout put it; the image slides.
            let texel = f32::from(bounds.size.width) / CELL as f32;
            let pad = px(PAD as f32 * texel);
            let lift = px(lift.clamp(-(PAD as f32 - 1.0) * texel, (PAD as f32 - 1.0) * texel));
            let whole = Bounds::new(
                point(bounds.origin.x - pad, bounds.origin.y - pad - lift),
                size(
                    bounds.size.width + pad * 2.0,
                    bounds.size.height + pad * 2.0,
                ),
            );
            let _ = window.paint_image(bounds, whole, Corners::default(), image, 0, false);
            if animating {
                window.request_animation_frame();
            }
        },
    )
    .size_full()
}

/// The mascot at `side` pixels, in the pose given. The lit frame is laid
/// over the base one and fades by opacity, so the eyes brighten smoothly.
pub fn figure(side: f32, pose: Pose) -> AnyElement {
    let mut root = div()
        .relative()
        .size(px(side))
        .flex_none()
        .child(paint_cell(pose.frame, pose.lift, pose.animating));
    if pose.glow > 0.001 {
        root = root.child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .opacity(pose.glow)
                .child(paint_cell(GLOW, pose.lift, false)),
        );
    }
    root.into_any_element()
}
