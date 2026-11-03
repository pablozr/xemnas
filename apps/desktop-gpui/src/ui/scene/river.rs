//! The river of the Context (`spec-lote2.md` §3.2, §4.5): each delivery of
//! the last seven days is a paper lantern the mascot lets go from the pier;
//! its height follows the tokens. The items the budget left out stay unlit
//! on the pier.

use super::{h2, noise, Cell, CellBuf, Owner, SceneLayer, MASCOT_ROWS, SIDE_FADE};
use crate::ui::tokens::night::{AMBER, DARK, GLOW, MUTED, ROCK, SLATE};

/// Under these the river is not drawn.
const MIN_COLS: usize = 40;
const MIN_ROWS: usize = 8;
/// Top fade of the spec.
const FADE_TOP: usize = 3;
/// From these rows on, the pier with the mascot shows (when the mascot also
/// clears the top fade).
const PIER_ROWS: usize = 17;
/// Under these columns, at most three lanterns float at once.
const NARROW_COLS: usize = 60;
const NARROW_LANTERNS: usize = 3;
/// Columns of the pier (0 to 15), its posts, and the mascot's column.
const PIER_END: i64 = 16;
const POSTS: [i64; 2] = [0, 14];
const MASCOT_COL: i64 = SIDE_FADE as i64;
/// Unlit lanterns shown, at most, and where they sit.
const UNLIT: usize = 2;
const UNLIT_ON_PIER: [i64; UNLIT] = [14, 15];
const UNLIT_ON_BANK: [i64; UNLIT] = [SIDE_FADE as i64, SIDE_FADE as i64 + 2];
/// Drift in cells per second, the lantern's width, the room each needs and
/// how long it takes to light up or go out at the ends of its drift.
const DRIFT: f64 = 1.6;
const WIDTH: i64 = 2;
const ROOM: f64 = 6.0;
const FADE_SECONDS: f64 = 0.6;
/// Tokens of the tallest lantern and of a bright one.
const TALL_TOKENS: f64 = 300.0;
const BRIGHT_TOKENS: usize = 250;

/// The river, with the tokens of each delivery (newest first) and the items
/// left out by the budget.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct River {
    tokens: Vec<usize>,
    unlit: usize,
}

impl River {
    /// A river for `tokens` (one lantern per delivery, newest first) and
    /// `omitted` items left out by the budget.
    pub fn new(tokens: Vec<usize>, omitted: usize) -> Self {
        Self {
            tokens,
            unlit: omitted.min(UNLIT),
        }
    }

    /// The row of the water line.
    fn water(rows: usize) -> i64 {
        rows as i64 - 4
    }

    /// Whether the pier with the mascot shows: enough rows, and the mascot
    /// stands whole under the top fade.
    pub fn has_pier(&self, rows: usize) -> bool {
        let fade = FADE_TOP.min(rows / 3) as i64;
        rows >= PIER_ROWS && Self::water(rows) >= MASCOT_ROWS as i64 + fade
    }

    fn ambient(cols: usize, rows: usize, t: f64, buf: &mut CellBuf) {
        let water = Self::water(rows);
        let ambient = Owner::Ambient;
        for cx in 0..cols as i64 {
            for cy in 0..water - 1 {
                if h2(cx, cy, 41) > 0.975 {
                    buf.set(cx, cy, Cell::glyph('.', MUTED, 0.30, ambient));
                }
            }
            // The far bank.
            let top = water - 1 - (2.0 * noise(cx as f64 / 11.0, 0.3, 42)) as i64;
            for cy in top..water {
                if (cx + cy) % 2 == 0 && h2(cx, cy, 43) > 0.5 {
                    buf.set(cx, cy, Cell::glyph('.', MUTED, 0.25, ambient));
                }
            }
            let x = cx as f64;
            let wave = (x * 0.45 - t * 1.2).sin() + (x * 0.17 + t * 0.6).sin();
            if wave > 0.9 {
                buf.set(cx, water, Cell::glyph('~', MUTED, 0.38, ambient));
            } else if h2(cx, 0, 44) > 0.6 {
                buf.set(cx, water, Cell::glyph('-', MUTED, 0.22, ambient));
            }
            for cy in water + 1..rows as i64 {
                if h2(cx + (t * 2.0) as i64, cy, 45) > 0.93 {
                    buf.set(cx, cy, Cell::glyph('-', MUTED, 0.16, ambient));
                }
            }
        }
    }

    fn pier(&self, rows: usize, t: f64, buf: &mut CellBuf) {
        let water = Self::water(rows);
        for cx in 0..PIER_END {
            buf.set(
                cx,
                water,
                Cell::tiled('=', (MUTED, 0.35), (ROCK, 0.55), false, Owner::Ambient),
            );
        }
        for cx in POSTS {
            for cy in water + 1..rows as i64 {
                buf.set(
                    cx,
                    cy,
                    Cell::tiled('|', (MUTED, 0.35), (ROCK, 0.30), false, Owner::Ambient),
                );
            }
        }
        let top = water - MASCOT_ROWS as i64;
        buf.put_mascot(MASCOT_COL, top, t, 1, Owner::Object(0));
        for (index, cx) in UNLIT_ON_PIER.into_iter().take(self.unlit).enumerate() {
            let owner = Owner::Object(1 + index as u16);
            buf.set(
                cx,
                water - 1,
                Cell::tiled(':', (MUTED, 0.35), (SLATE, 0.18), false, owner),
            );
            buf.set(cx, water - 2, Cell::glyph('-', MUTED, 0.35, owner));
        }
    }

    fn bank(&self, rows: usize, buf: &mut CellBuf) {
        for (index, cx) in UNLIT_ON_BANK.into_iter().take(self.unlit).enumerate() {
            let owner = Owner::Object(1 + index as u16);
            buf.set(
                cx,
                Self::water(rows) - 1,
                Cell::glyph('▫', MUTED, 0.35, owner),
            );
        }
    }

    /// The lanterns afloat: each drifts over `[start, cols − 3 − 2]`, lights
    /// up as it enters and goes out before the end.
    fn lanterns(&self, cols: usize, rows: usize, start: i64, t: f64, buf: &mut CellBuf) {
        let water = Self::water(rows);
        let end = cols as i64 - SIDE_FADE as i64 - WIDTH;
        let span = (end - start) as f64;
        if span < ROOM {
            return;
        }
        let most = if cols < NARROW_COLS {
            NARROW_LANTERNS
        } else {
            (span / ROOM) as usize
        };
        let shown = self.tokens.len().min(most);
        for (index, &tokens) in self.tokens.iter().take(shown).enumerate() {
            let i = index as f64;
            let along = (i * span / shown as f64 + t * DRIFT) % span;
            let presence = (along / DRIFT / FADE_SECONDS)
                .min((span - along) / DRIFT / FADE_SECONDS)
                .min(1.0);
            let cx = start + along as i64;
            let floors = (1 + (2.0 * tokens as f64 / TALL_TOKENS).round() as i64).min(3);
            let flick = (0.85 + 0.15 * (t * 3.1 + i * 1.3).sin()) * presence;
            let bright = tokens > BRIGHT_TOKENS;
            let (paper, alpha) = if bright { (GLOW, 0.85) } else { (AMBER, 0.70) };
            let owner = Owner::Object(10 + index as u16);
            let base = water - 1;
            for k in 0..floors {
                let ch = if k == floors - 1 { ':' } else { ' ' };
                for dx in 0..WIDTH {
                    let cell = Cell::tiled(
                        ch,
                        (DARK, 0.5 * presence),
                        (paper, alpha * flick),
                        true,
                        owner,
                    );
                    buf.set(cx + dx, base - k, cell);
                }
            }
            for dx in 0..WIDTH {
                buf.set(
                    cx + dx,
                    base - floors,
                    Cell::glyph('-', AMBER, 0.6 * presence, owner),
                );
                for k in 1..=floors {
                    let fade = 1.0 / k as f64;
                    let reflection = Cell::tiled(
                        '~',
                        (AMBER, 0.35 * fade * presence),
                        (AMBER, 0.12 * fade * presence),
                        false,
                        owner,
                    );
                    buf.set(cx + dx, water + k, reflection);
                }
            }
        }
    }
}

impl SceneLayer for River {
    fn min_size(&self) -> (usize, usize) {
        (MIN_COLS, MIN_ROWS)
    }

    fn fade_top(&self) -> usize {
        FADE_TOP
    }

    fn cells(&self, t: f64, buf: &mut CellBuf) {
        let (cols, rows) = (buf.cols(), buf.rows());
        Self::ambient(cols, rows, t, buf);
        let start = if self.has_pier(rows) {
            self.pier(rows, t, buf);
            PIER_END + 2
        } else {
            self.bank(rows, buf);
            UNLIT_ON_BANK[UNLIT - 1] + 2
        };
        self.lanterns(cols, rows, start, t, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::scene::frame;
    use crate::ui::scene::tests::assert_never_clipped;

    fn river() -> River {
        River::new(vec![214, 128, 296, 180, 150], 2)
    }

    #[test]
    fn scene_objects_are_never_clipped() {
        assert_never_clipped(&river(), (30, 160), (4, 40));
    }

    #[test]
    fn the_river_goes_under_its_minimum() {
        assert!(frame(&river(), 2.4, 39, 17).is_none());
        assert!(frame(&river(), 2.4, 98, 7).is_none());
        assert!(frame(&river(), 2.4, 40, 8).is_some());
    }

    #[test]
    fn the_pier_needs_room_for_the_whole_mascot() {
        let river = river();
        assert!(!river.has_pier(16));
        // 17 rows put the water on row 13: the mascot's hood would sit in
        // the three rows of the top fade, so the pier waits for 20.
        assert!(!river.has_pier(17));
        assert!(river.has_pier(20));
    }

    fn objects(cols: usize, rows: usize, t: f64) -> std::collections::BTreeSet<Owner> {
        frame(&river(), t, cols, rows)
            .map(|done| {
                done.cells
                    .iter()
                    .filter(|cell| cell.owner != Owner::Ambient && cell.visible())
                    .map(|cell| cell.owner)
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn the_mascot_and_the_unlit_lanterns_follow_the_rows() {
        let tall = objects(98, 24, 2.4);
        assert!(tall.contains(&Owner::Object(0)));
        assert!(tall.contains(&Owner::Object(1)) && tall.contains(&Owner::Object(2)));
        let short = objects(98, 15, 2.4);
        assert!(!short.contains(&Owner::Object(0)));
        assert!(short.contains(&Owner::Object(1)) && short.contains(&Owner::Object(2)));
    }

    #[test]
    fn a_narrow_river_floats_three_lanterns_at_most() {
        for k in 0..60 {
            let t = f64::from(k) / 4.0;
            let floating = objects(55, 15, t)
                .into_iter()
                .filter(|owner| matches!(owner, Owner::Object(id) if *id >= 10))
                .count();
            assert!(floating <= NARROW_LANTERNS, "t={t}: {floating}");
        }
    }
}
