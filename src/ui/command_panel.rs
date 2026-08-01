//! Command preview panel — shows the generated `magick` command, input/output paths,
//! image metadata, file count, and format override.

use std::path::Path;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

use ratatui::prelude::Stylize;

use crate::{magick::ImageInfo, recipe::Recipe};

/// Widget that renders the command preview panel.
pub struct CommandPanel<'a> {
    /// The currently selected recipe (if any).
    pub recipe: Option<&'a Recipe>,
    /// The input file path (if any).
    pub input_file: Option<&'a Path>,
    /// Active format override extension (e.g. `"webp"`, `"avif"`).
    pub format_override: Option<&'a str>,
    /// Parsed image metadata from `magick identify`.
    pub preview_info: Option<&'a ImageInfo>,
    /// Error message from the last identify attempt.
    pub preview_error: Option<&'a str>,
    /// Whether this panel has keyboard focus.
    pub focused: bool,
    /// Number of selected files for batch processing.
    pub selected_file_count: usize,
}

impl<'a> Widget for &CommandPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        let border_color = if self.focused {
            Color::Green
        } else {
            Color::DarkGray
        };

        let block = Block::default()
            .title(" 3: Command Preview ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        block.render(area, buf);

        if area.height < 4 {
            return;
        }

        let mut y = inner.y + 1;
        let width = inner.width.saturating_sub(2) as usize;

        // ── Recipe info ──────────────────────────────────────────
        if let Some(recipe) = self.recipe {
            // Recipe name + description
            buf.set_string(
                inner.x + 1,
                y,
                format!(" {}", recipe.name),
                Style::default().fg(Color::Cyan).bold(),
            );
            y += 1;
            let desc = if recipe.description.len() > width {
                format!("{}…", &recipe.description[..width.saturating_sub(1)])
            } else {
                recipe.description.clone()
            };
            buf.set_string(inner.x + 2, y, &desc, Style::default().fg(Color::DarkGray));
            y += 2;

            // ── Input / Output paths ────────────────────────────
            if let Some(input_file) = self.input_file {
                let input_name = input_file
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default();
                buf.set_string(
                    inner.x + 1,
                    y,
                    format!(" Input:  {input_name}"),
                    Style::default().fg(Color::White),
                );
                y += 1;

                let format_override = self.format_override;
                let output_ext = format_override.unwrap_or(&recipe.output_ext);
                let output_name = format!(
                    "{}.{output_ext}",
                    input_file
                        .file_stem()
                        .map(|s| s.to_string_lossy())
                        .unwrap_or_default()
                );
                buf.set_string(
                    inner.x + 1,
                    y,
                    format!(" Output: {output_name}"),
                    Style::default().fg(Color::White),
                );
                y += 2;

                // ── Command string (green) ──────────────────────
                let format_args: Vec<String> = format_override
                    .and_then(|fmt| recipe.formats.get(fmt))
                    .cloned()
                    .unwrap_or_default();

                let mut cmd_parts = vec!["magick".to_string()];
                cmd_parts.push(input_name.to_string());
                for arg in &recipe.args {
                    cmd_parts.push(arg.clone());
                }
                // Fallback to stages if args empty
                if recipe.args.is_empty()
                    && let Some(stage) = recipe.stages.first()
                {
                    for flag in &stage.flags {
                        cmd_parts.push(flag.clone());
                    }
                }
                for arg in &format_args {
                    cmd_parts.push(arg.clone());
                }
                cmd_parts.push(output_name);

                let cmd_str = cmd_parts.join(" ");
                let truncated = if cmd_str.len() > width {
                    format!("{}…", &cmd_str[..width.saturating_sub(1)])
                } else {
                    cmd_str
                };
                buf.set_string(
                    inner.x + 1,
                    y,
                    &truncated,
                    Style::default().fg(Color::Green),
                );
                y += 1;
            }

            // ── File count indicator ─────────────────────────────
            if self.selected_file_count > 1 {
                buf.set_string(
                    inner.x + 1,
                    y,
                    format!(
                        " {} file(s) selected — will run on each",
                        self.selected_file_count
                    ),
                    Style::default().fg(Color::Yellow),
                );
                y += 1;
            }

            // ── Format override indicator ────────────────────────
            if let Some(fmt) = self.format_override {
                let fmt_line = format!(" [Format override: {fmt}] ");
                buf.set_string(inner.x + 1, y, &fmt_line, Style::default().fg(Color::Cyan));
                y += 1;
            }

            y += 1;

            // ── Image metadata ───────────────────────────────────
            if let Some(info) = self.preview_info {
                let meta_lines = [
                    format!(" Path: {}", info.path),
                    format!(" Format: {}", info.format),
                    format!(" Dimensions: {}", info.dimensions),
                    format!(" Bit depth: {}", info.bit_depth),
                    format!(" Color space: {}", info.color_space),
                    format!(" File size: {}", info.file_size),
                ];
                for line in &meta_lines {
                    if y >= inner.y + inner.height - 1 {
                        break;
                    }
                    buf.set_string(inner.x + 1, y, line, Style::default().fg(Color::White));
                    y += 1;
                }
            }

            if let Some(err) = self.preview_error
                && y < inner.y + inner.height - 1
            {
                buf.set_string(
                    inner.x + 1,
                    y,
                    format!(" ⚠ {err}"),
                    Style::default().fg(Color::Red),
                );
            }
        } else {
            let hint = " Select a recipe and file ";
            if y < inner.y + inner.height - 1 {
                buf.set_string(inner.x + 1, y, hint, Style::default().fg(Color::DarkGray));
            }
        }

        // Bottom hint line
        let hint_y = inner.y + inner.height.saturating_sub(1);
        let hint = " [r] Run  [f] Format  [e] Edit  ";
        buf.set_string(
            inner.x + 1,
            hint_y,
            hint,
            Style::default().fg(Color::DarkGray),
        );
    }
}
