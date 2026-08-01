//! Edit parameters popup — interactive inline editing of output dir and extra args.

use std::path::Path;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Widget},
};

use crate::app::EditField;

/// Widget that renders the edit parameters popup with inline editing.
pub struct EditPopup<'a> {
    /// Output directory override, if set.
    pub output_dir: Option<&'a Path>,
    /// Extra ImageMagick arguments string.
    pub extra_args: &'a str,
    /// Current text buffer for the output dir field.
    pub output_buf: &'a str,
    /// Current text buffer for the extra args field.
    pub args_buf: &'a str,
    /// Which field is currently being edited.
    pub edit_field: EditField,
}

impl<'a> Widget for &EditPopup<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let popup_width = area.width.min(56);
        let popup_height = 12;

        let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        Clear.render(popup_area, buf);

        let block = Block::default()
            .title(" Edit Parameters ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));
        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let mut y = inner.y + 1;

        // ── Output directory field ─────────────────────────────
        let out_active = self.edit_field == EditField::OutputDir;
        let out_style = if out_active {
            Style::default().fg(Color::Cyan).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        buf.set_string(
            inner.x + 1,
            y,
            " Output directory: ",
            Style::default().fg(Color::Cyan),
        );
        let out_display = if self.output_buf.is_empty() {
            "(same as input)".to_string()
        } else {
            self.output_buf.to_string()
        };
        let out_x = inner.x + 20;
        let out_max = inner.x + inner.width.saturating_sub(2);
        let out_avail = out_max.saturating_sub(out_x) as usize;
        let out_trunc = if out_display.len() > out_avail {
            format!("{}…", &out_display[..out_avail.saturating_sub(1)])
        } else {
            out_display
        };
        if out_active {
            let cursor_str = format!("{}█", out_trunc);
            buf.set_string(out_x, y, &cursor_str, out_style);
        } else {
            buf.set_string(out_x, y, &out_trunc, out_style);
        }
        y += 1;

        // ── Extra args field ───────────────────────────────────
        let args_active = self.edit_field == EditField::ExtraArgs;
        let args_style = if args_active {
            Style::default().fg(Color::Cyan).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        buf.set_string(
            inner.x + 1,
            y,
            " Extra args:      ",
            Style::default().fg(Color::Cyan),
        );
        let args_display = if self.args_buf.is_empty() {
            "(none)".to_string()
        } else {
            self.args_buf.to_string()
        };
        let args_x = inner.x + 20;
        let args_avail = out_max.saturating_sub(args_x) as usize;
        let args_trunc = if args_display.len() > args_avail {
            format!("{}…", &args_display[..args_avail.saturating_sub(1)])
        } else {
            args_display
        };
        if args_active {
            let cursor_str = format!("{}█", args_trunc);
            buf.set_string(args_x, y, &cursor_str, args_style);
        } else {
            buf.set_string(args_x, y, &args_trunc, args_style);
        }
        y += 2;

        // ── Hint lines ─────────────────────────────────────────
        buf.set_string(
            inner.x + 1,
            y,
            " Tab: switch field  Backspace: delete  Type: insert ",
            Style::default().fg(Color::DarkGray),
        );
        y += 1;

        buf.set_string(
            inner.x + 1,
            y,
            " Enter: apply  Esc: cancel ",
            Style::default().fg(Color::DarkGray),
        );
    }
}
