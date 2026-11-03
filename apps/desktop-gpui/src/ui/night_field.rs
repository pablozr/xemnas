//! "Noturno" v5, the tiled lake: a living ASCII night behind the review.
//!
//! The scene is a grid of 10 × 11 px cells computed from a handful of numbers
//! and the time (a port of the design reference `prototipo_v5.py`). Each cell
//! has up to two layers: a rounded tile (one `paint_quad`) and a glyph (one
//! shaped line per row). The whole strip is painted by one `canvas`.
//!
//! Layers, back to front: stars, moon, the water's surface and its dust,
//! dashes and bottom, the cliff, the weeds, the koi (one per decision waiting,
//! four at most) and the mascot fishing from the cliff.
//!
//! The user's rule, in code: **art is never cut**. Cells are of two kinds.
//! *Ambient* ones (stars, halo, surface, dust, dashes, bottom, cliff) may fade
//! cell by cell: near the texts and over the last three columns. *Objects*
//! (moon, each koi, each weed, the mascot with its rod) never get a per-cell
//! mask: one `presence` factor multiplies all their cells, so an object shows
//! whole, shows whole but fainter, or does not show. Objects also stay inside
//! the strip and away from the texts (the clearing).

use std::cell::RefCell;
use std::f64::consts::{PI, TAU};
use std::rc::Rc;

use gpui::{
    canvas, font, point, px, quad, size, Bounds, Context, FontWeight, Hsla, IntoElement, Pixels,
    Render, Rgba, SharedString, Styled, TextAlign, TextRun, Window,
};

use crate::ui::motion::clock::{self, Rate};
use crate::ui::theme::Theme;

/// Width of a cell, in pixels.
pub const CELL_WIDTH: f32 = 10.0;
/// Height of a cell, in pixels.
pub const CELL_HEIGHT: f32 = 11.0;
/// Columns and rows of the reference strip.
const STRIP_WIDTH: usize = 130;
const STRIP_HEIGHT: usize = 35;
/// The smallest grid worth drawing: under 35 rows the mascot does not fit.
const MIN_COLUMNS: usize = 96;
const MIN_ROWS: usize = 35;
/// Rows of the tallest strip: the rows past 35 are only sky.
const MAX_ROWS: usize = 40;
/// Rows of water, from the surface to the bottom.
const WATER_ROWS: usize = 22;
/// Font size of the glyphs, in pixels.
const GLYPH_SIZE: f32 = 10.0;
/// Horizontal nudge that centres a glyph in its cell.
const GLYPH_NUDGE: f32 = 2.0;
/// Gap between a tile and its cell, and the radius of its corners.
const TILE_INSET: f32 = 0.5;
const TILE_RADIUS: f32 = 1.5;
/// Columns at each side where the ambient fades and no object goes.
const EDGE_COLS: usize = 3;
/// Distance over which the ambient comes back around a text, in pixels.
const FADE: f64 = 110.0;
/// How far the clearing for objects goes past a text: sideways and up/down.
const CLEAR_X: f64 = 150.0;
const CLEAR_Y: f64 = 40.0;
/// Distance over which a koi dives in as it nears the clearing.
/// A koi is a big, soft thing: it only keeps a hand's breadth from the texts
/// and dives whole over `DIVE` px before it reaches that.
const CARP_CLEAR_X: f64 = 24.0;
const CARP_CLEAR_Y: f64 = 6.0;
const DIVE: f64 = 40.0;
/// A fixed instant for reduced motion.
const STILL_TIME: f64 = 2.4;
/// Levels the alpha is quantized to.
const ALPHA_LEVELS: f32 = 16.0;
/// A tile fainter than this is not painted.
const TILE_MIN: f32 = 0.02;
/// Most koi in the lake.
const MAX_CARPS: usize = 4;

type Rgb = [u8; 3];

const MUTED: Rgb = [0x85, 0x8C, 0x9D];
const LAV: Rgb = [0xA8, 0x9E, 0xBA];
const AMBER: Rgb = [0xD4, 0xB5, 0x6E];
const PALE: Rgb = [0xE6, 0xE1, 0xF0];
const SLATE: Rgb = [0x7F, 0x8F, 0xB5];
const WEED: Rgb = [0x5E, 0x7D, 0x72];
const ROCK: Rgb = [0x3A, 0x41, 0x55];
const HOOD: Rgb = [0x5B, 0x55, 0x86];
const DARK: Rgb = [0x0D, 0x11, 0x1A];
const VOID: Rgb = [0x05, 0x06, 0x0B];
const GLOW: Rgb = [0xF2, 0xD3, 0x7A];
const WHITE: Rgb = [255, 255, 255];

/// Who a cell belongs to. Ambient cells fade by cell; the rest are objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    /// Stars, halo, surface, dust, dashes, bottom and cliff.
    Ambient,
    /// The moon disc.
    Moon,
    /// A koi, by index.
    Carp(u8),
    /// A weed, by index.
    Weed(u8),
    /// The mascot with its rod, line and hook.
    Mascot,
}

/// One cell of the grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    /// The glyph.
    pub ch: char,
    /// Colour of the glyph.
    pub glyph: Rgb,
    /// Opacity of the glyph, from 0 to 1.
    pub alpha: f32,
    /// Colour and opacity of the tile under the glyph.
    pub tile: Option<(Rgb, f32)>,
    /// Whether the glyph is bold.
    pub bold: bool,
    /// Who the cell belongs to.
    pub owner: Owner,
}

impl Cell {
    const BLANK: Cell = Cell {
        ch: ' ',
        glyph: MUTED,
        alpha: 0.0,
        tile: None,
        bold: false,
        owner: Owner::Ambient,
    };

    fn glyph(ch: char, glyph: Rgb, alpha: f64, owner: Owner) -> Self {
        Self {
            ch,
            glyph,
            alpha: alpha as f32,
            tile: None,
            bold: false,
            owner,
        }
    }

    fn tiled(ch: char, glyph: (Rgb, f64), tile: (Rgb, f64), bold: bool, owner: Owner) -> Self {
        Self {
            ch,
            glyph: glyph.0,
            alpha: glyph.1 as f32,
            tile: Some((tile.0, tile.1 as f32)),
            bold,
            owner,
        }
    }

    fn glyph_visible(&self) -> bool {
        self.ch != ' ' && level(self.alpha) > 0
    }

    fn tile_visible(&self) -> bool {
        self.tile.is_some_and(|(_, alpha)| alpha > TILE_MIN)
    }

    /// Whether anything of the cell shows.
    pub fn visible(&self) -> bool {
        self.glyph_visible() || self.tile_visible()
    }

    fn scale(&mut self, factor: f64) {
        self.alpha *= factor as f32;
        if let Some((_, alpha)) = self.tile.as_mut() {
            *alpha *= factor as f32;
        }
    }
}

/// A rectangle in pixels of the strip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// Left.
    pub x0: f64,
    /// Top.
    pub y0: f64,
    /// Right.
    pub x1: f64,
    /// Bottom.
    pub y1: f64,
}

impl Rect {
    fn grown(&self, dx: f64, dy: f64) -> Rect {
        Rect {
            x0: self.x0 - dx,
            y0: self.y0 - dy,
            x1: self.x1 + dx,
            y1: self.y1 + dy,
        }
    }

    /// Distance between the rectangles (zero when they touch or overlap),
    /// with the vertical gap scaled by `vertical`.
    fn gap(&self, other: &Rect, vertical: f64) -> f64 {
        let dx = (other.x0 - self.x1).max(self.x0 - other.x1).max(0.0);
        let dy = (other.y0 - self.y1).max(self.y0 - other.y1).max(0.0);
        dx.hypot(dy * vertical)
    }
}

/// The numbers that shape a scene (everything but the time and the texts).
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    /// Columns.
    pub width: usize,
    /// Rows.
    pub height: usize,
    /// Row of the water's surface.
    pub surface: usize,
    /// Decisions waiting: one koi each, four at most.
    pub waiting: usize,
}

impl Scene {
    /// The strip at the size of the design reference.
    pub fn strip() -> Self {
        Self::strip_sized(STRIP_WIDTH, STRIP_HEIGHT)
    }

    /// The strip over `width` columns and `height` rows: the water keeps its
    /// 22 rows and a taller strip only adds sky.
    pub fn strip_sized(width: usize, height: usize) -> Self {
        let (width, height) = (width.max(MIN_COLUMNS), height.max(MIN_ROWS));
        Self {
            width,
            height,
            surface: height - WATER_ROWS,
            waiting: 0,
        }
    }

    /// The lake behind the empty queue: no koi, only the moon and the mascot.
    pub fn full() -> Self {
        Self::strip_sized(STRIP_WIDTH, MAX_ROWS)
    }

    fn width_px(&self) -> f64 {
        self.width as f64 * f64::from(CELL_WIDTH)
    }

    fn height_px(&self) -> f64 {
        self.height as f64 * f64::from(CELL_HEIGHT)
    }

    fn surface_px(&self) -> f64 {
        self.surface as f64 * f64::from(CELL_HEIGHT)
    }

    fn cliff_x(&self) -> f64 {
        self.width_px() - 162.0
    }

    /// Where the texts sit before they are measured: "next" and the pill,
    /// centred and resting on the base of the strip.
    pub fn default_keep_out(&self) -> [Rect; 2] {
        let (middle, base) = (self.width_px() / 2.0, self.height_px());
        [
            Rect {
                x0: middle - 185.0,
                y0: base - 87.0,
                x1: middle + 185.0,
                y1: base - 61.0,
            },
            Rect {
                x0: middle - 210.0,
                y0: base - 56.0,
                x1: middle + 210.0,
                y1: base - 7.0,
            },
        ]
    }
}

/// The grid that fits a box of `width` by `height` pixels: columns, then
/// rows. Zero rows mean the box is too small for a night worth drawing.
pub fn strip_dims(width: f32, height: f32) -> (usize, usize) {
    let columns = ((width / CELL_WIDTH).floor().max(0.0) as usize).min(STRIP_WIDTH);
    let rows = ((height / CELL_HEIGHT).floor().max(0.0) as usize).min(MAX_ROWS);
    if columns < MIN_COLUMNS || rows < MIN_ROWS {
        (columns, 0)
    } else {
        (columns, rows)
    }
}

/// The alpha as one of the 16 levels (0 is invisible).
fn level(alpha: f32) -> u8 {
    (alpha.clamp(0.0, 1.0) * ALPHA_LEVELS).round() as u8
}

/// A small deterministic generator (mulberry32).
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6D2B_79F5);
        let mut q = (self.0 ^ (self.0 >> 15)).wrapping_mul(1 | self.0);
        q = q.wrapping_add((q ^ (q >> 7)).wrapping_mul(61 | q)) ^ q;
        f64::from(q ^ (q >> 14)) / 4_294_967_296.0
    }

    fn range(&mut self, low: f64, high: f64) -> f64 {
        low + self.next() * (high - low)
    }
}

fn smooth(k: f64) -> f64 {
    let k = k.clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

fn hash2(x: f64, y: f64, s: f64) -> f64 {
    let n = (x * 127.1 + y * 311.7 + s * 74.7).sin() * 43758.5453;
    n - n.floor()
}

fn vnoise(x: f64, y: f64, s: f64) -> f64 {
    let (xi, yi) = (x.floor(), y.floor());
    let (xf, yf) = (x - xi, y - yi);
    let (u, v) = (smooth(xf), smooth(yf));
    let (a, b) = (hash2(xi, yi, s), hash2(xi + 1.0, yi, s));
    let (c, d) = (hash2(xi, yi + 1.0, s), hash2(xi + 1.0, yi + 1.0, s));
    a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
}

fn mix(a: Rgb, b: Rgb, k: f64) -> Rgb {
    let channel =
        |i: usize| (f64::from(a[i]) + (f64::from(b[i]) - f64::from(a[i])) * k).round() as u8;
    [channel(0), channel(1), channel(2)]
}

/// Centre of a cell, in pixels of the strip.
fn centre(cx: usize, cy: usize) -> (f64, f64) {
    (
        (cx as f64 + 0.5) * f64::from(CELL_WIDTH),
        (cy as f64 + 0.5) * f64::from(CELL_HEIGHT),
    )
}

struct Grid {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl Grid {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::BLANK; width * height],
        }
    }

    fn set(&mut self, cx: i64, cy: i64, cell: Cell) {
        if cx >= 0 && cy >= 0 && (cx as usize) < self.width && (cy as usize) < self.height {
            self.cells[cy as usize * self.width + cx as usize] = cell;
        }
    }

    /// Writes only on a cell that is still empty.
    fn set_if_blank(&mut self, cx: i64, cy: i64, cell: Cell) {
        if cx >= 0 && cy >= 0 && (cx as usize) < self.width && (cy as usize) < self.height {
            let slot = &mut self.cells[cy as usize * self.width + cx as usize];
            if *slot == Cell::BLANK {
                *slot = cell;
            }
        }
    }
}

/// Everything about the scene in pixels, computed once per frame.
struct Geo {
    cols: usize,
    rows: usize,
    width: f64,
    height: f64,
    surface: usize,
    surf: f64,
    cliff_x: f64,
    moon: (f64, f64, f64),
}

impl Geo {
    fn new(scene: &Scene) -> Self {
        Self {
            cols: scene.width,
            rows: scene.height,
            width: scene.width_px(),
            height: scene.height_px(),
            surface: scene.surface,
            surf: scene.surface_px(),
            cliff_x: scene.cliff_x(),
            moon: (150.0, scene.surface_px() - 81.0, 46.0),
        }
    }

    /// Where the rod's tip and the hook hang, and where the hand is.
    fn rig(&self) -> ((f64, f64), (f64, f64), (f64, f64)) {
        let h = f64::from(CELL_HEIGHT);
        let tip = (self.width - 249.0, (self.surface as f64 - 8.0) * h);
        let hook = (tip.0, (self.surface as f64 + 11.0) * h);
        let hand = (
            self.mascot_col() as f64 * f64::from(CELL_WIDTH) + f64::from(CELL_WIDTH) / 2.0,
            (self.surface as f64 - 13.0 + 10.0) * h + h / 2.0,
        );
        (tip, hook, hand)
    }

    fn mascot_col(&self) -> usize {
        self.cols - MASCOT_COLS - EDGE_COLS
    }

    /// The box of the moon, and of the mascot with its rod, line and hook.
    fn moon_box(&self) -> Option<Rect> {
        let (x, y, r) = self.moon;
        (y - r >= 0.0).then_some(Rect {
            x0: x - r - 6.0,
            y0: y - r - 6.0,
            x1: x + r + 6.0,
            y1: y + r + 6.0,
        })
    }

    fn mascot_box(&self) -> Option<Rect> {
        if self.surface < MASCOT_ROWS {
            return None;
        }
        let (tip, ..) = self.rig();
        let h = f64::from(CELL_HEIGHT);
        Some(Rect {
            x0: tip.0 - 6.0,
            y0: (self.surface - MASCOT_ROWS) as f64 * h,
            x1: self.width - EDGE_COLS as f64 * f64::from(CELL_WIDTH),
            y1: (self.surface as f64 + 12.0) * h,
        })
    }
}

/// How far a box is from the clearing around the texts (zero when inside).
fn clearing_gap(bounds: &Rect, keep_out: &[Rect]) -> f64 {
    keep_out
        .iter()
        .map(|rect| bounds.gap(&rect.grown(CLEAR_X, CLEAR_Y), 1.0))
        .fold(f64::INFINITY, f64::min)
}

/// 1 far from the texts, 0 on them: the ambient's fade around a text.
fn keep_factor(px: f64, py: f64, keep_out: &[Rect]) -> f64 {
    let point = Rect {
        x0: px,
        y0: py,
        x1: px,
        y1: py,
    };
    let best = keep_out
        .iter()
        .map(|rect| point.gap(rect, 1.6))
        .fold(f64::INFINITY, f64::min);
    smooth(best / FADE)
}

// ---------------------------------------------------------------- koi

struct Carp {
    index: u8,
    length: f64,
    speed: f64,
    start: f64,
    lane: f64,
    bob: f64,
    phase: f64,
    seed: f64,
    colors: (Rgb, Rgb),
    depth: f64,
}

#[derive(Clone, Copy)]
enum Part {
    Eye,
    Body(f64, Rgb),
    Fin,
}

/// Where a koi is at time `t`: its head, its height and the direction.
struct Pose {
    head: f64,
    y: f64,
    dir: f64,
    bounds: Rect,
}

impl Carp {
    fn new(index: usize, geo: &Geo) -> Self {
        let mut rng = Rng(31 + 7 * index as u32);
        let length = [300.0, 250.0, 210.0, 270.0][index];
        let colors = [(LAV, PALE), (AMBER, PALE), (PALE, SLATE), (SLATE, LAV)][index];
        Self {
            index: index as u8,
            length,
            speed: rng.range(12.0, 22.0) * length / 250.0,
            start: [150.0, 700.0, 1100.0, 450.0][index],
            lane: geo.surf + [15.0, 85.0, 50.0, 108.0][index],
            bob: rng.range(0.0, TAU),
            phase: rng.range(0.0, TAU),
            seed: index as f64 * 13.0 + 3.0,
            colors,
            depth: 0.55 + 0.45 * (length - 200.0) / 100.0,
        }
    }

    /// The koi turns at the walls instead of crossing them. `None` when the
    /// room between the edge and the cliff cannot hold it whole.
    fn pose(&self, geo: &Geo, t: f64) -> Option<Pose> {
        let l = self.length;
        let left = EDGE_COLS as f64 * f64::from(CELL_WIDTH);
        let right = geo.cliff_x - 40.0;
        let run = right - left - 1.12 * l;
        if run <= 1.0 {
            return None;
        }
        let s = (self.start + self.speed * t).rem_euclid(2.0 * run);
        let (offset, dir) = if s < run {
            (s, 1.0)
        } else {
            (2.0 * run - s, -1.0)
        };
        let low = left + offset;
        let head = if dir > 0.0 {
            low + 1.1 * l
        } else {
            low + 0.02 * l
        };
        let y = self.lane + 10.0 * (t * 0.5 + self.bob).sin();
        let bounds = Rect {
            x0: low,
            y0: y - 0.36 * l - 4.0,
            x1: low + 1.12 * l,
            y1: y + 0.36 * l + 4.0,
        };
        (bounds.y0 >= 0.0 && bounds.y1 <= geo.height).then_some(Pose {
            head,
            y,
            dir,
            bounds,
        })
    }

    fn sample(&self, pose: &Pose, px: f64, py: f64, t: f64) -> Option<Part> {
        let (l, hh) = (self.length, self.length * 0.36);
        let u = -pose.dir * (px - pose.head) / l;
        if !(-0.02..=1.10).contains(&u) {
            return None;
        }
        let spine = 0.05 * l * (TAU * (u - t * 1.2) + self.phase).sin() * u.max(0.0).powf(1.5);
        let v = py - (pose.y + spine);
        if u <= 0.72 {
            let uu = u.max(0.0) / 0.72;
            let h = hh / 2.0 * (0.10 + 0.90 * (PI * uu.powf(0.55)).sin());
            if v.abs() <= h {
                if ((u - 0.09) * l).hypot(v + 0.10 * hh) < 3.0 {
                    return Some(Part::Eye);
                }
                let s = 0.5 - 0.5 * v / h;
                let spot = vnoise(u * 4.5 + self.seed, v / hh * 2.5 + self.seed, 0.0) > 0.55;
                return Some(Part::Body(
                    s,
                    if spot { self.colors.1 } else { self.colors.0 },
                ));
            }
            let dorsal = (0.20..=0.52).contains(&u)
                && -h
                    - hh * 0.34
                        * (PI * (u - 0.20) / 0.32).sin().powf(0.7)
                        * (1.0 - (u - 0.2) * 0.8)
                    <= v
                && v < -h;
            let pelvic = (0.24..=0.40).contains(&u)
                && h < v
                && v <= h + hh * 0.30 * (PI * (u - 0.24) / 0.16).sin();
            return (dorsal || pelvic).then_some(Part::Fin);
        }
        let k = (u - 0.72) / 0.38;
        let flap = 0.70 + 0.30 * (t * 4.0 + self.phase).sin();
        let half = hh / 2.0 * (0.12 + 1.15 * k.powf(0.9)) * flap;
        if v.abs() > half || (k > 0.45 && v.abs() < (k - 0.45) / 0.55 * half * 0.75) {
            return None;
        }
        Some(Part::Fin)
    }

    /// The cell at (`cx`, `cy`), if the koi draws one there.
    fn cell(&self, pose: &Pose, cx: usize, cy: usize, t: f64) -> Option<Cell> {
        let (px, py) = centre(cx, cy);
        let owner = Owner::Carp(self.index);
        let (mut body, mut fin, mut shade) = (0, 0, 0.0);
        let (mut colour, mut eye) = (None, false);
        for sx in [-0.33, 0.0, 0.33] {
            for sy in [-0.33, 0.0, 0.33] {
                let sample = self.sample(
                    pose,
                    px + sx * f64::from(CELL_WIDTH),
                    py + sy * f64::from(CELL_HEIGHT),
                    t,
                );
                match sample {
                    None => {}
                    Some(Part::Eye) => eye = true,
                    Some(Part::Fin) => fin += 1,
                    Some(Part::Body(s, c)) => {
                        body += 1;
                        shade += s;
                        colour = Some(c);
                    }
                }
            }
        }
        let a = self.depth;
        if eye {
            return Some(Cell::tiled(
                '@',
                (PALE, 0.95 * a),
                (DARK, 0.95),
                true,
                owner,
            ));
        }
        if body >= 5 {
            let s = shade / f64::from(body);
            let col = colour.unwrap_or(self.colors.0);
            let jitter = (hash2(cx as f64, cy as f64, (t * 4.0).floor() + self.seed) - 0.5) * 1.6;
            let (glyph, glyph_colour) = if s > 0.86 {
                ('S', (mix(col, WHITE, 0.45), 0.95 * a))
            } else {
                let index = ((1.0 - s) * 6.0 + jitter).trunc().clamp(0.0, 6.0) as usize;
                (":tSX%8@".as_bytes()[index] as char, (DARK, 0.78))
            };
            return Some(Cell::tiled(
                glyph,
                glyph_colour,
                (col, (0.22 + 0.58 * s) * a),
                true,
                owner,
            ));
        }
        if body + fin >= 4 {
            let col = colour.unwrap_or(self.colors.0);
            let ch = if fin > body { ':' } else { 'o' };
            return Some(Cell::tiled(
                ch,
                (mix(col, WHITE, 0.2), 0.55 * a),
                (col, 0.14 * a),
                false,
                owner,
            ));
        }
        if body + fin >= 2 {
            let col = colour.unwrap_or(self.colors.0);
            return Some(Cell::glyph('.', col, 0.45 * a, owner));
        }
        None
    }
}

/// How much of a koi shows: whole when it is away from the texts, diving
/// whole as it nears them.
fn carp_presence(bounds: &Rect, keep_out: &[Rect]) -> f64 {
    smooth(carp_gap(bounds, keep_out) / DIVE)
}

/// How far a koi is from the texts and their narrow margin.
fn carp_gap(bounds: &Rect, keep_out: &[Rect]) -> f64 {
    keep_out
        .iter()
        .map(|rect| bounds.gap(&rect.grown(CARP_CLEAR_X, CARP_CLEAR_Y), 1.0))
        .fold(f64::INFINITY, f64::min)
}

// ---------------------------------------------------------------- mascot

const MASCOT_COLS: usize = 11;
const MASCOT_ROWS: usize = 13;
const MASCOT: [&str; MASCOT_ROWS] = [
    "...HHHHH...",
    "..HhhHHHH..",
    ".HhhWWWHHH.",
    ".HhWWWWWHH.",
    ".HhWVWVWVH.",
    ".HVEEVEEVH.",
    ".HVVVVVVVH.",
    "..HHVVVHH..",
    ".HhsHzHsHH.",
    ".HhsHzHsHH.",
    "MHhoHzHoHHM",
    "MHhHHzHHHHM",
    ".HHHHHHHHH.",
];

/// The row of the eyes while the mascot looks ahead, left or right.
fn eye_row(look: i32) -> &'static str {
    match look {
        -1 => ".HEEVEEVVH.",
        1 => ".HVVEEVEEH.",
        _ => ".HVEEVEEVH.",
    }
}

fn hair(c: usize, r: usize) -> char {
    match (c, r) {
        (4, 2) | (4, 3) | (7, 3) => '\\',
        (6, 2) | (3, 3) | (6, 3) => '/',
        _ => '|',
    }
}

fn mascot_cell(c: usize, r: usize, t: f64) -> Option<Cell> {
    let row = if r == 5 {
        let phase = t % 9.0;
        eye_row(if phase > 5.0 && phase < 6.6 { -1 } else { 0 })
    } else {
        MASCOT[r]
    };
    let k = row.as_bytes()[c] as char;
    if k == '.' {
        return None;
    }
    let shade = 1.0 - 0.35 * c as f64 / (MASCOT_COLS - 1) as f64;
    let owner = Owner::Mascot;
    Some(match k {
        'H' | 'h' => {
            let rim = Some(c) == row.find('H') || (r > 0 && MASCOT[r - 1].as_bytes()[c] == b'.');
            let a =
                (0.42 + if k == 'h' { 0.12 } else { 0.0 } + if rim { 0.10 } else { 0.0 }) * shade;
            if rim {
                Cell::tiled(' ', (PALE, 0.0), (mix(HOOD, LAV, 0.45), a), false, owner)
            } else {
                let ch = if k == 'h' && (c + r).is_multiple_of(3) {
                    ':'
                } else {
                    ' '
                };
                let lift = if k == 'h' { 0.25 } else { 0.0 };
                Cell::tiled(ch, (PALE, 0.40), (mix(HOOD, LAV, lift), a), false, owner)
            }
        }
        'V' => Cell::tiled(' ', (DARK, 0.0), (VOID, 0.95), false, owner),
        'E' => {
            if (t + 1.3) % 4.2 < 0.16 {
                Cell::tiled(' ', (DARK, 0.0), (VOID, 0.95), false, owner)
            } else {
                let glow = 0.88 + 0.12 * (t * 1.7).sin();
                Cell::tiled(' ', (DARK, 0.0), (GLOW, glow), false, owner)
            }
        }
        'W' => Cell::tiled(
            hair(c, r),
            (DARK, 0.80),
            (PALE, 0.80 * shade + 0.1),
            true,
            owner,
        ),
        's' => Cell::tiled('|', (PALE, 0.90), (HOOD, 0.30 * shade), true, owner),
        'o' => Cell::tiled('o', (PALE, 0.95), (HOOD, 0.30 * shade), true, owner),
        'z' => Cell::tiled(':', (PALE, 0.55), (HOOD, 0.30 * shade), false, owner),
        'M' => Cell::tiled(' ', (DARK, 0.0), (PALE, 0.75 * shade + 0.1), false, owner),
        _ => return None,
    })
}

/// Rod, line and hook of the mascot at the cell (`cx`, `cy`).
fn rig_cell(geo: &Geo, cx: usize, cy: usize) -> Option<Cell> {
    let (tip, hook, hand) = geo.rig();
    let (px, py) = centre(cx, cy);
    let (cw, ch) = (f64::from(CELL_WIDTH), f64::from(CELL_HEIGHT));
    let owner = Owner::Mascot;
    let mut out = None;
    if px >= tip.0.min(hand.0) && px <= tip.0.max(hand.0) {
        let row =
            |x: f64| ((tip.1 + (hand.1 - tip.1) * (x - tip.0) / (hand.0 - tip.0)) / ch).floor();
        if row(px) == cy as f64 {
            let glyph = if row(px - cw) != cy as f64 { '\\' } else { '-' };
            out = Some(Cell::glyph(glyph, MUTED, 0.55, owner));
        }
    }
    if (px - tip.0).abs() < cw * 0.5 {
        if tip.1 <= py && py < hook.1 {
            out = Some(Cell::glyph(':', AMBER, 0.55, owner));
        }
        if (hook.0 / cw).floor() as usize == cx && (hook.1 / ch).floor() as usize == cy {
            out = Some(Cell {
                bold: true,
                ..Cell::glyph('J', AMBER, 0.9, owner)
            });
        }
    }
    out
}

// ---------------------------------------------------------------- ambient

fn moon_halo(cx: usize, cy: usize, geo: &Geo) -> Option<Cell> {
    let (px, py) = centre(cx, cy);
    let (mx, my, mr) = geo.moon;
    let d = (px - mx).hypot(py - my);
    (d < mr * 2.0 && d >= mr && hash2(cx as f64, cy as f64, 2.0) > 0.80)
        .then(|| Cell::glyph('.', PALE, 0.16, Owner::Ambient))
}

fn moon_cell(cx: usize, cy: usize, geo: &Geo) -> Option<Cell> {
    let (px, py) = centre(cx, cy);
    let (mx, my, mr) = geo.moon;
    let (mut hits, mut shade) = (0, 0.0);
    for sx in [-0.33, 0.0, 0.33] {
        for sy in [-0.33, 0.0, 0.33] {
            let qx = px + sx * f64::from(CELL_WIDTH);
            let qy = py + sy * f64::from(CELL_HEIGHT);
            let d = (qx - mx).hypot(qy - my);
            if d < mr {
                hits += 1;
                let crater = vnoise(qx / 14.0, qy / 14.0, 3.0);
                let relief = if crater < 0.6 { 1.0 } else { 0.0 };
                shade += (1.0 - (d / mr).powi(3) * 0.5) * (0.55 + 0.45 * relief);
            }
        }
    }
    if hits == 0 {
        return None;
    }
    let (coverage, s) = (f64::from(hits) / 9.0, shade / f64::from(hits));
    Some(if coverage > 0.5 {
        let ch = if s < 0.62 {
            ':'
        } else if s < 0.72 {
            '.'
        } else {
            ' '
        };
        Cell::tiled(ch, (DARK, 0.6), (PALE, 0.22 + 0.62 * s), true, Owner::Moon)
    } else {
        Cell::glyph('.', PALE, 0.40, Owner::Moon)
    })
}

fn surface_cell(cx: usize, cy: usize, geo: &Geo, t: f64) -> Option<Cell> {
    let (px, _) = centre(cx, cy);
    if cy != geo.surface || px >= geo.cliff_x {
        return None;
    }
    let n = vnoise(px / 16.0 - t * 0.8, 0.0, 4.0);
    if (px - geo.moon.0).abs() < 70.0 && n > 0.5 && hash2(cx as f64, (t * 3.0).floor(), 4.0) > 0.3 {
        Some(Cell::tiled(
            '=',
            (DARK, 0.7),
            (PALE, 0.20 + 0.40 * (n - 0.5)),
            true,
            Owner::Ambient,
        ))
    } else if n > 0.45 {
        let ch = if n > 0.65 { '~' } else { '-' };
        Some(Cell::glyph(ch, MUTED, 0.28, Owner::Ambient))
    } else {
        None
    }
}

fn cliff_cell(cx: usize, cy: usize, geo: &Geo) -> Option<Cell> {
    let (px, py) = centre(cx, cy);
    let edge = geo.cliff_x + 14.0 * vnoise(py / 26.0, 0.2, 7.0) - ((py - geo.surf) * 0.10).max(0.0);
    let top = geo.surf - 7.0 + 6.0 * vnoise(px / 20.0, 0.5, 12.0);
    if px < edge || py < top {
        return None;
    }
    let tex = vnoise(px / 11.0, py / 11.0, 11.0);
    let ch = if tex > 0.70 {
        ':'
    } else if tex > 0.55 {
        '.'
    } else {
        ' '
    };
    if (px - edge) < 16.0 || (py - top) < 14.0 {
        Some(Cell::tiled(
            ch,
            (MUTED, 0.20),
            (ROCK, 0.42 + 0.15 * tex),
            false,
            Owner::Ambient,
        ))
    } else if hash2(cx as f64, cy as f64, 13.0) > 0.55 {
        Some(Cell::tiled(
            ch,
            (MUTED, 0.10),
            (ROCK, 0.14 + 0.10 * tex),
            false,
            Owner::Ambient,
        ))
    } else {
        None
    }
}

fn ambient(grid: &mut Grid, geo: &Geo, t: f64) {
    for cy in 0..geo.rows {
        for cx in 0..geo.cols {
            let cell = cliff_cell(cx, cy, geo)
                .or_else(|| surface_cell(cx, cy, geo, t))
                .or_else(|| moon_halo(cx, cy, geo));
            if let Some(cell) = cell {
                grid.set(cx as i64, cy as i64, cell);
            }
        }
    }
    let mut rng = Rng(17);
    let n = (70.0 * geo.cols as f64 / 130.0 * (geo.surface as f64 - 1.0) / 12.0).round() as usize;
    for _ in 0..n {
        let cx = (rng.next() * geo.cols as f64) as i64;
        let cy = (rng.next() * (geo.surface as f64 - 1.0)) as i64;
        let r = rng.next();
        let twinkle = 0.5 + 0.5 * (t * (0.6 + r) + r * 20.0).sin();
        let (ch, colour) = if r > 0.92 { ('+', PALE) } else { ('.', MUTED) };
        grid.set_if_blank(
            cx,
            cy,
            Cell::glyph(ch, colour, 0.12 + 0.30 * twinkle, Owner::Ambient),
        );
    }
    let (w, h, surf) = (geo.width, geo.height, geo.surf);
    let (cw, ch) = (f64::from(CELL_WIDTH), f64::from(CELL_HEIGHT));
    let mut rng = Rng(23);
    for _ in 0..35 {
        let (dx, dy, phase) = (
            rng.range(0.0, w),
            rng.range(surf + 10.0, h),
            rng.range(0.0, TAU),
        );
        let qx = (dx + 4.0 * (t * 0.7 + phase).sin()).rem_euclid(w);
        let qy = surf + 10.0 + (dy - surf - 10.0 - t * 6.0).rem_euclid(h - surf - 10.0);
        let glyph = if phase > 2.0 { '.' } else { '\'' };
        let alpha = 0.22 + 0.12 * (t + phase).sin();
        grid.set_if_blank(
            (qx / cw) as i64,
            (qy / ch) as i64,
            Cell::glyph(glyph, MUTED, alpha, Owner::Ambient),
        );
    }
    let mut rng = Rng(29);
    for _ in 0..5 {
        let first = rng.range(10.0, geo.cols as f64 - 30.0).floor();
        let low = geo.surface as f64 + 2.0;
        let row = (low + (rng.next() * (geo.rows as f64 - 3.0 - low)).floor()) as i64;
        let count = (rng.range(3.0, 7.0)).floor() as i64;
        for k in 0..count {
            let x = first + 2.0 * k as f64;
            let on = 0.5 + 0.5 * (t * 1.5 + first).sin();
            if hash2(x, (t * 2.0).floor(), first) < 0.85 {
                grid.set_if_blank(
                    x as i64,
                    row,
                    Cell::glyph('-', MUTED, 0.14 + 0.22 * on, Owner::Ambient),
                );
            }
        }
    }
    let last = geo.rows as i64 - 1;
    for cx in 0..geo.cols {
        if hash2(cx as f64, 1.0, 8.0) > 0.6 {
            let ch = if hash2(cx as f64, 2.0, 8.0) > 0.5 {
                '.'
            } else {
                ','
            };
            grid.set_if_blank(
                cx as i64,
                last,
                Cell::glyph(ch, MUTED, 0.16, Owner::Ambient),
            );
        }
    }
}

// ---------------------------------------------------------------- weeds

struct Weed {
    index: u8,
    x: f64,
    height: usize,
    phase: f64,
}

fn weeds(geo: &Geo) -> Vec<Weed> {
    let mut rng = Rng(41);
    (0..4)
        .map(|index| Weed {
            index,
            x: rng.range(60.0, geo.width - 260.0),
            height: rng.range(4.0, 9.99) as usize,
            phase: rng.range(0.0, TAU),
        })
        .collect()
}

impl Weed {
    fn bounds(&self, geo: &Geo) -> Rect {
        Rect {
            x0: self.x - 16.0,
            y0: geo.height - self.height as f64 * f64::from(CELL_HEIGHT),
            x1: self.x + 16.0,
            y1: geo.height,
        }
    }

    fn cells(&self, geo: &Geo, t: f64) -> Vec<(usize, usize, Cell)> {
        let mut out = Vec::new();
        for r in 0..self.height {
            let k = r as f64 / self.height as f64;
            let arg = t * 1.1 + self.phase + k * 2.2;
            let x = self.x + 9.0 * arg.sin() * k;
            let cy = geo.rows - 1 - r;
            let first = ((x - 6.0) / f64::from(CELL_WIDTH)).floor().max(0.0) as usize;
            for cx in first..=((x + 6.0) / f64::from(CELL_WIDTH)).floor() as usize {
                if (centre(cx, cy).0 - x).abs() < f64::from(CELL_WIDTH) * 0.55 {
                    let slope = arg.cos();
                    let glyph = if slope > 0.3 {
                        '('
                    } else if slope < -0.3 {
                        ')'
                    } else {
                        '|'
                    };
                    let cell = Cell::tiled(
                        glyph,
                        (DARK, 0.8),
                        (WEED, 0.30 + 0.25 * (1.0 - k)),
                        true,
                        Owner::Weed(self.index),
                    );
                    out.push((cx, cy, cell));
                }
            }
        }
        out
    }
}

// ---------------------------------------------------------------- frame

/// Computes the scene at time `t` (seconds) with the texts of `keep_out`
/// (rectangles in pixels of the strip) kept clear.
pub fn render(scene: &Scene, t: f64, keep_out: &[Rect]) -> Vec<Cell> {
    build(scene, t, keep_out, true)
}

/// `finish` off gives the raw art: no fade of the ambient, every object at
/// full presence. The tests compare it with the finished frame.
fn build(scene: &Scene, t: f64, keep_out: &[Rect], finish: bool) -> Vec<Cell> {
    let geo = Geo::new(scene);
    let mut grid = Grid::new(geo.cols, geo.rows);
    ambient(&mut grid, &geo, t);
    if finish {
        fade_ambient(&mut grid, keep_out);
    }
    let presence = |bounds: &Rect, dive: bool| {
        if !finish {
            1.0
        } else if dive {
            carp_presence(bounds, keep_out)
        } else if clearing_gap(bounds, keep_out) > 0.0 {
            1.0
        } else {
            0.0
        }
    };

    if let Some(bounds) = geo.moon_box() {
        let k = presence(&bounds, false);
        if k > 0.0 {
            for cy in 0..geo.rows {
                for cx in 0..geo.cols {
                    if let Some(mut cell) = moon_cell(cx, cy, &geo) {
                        cell.scale(k);
                        grid.set(cx as i64, cy as i64, cell);
                    }
                }
            }
        }
    }
    for weed in weeds(&geo) {
        let k = presence(&weed.bounds(&geo), false);
        if k > 0.0 {
            for (cx, cy, mut cell) in weed.cells(&geo, t) {
                cell.scale(k);
                grid.set(cx as i64, cy as i64, cell);
            }
        }
    }
    let mut carps: Vec<Carp> = (0..scene.waiting.min(MAX_CARPS))
        .map(|index| Carp::new(index, &geo))
        .collect();
    carps.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    for carp in &carps {
        let Some(pose) = carp.pose(&geo, t) else {
            continue;
        };
        let k = presence(&pose.bounds, true);
        if k < 0.02 {
            continue;
        }
        let cw = f64::from(CELL_WIDTH);
        let ch = f64::from(CELL_HEIGHT);
        let (x0, x1) = (
            (pose.bounds.x0 / cw).floor().max(0.0) as usize,
            (pose.bounds.x1 / cw).ceil() as usize,
        );
        let (y0, y1) = (
            (pose.bounds.y0 / ch).floor().max(0.0) as usize,
            (pose.bounds.y1 / ch).ceil() as usize,
        );
        for cy in y0..y1.min(geo.rows) {
            for cx in x0..x1.min(geo.cols) {
                if let Some(mut cell) = carp.cell(&pose, cx, cy, t) {
                    cell.scale(k);
                    grid.set(cx as i64, cy as i64, cell);
                }
            }
        }
    }
    if let Some(bounds) = geo.mascot_box() {
        if presence(&bounds, false) > 0.0 {
            let first = geo.mascot_col();
            for cy in geo.surface - MASCOT_ROWS..geo.surface + 12 {
                for cx in first.saturating_sub(15)..geo.cols {
                    let (mc, mr) = (cx.wrapping_sub(first), cy + MASCOT_ROWS - geo.surface);
                    let body = (mc < MASCOT_COLS && mr < MASCOT_ROWS)
                        .then(|| mascot_cell(mc, mr, t))
                        .flatten();
                    if let Some(cell) = body.or_else(|| rig_cell(&geo, cx, cy)) {
                        grid.set(cx as i64, cy as i64, cell);
                    }
                }
            }
        }
    }
    grid.cells
}

/// The ambient fades by cell: out around the texts, and over the first and
/// last three columns. Objects are not touched.
fn fade_ambient(grid: &mut Grid, keep_out: &[Rect]) {
    let width = grid.width;
    for (index, cell) in grid.cells.iter_mut().enumerate() {
        if cell.owner != Owner::Ambient || *cell == Cell::BLANK {
            continue;
        }
        let (cx, cy) = (index % width, index / width);
        let (px, py) = centre(cx, cy);
        let edge = ((cx as f64 + 0.5) / EDGE_COLS as f64)
            .min((width as f64 - cx as f64 - 0.5) / EDGE_COLS as f64)
            .min(1.0);
        cell.scale(keep_factor(px, py, keep_out) * edge * edge);
    }
}

// ---------------------------------------------------------------- paint

/// A run: its length in bytes, its colour and level (None for blanks) and
/// whether it is bold.
type Run = (usize, Option<(Rgb, u8)>, bool);

/// Runs of one line: colour (None for blanks), alpha level and weight.
struct Line {
    text: String,
    runs: Vec<Run>,
}

fn lines(cells: &[Cell], width: usize) -> Vec<Line> {
    cells
        .chunks(width)
        .map(|row| {
            let mut text = String::with_capacity(width);
            let mut runs: Vec<Run> = Vec::new();
            for cell in row {
                let (ch, key, bold) = if cell.glyph_visible() {
                    (cell.ch, Some((cell.glyph, level(cell.alpha))), cell.bold)
                } else {
                    (' ', None, false)
                };
                text.push(ch);
                match runs.last_mut() {
                    Some(last) if last.1 == key && last.2 == bold => last.0 += ch.len_utf8(),
                    _ => runs.push((ch.len_utf8(), key, bold)),
                }
            }
            Line { text, runs }
        })
        .collect()
}

fn hsla(rgb: Rgb, alpha: f32) -> Hsla {
    Hsla::from(Rgba {
        r: f32::from(rgb[0]) / 255.0,
        g: f32::from(rgb[1]) / 255.0,
        b: f32::from(rgb[2]) / 255.0,
        a: alpha.clamp(0.0, 1.0),
    })
}

/// The two rectangles the texts take, measured in the window: "next" and the
/// pill. Written by the ticket while it paints, read by the scene.
pub type Clearings = Rc<RefCell<[Option<Bounds<Pixels>>; 2]>>;

fn paint_scene(
    scene: &Scene,
    t: f64,
    clearings: &Clearings,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut gpui::App,
) {
    let keep_out = {
        let measured = clearings.borrow();
        if measured[1].is_none() {
            scene.default_keep_out().to_vec()
        } else {
            measured
                .iter()
                .flatten()
                .map(|rect| Rect {
                    x0: f64::from(f32::from(rect.origin.x - bounds.origin.x)),
                    y0: f64::from(f32::from(rect.origin.y - bounds.origin.y)),
                    x1: f64::from(f32::from(rect.origin.x + rect.size.width - bounds.origin.x)),
                    y1: f64::from(f32::from(
                        rect.origin.y + rect.size.height - bounds.origin.y,
                    )),
                })
                .collect()
        }
    };
    let cells = render(scene, t, &keep_out);
    for (index, cell) in cells.iter().enumerate() {
        if let Some((rgb, alpha)) = cell.tile.filter(|_| cell.tile_visible()) {
            let (col, row) = ((index % scene.width) as f32, (index / scene.width) as f32);
            let origin = bounds.origin
                + point(
                    px(col * CELL_WIDTH + TILE_INSET),
                    px(row * CELL_HEIGHT + TILE_INSET),
                );
            window.paint_quad(quad(
                Bounds::new(origin, size(px(CELL_WIDTH - 1.0), px(CELL_HEIGHT - 1.0))),
                px(TILE_RADIUS),
                hsla(rgb, alpha),
                px(0.0),
                gpui::transparent_black(),
                gpui::BorderStyle::default(),
            ));
        }
    }
    let mono = Theme::font_mono();
    for (row, line) in lines(&cells, scene.width).into_iter().enumerate() {
        if line.runs.iter().all(|run| run.1.is_none()) {
            continue;
        }
        let runs: Vec<TextRun> = line
            .runs
            .iter()
            .map(|&(len, key, bold)| {
                let mut face = font(mono.clone());
                if bold {
                    face.weight = FontWeight::BOLD;
                }
                TextRun {
                    len,
                    font: face,
                    color: key.map_or(gpui::transparent_black(), |(rgb, step)| {
                        hsla(rgb, f32::from(step) / ALPHA_LEVELS)
                    }),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                }
            })
            .collect();
        let shaped = window.text_system().shape_line(
            SharedString::from(line.text),
            px(GLYPH_SIZE),
            &runs,
            Some(px(CELL_WIDTH)),
        );
        let _ = shaped.paint(
            bounds.origin + point(px(GLYPH_NUDGE), px(row as f32 * CELL_HEIGHT)),
            px(CELL_HEIGHT),
            TextAlign::Left,
            None,
            window,
            cx,
        );
    }
}

/// The lake as a view: knows how many decisions wait and where the texts
/// sit, and paints itself at the clock's pace.
pub struct NightField {
    scene: Scene,
    clearings: Clearings,
}

impl NightField {
    /// A view of `scene`.
    pub fn new(scene: Scene) -> Self {
        Self {
            scene,
            clearings: Clearings::default(),
        }
    }

    /// Sets how many decisions wait (one koi each, four at most). The parent
    /// is already repainting, so this does not notify.
    pub fn set_waiting(&mut self, waiting: usize) {
        self.scene.waiting = waiting;
    }

    /// Shares the rectangles of the texts the lake keeps clear.
    pub fn set_clearings(&mut self, clearings: &Clearings) {
        self.clearings = clearings.clone();
    }

    /// Fits the strip to `width` columns by `height` rows (a no-op when it
    /// already has them). The parent is repainting, so this does not notify.
    pub fn set_size(&mut self, width: usize, height: usize) {
        if (self.scene.width, self.scene.height) == (width, height) {
            return;
        }
        let waiting = self.scene.waiting;
        self.scene = Scene::strip_sized(width, height);
        self.scene.waiting = waiting;
    }

    /// Width and height in pixels of the whole grid.
    pub fn size(&self) -> (f32, f32) {
        (
            self.scene.width as f32 * CELL_WIDTH,
            self.scene.height as f32 * CELL_HEIGHT,
        )
    }
}

impl Render for NightField {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = if cx.reduce_motion() {
            STILL_TIME
        } else {
            f64::from(clock::seconds(cx.entity_id(), Rate::Calm, cx))
        };
        let scene = self.scene.clone();
        let clearings = self.clearings.clone();
        let (width, height) = self.size();
        canvas(
            |_, _, _| (),
            move |bounds, _, window, cx| paint_scene(&scene, t, &clearings, bounds, window, cx),
        )
        .w(px(width))
        .h(px(height))
        .flex_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZES: [(usize, usize); 3] = [(130, 35), (110, 40), (96, 35)];

    fn frames() -> impl Iterator<Item = f64> {
        (0..720).map(|k| f64::from(k) / 12.0)
    }

    fn scene(width: usize, height: usize, waiting: usize) -> Scene {
        Scene {
            waiting,
            ..Scene::strip_sized(width, height)
        }
    }

    /// The rectangle of a cell in pixels of the strip.
    fn cell_rect(index: usize, width: usize) -> Rect {
        let (cx, cy) = ((index % width) as f64, (index / width) as f64);
        let (w, h) = (f64::from(CELL_WIDTH), f64::from(CELL_HEIGHT));
        Rect {
            x0: cx * w,
            y0: cy * h,
            x1: (cx + 1.0) * w,
            y1: (cy + 1.0) * h,
        }
    }

    fn shifted(scene: &Scene) -> [Rect; 2] {
        let mut rects = scene.default_keep_out();
        rects[1].x0 -= 80.0;
        rects[1].x1 -= 80.0;
        rects
    }

    #[test]
    fn the_grid_has_the_size_of_the_scene() {
        for scene in [Scene::strip(), Scene::full()] {
            let cells = render(&scene, 3.0, &scene.default_keep_out());
            assert_eq!(cells.len(), scene.width * scene.height);
        }
    }

    #[test]
    fn a_frame_is_deterministic() {
        let scene = scene(130, 35, 4);
        let keep = scene.default_keep_out();
        assert_eq!(render(&scene, 12.5, &keep), render(&scene, 12.5, &keep));
        assert_ne!(render(&scene, 12.5, &keep), render(&scene, 20.0, &keep));
    }

    #[test]
    fn no_time_or_size_panics_or_yields_bad_alpha() {
        for scene in [scene(96, 35, 4), scene(130, 35, 9), scene(130, 40, 0)] {
            let keep = scene.default_keep_out();
            for t in [0.0, 0.1, 4.19, 8.7, 17.0, 123.456, 86_400.0] {
                for cell in render(&scene, t, &keep) {
                    assert!(cell.alpha.is_finite() && (0.0..=1.0 + 1e-3).contains(&cell.alpha));
                    if let Some((_, alpha)) = cell.tile {
                        assert!(alpha.is_finite() && (0.0..=1.0 + 1e-3).contains(&alpha));
                    }
                }
            }
        }
    }

    #[test]
    fn the_strip_is_as_light_as_the_reference() {
        let scene = scene(130, 35, 4);
        let cells = render(&scene, 2.4, &scene.default_keep_out());
        let lit = cells.iter().filter(|cell| cell.visible()).count();
        let share = lit as f64 / cells.len() as f64;
        assert!(share < 0.25, "{share}");
    }

    #[test]
    fn a_taller_strip_only_adds_sky() {
        let (short, tall) = (scene(130, 35, 4), scene(130, 40, 4));
        assert_eq!(tall.height - tall.surface, short.height - short.surface);
        let keep = tall.default_keep_out();
        let cells = render(&tall, 5.0, &keep);
        // The top rows are only stars: no object reaches them.
        for (index, cell) in cells.iter().enumerate().take(tall.width * 3) {
            assert!(!cell.visible() || cell.owner == Owner::Ambient, "{index}");
        }
    }

    #[test]
    fn the_strip_does_not_get_darker_when_taller() {
        let ink = |height| {
            let scene = scene(130, height, 4);
            let cells = render(&scene, 5.0, &scene.default_keep_out());
            cells
                .iter()
                .map(|cell| f64::from(level(cell.alpha)))
                .sum::<f64>()
                / cells.len() as f64
        };
        assert!(ink(40) <= ink(35), "{} > {}", ink(40), ink(35));
    }

    #[test]
    fn both_edges_fade_at_every_size() {
        for (width, height) in SIZES {
            let scene = scene(width, height, 0);
            let cells = render(&scene, 5.0, &scene.default_keep_out());
            for y in 0..height {
                let first = &cells[y * width];
                let last = &cells[y * width + width - 1];
                assert!(
                    first.alpha <= 0.2 && last.alpha <= 0.2,
                    "{width}x{height} row {y}"
                );
                assert!(first.tile.map_or(0.0, |t| t.1) <= 0.2);
                assert!(last.tile.map_or(0.0, |t| t.1) <= 0.2);
            }
        }
    }

    #[test]
    fn the_strip_fits_the_room_it_is_given() {
        assert_eq!(strip_dims(2000.0, 600.0), (STRIP_WIDTH, MAX_ROWS));
        assert_eq!(strip_dims(1000.0, 385.0), (100, 35));
        assert_eq!(strip_dims(1000.0, 374.0).1, 0);
        assert_eq!(strip_dims(900.0, 400.0).1, 0);
        let mut field = NightField::new(Scene::strip());
        field.set_waiting(3);
        field.set_size(110, 38);
        assert_eq!((field.scene.width, field.scene.height), (110, 38));
        assert_eq!(field.scene.surface, 38 - WATER_ROWS);
        assert_eq!(field.scene.waiting, 3);
    }

    #[test]
    fn carps_follow_the_waiting_decisions() {
        let keep: [Rect; 0] = [];
        for (waiting, expected) in [(0, 0), (1, 1), (3, 3), (4, 4), (9, 4)] {
            let scene = scene(130, 35, waiting);
            let mut seen = std::collections::BTreeSet::new();
            for cell in render(&scene, 2.4, &keep) {
                if let Owner::Carp(index) = cell.owner {
                    if cell.visible() {
                        seen.insert(index);
                    }
                }
            }
            assert_eq!(seen.len(), expected, "waiting {waiting}");
        }
    }

    /// The rule of the user: an object is whole or absent, never cut.
    #[test]
    fn objects_are_whole_or_absent() {
        for (width, height) in SIZES {
            let scene = scene(width, height, 4);
            for keep in [scene.default_keep_out(), shifted(&scene)] {
                for t in frames() {
                    let raw = build(&scene, t, &keep, false);
                    let done = build(&scene, t, &keep, true);
                    let mut ratios: std::collections::BTreeMap<String, Vec<f32>> =
                        Default::default();
                    for (before, after) in raw.iter().zip(&done) {
                        if before.owner == Owner::Ambient {
                            continue;
                        }
                        let entry = ratios.entry(format!("{:?}", before.owner)).or_default();
                        // A koi that dives shows what was behind it: it is gone.
                        let same = after.owner == before.owner;
                        if before.alpha > 0.05 {
                            entry.push(if same {
                                after.alpha / before.alpha
                            } else {
                                0.0
                            });
                        }
                        if let Some((_, a)) = before.tile.filter(|(_, a)| *a > 0.05) {
                            let b = after.tile.map_or(0.0, |t| t.1);
                            entry.push(if same { b / a } else { 0.0 });
                        }
                    }
                    for (owner, list) in ratios {
                        let (low, high) = list
                            .iter()
                            .fold((f32::MAX, f32::MIN), |(l, h), r| (l.min(*r), h.max(*r)));
                        assert!(
                            list.is_empty() || high - low < 1.0 / 16.0 + 1e-3,
                            "{width}x{height} t={t} {owner}: {low}..{high}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn objects_stay_inside_the_strip() {
        for (width, height) in SIZES {
            let scene = scene(width, height, 4);
            let keep = scene.default_keep_out();
            for t in frames() {
                for (index, cell) in render(&scene, t, &keep).iter().enumerate() {
                    if cell.owner != Owner::Ambient && cell.visible() {
                        let column = index % width;
                        assert!(
                            (EDGE_COLS..width - EDGE_COLS).contains(&column),
                            "{width}x{height} t={t} column {column} {:?}",
                            cell.owner
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_mascot_is_whole() {
        let art: usize = MASCOT
            .iter()
            .map(|row| row.chars().filter(|c| *c != '.').count())
            .sum();
        let scene = scene(130, 35, 0);
        let keep = scene.default_keep_out();
        for k in 0..600 {
            let t = f64::from(k) / 10.0;
            let tiles = render(&scene, t, &keep)
                .iter()
                .filter(|cell| cell.owner == Owner::Mascot && cell.tile_visible())
                .count();
            assert_eq!(tiles, art, "t={t}");
        }
    }

    #[test]
    fn reading_zone_stays_clear() {
        for (width, height) in SIZES {
            let scene = scene(width, height, 4);
            for keep in [scene.default_keep_out(), shifted(&scene)] {
                for t in frames() {
                    for (index, cell) in render(&scene, t, &keep).iter().enumerate() {
                        let rect = cell_rect(index, width);
                        if keep.iter().any(|k| rect.gap(k, 1.0) == 0.0) {
                            assert!(
                                !cell.visible(),
                                "{width}x{height} t={t} cell {index} on a text"
                            );
                        }
                        if cell.owner != Owner::Ambient && cell.visible() {
                            let gap = if matches!(cell.owner, Owner::Carp(_)) {
                                carp_gap(&rect, &keep)
                            } else {
                                clearing_gap(&rect, &keep)
                            };
                            assert!(
                                gap > 0.0,
                                "{width}x{height} t={t} {:?} in the clearing",
                                cell.owner
                            );
                        }
                        if cell.owner == Owner::Ambient
                            && keep.iter().any(|k| rect.gap(k, 1.0) < 20.0)
                        {
                            assert!(cell.alpha <= 0.15, "{width}x{height} t={t} cell {index}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_clearing_follows_the_measured_bounds() {
        let scene = scene(130, 35, 4);
        let base = scene.default_keep_out();
        let moved = shifted(&scene);
        let ink = |keep: &[Rect], area: &Rect| {
            let cells = render(&scene, 2.4, keep);
            (0..cells.len())
                .filter(|index| cell_rect(*index, scene.width).gap(area, 1.0) == 0.0)
                .filter(|index| cells[*index].visible())
                .count()
        };
        // Inside the pill at its old place, which the moved pill leaves free.
        let old = Rect {
            x0: base[1].x1 - 70.0,
            y0: base[1].y0 + 10.0,
            x1: base[1].x1 - 20.0,
            y1: base[1].y1 - 10.0,
        };
        assert_eq!(ink(&base, &old), 0);
        let new = Rect {
            x0: moved[1].x0 + 20.0,
            y0: moved[1].y0 + 10.0,
            x1: moved[1].x0 + 50.0,
            y1: moved[1].y1 - 10.0,
        };
        assert_eq!(ink(&moved, &new), 0);
    }

    #[test]
    fn runs_are_grouped_by_colour_level_and_weight() {
        let cell = |ch, alpha| Cell::glyph(ch, MUTED, alpha, Owner::Ambient);
        let cells = [
            cell('a', 0.5),
            cell('b', 0.51),
            Cell {
                bold: true,
                ..cell('c', 0.5)
            },
            cell(' ', 0.9),
            cell('d', 0.001),
        ];
        let rows = lines(&cells, 5);
        assert_eq!(rows[0].text, "abc  ");
        assert_eq!(rows[0].runs.len(), 3);
        assert_eq!(rows[0].runs[0].0, 2);
        assert_eq!(rows[0].runs.iter().map(|run| run.0).sum::<usize>(), 5);
    }
}
