//! Before/after comparison popup — shows original and processed image metadata side by side.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Widget},
};

use crate::{config::ThemeColors, magick::BeforeAfterInfo};

/// Renders a before/after comparison popup.
pub fn render(area: Rect, buf: &mut Buffer, info: &BeforeAfterInfo, theme: &ThemeColors) {
    Clear.render(area, buf);

    let popup_width = area.width.min(72);
    let popup_height = 24;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let border_color = theme.accent_fg;
    let block = Block::default()
        .title(" Before / After Comparison ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);

    let mut y = inner.y + 1;
    let col_width = (inner.width.saturating_sub(3)) / 2;

    // ── Headers ────────────────────────────────────────────────
    let before_header = " BEFORE ";
    let after_header = " AFTER ";
    let separator_x = inner.x + 1 + col_width + 1;

    buf.set_string(
        inner.x + 1,
        y,
        before_header,
        Style::default().fg(Color::Cyan).bold(),
    );
    buf.set_string(
        separator_x + 1,
        y,
        after_header,
        Style::default().fg(Color::Green).bold(),
    );
    y += 1;

    // Separator line
    let sep = "─".repeat(col_width as usize);
    buf.set_string(inner.x + 1, y, &sep, Style::default().fg(Color::DarkGray));
    buf.set_string(
        separator_x + 1,
        y,
        &sep,
        Style::default().fg(Color::DarkGray),
    );
    y += 1;
    let divider = " │ ";
    buf.set_string(
        separator_x,
        y,
        divider,
        Style::default().fg(Color::DarkGray),
    );
    // We need to keep the divider at the same position for all lines

    // ── Before info ─────────────────────────────────────────────
    let before_lines = [
        format!(" Format: {}", info.original.format),
        format!(" Size: {}", info.original.dimensions),
        format!(" Depth: {}", info.original.bit_depth),
        format!(" Color: {}", info.original.color_space),
        format!(" File: {}", info.original.file_size),
    ];
    let after_lines = [
        format!(" Format: {}", info.processed.format),
        format!(" Size: {}", info.processed.dimensions),
        format!(" Depth: {}", info.processed.bit_depth),
        format!(" Color: {}", info.processed.color_space),
        format!(" File: {}", info.processed.file_size),
    ];

    for i in 0..before_lines.len().max(after_lines.len()) {
        if y >= inner.y + inner.height.saturating_sub(1) {
            break;
        }

        let txt_color = theme.text_fg;
        let dim_color = theme.dim_text_fg;

        if i < before_lines.len() {
            let label = &before_lines[i];
            let (field, value) = label.split_once(": ").unwrap_or((label, ""));
            buf.set_string(
                inner.x + 1,
                y,
                format!(" {field}:"),
                Style::default().fg(dim_color),
            );
            buf.set_string(
                inner.x + 1 + field.len() as u16 + 2,
                y,
                value,
                Style::default().fg(txt_color),
            );
        }

        // Divider
        let divider_y = y;
        buf.set_string(
            separator_x,
            divider_y,
            " │ ",
            Style::default().fg(Color::DarkGray),
        );

        if i < after_lines.len() {
            let label = &after_lines[i];
            let (field, value) = label.split_once(": ").unwrap_or((label, ""));
            buf.set_string(
                separator_x + 3,
                y,
                format!(" {field}:"),
                Style::default().fg(dim_color),
            );
            buf.set_string(
                separator_x + 3 + field.len() as u16 + 2,
                y,
                value,
                Style::default().fg(txt_color),
            );
        }

        y += 1;
    }

    // ── Separator ──────────────────────────────────────────────
    y += 1;
    if y < inner.y + inner.height.saturating_sub(1) {
        let full_sep = "─".repeat(inner.width.saturating_sub(2) as usize);
        buf.set_string(
            inner.x + 1,
            y,
            &full_sep,
            Style::default().fg(Color::DarkGray),
        );
        y += 1;
    }

    // ── Original vs Processed paths ────────────────────────────
    if y < inner.y + inner.height.saturating_sub(1) {
        let before_path = info.original.path.as_str();
        let max_w = inner.width.saturating_sub(4) as usize;
        let truncated = if before_path.len() > max_w {
            format!(
                "…{}",
                &before_path[before_path.len().saturating_sub(max_w.saturating_sub(1))..]
            )
        } else {
            before_path.to_string()
        };
        buf.set_string(
            inner.x + 1,
            y,
            format!(" Original: {truncated}"),
            Style::default().fg(Color::Cyan),
        );
        y += 1;
    }
    if y < inner.y + inner.height.saturating_sub(1) {
        let after_path = info.processed.path.as_str();
        let max_w = inner.width.saturating_sub(4) as usize;
        let truncated = if after_path.len() > max_w {
            format!(
                "…{}",
                &after_path[after_path.len().saturating_sub(max_w.saturating_sub(1))..]
            )
        } else {
            after_path.to_string()
        };
        buf.set_string(
            inner.x + 1,
            y,
            format!(" Processed: {truncated}"),
            Style::default().fg(Color::Green),
        );
    }

    // ── Help ───────────────────────────────────────────────────
    let help_y = inner.y + inner.height.saturating_sub(1);
    buf.set_string(
        inner.x + 1,
        help_y,
        " Esc/B: Close ",
        Style::default().fg(Color::DarkGray),
    );
}
