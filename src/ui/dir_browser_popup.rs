//! Directory browser popup — navigate the filesystem to pick an output directory.

use std::path::Path;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Clear, Widget},
};

use crate::config;

/// Renders a directory browser popup for selecting an output directory.
///
/// Shows the current path and a scrollable list of subdirectories.
/// Called from within the edit popup when the user presses a key to browse.
pub fn render(
    area: Rect,
    buf: &mut Buffer,
    current_path: &Path,
    cursor: usize,
    theme: &config::ThemeColors,
) {
    // Dim the background
    Clear.render(area, buf);

    // Calculate popup area (60% width, 60% height, centered)
    let popup_width = (area.width as f32 * 0.6) as u16;
    let popup_height = (area.height as f32 * 0.6) as u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let block = Block::default()
        .title(" Directory Browser ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent_fg));
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);

    // ── Current path label ───────────────────────────────────────
    let path_str = current_path.to_string_lossy();
    let max_path_w = inner.width.saturating_sub(2) as usize;
    let display_path = if path_str.len() > max_path_w {
        format!(
            "…{}",
            &path_str[(path_str.len().saturating_sub(max_path_w.saturating_sub(1)))..]
        )
    } else {
        path_str.to_string()
    };
    buf.set_string(
        inner.x + 1,
        inner.y,
        &display_path,
        Style::default().fg(theme.title_fg),
    );

    // ── Parent directory entry ───────────────────────────────────
    let entries_start = inner.y + 2;
    let available_height = inner.height.saturating_sub(3) as usize;

    // Try to list child directories
    let mut entries: Vec<(String, bool)> = Vec::new();
    entries.push((".. (parent)".to_string(), true));

    if let Ok(read_dir) = std::fs::read_dir(current_path) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir()
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
                && !name.starts_with('.')
            {
                entries.push((name.to_string(), false));
            }
        }
    }
    // Sort directory names alphabetically
    entries.sort_by(|a, b| {
        if a.1 && !b.1 {
            return std::cmp::Ordering::Less; // parent always first
        }
        if !a.1 && b.1 {
            return std::cmp::Ordering::Greater;
        }
        a.0.to_lowercase().cmp(&b.0.to_lowercase())
    });

    // Scroll
    let scroll_offset = cursor.saturating_sub(available_height.saturating_sub(1));

    for (i, (name, is_parent)) in entries
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(available_height)
    {
        let y = entries_start + (i - scroll_offset) as u16;
        if y >= inner.y + inner.height.saturating_sub(1) {
            break;
        }
        let is_cursor = i == cursor;
        let prefix = if is_cursor { "> " } else { "  " };
        let marker = if *is_parent { "📁" } else { "📂" };
        let line = format!("{prefix}{marker} {name}");

        let style = if is_cursor {
            Style::default().fg(theme.cursor_fg).bg(theme.cursor_bg)
        } else {
            Style::default().fg(theme.text_fg)
        };
        buf.set_string(inner.x + 1, y, &line, style);
    }

    // ── Help bar ────────────────────────────────────────────────
    let help_y = inner.y + inner.height.saturating_sub(1);
    let help = " j/k: Move  Enter: Enter dir  Space: Select  Esc: Close ";
    buf.set_string(
        inner.x + 1,
        help_y,
        help,
        Style::default().fg(theme.dim_text_fg),
    );
}
