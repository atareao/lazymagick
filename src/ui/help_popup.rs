//! Help overlay popup — shows keybindings reference.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Widget},
};

/// Keybinding help popup widget.
pub struct HelpPopup;

impl Widget for &HelpPopup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let popup_width = area.width.min(56);
        let popup_height = area.height.min(24);

        let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        Clear.render(popup_area, buf);

        let block = Block::default()
            .title(" Help — Keybindings ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));
        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let lines = [
            ("── Navigation ────────────────", "", true),
            (
                "Tab / 1-4",
                "Focus panels (Recipe / Files / Command / Log)",
                false,
            ),
            ("j / ↓", "Move cursor down", false),
            ("k / ↑", "Move cursor up", false),
            (
                "Enter",
                "Select recipe / confirm selection / enter dir",
                false,
            ),
            ("", "", false),
            ("── File browser ──────────────", "", true),
            ("h / ←", "Parent directory", false),
            ("l / →", "Enter directory", false),
            ("Space", "Toggle file multi-select", false),
            (".", "Toggle hidden files", false),
            ("", "", false),
            ("── Actions ────────────────────", "", true),
            ("r", "Run current recipe on selected file(s)", false),
            ("c", "Cancel running process", false),
            ("f", "Open format picker", false),
            ("e", "Edit output dir / extra args", false),
            ("R", "Toggle recursive directory processing", false),
            ("x", "Toggle EXIF metadata panel", false),
            ("n", "Toggle dry-run mode", false),
            ("", "", false),
            ("── General ────────────────────", "", true),
            ("?", "Toggle this help", false),
            ("Esc", "Close popup / cancel", false),
            ("q / Ctrl+Q", "Quit (Browse mode only)", false),
        ];

        for (y_offset, (key, desc, is_header)) in (inner.y + 1..).zip(&lines) {
            if y_offset >= inner.y + inner.height.saturating_sub(1) {
                break;
            }
            if *is_header {
                buf.set_string(
                    inner.x + 1,
                    y_offset,
                    key,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
            } else if key.is_empty() {
                // blank separator
            } else {
                buf.set_string(
                    inner.x + 2,
                    y_offset,
                    key,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );
                let x = inner.x + 2 + key.len() as u16 + 2;
                if x < inner.x + inner.width.saturating_sub(2) {
                    let max_desc = (inner.x + inner.width - 2 - x) as usize;
                    let desc_trunc = if desc.len() > max_desc {
                        format!("{}…", &desc[..max_desc.saturating_sub(1)])
                    } else {
                        desc.to_string()
                    };
                    buf.set_string(x, y_offset, &desc_trunc, Style::default().fg(Color::White));
                }
            }
        }

        // Bottom hint
        let hint_y = inner.y + inner.height.saturating_sub(1);
        let hint = " Press ? or Esc to close ";
        buf.set_string(
            inner.x + (inner.width.saturating_sub(hint.len() as u16)) / 2,
            hint_y,
            hint,
            Style::default().fg(Color::DarkGray),
        );
    }
}
