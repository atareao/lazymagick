//! Log / output panel — shows execution log and live process output.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

use crate::app::{LogEntry, LogLevel};
use crate::config;

/// Widget that renders the log/output panel.
pub struct LogPanel<'a> {
    /// Log entries (oldest first, most recent at the end).
    pub entries: &'a [LogEntry],
    /// Whether a process is currently running.
    pub process_running: bool,
    /// Live process stderr output streamed so far.
    pub process_output: &'a str,
    /// Whether this panel has keyboard focus.
    pub focused: bool,
    /// Spinner frame character to show when running.
    pub spinner_char: char,
    /// Parsed theme colors for the UI.
    pub theme: &'a config::ThemeColors,
}

impl<'a> Widget for &LogPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        let border_color = if self.focused {
            self.theme.border_focused
        } else {
            self.theme.border_unfocused
        };

        let title = if self.process_running {
            format!(" 4: Log [{} Running…] ", self.spinner_char)
        } else {
            " 4: Log ".to_string()
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        block.render(area, buf);

        let available_height = inner.height as usize;
        if available_height == 0 {
            return;
        }

        // Collect all lines to display: existing entries + live output
        let mut display_lines: Vec<(String, Color)> = Vec::new();

        for entry in self.entries.iter() {
            let (color, prefix) = match entry.level {
                LogLevel::Info => (self.theme.info_fg, "ℹ"),
                LogLevel::Success => (self.theme.success_fg, "✔"),
                LogLevel::Error => (self.theme.error_fg, "✗"),
                LogLevel::Magick => (self.theme.dim_text_fg, "⚙"),
            };
            display_lines.push((format!("{prefix} {}", entry.message), color));
        }

        // Add live process output lines
        if self.process_running && !self.process_output.is_empty() {
            let lines: Vec<&str> = self.process_output.lines().collect();
            let start = lines.len().saturating_sub(5);
            for line in &lines[start..] {
                display_lines.push((line.to_string(), self.theme.dim_text_fg));
            }
            display_lines.push((
                format!(" {} Processing…", self.spinner_char),
                self.theme.progress_fg,
            ));
        }

        // Show from the end (most recent)
        let start = display_lines.len().saturating_sub(available_height);
        for (i, (text, color)) in display_lines.iter().enumerate().skip(start) {
            let y = inner.y + (i - start) as u16;
            let max_len = inner.width.saturating_sub(2) as usize;
            let truncated = if text.len() > max_len {
                format!("{}…", &text[..max_len.saturating_sub(1)])
            } else {
                text.clone()
            };
            buf.set_string(inner.x + 1, y, &truncated, Style::default().fg(*color));
        }
    }
}
