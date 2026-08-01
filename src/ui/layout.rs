//! Layout calculation for the four-panel TUI layout.
//!
//! The screen is divided as follows:
//!
//! ```text
//! ┌─ title_bar (1 line) ───────────────────────────────────────┐
//! ├─ recipe_panel ─┬─ file_panel ──┬─ command_panel ──────────┤
//! │  (30% width)   │  (35% width)  │  (35% width)             │
//! ├─ log_panel (30% height of the main area) ──────────────────┤
//! ├─ status_bar (1 line) ──────────────────────────────────────┤
//! └────────────────────────────────────────────────────────────┘
//! ```

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Pre-computed screen regions for all panels.
#[derive(Debug, Clone, Copy)]
pub struct LayoutAreas {
    /// Top bar with app name and quick status.
    pub title_bar: Rect,
    /// Main content area (split horizontally into 3 panels).
    pub recipe_panel: Rect,
    pub file_panel: Rect,
    pub command_panel: Rect,
    /// Bottom log/output panel (below the three main panels).
    pub log_panel: Rect,
    /// Bottom status bar with keybinding hints.
    pub status_bar: Rect,
}

/// Divide the terminal area into named regions.
///
/// The layout is calculated as:
///
/// 1. Vertical split: title (1 line) | main (fill) | status (1 line)
/// 2. Main split vertically: top (70%) | log (30%)
/// 3. Top split horizontally: recipes (30%) | files (35%) | command (35%)
pub fn chunk_areas(area: Rect) -> LayoutAreas {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(0),    // main area (fill)
            Constraint::Length(1), // status bar
        ])
        .split(area);

    let title_bar = vertical[0];
    let main_area = vertical[1];
    let status_bar = vertical[2];

    // Split main area: top 70% for the three panels, bottom 30% for log
    let main_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(main_area);

    let top_area = main_split[0];
    let log_panel = main_split[1];

    // Split top area into three columns
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
        ])
        .split(top_area);

    LayoutAreas {
        title_bar,
        recipe_panel: horizontal[0],
        file_panel: horizontal[1],
        command_panel: horizontal[2],
        log_panel,
        status_bar,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn chunk_areas_divides_correctly() {
        let area = Rect::new(0, 0, 120, 30);
        let areas = chunk_areas(area);

        // Title is 1 line high
        assert_eq!(areas.title_bar.height, 1);
        assert_eq!(areas.title_bar.y, 0);

        // Status is 1 line high at the bottom
        assert_eq!(areas.status_bar.height, 1);
        assert_eq!(areas.status_bar.y, 29);

        // Log panel takes 30% of the middle
        // Main area is 28 lines (30 - 2), log is 30% ≈ 8
        assert_eq!(areas.log_panel.height, 8);

        // Three top panels are side by side, full height
        assert_eq!(areas.recipe_panel.height, 20);
        assert_eq!(areas.file_panel.height, 20);
        assert_eq!(areas.command_panel.height, 20);
    }

    #[test]
    fn chunk_areas_widths_sum() {
        let area = Rect::new(0, 0, 120, 30);
        let areas = chunk_areas(area);

        let recipe = areas.recipe_panel.width;
        let files = areas.file_panel.width;
        let command = areas.command_panel.width;

        // Should all be within 1 pixel of the total (120)
        let total = recipe + files + command;
        assert!(
            total == 119 || total == 120 || total == 121,
            "widths {recipe}+{files}+{command} = {total}, expected ~120"
        );
    }
}
