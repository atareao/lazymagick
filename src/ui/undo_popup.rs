//! Undo / Revert popup — displays generated output files for potential deletion.

use std::path::PathBuf;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, Widget},
};

use crate::config;

/// Render the undo list popup.
pub fn render(
    area: Rect,
    buf: &mut Buffer,
    outputs: &[PathBuf],
    cursor: usize,
    theme: &config::ThemeColors,
) {
    let popup_width = area.width.min(64);
    let list_lines = outputs.len().min(16) + 3; // title + entries + help
    let popup_height = (list_lines as u16)
        .max(8)
        .min(area.height.saturating_sub(2));

    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    Clear.render(popup_area, buf);

    let block = Block::default()
        .title(" Undo / Revert ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent_fg))
        .style(Style::default().bg(theme.background));
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);

    let max_visible = inner.height.saturating_sub(3) as usize; // 1 for top pad + 1 for help line
    let scroll_offset = if outputs.is_empty() || cursor < max_visible {
        0
    } else {
        cursor.saturating_sub(max_visible).saturating_add(1)
    };
    let visible_outputs: Vec<&PathBuf> = outputs
        .iter()
        .skip(scroll_offset)
        .take(max_visible)
        .collect();

    if visible_outputs.is_empty() {
        let empty_text = if outputs.is_empty() {
            " No generated files yet "
        } else {
            " (empty) "
        };
        buf.set_string(
            inner.x + 1,
            inner.y + 1,
            empty_text,
            Style::default().fg(theme.dim_text_fg),
        );
    }

    for (rel_y, (abs_idx, path)) in
        (inner.y + 1..).zip((scroll_offset..).zip(visible_outputs).take(max_visible))
    {
        if rel_y >= inner.y + inner.height.saturating_sub(2) {
            break;
        }
        let is_cursor = abs_idx == cursor;
        let display = path.to_string_lossy();
        // Truncate to fit inside the popup
        let max_w = inner.width.saturating_sub(4) as usize;
        let text = if display.len() > max_w {
            format!("{}…", &display[..max_w.saturating_sub(1)])
        } else {
            display.to_string()
        };

        let style = if is_cursor {
            Style::default()
                .fg(theme.cursor_fg)
                .bg(theme.cursor_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_fg)
        };

        let indicator = if is_cursor { "▸ " } else { "  " };
        buf.set_string(inner.x + 1, rel_y, indicator, style);
        buf.set_string(inner.x + 3, rel_y, &text, style);
    }

    // Help line at the bottom
    let help_y = inner.y + inner.height.saturating_sub(1);
    let help_text = " j/k: Move | Enter: Delete file | c: Clear all | Esc: Close ";
    buf.set_string(
        inner.x + 1,
        help_y,
        help_text,
        Style::default().fg(theme.dim_text_fg),
    );
}
