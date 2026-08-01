//! File browser panel — navigate the filesystem and select image files.
//! Shows file sizes, `../` parent entry, and green-border focus.

use std::path::{Path, PathBuf};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

use crate::fs_utils::DirListing;

/// Widget that renders the file browser panel.
pub struct FilePanel<'a> {
    /// The directory currently being browsed.
    pub current_dir: &'a Path,
    /// Parent directory path (None when at filesystem root).
    pub parent: Option<&'a Path>,
    /// Directory listing (directories, image files, other files).
    pub listing: &'a DirListing,
    /// Cursor offset into the combined visible entries list.
    pub cursor: usize,
    /// Paths of files currently selected for processing.
    pub selected_files: &'a [PathBuf],
    /// Whether to show hidden entries (names starting with `.`).
    pub show_hidden: bool,
    /// Whether this panel has keyboard focus.
    pub focused: bool,
}

/// Category of a filesystem entry in the combined list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    Directory,
    Image,
    Other,
}

impl FilePanel<'_> {
    /// Build a flat list of visible entries in render order.
    fn build_visible_entries(&self) -> Vec<(&Path, EntryKind, u64)> {
        let mut entries: Vec<(&Path, EntryKind, u64)> = Vec::new();

        // Synthetic parent directory entry
        if self.parent.is_some() {
            // Use current_dir for size (just 0 for parent marker)
            entries.push((self.current_dir, EntryKind::Directory, 0));
        }

        for dir in &self.listing.directories {
            if self.show_hidden
                || !dir
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with('.'))
            {
                let size = std::fs::metadata(dir).map(|m| m.len()).unwrap_or(0);
                entries.push((dir.as_path(), EntryKind::Directory, size));
            }
        }
        for img in &self.listing.image_files {
            if self.show_hidden
                || !img
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with('.'))
            {
                let size = std::fs::metadata(img).map(|m| m.len()).unwrap_or(0);
                entries.push((img.as_path(), EntryKind::Image, size));
            }
        }
        for other in &self.listing.other_files {
            if self.show_hidden
                || !other
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with('.'))
            {
                let size = std::fs::metadata(other).map(|m| m.len()).unwrap_or(0);
                entries.push((other.as_path(), EntryKind::Other, size));
            }
        }

        entries
    }
}

impl<'a> Widget for &FilePanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        let border_color = if self.focused {
            Color::Green
        } else {
            Color::DarkGray
        };

        let dir_display = self.current_dir.display().to_string();
        let title = format!(" 2: Files [{}] ", dir_display);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        block.render(area, buf);

        // Build visible entries
        let visible_entries = self.build_visible_entries();
        let _real_count = visible_entries
            .iter()
            .filter(|(p, _, _)| p != &self.current_dir)
            .count();

        if visible_entries.is_empty() {
            let text = if self.show_hidden {
                " (empty directory) "
            } else {
                " (no visible files — press . to show hidden) "
            };
            buf.set_string(
                inner.x + 1,
                inner.y + 1,
                text,
                Style::default().fg(Color::DarkGray),
            );
            return;
        }

        let available_height = inner.height as usize;
        let safe_cursor = self.cursor.min(visible_entries.len().saturating_sub(1));
        let scroll_offset = safe_cursor.saturating_sub(available_height.saturating_sub(1));

        for (display_idx, (path, kind, size)) in visible_entries
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(available_height)
        {
            let y = inner.y + (display_idx - scroll_offset) as u16;

            let is_cursor = display_idx == safe_cursor;
            let is_selected = self.selected_files.iter().any(|p| p == path);
            let is_parent = **path == *self.current_dir;

            let filename = if is_parent {
                "../".to_string()
            } else {
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            };

            let is_hidden = !is_parent && filename.starts_with('.');

            let mut style = Style::default();
            if is_cursor {
                style = style.fg(Color::Cyan).bg(Color::DarkGray);
            }
            if !is_cursor {
                match kind {
                    EntryKind::Directory => {
                        style = style.fg(Color::Blue);
                    }
                    EntryKind::Image => {
                        style = if is_selected {
                            style.fg(Color::Green)
                        } else {
                            style.fg(Color::White)
                        };
                    }
                    EntryKind::Other => {
                        style = style.fg(Color::DarkGray);
                    }
                }
            }
            if is_hidden && !is_cursor {
                style = style.fg(Color::DarkGray);
            }

            let (prefix, suffix) = if *kind == EntryKind::Directory {
                let pre_str = if is_cursor { ">├─ " } else { " ├─ " };
                let dir_label = if is_parent {
                    "../".to_string()
                } else {
                    format!("{filename}/")
                };
                (pre_str.to_string(), dir_label)
            } else {
                let sel = if is_selected { "◉" } else { " " };
                let pre = format!(" {sel} ");

                // Show file size for images
                let size_str = if *kind == EntryKind::Image && *size > 0 {
                    format!(" ({})", crate::fs_utils::format_file_size(*size))
                } else {
                    String::new()
                };

                (pre, format!("{filename}{size_str}"))
            };

            buf.set_string(inner.x + 1, y, format!("{prefix}{suffix}"), style);
        }
    }
}
