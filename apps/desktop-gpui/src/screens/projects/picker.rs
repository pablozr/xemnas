//! The keyboard model of the project picker (the crumb's popover): which row
//! is highlighted, what ↑/↓/Enter/Esc do and which projects the filter keeps.
//! Pure, so the behaviour is tested without a window.

/// A key the picker understands while it is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerKey {
    /// Highlight the previous row (wraps to the last).
    Up,
    /// Highlight the next row (wraps to the first).
    Down,
    /// Switch to the highlighted project.
    Enter,
    /// Close without switching.
    Escape,
}

/// What a key asks the screen to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickerOutcome {
    /// The highlight moved (or stayed); repaint.
    Moved,
    /// Switch to this project and close.
    Pick(String),
    /// Close without switching.
    Close,
}

/// The highlighted row of the visible (filtered) list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Picker {
    pub highlighted: usize,
}

impl Picker {
    /// Opens on the current project's row, so Enter alone changes nothing.
    pub fn open(visible: &[String], selected: Option<&str>) -> Self {
        Self {
            highlighted: selected
                .and_then(|id| visible.iter().position(|row| row == id))
                .unwrap_or(0),
        }
    }

    /// Applies one key to the visible rows (ids, in painted order).
    pub fn key(&mut self, key: PickerKey, visible: &[String]) -> PickerOutcome {
        let count = visible.len();
        match key {
            PickerKey::Down | PickerKey::Up if count > 0 => {
                let step = if key == PickerKey::Down { 1 } else { -1 };
                self.highlighted = (self.highlighted.min(count - 1) as isize + step)
                    .rem_euclid(count as isize) as usize;
                PickerOutcome::Moved
            }
            PickerKey::Down | PickerKey::Up => PickerOutcome::Moved,
            PickerKey::Enter => visible
                .get(self.highlighted.min(count.saturating_sub(1)))
                .map_or(PickerOutcome::Moved, |id| PickerOutcome::Pick(id.clone())),
            PickerKey::Escape => PickerOutcome::Close,
        }
    }
}

/// Whether a project matches the filter: by name or by location, ignoring
/// case; an empty filter keeps every project.
pub fn matches(name: &str, location: &str, query: &str) -> bool {
    let needle = query.trim().to_lowercase();
    needle.is_empty()
        || name.to_lowercase().contains(&needle)
        || location.to_lowercase().contains(&needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO: [(&str, &str); 2] = [
        ("xemnas", "C:/Projects/xemnas"),
        ("kpi-front", "C:/Projects/kpi-front"),
    ];

    fn ids() -> Vec<String> {
        DEMO.iter().map(|(name, _)| (*name).to_owned()).collect()
    }

    #[test]
    fn ctrl_p_then_down_then_enter_switches_project() {
        let visible = ids();
        let mut picker = Picker::open(&visible, Some("xemnas"));
        assert_eq!(picker.highlighted, 0, "opens on the current project");
        assert_eq!(picker.key(PickerKey::Down, &visible), PickerOutcome::Moved);
        assert_eq!(
            picker.key(PickerKey::Enter, &visible),
            PickerOutcome::Pick("kpi-front".into())
        );
    }

    #[test]
    fn the_kpi_filter_leaves_one_row() {
        let rows: Vec<_> = DEMO
            .iter()
            .filter(|(name, location)| matches(name, location, "kpi"))
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "kpi-front");
        assert_eq!(
            DEMO.iter()
                .filter(|(name, location)| matches(name, location, " "))
                .count(),
            2
        );
    }

    #[test]
    fn escape_closes_and_empty_lists_never_pick() {
        let mut picker = Picker::default();
        assert_eq!(picker.key(PickerKey::Escape, &ids()), PickerOutcome::Close);
        assert_eq!(picker.key(PickerKey::Down, &[]), PickerOutcome::Moved);
        assert_eq!(picker.key(PickerKey::Enter, &[]), PickerOutcome::Moved);
        let visible = ids();
        assert_eq!(picker.key(PickerKey::Up, &visible), PickerOutcome::Moved);
        assert_eq!(picker.highlighted, 1, "up from the first row wraps");
    }
}
