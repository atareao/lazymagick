//! Image preview widget — renders an image in the command panel slot
//! via Kitty/Sixel/Halfblocks.
//!
//! Uses `ratatui-image` to auto-detect the best available protocol and render
//! the image inside a bordered panel.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Widget},
};
use ratatui_image::Image;

use crate::config::ThemeColors;

/// Render an image preview inside a fixed-area bordered panel.
///
/// `area` is the exact rectangle to draw into (typically `areas.command_panel`).
/// The image is fitted inside the border with aspect ratio preserved.
pub fn render(
    area: Rect,
    buf: &mut Buffer,
    protocol: &ratatui_image::protocol::Protocol,
    theme: &ThemeColors,
) {
    if area.is_empty() {
        return;
    }

    let block = Block::default()
        .title(" 3: Image Preview ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent_fg));
    let inner = block.inner(area);
    block.render(area, buf);

    if inner.is_empty() || inner.height < 2 {
        return;
    }

    // Render the image fitted inside the inner area
    let image = Image::new(protocol);
    image.render(inner, buf);

    // Close hint at the bottom
    let hint = " [p] Close  ";
    let hint_x = inner.x + 1;
    let hint_y = inner.y + inner.height.saturating_sub(1);
    buf.set_string(hint_x, hint_y, hint, Style::default().fg(theme.dim_text_fg));
}

/// Render a placeholder when no image is loaded for preview.
pub fn render_placeholder(area: Rect, buf: &mut Buffer, theme: &ThemeColors) {
    if area.is_empty() {
        return;
    }

    let block = Block::default()
        .title(" 3: Image Preview ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_unfocused));
    let inner = block.inner(area);
    block.render(area, buf);

    if inner.is_empty() {
        return;
    }

    let msg = " [p] Select an image file to preview ";
    buf.set_string(
        inner.x + 1,
        inner.y + 1,
        msg,
        Style::default().fg(theme.dim_text_fg),
    );
}
