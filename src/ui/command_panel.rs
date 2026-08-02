//! Command preview panel — shows the generated `magick` command, input/output paths,
//! image metadata, file count, format override, and real-time progress bar.

use std::path::Path;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Widget},
};

use crate::config;
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
    /// Whether a magick process is currently running.
    pub is_running: bool,
    /// Current progress numerator (from `-monitor`).
    pub progress_current: u64,
    /// Current progress denominator (from `-monitor`).
    pub progress_total: u64,
    /// Current processing stage name.
    pub progress_stage: String,
    /// Parsed theme colors for the UI.
    pub theme: &'a config::ThemeColors,
}

impl<'a> Widget for &CommandPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        let border_color = if self.focused {
            self.theme.border_focused
        } else {
            self.theme.border_unfocused
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

        // ── Progress bar (shown when a process is running) ─────────
        if self.is_running && self.progress_total > 0 {
            let bar_width = width.min(40);
            let progress =
                (self.progress_current as f64 / self.progress_total as f64).clamp(0.0, 1.0);
            let filled = (progress * bar_width as f64) as usize;
            let empty = bar_width.saturating_sub(filled);

            // Stage name
            if !self.progress_stage.is_empty() && y < inner.y + inner.height - 1 {
                buf.set_string(
                    inner.x + 1,
                    y,
                    format!(" {}", self.progress_stage),
                    Style::default().fg(self.theme.progress_fg),
                );
                y += 1;
            }

            // Progress bar
            if y < inner.y + inner.height - 1 {
                let pct_str = format!(" {:3.0}%", progress * 100.0);
                let max_bar_chars = inner
                    .width
                    .saturating_sub(2)
                    .saturating_sub(pct_str.len() as u16)
                    as usize;
                let display_filled = filled.min(max_bar_chars);
                let display_empty = empty.min(max_bar_chars.saturating_sub(display_filled));
                let bar_line: String = std::iter::repeat_n('█', display_filled)
                    .chain(std::iter::repeat_n('░', display_empty))
                    .chain(pct_str.chars())
                    .collect();
                buf.set_string(
                    inner.x + 1,
                    y,
                    &bar_line,
                    Style::default().fg(self.theme.progress_fg),
                );
                y += 1;
            }
            y += 1;
        }

        // ── Recipe info ──────────────────────────────────────────
        if let Some(recipe) = self.recipe {
            // Recipe name + description
            buf.set_string(
                inner.x + 1,
                y,
                format!(" {}", recipe.name),
                Style::default().fg(self.theme.accent_fg).bold(),
            );
            y += 1;
            let desc = if recipe.description.len() > width {
                format!("{}…", &recipe.description[..width.saturating_sub(1)])
            } else {
                recipe.description.clone()
            };
            buf.set_string(
                inner.x + 2,
                y,
                &desc,
                Style::default().fg(self.theme.dim_text_fg),
            );
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
                    Style::default().fg(self.theme.text_fg),
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
                    Style::default().fg(self.theme.text_fg),
                );
                y += 2;

                // ── Command string ──────────────────────────────
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
                    Style::default().fg(self.theme.selected_fg),
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
                    Style::default().fg(self.theme.warning_fg),
                );
                y += 1;
            }

            // ── Format override indicator ────────────────────────
            if let Some(fmt) = self.format_override {
                let fmt_line = format!(" [Format override: {fmt}] ");
                buf.set_string(
                    inner.x + 1,
                    y,
                    &fmt_line,
                    Style::default().fg(self.theme.accent_fg),
                );
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
                    buf.set_string(
                        inner.x + 1,
                        y,
                        line,
                        Style::default().fg(self.theme.text_fg),
                    );
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
                    Style::default().fg(self.theme.error_fg),
                );
            }
        } else {
            let hint = " Select a recipe and file ";
            if y < inner.y + inner.height - 1 {
                buf.set_string(
                    inner.x + 1,
                    y,
                    hint,
                    Style::default().fg(self.theme.dim_text_fg),
                );
            }
        }

        // Bottom hint line
        let hint_y = inner.y + inner.height.saturating_sub(1);
        let hint = " [r] Run  [f] Format  [e] Edit  ";
        buf.set_string(
            inner.x + 1,
            hint_y,
            hint,
            Style::default().fg(self.theme.dim_text_fg),
        );
    }
}
