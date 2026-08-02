//! Image preview panel — renders an image in the command panel slot
//! using Halfblocks protocol (works in any terminal with 24-bit color).

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders},
};
use ratatui_image::Image;

use crate::config::ThemeColors;

/// Render an image preview inside a fixed-area bordered panel.
///
/// IMPORTANT: `allow_clipping(true)` is required — without it the image
/// won't render if the available area is smaller than the protocol size.
pub fn render(
    frame: &mut Frame,
    area: Rect,
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
    frame.render_widget(&block, area);

    if inner.is_empty() || inner.height < 2 {
        return;
    }

    // allow_clipping is critical — without it the image is invisible
    // when the render area is smaller than the protocol size
    frame.render_widget(Image::new(protocol).allow_clipping(true), inner);

    // Bottom close hint
    let hint = " [p/Esc] Close ";
    frame.buffer_mut().set_string(
        inner.x + 1,
        inner.y + inner.height.saturating_sub(1),
        hint,
        Style::default().fg(theme.dim_text_fg),
    );
}

/// Placeholder shown when toggling preview without a loaded image.
pub fn render_placeholder(frame: &mut Frame, area: Rect, theme: &ThemeColors) {
    if area.is_empty() {
        return;
    }

    let block = Block::default()
        .title(" 3: Image Preview ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent_fg));
    let inner = block.inner(area);
    frame.render_widget(&block, area);

    if inner.is_empty() {
        return;
    }

    let msg = " [p] Select an image file to preview ";
    frame.buffer_mut().set_string(
        inner.x + 1,
        inner.y + 1,
        msg,
        Style::default().fg(theme.dim_text_fg),
    );
}
