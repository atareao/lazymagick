//! Image preview popup — renders an image in the terminal via Kitty/Sixel/Halfblocks.
//!
//! Uses `ratatui-image` to auto-detect the best available protocol and render
//! the image inside a bordered popup.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Clear, Widget},
};
use ratatui_image::Image;

use crate::config::ThemeColors;

/// Renders an image preview popup centered on screen.
///
/// The popup occupies ≈60 % × 60 % of the available area. The image is
/// fitted inside with aspect ratio preserved. A border with title and a
/// close hint are drawn around the image.
pub fn render(
    area: Rect,
    buf: &mut Buffer,
    protocol: &ratatui_image::protocol::Protocol,
    theme: &ThemeColors,
) {
    Clear.render(area, buf);

    let popup_w = (area.width as f32 * 0.6) as u16;
    let popup_h = (area.height as f32 * 0.6) as u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_w, popup_h);

    let block = Block::default()
        .title(" Image Preview ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent_fg));
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);

    // Render the image fitted inside the inner area
    let image = Image::new(protocol);
    image.render(inner, buf);

    // Close hint at the bottom
    let hint = " Esc/p: Close ";
    let hint_x = inner.x + (inner.width.saturating_sub(hint.len() as u16)) / 2;
    let hint_y = inner.y + inner.height.saturating_sub(1);
    buf.set_string(hint_x, hint_y, hint, Style::default().fg(theme.dim_text_fg));
}
