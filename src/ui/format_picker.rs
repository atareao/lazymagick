//! Format picker popup overlay — choose an output format at runtime.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Widget},
};

/// Widget that renders a centered format picker popup.
pub struct FormatPicker<'a> {
    /// Available format names (e.g. `["webp", "avif", "jpeg"]`).
    pub formats: &'a [String],
    /// Current cursor position (index into `formats`).
    pub cursor: usize,
    /// The currently active format override, if any.
    pub current_format: Option<&'a str>,
}

impl<'a> Widget for &FormatPicker<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.formats.is_empty() {
            return;
        }

        // Dimensions for the centered popup
        let popup_width = area.width.min(40);
        let popup_height = (self.formats.len() as u16 + 4).min(area.height.saturating_sub(4));

        let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Clear behind the popup
        Clear.render(popup_area, buf);

        let block = Block::default()
            .title(" Select output format ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));
        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        // Render format list
        let available_height = inner.height.saturating_sub(2) as usize;
        let scroll_offset = self
            .cursor
            .saturating_sub(available_height.saturating_sub(1));

        for i in scroll_offset..self.formats.len().min(scroll_offset + available_height) {
            let y = inner.y + 1 + (i - scroll_offset) as u16;
            let fmt = &self.formats[i];

            let is_cursor = i == self.cursor;
            let is_current = self.current_format.is_some_and(|c| c == fmt);

            let mut style = Style::default().fg(Color::White);
            if is_cursor {
                style = style.fg(Color::Cyan).bg(Color::DarkGray);
            } else if is_current {
                style = style.fg(Color::Green);
            }

            let prefix = if is_cursor {
                ">"
            } else if is_current {
                "●"
            } else {
                " "
            };
            let suffix = if is_current { " (current)" } else { "" };

            let line = format!(" {prefix} {fmt}{suffix} ");
            buf.set_string(inner.x + 1, y, &line, style);
        }

        // Bottom hint
        let hint_y = inner.y + inner.height.saturating_sub(1);
        let hint = " Enter: confirm  Esc: cancel ";
        buf.set_string(
            inner.x + 1,
            hint_y,
            hint,
            Style::default().fg(Color::DarkGray),
        );
    }
}
