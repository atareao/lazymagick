//! Layout calculation for the TUI panels.
//!
//! The screen is divided as follows:
//!
//! ```text
//! ┌─ title_bar (1 line) ────────────────────────────────────────┐
//! ├─ 40% ────────────────────┬─ 60% ───────────────────────────┤
//! │ recipe_panel (40% left)  │                                  │
//! │                          │  command_panel / image_preview   │
//! │ file_panel   (60% left)  │                                  │
//! ├──────────────────────────┴──────────────────────────────────┤
//! ├─ log_panel (30% height of main area) ───────────────────────┤
//! ├─ status_bar (1 line) ───────────────────────────────────────┤
//! └─────────────────────────────────────────────────────────────┘
//! ```

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Pre-computed screen regions for all panels.
#[derive(Debug, Clone, Copy)]
pub struct LayoutAreas {
    /// Top bar with app name and quick status.
    pub title_bar: Rect,
    /// Recipe list panel (top-left).
    pub recipe_panel: Rect,
    /// File browser panel (bottom-left).
    pub file_panel: Rect,
    /// Command preview or image preview (right column, 60% width).
    pub command_panel: Rect,
    /// Bottom log/output panel (full width below the columns).
    pub log_panel: Rect,
    /// Bottom status bar with keybinding hints.
    pub status_bar: Rect,
}

/// Divide the terminal area into named regions.
///
/// Layout:
/// 1. Vertical split: title (1 line) | content (fill) | status (1 line)
/// 2. Content → columns_row (70%) | log (30%)
/// 3. columns_row → left (40%) | right (60%)
/// 4. Left → recipe (40%) | files (60%)
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

    // Split main area: top row with columns (70%) | log panel (30%)
    let main_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(main_area);

    let columns_row = main_split[0];
    let log_panel = main_split[1];

    // Split columns row: left (40%) | right (60%)
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(columns_row);

    let left_column = horizontal[0];
    let right_column = horizontal[1];

    // Split left column: recipe (40%) | files (60%)
    let left_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(left_column);

    LayoutAreas {
        title_bar,
        recipe_panel: left_split[0],
        file_panel: left_split[1],
        command_panel: right_column,
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

        // Recipe and file panels are in left column, stacked
        let total_left_height = areas.recipe_panel.height + areas.file_panel.height;
        assert_eq!(total_left_height, 20);

        // Command panel is in right column, same height as left column
        assert_eq!(areas.command_panel.height, 20);
    }

    #[test]
    fn chunk_areas_widths_sum() {
        let area = Rect::new(0, 0, 120, 30);
        let areas = chunk_areas(area);

        let left_w = areas.recipe_panel.width;
        let right_w = areas.command_panel.width;

        // Should all be within 1 pixel of the total (120)
        let total = left_w + right_w;
        assert!(
            total == 119 || total == 120 || total == 121,
            "left {left_w} + right {right_w} = {total}, expected ~120"
        );
    }
}
