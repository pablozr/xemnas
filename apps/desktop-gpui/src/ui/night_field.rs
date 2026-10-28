//! "Noturno": a living ASCII night landscape behind the review.
//!
//! The scene is a grid of `(char, alpha, tone)` computed from a handful of
//! numbers and the time (a direct port of the design reference `noturno.js`,
//! v2). It is drawn as one line of text per row, with runs grouped by
//! (tone, alpha quantized to 16 levels): text and colour only, no blur,
//! gradient or transform. `NightField` is the only thing the screens know:
//! they give it the day's progress and ask for a shooting star.
//!
//! Layers, back to front: stars, crescent moon, clouds, the arc of today (one
//! star per decision), birds, two parallax ridgelines with mist, water with
//! the reflection, the cliff with its grass, the fisher (Nº I), fireflies and
//! the shooting star. Masks fade the edges and open holes behind the texts.

use std::ops::Range;

use gpui::{
    div, px, Context, FontWeight, HighlightStyle, Hsla, IntoElement, ParentElement, Render,
    SharedString, Styled, StyledText, Window,
};

use crate::ui::motion::clock::{self, Rate};
use crate::ui::theme::Theme;

/// Advance of a JetBrains Mono cell at the strip's 12 px, in pixels.
pub const CELL_WIDTH: f32 = 7.2;
/// Line height of the strip, in pixels.
pub const CELL_HEIGHT: f32 = 13.0;
/// A fixed instant for reduced motion: the moon high, the fisher awake.
const STILL_TIME: f64 = 9.0;
/// How long a shooting star lasts after it is fired, in seconds.
const SHOOTING_SECONDS: f64 = 1.6;
/// Levels the alpha is quantized to.
const ALPHA_LEVELS: f32 = 16.0;

/// Which colour a cell takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Quiet grey (`text_muted`).
    Ground,
    /// Lavender (`accent_emphasis`).
    Accent,
    /// Amber (`status_warning`): the fisher's eyes, the bobber, fireflies.
    Warn,
}

/// One cell of the grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    /// The glyph.
    pub ch: char,
    /// Opacity, from 0 to 1.
    pub alpha: f32,
    /// The colour.
    pub tone: Tone,
}

impl Cell {
    const BLANK: Cell = Cell {
        ch: ' ',
        alpha: 0.0,
        tone: Tone::Ground,
    };

    fn visible(&self) -> bool {
        self.ch != ' ' && level(self.alpha) > 0
    }
}

/// The numbers that shape a scene (everything but time and progress).
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    /// Columns.
    pub width: usize,
    /// Rows.
    pub height: usize,
    /// Rows of water at the bottom.
    pub water: usize,
    /// Moon: column, row, radius.
    pub moon: (f64, f64, f64),
    /// Number of stars.
    pub stars: usize,
    /// Decisions of the day: done and total (the arc has one star each).
    pub done: usize,
    /// Decisions of the day, all told.
    pub total: usize,
    /// Arc of the day: centre column, centre row, radius.
    pub arc: (f64, f64, f64),
    /// Height of the back and the front ridgeline.
    pub mountains: (f64, f64),
    /// Parallax speed of the back and the front ridgeline.
    pub parallax: (f64, f64),
    /// Speed of the clouds.
    pub wind: f64,
    /// Row of the birds.
    pub birds: f64,
    /// Cliff: column of its edge, height.
    pub cliff: (f64, f64),
    /// Number of fireflies.
    pub flies: usize,
    /// Where the shooting star starts: column, row.
    pub shoot: (f64, f64),
    /// Columns that fade at each side.
    pub edge: f64,
    /// Vertical scale of the scene (1 in the strip, about 3 full screen).
    pub scale: f64,
    /// Rectangles left empty behind texts: first column, last column, first
    /// row, last row.
    pub holes: Vec<(f64, f64, f64, f64)>,
}

impl Scene {
    /// The strip between the ticket and the bottom of the review.
    pub fn strip() -> Self {
        Self {
            width: 198,
            height: 18,
            water: 5,
            moon: (30.0, 5.0, 3.6),
            stars: 110,
            done: 0,
            total: 10,
            arc: (118.0, 7.0, 22.0),
            mountains: (8.0, 5.0),
            parallax: (0.25, 0.6),
            wind: 1.2,
            birds: 2.0,
            cliff: (166.0, 4.0),
            flies: 5,
            shoot: (40.0, 0.0),
            edge: 14.0,
            scale: 1.0,
            holes: vec![(66.0, 134.0, 10.0, 17.0)],
        }
    }

    /// The whole screen, behind the empty queue.
    pub fn full() -> Self {
        Self {
            width: 200,
            height: 66,
            water: 15,
            moon: (40.0, 13.0, 8.0),
            stars: 300,
            done: 10,
            total: 10,
            arc: (100.0, 34.0, 44.0),
            mountains: (24.0, 15.0),
            parallax: (0.3, 0.8),
            wind: 1.5,
            birds: 8.0,
            cliff: (158.0, 9.0),
            flies: 9,
            shoot: (24.0, 4.0),
            edge: 18.0,
            scale: 2.6,
            holes: vec![(72.0, 128.0, 23.0, 34.0)],
        }
    }
}

/// The alpha as one of the 16 levels (0 is invisible).
fn level(alpha: f32) -> u8 {
    (alpha.clamp(0.0, 1.0) * ALPHA_LEVELS).round() as u8
}

/// A small deterministic generator (mulberry32), so a scene has the same
/// stars every time.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6D2B_79F5);
        let mut q = (self.0 ^ (self.0 >> 15)).wrapping_mul(1 | self.0);
        q = q.wrapping_add((q ^ (q >> 7)).wrapping_mul(61 | q)) ^ q;
        f64::from(q ^ (q >> 14)) / 4_294_967_296.0
    }
}

fn clamp(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

/// `Math.round`: halves go up.
fn round(value: f64) -> i64 {
    (value + 0.5).floor() as i64
}

/// A glyph of a density ramp, picked by `value`.
fn ramp(chars: &str, value: f64, low: usize, high: usize) -> char {
    let index = round(value).clamp(low as i64, high as i64) as usize;
    chars.chars().nth(index).unwrap_or(' ')
}

struct Grid {
    width: i64,
    height: i64,
    cells: Vec<Cell>,
}

impl Grid {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width: width as i64,
            height: height as i64,
            cells: vec![Cell::BLANK; width * height],
        }
    }

    fn index(&self, x: i64, y: i64) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.width && y < self.height)
            .then(|| (y * self.width + x) as usize)
    }

    fn set(&mut self, x: f64, y: f64, ch: char, alpha: f64, tone: Tone) {
        if let Some(index) = self.index(round(x), round(y)) {
            self.cells[index] = Cell {
                ch,
                alpha: alpha as f32,
                tone,
            };
        }
    }

    fn get(&self, x: i64, y: i64) -> Cell {
        self.index(x, y)
            .map_or(Cell::BLANK, |index| self.cells[index])
    }
}

/// How a ridgeline is painted.
struct Ridge {
    edge: f64,
    fill: f64,
    lit: f64,
}

fn ridge_height(x: f64, s: f64, amp: f64, base: f64, scale: f64) -> f64 {
    base - amp
        * (0.58 * (1.0 - (x * 0.026 / scale * s + 1.3 * s).sin().abs())
            + 0.30 * (1.0 - (x * 0.071 / scale * s + 0.7).sin().abs())
            + 0.12 * (x * 0.29 / scale + s).sin())
}

/// Computes the scene at time `t` (seconds). `shooting` is the age of a
/// shooting star, when one is in flight.
pub fn render(scene: &Scene, t: f64, shooting: Option<f64>) -> Vec<Cell> {
    let (w, h) = (scene.width as i64, scene.height as i64);
    let mut grid = Grid::new(scene.width, scene.height);
    let hz = (h - scene.water as i64) as f64;
    let (mx, _, mr) = scene.moon;
    let s = scene.scale;
    let wf = w as f64;

    stars(&mut grid, scene, t, hz);
    moon(&mut grid, scene, t);
    clouds(&mut grid, scene, t, hz);
    arc(&mut grid, scene, t);

    // Birds: a flock crosses every 17 s, flapping.
    let flight = (t % 17.0) / 17.0;
    for k in 0..5 {
        let kf = f64::from(k);
        let x = -12.0 + flight * (wf + 30.0) - kf * 3.2 - f64::from(k % 2) * 1.5;
        let y = scene.birds
            + (flight * 7.0 + kf).sin() * 0.8 * s
            + f64::from(k % 2)
            + (kf - 2.0).abs() * 0.6;
        let flap = (t * 5.0 + kf).floor() as i64 % 2 != 0;
        grid.set(x, y, if flap { 'v' } else { '-' }, 0.42, Tone::Ground);
    }

    // Ridgelines shaded by the moon, with mist between them.
    ridgeline(
        &mut grid,
        scene,
        t * scene.parallax.0,
        1.0,
        scene.mountains.0,
        hz,
        &Ridge {
            edge: 0.32,
            fill: 0.06,
            lit: 0.24,
        },
    );
    let mist_top = (hz - scene.mountains.1 * 0.9).floor() as i64;
    for y in mist_top..hz as i64 {
        for x in 0..w {
            let xf = x as f64;
            let v = ((xf - t * scene.wind * 0.6) * 0.06 + y as f64 * 0.8).sin()
                + ((xf + t * scene.wind * 0.3) * 0.023 - y as f64 * 0.5).sin();
            if v > 1.25 && grid.get(x, y).ch == ' ' {
                let ch = if v > 1.6 { '~' } else { '-' };
                grid.set(xf, y as f64, ch, 0.06 + (v - 1.25) * 0.10, Tone::Ground);
            }
        }
    }
    ridgeline(
        &mut grid,
        scene,
        40.0 + t * scene.parallax.1,
        1.7,
        scene.mountains.1,
        hz + 0.4,
        &Ridge {
            edge: 0.52,
            fill: 0.08,
            lit: 0.44,
        },
    );

    // Water: wavy reflection of everything above, the moon's path, ripples.
    for y in hz as i64..h {
        let yf = y as f64;
        let k = (yf - hz) / scene.water as f64;
        for x in 0..w {
            let xf = x as f64;
            let wobble = round((yf * 1.9 + t * 2.2 + xf * 0.05).sin() * (1.0 + k * 2.0));
            let source = grid.get(x + wobble, 2 * hz as i64 - 1 - y);
            let (mut ch, mut alpha, mut tone) = (' ', 0.0, Tone::Ground);
            if source.ch != ' ' && source.alpha > 0.08 {
                ch = if source.tone == Tone::Accent {
                    '='
                } else {
                    '-'
                };
                alpha = f64::from(source.alpha) * 0.38 * (1.0 - k * 0.6);
                tone = source.tone;
            }
            let n = (xf * 0.17 + t * 0.9 + yf * 2.3).sin()
                * (xf * 0.043 - t * 0.4 + yf * 1.1).sin()
                * 1.6
                + ((xf * 0.61 + yf * 5.3) + t * 1.7).sin() * 0.55;
            let path = 1.4 + k * mr * 2.4;
            let dx = (xf - mx).abs();
            if dx < path && n > -0.7 {
                ch = if n > 0.5 { '=' } else { '-' };
                alpha = alpha.max((0.5 - k * 0.25) * (1.0 - dx / path * 0.6));
                tone = Tone::Accent;
            } else if n > 1.15 && alpha < 0.1 {
                ch = '~';
                alpha = 0.11 + 0.07 * (1.0 - k);
            } else if n > 0.7 && alpha < 0.06 {
                ch = '-';
                alpha = 0.07;
            }
            if alpha > 0.0 {
                grid.set(xf, yf, ch, alpha, tone);
            } else {
                grid.set(xf, yf, ' ', 0.0, Tone::Ground);
            }
        }
    }

    let (cliff_x, cliff_h) = scene.cliff;
    let top = hz - cliff_h;
    cliff(&mut grid, scene, t, hz, top);
    fisher(&mut grid, scene, t, hz, top);

    // Fireflies over the water and the cliff.
    for i in 0..scene.flies {
        let i = i as f64;
        let x = cliff_x - 8.0
            + (t * 0.37 + i * 2.1).sin() * (10.0 + i * 2.0) * s * 0.6
            + (t * 1.3 + i).sin() * 2.0;
        let y = top - 2.0 + (t * 0.53 + i * 1.7).sin() * 2.5 * s + (t * 0.9 + i).cos() * 0.8;
        let p = 0.5 + 0.5 * (t * 2.6 + i * 1.9).sin();
        if p > 0.25 {
            grid.set(
                x,
                y,
                if p > 0.75 { '•' } else { '·' },
                0.35 + p * 0.55,
                Tone::Warn,
            );
        }
    }

    if let Some(age) = shooting.filter(|age| (0.0..SHOOTING_SECONDS).contains(age)) {
        shooting_star(&mut grid, scene, age);
    }

    mask(&mut grid, scene);
    grid.cells
}

fn stars(grid: &mut Grid, scene: &Scene, t: f64, hz: f64) {
    let (mx, my, mr) = scene.moon;
    let mut rng = Rng(17);
    for _ in 0..scene.stars {
        let x = rng.next() * grid.width as f64;
        let y = rng.next() * (hz - 2.0 * scene.scale);
        let b = rng.next();
        let phase = rng.next() * std::f64::consts::TAU;
        if ((x - mx) / 1.8).hypot(y - my) < mr + 2.0 {
            continue;
        }
        let twinkle = 0.5 + 0.5 * (t * (0.5 + b * 1.4) + phase).sin();
        let ch = if b > 0.9 {
            if twinkle > 0.8 {
                '*'
            } else if twinkle > 0.4 {
                '+'
            } else {
                '·'
            }
        } else if b > 0.55 {
            '·'
        } else {
            '.'
        };
        let alpha = (0.06 + b * 0.24) * (0.45 + 0.55 * twinkle) * (1.0 - y / hz * 0.5);
        grid.set(x, y, ch, alpha, Tone::Ground);
    }
}

fn moon(grid: &mut Grid, scene: &Scene, t: f64) {
    let (mx, my, mr) = scene.moon;
    let (y0, y1) = (
        (my - mr * 1.8).floor() as i64,
        (my + mr * 1.8).floor() as i64,
    );
    let (x0, x1) = (
        (mx - mr * 3.4).floor() as i64,
        (mx + mr * 3.4).floor() as i64,
    );
    for y in y0..=y1 {
        for x in x0..=x1 {
            let (xf, yf) = (x as f64, y as f64);
            let d = ((xf - mx) / 1.8).hypot(yf - my) / mr;
            let shadow = ((xf - mx - mr * 0.62 * 1.8) / 1.8).hypot(yf - my + mr * 0.22) / mr;
            if d < 1.0 && shadow > 1.0 {
                let v = ((shadow - 1.0) * 2.2).min(1.0) * (1.0 - d.powi(6) * 0.5);
                grid.set(
                    xf,
                    yf,
                    ramp(" .:+*#%@", v * 7.0, 1, 7),
                    0.34 + v * 0.6,
                    Tone::Accent,
                );
            } else if d < 1.0 {
                if (x * 5 + y * 3) % 7 == 0 {
                    grid.set(xf, yf, '.', 0.09, Tone::Accent);
                }
            } else if d < 1.9 {
                let glow = (1.9 - d) * 0.11 * (0.8 + 0.2 * (t * 0.8).sin());
                if (x * 7 + y * 13) % 4 == 0 {
                    grid.set(xf, yf, '·', glow, Tone::Accent);
                }
            }
        }
    }
}

fn clouds(grid: &mut Grid, scene: &Scene, t: f64, hz: f64) {
    let (mx, my, mr) = scene.moon;
    let s = scene.scale;
    let drift = t * scene.wind;
    let mut y = 1;
    while (y as f64) < hz * 0.55 {
        let yf = y as f64;
        for x in 0..grid.width {
            let xf = x as f64;
            let u = xf - drift;
            let v = (u * 0.035 + yf * 0.9 / s).sin() * (u * 0.083 - yf * 0.4 / s + 1.7).sin()
                + 0.45 * (u * 0.19 + yf * 1.3 / s).sin()
                - 0.18 * (yf - hz * 0.28).abs() / s;
            if v < 0.74 {
                continue;
            }
            let near = ((xf - mx) / 1.8).hypot(yf - my) / mr;
            let lit = clamp(1.8 - near * 0.55);
            let ch = if v > 1.0 {
                '='
            } else if v > 0.8 {
                '~'
            } else {
                '-'
            };
            if lit > 0.05 {
                grid.set(xf, yf, ch, 0.10 + lit * 0.30, Tone::Accent);
            } else {
                grid.set(xf, yf, ch, 0.07 + (v - 0.74) * 0.14, Tone::Ground);
            }
        }
        y += 1;
    }
}

/// The arc of today: one star per decision.
fn arc(grid: &mut Grid, scene: &Scene, t: f64) {
    let (cx, cy, cr) = scene.arc;
    let (done, total) = (scene.done.min(scene.total), scene.total.max(2));
    let angle = |i: usize| std::f64::consts::PI * (1.08 - i as f64 / (total - 1) as f64 * 1.16);
    let at = |i: usize| (cx + angle(i).cos() * cr, cy - angle(i).sin() * cr * 0.42);
    for i in 0..total {
        let (x, y) = at(i);
        if i + 1 < total {
            let (x2, y2) = at(i + 1);
            let n = (x2 - x).hypot(y2 - y).ceil() as i64;
            let reached = i + 1 < done;
            let mut step = 1;
            while step < n {
                let f = step as f64 / n as f64;
                let (alpha, tone) = if reached {
                    (0.34, Tone::Accent)
                } else {
                    (0.08, Tone::Ground)
                };
                grid.set(x + (x2 - x) * f, y + (y2 - y) * f, '·', alpha, tone);
                step += 2;
            }
        }
        if i < done {
            grid.set(
                x,
                y,
                '*',
                0.62 + 0.2 * (t * 1.7 + i as f64).sin(),
                Tone::Accent,
            );
        } else if i == done {
            let ch = ['+', '✦', '*', '✦'][(t * 4.0).floor() as usize % 4];
            grid.set(x, y, ch, 0.95, Tone::Accent);
        } else {
            grid.set(x, y, '·', 0.22, Tone::Ground);
        }
    }
}

fn ridgeline(
    grid: &mut Grid,
    scene: &Scene,
    offset: f64,
    stretch: f64,
    amp: f64,
    base: f64,
    paint: &Ridge,
) {
    let (mx, ..) = scene.moon;
    let w = grid.width;
    let hz = (grid.height - scene.water as i64) as f64;
    let heights: Vec<f64> = (-1..=w)
        .map(|x| ridge_height(x as f64 + offset, stretch, amp, base, scene.scale))
        .collect();
    for x in 0..w {
        let at = (x + 1) as usize;
        let (m, left, right) = (heights[at], heights[at - 1], heights[at + 1]);
        let slope = right - left;
        let top = m.ceil() as i64;
        let neighbour = left.ceil().max(right.ceil()) as i64;
        let toward = if mx < x as f64 { -slope } else { slope };
        let lit = clamp(toward * 1.4 + 0.25);
        let edge = if slope < -0.5 {
            '/'
        } else if slope > 0.5 {
            '\\'
        } else {
            '_'
        };
        for y in top..hz as i64 {
            if y <= top.max(neighbour - 1) {
                let ch = if edge == '_' && y > top { '|' } else { edge };
                grid.set(
                    x as f64,
                    y as f64,
                    ch,
                    paint.edge * (0.6 + 0.4 * lit),
                    Tone::Ground,
                );
                continue;
            }
            let depth = (y as f64 - m) / (amp * 0.55);
            let v = lit * (-depth * 1.4).exp();
            let noise = ((x * 7 + y * 11) % 5) as f64 / 5.0;
            if v + noise * 0.18 < 0.12 {
                grid.set(x as f64, y as f64, ' ', 0.0, Tone::Ground);
                continue;
            }
            grid.set(
                x as f64,
                y as f64,
                ramp(" .·:-=+", v * 6.0 + noise, 1, 6),
                paint.fill + v * paint.lit,
                Tone::Ground,
            );
        }
    }
}

fn cliff(grid: &mut Grid, scene: &Scene, t: f64, hz: f64, top: f64) {
    let (cx, ch) = scene.cliff;
    let s = scene.scale;
    let w = grid.width;
    let mut y = top.round() as i64;
    while y < hz as i64 + 1 {
        let yf = y as f64;
        let edge = cx + ((yf - top) / ch).powf(1.6) * ch * 1.1;
        for x in edge.floor() as i64..w {
            let xf = x as f64;
            if xf < edge + 1.0 {
                grid.set(xf, yf, '\\', 0.42, Tone::Ground);
                continue;
            }
            let v = clamp(1.0 - (xf - edge) / (14.0 * s));
            let noise = (((xf * 12.9898 + yf * 78.233).sin() * 43758.5453) % 1.0 + 1.0) % 1.0;
            grid.set(
                xf,
                yf,
                ramp(" .:-=", v * 3.0 + noise, 0, 4),
                0.10 + v * 0.22,
                Tone::Ground,
            );
        }
        y += 1;
    }
    // Grass in the wind.
    let mut x = cx.ceil() as i64;
    while x < w {
        let xf = x as f64;
        let wind = (xf * 1.3 + t * 2.4).sin() + (t * 0.7 + xf * 0.11).sin() * 1.4;
        let blade = if wind > 1.2 {
            '/'
        } else if wind < -1.2 {
            '\\'
        } else if x % 3 != 0 {
            ','
        } else {
            '|'
        };
        grid.set(xf, top - 1.0, blade, 0.34 + 0.1 * xf.sin(), Tone::Ground);
        x += 1;
    }
    if s > 1.5 {
        let mut x = (cx + 4.0).ceil() as i64;
        while x < w {
            let xf = x as f64;
            let wind = (xf * 0.9 + t * 2.0 + 1.0).sin();
            let (shift, blade) = if wind > 0.6 {
                (1.0, '/')
            } else if wind < -0.6 {
                (-1.0, '\\')
            } else {
                (0.0, '|')
            };
            grid.set(xf + shift, top - 2.0, blade, 0.22, Tone::Ground);
            x += 3;
        }
    }
}

/// Nº I sitting on the edge, fishing: rod, line, bobber, ripples.
fn fisher(grid: &mut Grid, scene: &Scene, t: f64, hz: f64, top: f64) {
    let s = scene.scale;
    let big = s > 1.5;
    let px = round(scene.cliff.0) as f64 + if big { 10.0 } else { 6.0 };
    let py = top - 1.0;
    let blink = t % 4.2 < 0.16;
    let look = t % 11.0 > 8.5;
    let figure: &[&str] = if big {
        &[
            "    .-\"\"\"\"-.    ",
            "   /  .--.  \\   ",
            "  /  /    \\  \\  ",
            if look {
                " |  |(ee)  |  | "
            } else {
                " |  | (ee) |  | "
            },
            "  |  \\    /  |  ",
            "   \\  `--´  /   ",
            "  __\\      /__  ",
            " /   |    |   \\ ",
        ]
    } else {
        &[
            "  .--.  ",
            " / __ \\ ",
            if look { "|(ee)  |" } else { "| (ee) |" },
            " \\    / ",
            "_/|  |\\_",
        ]
    };
    for (j, row) in figure.iter().enumerate() {
        let row: Vec<char> = row.chars().collect();
        let first = row.iter().position(|c| *c != ' ');
        let last = row.iter().rposition(|c| *c != ' ');
        for (i, &ch) in row.iter().enumerate() {
            let x = px - if big { 8.0 } else { 4.0 } + i as f64;
            let y = py - figure.len() as f64 + j as f64;
            if ch == ' ' {
                if first.is_some_and(|a| i > a) && last.is_some_and(|b| i < b) {
                    grid.set(x, y, ' ', 0.0, Tone::Ground);
                }
                continue;
            }
            if ch == 'e' {
                grid.set(x, y, if blink { '-' } else { '•' }, 1.0, Tone::Warn);
            } else {
                grid.set(x, y, ch, 0.66, Tone::Ground);
            }
        }
    }
    let tip_x = px - if big { 19.0 } else { 10.0 };
    let tip_y = py - 2.0 - if big { 6.0 } else { 3.0 };
    let hand_x = px - if big { 7.0 } else { 4.0 };
    let length = if big { 12 } else { 6 };
    for k in 1..=length {
        let ch = if k < length { '/' } else { '·' };
        grid.set(
            hand_x - f64::from(k),
            py - 2.0 - f64::from(k) * 0.5,
            ch,
            0.40,
            Tone::Ground,
        );
    }
    let bob = (t * 2.0).sin() * 0.4 + if t % 6.0 < 0.35 { 0.6 } else { 0.0 };
    let by = hz + round(bob) as f64;
    for y in round(tip_y) + 1..round(by) {
        grid.set(tip_x, y as f64, '¦', 0.26, Tone::Ground);
    }
    grid.set(tip_x, by, 'o', 0.95, Tone::Warn);
    for k in 0..2 {
        let age = ((t + f64::from(k) * 1.5) % 3.0) / 3.0;
        let reach = 1.5 + age * 9.0 * s;
        let alpha = 0.32 * (1.0 - age);
        for side in [-1.0, 1.0] {
            let ch = if side < 0.0 { '(' } else { ')' };
            grid.set(tip_x + side * reach, by, ch, alpha, Tone::Ground);
            grid.set(
                tip_x + side * reach * 0.6,
                by - 1.0,
                '-',
                alpha * 0.6,
                Tone::Ground,
            );
            grid.set(
                tip_x + side * reach * 0.6,
                by + 1.0,
                '-',
                alpha * 0.6,
                Tone::Ground,
            );
        }
    }
}

/// A shooting star with sparks that stay behind; `age` in seconds.
fn shooting_star(grid: &mut Grid, scene: &Scene, age: f64) {
    let (sx, sy) = scene.shoot;
    let drop = 10.0 * scene.scale.min(1.6);
    let p = (age / 1.1).min(1.0);
    if age < 1.1 {
        let (hx, hy) = (sx + p * 50.0, sy + p * drop);
        for k in 0..12 {
            let kf = f64::from(k);
            let ch = match k {
                0 => '*',
                1..=3 => '=',
                _ => '-',
            };
            let tone = if k < 4 { Tone::Accent } else { Tone::Ground };
            grid.set(
                hx - kf * 1.6,
                hy - kf * 0.32,
                ch,
                (1.0 - kf / 12.0) * 0.85,
                tone,
            );
        }
    }
    for k in 0..6 {
        let kf = f64::from(k);
        let fx = sx + (kf / 6.0) * p * 50.0 + f64::from((k * 13) % 5 - 2);
        let fy = sy + (kf / 6.0) * p * drop + f64::from((k * 7) % 3 - 1);
        let fade = clamp(1.0 - (age - kf * 0.15) / 1.4);
        if fade > 0.0 {
            grid.set(fx, fy, '·', 0.4 * fade, Tone::Accent);
        }
    }
}

/// Edge fade, top fade and the holes behind texts and actions.
fn mask(grid: &mut Grid, scene: &Scene) {
    let (w, h) = (grid.width, grid.height);
    for y in 0..h {
        for x in 0..w {
            let index = (y * w + x) as usize;
            if grid.cells[index].ch == ' ' {
                continue;
            }
            let (xf, yf) = (x as f64, y as f64);
            let mut m = (xf.min((w - 1 - x) as f64) / scene.edge).min(1.0);
            m *= ((yf + 1.0) / 3.0).min(1.0);
            for &(x0, x1, y0, y1) in &scene.holes {
                if yf >= y0 && yf <= y1 && xf > x0 - 8.0 && xf < x1 + 8.0 {
                    let dx = if xf < x0 {
                        (x0 - xf) / 8.0
                    } else if xf > x1 {
                        (xf - x1) / 8.0
                    } else {
                        0.0
                    };
                    let dy = if yf < y0 + 2.0 {
                        (y0 + 2.0 - yf) / 3.0
                    } else {
                        0.0
                    };
                    m *= (0.10 + 0.90 * dx.max(dy)).min(1.0);
                }
            }
            grid.cells[index].alpha *= m as f32;
        }
    }
}

/// One line of text and the colour runs that paint it.
struct Line {
    text: String,
    runs: Vec<(Range<usize>, Tone, u8)>,
}

/// Rows of text with runs grouped by (tone, alpha level).
fn lines(cells: &[Cell], width: usize) -> Vec<Line> {
    cells
        .chunks(width)
        .map(|row| {
            let mut text = String::with_capacity(width);
            let mut runs: Vec<(Range<usize>, Tone, u8)> = Vec::new();
            for cell in row {
                let start = text.len();
                if !cell.visible() {
                    text.push(' ');
                    continue;
                }
                text.push(cell.ch);
                let (tone, step) = (cell.tone, level(cell.alpha));
                match runs.last_mut() {
                    Some(last) if last.1 == tone && last.2 == step && last.0.end == start => {
                        last.0.end = text.len();
                    }
                    _ => runs.push((start..text.len(), tone, step)),
                }
            }
            Line { text, runs }
        })
        .collect()
}

/// The landscape as a view: gives it the progress of the day, fires shooting
/// stars, and paints itself at the clock's pace.
pub struct NightField {
    scene: Scene,
    /// Clock time at which the last shooting star was fired.
    fired: Option<f64>,
}

impl NightField {
    /// A view of `scene`.
    pub fn new(scene: Scene) -> Self {
        Self { scene, fired: None }
    }

    /// Sets how many decisions of the day are done out of how many. The
    /// parent is already repainting, so this does not notify.
    pub fn set_today(&mut self, done: usize, total: usize) {
        self.scene.done = done;
        self.scene.total = total;
    }

    /// Fires a shooting star (a confirmed decision).
    pub fn shoot(&mut self, cx: &mut Context<Self>) {
        if cx.reduce_motion() {
            return;
        }
        self.fired = Some(f64::from(clock::seconds(cx.entity_id(), Rate::Calm, cx)));
        cx.notify();
    }

    /// Width and height in pixels of the whole grid.
    pub fn size(&self) -> (f32, f32) {
        (
            self.scene.width as f32 * CELL_WIDTH,
            self.scene.height as f32 * CELL_HEIGHT,
        )
    }

    fn color(theme: &Theme, tone: Tone, step: u8) -> Hsla {
        let base: Hsla = match tone {
            Tone::Ground => theme.colors.text_muted().into(),
            Tone::Accent => theme.colors.accent_emphasis().into(),
            Tone::Warn => theme.colors.status_warning().into(),
        };
        Hsla {
            a: f32::from(step) / ALPHA_LEVELS,
            ..base
        }
    }
}

impl Render for NightField {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let (t, shooting) = if cx.reduce_motion() {
            (STILL_TIME, None)
        } else {
            let now = f64::from(clock::seconds(cx.entity_id(), Rate::Calm, cx));
            (now, self.fired.map(|fired| now - fired))
        };
        if shooting.is_some_and(|age| age >= SHOOTING_SECONDS) {
            self.fired = None;
        }
        let cells = render(&self.scene, t, shooting);
        let (width, _) = self.size();
        div()
            .w(px(width))
            .flex_none()
            .flex()
            .flex_col()
            .font_family(Theme::font_mono())
            .font_weight(FontWeight::NORMAL)
            .text_size(px(12.0))
            .line_height(px(CELL_HEIGHT))
            .text_color(theme.colors.text_muted())
            .children(lines(&cells, self.scene.width).into_iter().map(|line| {
                let highlights = line
                    .runs
                    .into_iter()
                    .map(|(range, tone, step)| {
                        (
                            range,
                            HighlightStyle {
                                color: Some(Self::color(&theme, tone, step)),
                                ..HighlightStyle::default()
                            },
                        )
                    })
                    .collect::<Vec<_>>();
                div()
                    .h(px(CELL_HEIGHT))
                    .flex_none()
                    .whitespace_nowrap()
                    .child(
                        StyledText::new(SharedString::from(line.text)).with_highlights(highlights),
                    )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(cells: &[Cell], width: usize) -> String {
        lines(cells, width)
            .into_iter()
            .map(|line| line.text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_grid_has_the_size_of_the_scene() {
        for scene in [Scene::strip(), Scene::full()] {
            let cells = render(&scene, 3.0, None);
            assert_eq!(cells.len(), scene.width * scene.height);
        }
    }

    #[test]
    fn a_frame_is_deterministic() {
        let scene = Scene::strip();
        assert_eq!(render(&scene, 12.5, None), render(&scene, 12.5, None));
        assert_ne!(render(&scene, 12.5, None), render(&scene, 20.0, None));
    }

    #[test]
    fn no_time_or_size_panics_or_yields_bad_alpha() {
        let mut small = Scene::strip();
        small.width = 12;
        small.height = 6;
        small.water = 2;
        for scene in [small, Scene::strip(), Scene::full()] {
            for t in [0.0, 0.1, 4.19, 8.7, 17.0, 123.456, 86_400.0] {
                for cell in render(&scene, t, Some(0.5)) {
                    assert!(
                        cell.alpha.is_finite() && cell.alpha >= 0.0 && cell.alpha <= 1.0 + 1e-3
                    );
                }
            }
        }
    }

    #[test]
    fn the_edges_fade_out() {
        let scene = Scene::strip();
        let cells = render(&scene, 5.0, None);
        for y in 0..scene.height {
            assert!(!cells[y * scene.width].visible(), "left edge, row {y}");
        }
    }

    #[test]
    fn the_hole_behind_the_actions_is_quiet() {
        let scene = Scene::strip();
        let cells = render(&scene, 5.0, None);
        let loudest = |x0: usize, x1: usize| {
            (12..scene.height)
                .flat_map(|y| (x0..x1).map(move |x| (x, y)))
                .map(|(x, y)| cells[y * scene.width + x].alpha)
                .fold(0.0_f32, f32::max)
        };
        assert!(loudest(68, 132) < 0.1, "{}", loudest(68, 132));
    }

    #[test]
    fn a_shooting_star_adds_cells_and_ends() {
        let scene = Scene::strip();
        let count = |shooting| {
            render(&scene, 5.0, shooting)
                .iter()
                .filter(|c| c.visible())
                .count()
        };
        assert_ne!(count(Some(0.8)), count(None));
        assert_eq!(count(Some(SHOOTING_SECONDS + 1.0)), count(None));
    }

    #[test]
    fn the_arc_marks_the_decision_in_course() {
        let mut scene = Scene::full();
        scene.done = 3;
        scene.total = 10;
        let frame = text(&render(&scene, 1.0, None), scene.width);
        assert!(frame.contains('✦') || frame.contains('+'));
    }

    #[test]
    fn runs_are_grouped_by_tone_and_level() {
        let cell = |ch, alpha, tone| Cell { ch, alpha, tone };
        let cells = [
            cell('a', 0.5, Tone::Ground),
            cell('b', 0.51, Tone::Ground),
            cell('c', 0.5, Tone::Accent),
            cell(' ', 0.9, Tone::Ground),
            cell('d', 0.001, Tone::Ground),
        ];
        let rows = lines(&cells, 5);
        assert_eq!(rows[0].text, "abc  ");
        assert_eq!(rows[0].runs.len(), 2);
        assert_eq!(rows[0].runs[0].0, 0..2);
    }
}
