//! Recipe list panel — displays available recipes with categories, usage, sort, and filter support.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Widget},
};

use crate::app::SortOrder;
use crate::config;
use crate::recipe::Recipe;

/// Widget that renders the recipe list panel.
pub struct RecipePanel<'a> {
    /// All available recipes.
    pub recipes: &'a [Recipe],
    /// Current cursor position (index into `recipes`).
    pub cursor: usize,
    /// Name of the currently selected recipe, if any.
    pub selected: Option<&'a str>,
    /// Whether this panel currently has keyboard focus.
    pub focused: bool,
    /// Current sort order for display in title.
    pub sort_order: SortOrder,
    /// Whether dry-run mode is active.
    pub dry_run: bool,
    /// Current filter text (empty = no filter).
    pub filter: &'a str,
    /// Whether the user is actively typing a filter.
    pub is_filtering: bool,
    /// Parsed theme colors for the UI.
    pub theme: &'a config::ThemeColors,
}

impl<'a> Widget for &RecipePanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        let border_color = if self.focused {
            self.theme.border_focused
        } else {
            self.theme.border_unfocused
        };

        let sort_label = match self.sort_order {
            SortOrder::Name => "A→Z",
            SortOrder::Usage => "by use",
            SortOrder::Category => "by cat",
        };
        let dry_run_label = if self.dry_run { " [DRY RUN]" } else { "" };
        let filter_label = if self.is_filtering || !self.filter.is_empty() {
            format!("[/{}]", self.filter)
        } else {
            String::new()
        };

        let block = Block::default()
            .title(format!(
                " 1: Recipes [{sort_label}]{dry_run_label}{filter_label} "
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        block.render(area, buf);

        // Build filtered view
        let filtered: Vec<&Recipe> = if self.filter.is_empty() {
            self.recipes.iter().collect()
        } else {
            let filter_lower = self.filter.to_lowercase();
            self.recipes
                .iter()
                .filter(|r| {
                    r.name.to_lowercase().contains(&filter_lower)
                        || r.category
                            .as_deref()
                            .is_some_and(|c| c.to_lowercase().contains(&filter_lower))
                        || r.tags
                            .iter()
                            .any(|t| t.to_lowercase().contains(&filter_lower))
                })
                .collect()
        };

        if filtered.is_empty() {
            let text = if self.filter.is_empty() {
                " No recipes loaded "
            } else {
                " No matching recipes "
            };
            buf.set_string(
                inner.x + 1,
                inner.y + 1,
                text,
                Style::default().fg(self.theme.dim_text_fg),
            );
            return;
        }

        let available_height = inner.height as usize;
        let scroll_offset = self
            .cursor
            .saturating_sub(available_height.saturating_sub(1));

        for (i, recipe) in filtered
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(available_height)
        {
            let y = inner.y + (i - scroll_offset) as u16;
            if y >= inner.y + inner.height {
                break;
            }

            let is_cursor = i == self.cursor;
            let is_selected = self.selected.is_some_and(|s| s == recipe.name);

            // First line: cursor + selection + cat_tag + name + usage
            let prefix = if is_cursor { ">" } else { " " };
            let sel_mark = if is_selected { "●" } else { " " };
            let cat_tag = recipe
                .category
                .as_deref()
                .map(|c| format!("[{}] ", c))
                .unwrap_or_default();
            let usage = if recipe.usage_count > 0 {
                format!(" (×{})", recipe.usage_count)
            } else {
                String::new()
            };

            let line1 = format!("{prefix} {sel_mark} {cat_tag}{}{usage}", recipe.name);

            let style = if is_cursor {
                Style::default()
                    .fg(self.theme.cursor_fg)
                    .bg(self.theme.cursor_bg)
            } else if is_selected {
                Style::default().fg(self.theme.selected_fg)
            } else {
                Style::default().fg(self.theme.text_fg)
            };
            buf.set_string(inner.x + 1, y, &line1, style);

            // Second line: description (dimmed)
            let desc_y = y + 1;
            if desc_y < inner.y + inner.height {
                let max_desc_w = inner.width.saturating_sub(2) as usize;
                let desc = if recipe.description.len() > max_desc_w {
                    format!("{}…", &recipe.description[..max_desc_w.saturating_sub(1)])
                } else {
                    recipe.description.clone()
                };
                buf.set_string(
                    inner.x + 2,
                    desc_y,
                    &desc,
                    Style::default().fg(self.theme.dim_text_fg),
                );
            }
        }
    }
}
