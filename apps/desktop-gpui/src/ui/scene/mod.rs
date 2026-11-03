//! Tiled ASCII scenes: the nights painted cell by cell behind a screen.
//!
//! A scene is a grid of 10 × 11 px cells. Each cell has up to two layers: a
//! rounded tile (one `paint_quad`) and a glyph (one shaped line per row). A
//! [`SceneLayer`] writes its cells into a [`CellBuf`]; [`frame`] turns them
//! into what is painted, keeping the user's rule: **art is never cut**.
//!
//! Cells are of two kinds. *Ambient* ones (stars, ground, water) fade cell by
//! cell over the side and top bands. *Objects* (the mascot, a lantern, a
//! constellation, a comet) are drawn whole or not at all: an object with any
//! cell outside the grid or inside a fade band is dropped, and the fade never
//! touches an object's cell.

use std::rc::Rc;

use gpui::{
    canvas, font, point, px, quad, size, App, Bounds, Canvas, EntityId, FontWeight, Hsla, Pixels,
    Point, Rgba, SharedString, TextAlign, TextRun, Window,
};

use crate::ui::motion::clock::{self, Rate};
use crate::ui::theme::Theme;
pub use crate::ui::tokens::night::Rgb;
use crate::ui::tokens::night::{DARK, GLOW, HOOD, LAV, MUTED, PALE, VOID};

/// Width of a cell, in pixels.
pub const CELL_WIDTH: f32 = 10.0;
/// Height of a cell, in pixels.
pub const CELL_HEIGHT: f32 = 11.0;
/// A fixed instant for reduced motion.
pub const STILL_TIME: f64 = 2.4;
/// Font size of the glyphs and the labels, in pixels.
const GLYPH_SIZE: f32 = 10.0;
/// Horizontal nudge that centres a glyph in its cell.
const GLYPH_NUDGE: f32 = 2.0;
/// Advance of one character of a label (JetBrains Mono, 0.6 em at 10 px).
const LABEL_ADVANCE: f32 = 6.0;
/// Gap between a tile and its cell, and the radius of its corners.
const TILE_INSET: f32 = 0.5;
const TILE_RADIUS: f32 = 1.5;
/// Levels the alpha is quantized to.
const ALPHA_LEVELS: f32 = 16.0;
/// A tile fainter than this is not painted.
const TILE_MIN: f32 = 0.02;
/// Columns at each side where the ambient fades and no object goes.
pub const SIDE_FADE: usize = 3;
/// A tile stronger than this keeps a label away.
const LABEL_TILE_MAX: f32 = 0.15;

/// Who a cell belongs to. Ambient cells fade by cell; the rest are objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Owner {
    /// Stars, halo, surface, dust, ground, water.
    Ambient,
    /// The lake's moon disc.
    Moon,
    /// A koi of the lake, by index.
    Carp(u8),
    /// A weed of the lake, by index.
    Weed(u8),
    /// The lake's mascot with its rod, line and hook.
    Mascot,
    /// An object of the other scenes, by index (drawn in index order).
    Object(u16),
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
    /// The empty ambient cell.
    pub const BLANK: Cell = Cell {
        ch: ' ',
        glyph: MUTED,
        alpha: 0.0,
        tile: None,
        bold: false,
        owner: Owner::Ambient,
    };

    /// A glyph without a tile.
    pub fn glyph(ch: char, glyph: Rgb, alpha: f64, owner: Owner) -> Self {
        Self {
            ch,
            glyph,
            alpha: alpha as f32,
            tile: None,
            bold: false,
            owner,
        }
    }

    /// A glyph over a tile.
    pub fn tiled(ch: char, glyph: (Rgb, f64), tile: (Rgb, f64), bold: bool, owner: Owner) -> Self {
        Self {
            ch,
            glyph: glyph.0,
            alpha: glyph.1 as f32,
            tile: Some((tile.0, tile.1 as f32)),
            bold,
            owner,
        }
    }

    /// Whether the glyph shows.
    pub fn glyph_visible(&self) -> bool {
        self.ch != ' ' && level(self.alpha) > 0
    }

    /// Whether the tile shows.
    pub fn tile_visible(&self) -> bool {
        self.tile.is_some_and(|(_, alpha)| alpha > TILE_MIN)
    }

    /// Whether anything of the cell shows.
    pub fn visible(&self) -> bool {
        self.glyph_visible() || self.tile_visible()
    }

    /// Multiplies the opacity of the glyph and the tile by `factor`.
    pub fn scale(&mut self, factor: f64) {
        self.alpha *= factor as f32;
        if let Some((_, alpha)) = self.tile.as_mut() {
            *alpha *= factor as f32;
        }
    }
}

/// The alpha as one of the 16 levels (0 is invisible).
pub fn level(alpha: f32) -> u8 {
    (alpha.clamp(0.0, 1.0) * ALPHA_LEVELS).round() as u8
}

/// Smoothstep of `k` clamped to `[0, 1]`.
pub fn smooth(k: f64) -> f64 {
    let k = k.clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

/// `a` towards `b` by `k`.
pub fn mix(a: Rgb, b: Rgb, k: f64) -> Rgb {
    let channel =
        |i: usize| (f64::from(a[i]) + (f64::from(b[i]) - f64::from(a[i])) * k).round() as u8;
    [channel(0), channel(1), channel(2)]
}

/// An integer hash of a cell in `[0, 1]` (`h2` of the reference `cenas_v5.py`).
pub fn h2(x: i64, y: i64, s: i64) -> f64 {
    let n = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263))
        .wrapping_add(s.wrapping_mul(982_451_653)) as u32;
    let n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
    f64::from((n ^ (n >> 16)) & 0xFFFF) / 65535.0
}

/// Value noise over [`h2`].
pub fn noise(x: f64, y: f64, s: i64) -> f64 {
    let (xi, yi) = (x.floor(), y.floor());
    let (u, v) = (smooth(x - xi), smooth(y - yi));
    let (xi, yi) = (xi as i64, yi as i64);
    let (a, b) = (h2(xi, yi, s), h2(xi + 1, yi, s));
    let (c, d) = (h2(xi, yi + 1, s), h2(xi + 1, yi + 1, s));
    a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
}

/// The glyph of a straight stroke going `dx` across and `dy` down.
pub fn line_glyph(dx: f64, dy: f64) -> char {
    if dy.abs() < 0.35 * dx.abs() {
        '-'
    } else if dx.abs() < 0.35 * dy.abs() {
        '|'
    } else if dx * dy > 0.0 {
        '\\'
    } else {
        '/'
    }
}

// ---------------------------------------------------------------- mascot

/// Columns and rows of the mascot.
pub const MASCOT_COLS: usize = 11;
/// Rows of the mascot.
pub const MASCOT_ROWS: usize = 13;
/// The mascot, row by row.
pub const MASCOT: [&str; MASCOT_ROWS] = [
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

/// The mascot's cell at column `c` and row `r` of its art, at time `t`.
/// `look` is where it looks (-1 left, 0 ahead, 1 right); now and then it
/// glances away (left, or ahead when it already looks left).
pub fn mascot_cell(c: usize, r: usize, t: f64, look: i32, owner: Owner) -> Option<Cell> {
    let row = if r == 5 {
        let phase = t % 9.0;
        let glance = if look >= 0 { -1 } else { 0 };
        eye_row(if phase > 5.0 && phase < 6.6 {
            glance
        } else {
            look
        })
    } else {
        MASCOT[r]
    };
    let k = row.as_bytes()[c] as char;
    if k == '.' {
        return None;
    }
    let shade = 1.0 - 0.35 * c as f64 / (MASCOT_COLS - 1) as f64;
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

// ---------------------------------------------------------------- buffer

/// A cell an object wrote, with its column and row (maybe off the grid).
type Placed = (i64, i64, Cell);

/// The cells a [`SceneLayer`] writes: the ambient on a grid, each object
/// apart, with every write kept (also those off the grid) so [`frame`] can
/// tell a whole object from a cut one.
pub struct CellBuf {
    cols: usize,
    rows: usize,
    ambient: Vec<Cell>,
    objects: Vec<(Owner, Vec<Placed>)>,
}

impl CellBuf {
    /// An empty buffer of `cols` × `rows`.
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            ambient: vec![Cell::BLANK; cols * rows],
            objects: Vec::new(),
        }
    }

    /// Columns of the grid.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Rows of the grid.
    pub fn rows(&self) -> usize {
        self.rows
    }

    fn index(&self, cx: i64, cy: i64) -> Option<usize> {
        (cx >= 0 && cy >= 0 && (cx as usize) < self.cols && (cy as usize) < self.rows)
            .then(|| cy as usize * self.cols + cx as usize)
    }

    /// Writes `cell` at (`cx`, `cy`). An ambient cell off the grid is
    /// dropped; an object's is kept, and it drops the whole object.
    pub fn set(&mut self, cx: i64, cy: i64, cell: Cell) {
        if cell.owner == Owner::Ambient {
            if let Some(index) = self.index(cx, cy) {
                self.ambient[index] = cell;
            }
            return;
        }
        let slot = match self.objects.iter().position(|(o, _)| *o == cell.owner) {
            Some(slot) => slot,
            None => {
                self.objects.push((cell.owner, Vec::new()));
                self.objects.len() - 1
            }
        };
        let cells = &mut self.objects[slot].1;
        cells.retain(|&(x, y, _)| (x, y) != (cx, cy));
        cells.push((cx, cy, cell));
    }

    /// Writes an ambient `cell` only where the ambient is still empty.
    pub fn set_if_blank(&mut self, cx: i64, cy: i64, cell: Cell) {
        if let Some(index) = self.index(cx, cy) {
            if self.ambient[index] == Cell::BLANK {
                self.ambient[index] = cell;
            }
        }
    }

    /// The ambient cell at (`cx`, `cy`), when it is on the grid.
    pub fn ambient_mut(&mut self, cx: i64, cy: i64) -> Option<&mut Cell> {
        self.index(cx, cy).map(|index| &mut self.ambient[index])
    }

    /// The mascot whole at column `mc`, row `mr` (its top left corner).
    pub fn put_mascot(&mut self, mc: i64, mr: i64, t: f64, look: i32, owner: Owner) {
        for r in 0..MASCOT_ROWS {
            for c in 0..MASCOT_COLS {
                if let Some(cell) = mascot_cell(c, r, t, look, owner) {
                    self.set(mc + c as i64, mr + r as i64, cell);
                }
            }
        }
    }

    /// A straight stroke of `glyph` (or the one that fits its slope) from `a`
    /// to `b`, without its ends, over cells still empty.
    pub fn put_line(&mut self, a: (i64, i64), b: (i64, i64), cell: Cell) {
        let n = (b.0 - a.0).abs().max((b.1 - a.1).abs());
        let ch = if cell.ch == ' ' {
            line_glyph(
                (b.0 - a.0) as f64,
                (b.1 - a.1) as f64 * f64::from(CELL_HEIGHT) / f64::from(CELL_WIDTH),
            )
        } else {
            cell.ch
        };
        for i in 1..n {
            let k = i as f64 / n as f64;
            let x = (a.0 as f64 + (b.0 - a.0) as f64 * k).round() as i64;
            let y = (a.1 as f64 + (b.1 - a.1) as f64 * k).round() as i64;
            let taken = self.objects.iter().any(|(owner, cells)| {
                *owner == cell.owner && cells.iter().any(|&(cx, cy, _)| (cx, cy) == (x, y))
            });
            if cell.owner == Owner::Ambient {
                self.set_if_blank(x, y, Cell { ch, ..cell });
            } else if !taken {
                self.set(x, y, Cell { ch, ..cell });
            }
        }
    }
}

// ---------------------------------------------------------------- layer

/// A label of a scene: running text (not on the grid), mono 10.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneLabel {
    /// Column and row where the text starts.
    pub at: (i64, i64),
    /// The whole text, then the short one tried when it does not fit.
    pub texts: Vec<String>,
    /// Colour and opacity.
    pub color: (Rgb, f32),
}

/// A label placed by [`frame`]: the text that fits, where it starts.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedLabel {
    /// Column and row where the text starts.
    pub at: (usize, usize),
    /// The text.
    pub text: String,
    /// Colour and opacity.
    pub color: (Rgb, f32),
}

/// A scene drawn behind a screen.
pub trait SceneLayer {
    /// The smallest grid (columns, rows) the scene draws in; under it the
    /// scene is not drawn at all.
    fn min_size(&self) -> (usize, usize);

    /// Rows of the top fade band, as the spec gives it (the frame keeps it
    /// under a third of the rows).
    fn fade_top(&self) -> usize;

    /// Columns of each side's fade band.
    fn side_fade(&self) -> usize {
        SIDE_FADE
    }

    /// Writes the cells at time `t` (seconds) over the grid of `buf`.
    fn cells(&self, t: f64, buf: &mut CellBuf);

    /// The labels over a grid of `cols` × `rows`.
    fn labels(&self, _cols: usize, _rows: usize) -> Vec<SceneLabel> {
        Vec::new()
    }
}

/// What is painted: the cells, row by row, and the labels that fit.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// Columns.
    pub cols: usize,
    /// Rows.
    pub rows: usize,
    /// The cells, row by row.
    pub cells: Vec<Cell>,
    /// The labels that fit whole.
    pub labels: Vec<PlacedLabel>,
}

/// The bands of a grid where the ambient fades and no object goes.
#[derive(Clone, Copy, Debug)]
struct Bands {
    cols: usize,
    rows: usize,
    side: usize,
    top: usize,
}

impl Bands {
    fn of(layer: &dyn SceneLayer, cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            side: layer.side_fade(),
            top: layer.fade_top().min(rows / 3),
        }
    }

    /// Whether (`cx`, `cy`) is on the grid and out of every band.
    fn clear(&self, cx: i64, cy: i64) -> bool {
        let (side, top) = (self.side as i64, self.top as i64);
        cx >= side && cx < self.cols as i64 - side && cy >= top && cy < self.rows as i64
    }

    /// The ambient's opacity at (`cx`, `cy`).
    fn mask(&self, cx: usize, cy: usize) -> f64 {
        let mut mask = 1.0;
        if self.side > 0 {
            let edge = cx.min(self.cols - 1 - cx) as f64 + 0.5;
            mask *= smooth(edge / self.side as f64);
        }
        if self.top > 0 {
            mask *= smooth((cy as f64 + 0.5) / self.top as f64);
        }
        mask
    }
}

/// The scene at time `t` over `cols` × `rows`, or `None` under its minimum.
pub fn frame(layer: &dyn SceneLayer, t: f64, cols: usize, rows: usize) -> Option<Frame> {
    let (min_cols, min_rows) = layer.min_size();
    if cols < min_cols.max(1) || rows < min_rows.max(1) {
        return None;
    }
    let bands = Bands::of(layer, cols, rows);
    let mut buf = CellBuf::new(cols, rows);
    layer.cells(t, &mut buf);
    let mut cells = buf.ambient;
    for (index, cell) in cells.iter_mut().enumerate() {
        if *cell == Cell::BLANK {
            continue;
        }
        let mask = bands.mask(index % cols, index / cols);
        if mask < 1.0 {
            cell.scale(mask);
        }
    }
    for (_, object) in &buf.objects {
        if object.iter().all(|&(cx, cy, _)| bands.clear(cx, cy)) {
            for &(cx, cy, cell) in object {
                cells[cy as usize * cols + cx as usize] = cell;
            }
        }
    }
    let labels = layer
        .labels(cols, rows)
        .into_iter()
        .filter_map(|label| place(&label, &cells, &bands))
        .collect();
    Some(Frame {
        cols,
        rows,
        cells,
        labels,
    })
}

/// Cells a text of `chars` characters takes.
fn label_span(chars: usize) -> i64 {
    (chars as f32 * LABEL_ADVANCE / CELL_WIDTH).ceil() as i64
}

/// The first text of `label` that fits whole with a cell to spare around it,
/// out of the bands and off any strong tile.
fn place(label: &SceneLabel, cells: &[Cell], bands: &Bands) -> Option<PlacedLabel> {
    let (x, y) = label.at;
    let text = label.texts.iter().find(|text| {
        let span = label_span(text.chars().count());
        (y - 1..=y + 1).all(|cy| {
            (x - 1..=x + span).all(|cx| {
                bands.clear(cx, cy)
                    && cells[cy as usize * bands.cols + cx as usize]
                        .tile
                        .is_none_or(|(_, alpha)| alpha <= LABEL_TILE_MAX)
            })
        })
    })?;
    Some(PlacedLabel {
        at: (x as usize, y as usize),
        text: text.clone(),
        color: label.color,
    })
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

fn tile_color(rgb: Rgb, alpha: f32) -> Hsla {
    Hsla::from(Rgba {
        r: f32::from(rgb[0]) / 255.0,
        g: f32::from(rgb[1]) / 255.0,
        b: f32::from(rgb[2]) / 255.0,
        a: alpha.clamp(0.0, 1.0),
    })
}

fn text_run(len: usize, bold: bool, color: Hsla) -> TextRun {
    let mut face = font(Theme::font_mono());
    if bold {
        face.weight = FontWeight::BOLD;
    }
    TextRun {
        len,
        font: face,
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    }
}

/// Paints `cells` (`cols` per row) with their top left corner at `origin`.
pub fn paint_cells(
    cells: &[Cell],
    cols: usize,
    origin: Point<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    for (index, cell) in cells.iter().enumerate() {
        if let Some((rgb, alpha)) = cell.tile.filter(|_| cell.tile_visible()) {
            let (col, row) = ((index % cols) as f32, (index / cols) as f32);
            let at = origin
                + point(
                    px(col * CELL_WIDTH + TILE_INSET),
                    px(row * CELL_HEIGHT + TILE_INSET),
                );
            window.paint_quad(quad(
                Bounds::new(at, size(px(CELL_WIDTH - 1.0), px(CELL_HEIGHT - 1.0))),
                px(TILE_RADIUS),
                tile_color(rgb, alpha),
                px(0.0),
                gpui::transparent_black(),
                gpui::BorderStyle::default(),
            ));
        }
    }
    for (row, line) in lines(cells, cols).into_iter().enumerate() {
        if line.runs.iter().all(|run| run.1.is_none()) {
            continue;
        }
        let runs: Vec<TextRun> = line
            .runs
            .iter()
            .map(|&(len, key, bold)| {
                let color = key.map_or(gpui::transparent_black(), |(rgb, step)| {
                    tile_color(rgb, f32::from(step) / ALPHA_LEVELS)
                });
                text_run(len, bold, color)
            })
            .collect();
        let shaped = window.text_system().shape_line(
            SharedString::from(line.text),
            px(GLYPH_SIZE),
            &runs,
            Some(px(CELL_WIDTH)),
        );
        let _ = shaped.paint(
            origin + point(px(GLYPH_NUDGE), px(row as f32 * CELL_HEIGHT)),
            px(CELL_HEIGHT),
            TextAlign::Left,
            None,
            window,
            cx,
        );
    }
}

/// Paints a frame: its cells, then its labels as running text.
pub fn paint_frame(frame: &Frame, origin: Point<Pixels>, window: &mut Window, cx: &mut App) {
    paint_cells(&frame.cells, frame.cols, origin, window, cx);
    for label in &frame.labels {
        let (rgb, alpha) = label.color;
        let runs = [text_run(label.text.len(), false, tile_color(rgb, alpha))];
        let shaped = window.text_system().shape_line(
            SharedString::from(label.text.clone()),
            px(GLYPH_SIZE),
            &runs,
            None,
        );
        let (cx0, cy0) = (label.at.0 as f32, label.at.1 as f32);
        let _ = shaped.paint(
            origin + point(px(cx0 * CELL_WIDTH), px(cy0 * CELL_HEIGHT)),
            px(CELL_HEIGHT),
            TextAlign::Left,
            None,
            window,
            cx,
        );
    }
}

/// The grid that fits `bounds`: columns, rows and the top left corner. The
/// spare width is split between the sides; the spare height goes on top, so
/// the scene rests on the bottom.
pub fn grid_in(bounds: Bounds<Pixels>) -> (usize, usize, Point<Pixels>) {
    let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    let cols = (w / CELL_WIDTH).floor().max(0.0) as usize;
    let rows = (h / CELL_HEIGHT).floor().max(0.0) as usize;
    let dx = ((w - cols as f32 * CELL_WIDTH) / 2.0).floor();
    let dy = h - rows as f32 * CELL_HEIGHT;
    (cols, rows, bounds.origin + point(px(dx), px(dy)))
}

/// The time of a scene for `view`: the clock's, or a fixed instant when the
/// user asks for reduced motion.
pub fn scene_time(view: EntityId, cx: &mut App) -> f64 {
    if cx.reduce_motion() {
        STILL_TIME
    } else {
        f64::from(clock::seconds(view, Rate::Calm, cx))
    }
}

/// A canvas that paints `layer` at time `t` over whatever room it is given,
/// recomputing the grid from its bounds on every paint.
pub fn scene_canvas(layer: Rc<dyn SceneLayer>, t: f64) -> Canvas<()> {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| {
            let (cols, rows, origin) = grid_in(bounds);
            if let Some(frame) = frame(layer.as_ref(), t, cols, rows) {
                paint_frame(&frame, origin, window, cx);
            }
        },
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A small generator with a fixed seed for the property tests.
    pub(crate) struct Seeded(u64);

    impl Seeded {
        pub(crate) fn new(seed: u64) -> Self {
            Self(seed)
        }

        /// A number in `low..high`.
        pub(crate) fn below(&mut self, low: usize, high: usize) -> usize {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            low + ((self.0 >> 33) as usize) % (high - low).max(1)
        }
    }

    /// The user's rule over 200 random grids and instants: every object is
    /// whole or absent, inside the grid and out of the fade bands.
    pub(crate) fn assert_never_clipped(
        layer: &dyn SceneLayer,
        cols: (usize, usize),
        rows: (usize, usize),
    ) {
        let mut rng = Seeded::new(0x5CE7E);
        for _ in 0..200 {
            let (c, r) = (rng.below(cols.0, cols.1), rng.below(rows.0, rows.1));
            let t = rng.below(0, 72_000) as f64 / 100.0;
            let Some(done) = frame(layer, t, c, r) else {
                continue;
            };
            let bands = Bands::of(layer, c, r);
            let mut buf = CellBuf::new(c, r);
            layer.cells(t, &mut buf);
            for (owner, object) in &buf.objects {
                let shown = object
                    .iter()
                    .filter(|&&(x, y, cell)| {
                        bands.clear(x, y) && done.cells[y as usize * c + x as usize] == cell
                    })
                    .count();
                // Another object drawn later may cover part of this one.
                let covered = object.iter().any(|&(x, y, _)| {
                    bands.clear(x, y) && {
                        let other = done.cells[y as usize * c + x as usize].owner;
                        other != Owner::Ambient && other != *owner
                    }
                });
                assert!(
                    shown == 0 || shown == object.len() || covered,
                    "{c}x{r} t={t} {owner:?}: {shown} of {} cells",
                    object.len()
                );
            }
            for (index, cell) in done.cells.iter().enumerate() {
                if cell.owner != Owner::Ambient && cell.visible() {
                    let (x, y) = ((index % c) as i64, (index / c) as i64);
                    assert!(
                        bands.clear(x, y),
                        "{c}x{r} t={t} {:?} in a band",
                        cell.owner
                    );
                }
            }
        }
    }

    struct Probe;

    impl SceneLayer for Probe {
        fn min_size(&self) -> (usize, usize) {
            (10, 6)
        }

        fn fade_top(&self) -> usize {
            3
        }

        fn cells(&self, t: f64, buf: &mut CellBuf) {
            for cx in 0..buf.cols() as i64 {
                buf.set(
                    cx,
                    buf.rows() as i64 - 1,
                    Cell::glyph('~', MUTED, 1.0, Owner::Ambient),
                );
            }
            // A box that slides across the grid and off it.
            let x = (t * 3.0) as i64 % (buf.cols() as i64 + 6) - 3;
            for dx in 0..3 {
                for dy in 0..2 {
                    let cell = Cell::tiled(' ', (DARK, 0.0), (PALE, 0.8), false, Owner::Object(0));
                    buf.set(x + dx, 3 + dy, cell);
                }
            }
        }

        fn labels(&self, _cols: usize, _rows: usize) -> Vec<SceneLabel> {
            vec![SceneLabel {
                at: (4, 4),
                texts: vec!["a long label here".into(), "short".into()],
                color: (MUTED, 0.7),
            }]
        }
    }

    #[test]
    fn scene_objects_are_never_clipped() {
        assert_never_clipped(&Probe, (8, 40), (4, 20));
    }

    #[test]
    fn a_grid_under_the_minimum_draws_nothing() {
        assert!(frame(&Probe, 1.0, 9, 10).is_none());
        assert!(frame(&Probe, 1.0, 10, 5).is_none());
        assert!(frame(&Probe, 1.0, 10, 6).is_some());
    }

    #[test]
    fn the_ambient_fades_on_the_sides_and_on_top() {
        let done = frame(&Probe, 0.0, 20, 12).expect("drawn");
        let bottom = |cx: usize| done.cells[11 * 20 + cx].alpha;
        assert!(bottom(0) < 0.1 && bottom(19) < 0.1);
        assert!((bottom(10) - 1.0).abs() < 1e-6);
        assert!(bottom(1) < bottom(2) && bottom(2) < bottom(3));
    }

    #[test]
    fn a_label_shortens_then_goes() {
        let label = |cols| {
            let done = frame(&Probe, 100.0, cols, 12).expect("drawn");
            done.labels.first().map(|label| label.text.clone())
        };
        assert_eq!(label(40).as_deref(), Some("a long label here"));
        assert_eq!(label(12).as_deref(), Some("short"));
        assert_eq!(label(10), None);
    }

    #[test]
    fn the_grid_rests_on_the_bottom_and_centres_across() {
        let bounds = Bounds::new(point(px(100.0), px(50.0)), size(px(205.0), px(60.0)));
        let (cols, rows, origin) = grid_in(bounds);
        assert_eq!((cols, rows), (20, 5));
        assert_eq!(origin, point(px(102.0), px(55.0)));
    }

    const H2_REFERENCE: [(i64, i64, i64, f64); 5] = [
        (0, 0, 0, 0.0),
        (5, 9, 3, 0.22525368123903258),
        (-7, 3, 41, 0.13585107194628823),
        (119, 35, 7, 0.46424048218509195),
        (-1, -1, 45, 0.9770046540016785),
    ];

    #[test]
    fn h2_matches_the_reference() {
        // Values of `h2` in cenas_v5.py.
        for (x, y, s, value) in H2_REFERENCE {
            assert!((h2(x, y, s) - value).abs() < 1e-9, "h2({x}, {y}, {s})");
        }
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
