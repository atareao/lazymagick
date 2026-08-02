//! EXIF metadata panel — displays parsed EXIF data from `magick identify -verbose`.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Widget},
};

use crate::magick::ExifInfo;

fn or_dash(s: &str) -> &str {
    if s.is_empty() { "-" } else { s }
}

/// Renders an EXIF metadata overlay popup.
pub fn render(area: Rect, buf: &mut Buffer, info: &ExifInfo) {
    let popup_width = area.width.min(60);
    let popup_height = 22;

    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    Clear.render(popup_area, buf);

    let block = Block::default()
        .title(" EXIF Metadata ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup_area);
    block.render(popup_area, buf);

    let _y = inner.y + 1;

    let lines: Vec<String> = vec![
        format!(" Camera:"),
        format!("  Make:       {}", or_dash(&info.make)),
        format!("  Model:      {}", or_dash(&info.model)),
        format!("  Software:   {}", or_dash(&info.software)),
        format!("  Date taken: {}", or_dash(&info.date_taken)),
        String::new(),
        format!(" Exposure:"),
        format!("  ISO:        {}", or_dash(&info.iso)),
        format!("  Shutter:    {}", or_dash(&info.exposure)),
        format!("  Aperture:   {}", or_dash(&info.aperture)),
        format!("  Focal len:  {}", or_dash(&info.focal_length)),
        String::new(),
        format!(" GPS:"),
        format!("  Latitude:   {}", or_dash(&info.gps_latitude)),
        format!("  Longitude:  {}", or_dash(&info.gps_longitude)),
        String::new(),
        format!(" Other:"),
        format!("  Orientation: {}", or_dash(&info.orientation)),
    ];

    for (y, line) in (inner.y + 1..).zip(lines.iter()) {
        if y >= inner.y + inner.height.saturating_sub(1) {
            break;
        }
        let style = if line.is_empty() {
            Style::default()
        } else if line.ends_with(':') {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };
        buf.set_string(inner.x + 1, y, line, style);
    }

    // Close hint
    let hint_y = inner.y + inner.height.saturating_sub(1);
    buf.set_string(
        inner.x + 1,
        hint_y,
        " x: Close ",
        Style::default().fg(Color::DarkGray),
    );
}
