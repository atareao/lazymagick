# Phase 2 — P1 Features Implementation Plan

> **For agentic workers:** Implement tasks in order — each builds on the previous.

**Goal:** Add 3 high-priority features: CLI headless mode (clap), export built-in recipes (E key), and inline recipe search/filter (fzf-style).

**Architecture:** Extend `main.rs` with a CLI parser before the TUI entry point; add export logic to `recipe.rs` and wire the `E` key in `app.rs`; add filtering state to `App` and modify the recipe panel to respect filters.

**Tech Stack:** Rust 2024 edition, ratatui 0.29, crossterm 0.28, clap 4 (derive), glob 0.3, serde, toml, dirs

---

## Global Constraints

- Edition 2024: `gen` is reserved, `let` chain patterns in `if`/`match`
- All 62 existing tests must continue to pass
- clippy must remain clean (`cargo clippy`)
- Format with `cargo fmt` (2024 edition style)
- No new files outside `src/` — add `src/cli.rs` only for the CLI struct

---

### Task 1: CLI / Batch Headless Mode

**Files:**
- Modify: `Cargo.toml` — add `clap` + `glob` deps
- Create: `src/cli.rs` — Clap derive struct
- Modify: `src/main.rs` — parse CLI, run headless or TUI

**Interfaces:**
- Consumes: `recipe::load_all()`, `magick::CommandBuilder::build_argv()`, `magick::CommandBuilder::check_available()`
- Produces: `cli::Cli` struct, `run_headless(cli: Cli)` function

- [ ] **Step 1: Add dependencies to Cargo.toml**

```toml
clap = { version = "4", features = ["derive"] }
glob = "0.3"
```

Add after the existing `color-eyre` line (line 13).

- [ ] **Step 2: Create `src/cli.rs` with Clap derive struct**

```rust
//! CLI argument parsing for headless batch mode.

use clap::Parser;

/// A TUI for ImageMagick — compose and run magick recipes interactively
/// or batch-process images from the command line.
#[derive(Debug, Parser)]
#[command(name = "lazymagick", version, about)]
pub struct Cli {
    /// Recipe name to apply (activates headless batch mode).
    #[arg(short = 'r', long = "recipe")]
    pub recipe: Option<String>,

    /// Output format override (e.g. "webp", "avif", "jpg").
    #[arg(short = 'f', long = "format")]
    pub format: Option<String>,

    /// Output directory (default: same directory as input).
    #[arg(short = 'o', long = "output")]
    pub output: Option<String>,

    /// Print commands without executing them.
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Input file paths / glob patterns (e.g. `*.png`, `photo.jpg`).
    #[arg(required_unless_present = "recipe")]
    pub paths: Vec<String>,
}
```

- [ ] **Step 3: Add `run_headless` function to `src/main.rs`**

Add at the end of main.rs (before the closing `// cli.rs` module marker):

```rust
/// Run lazymagick in headless batch mode.
///
/// Loads recipes, expands paths, builds commands for each matching file,
/// and either prints the commands (dry-run) or executes them sequentially.
fn run_headless(cli: cli::Cli) -> Result<()> {
    if !magick::CommandBuilder::check_available() {
        eprintln!("Error: 'magick' not found on $PATH. Install ImageMagick 7+ first.");
        std::process::exit(1);
    }

    let recipe_name = match cli.recipe {
        Some(ref n) => n.to_lowercase(),
        None => {
            eprintln!("Error: --recipe is required in batch mode");
            std::process::exit(1);
        }
    };

    let recipes = recipe::load_all();
    let recipe = match recipes.iter().find(|r| r.name == recipe_name) {
        Some(r) => r,
        None => {
            eprintln!("Error: recipe '{recipe_name}' not found");
            std::process::exit(1);
        }
    };

    // Expand glob patterns into file paths
    let mut files: Vec<PathBuf> = Vec::new();
    for pattern in &cli.paths {
        match glob::glob(pattern) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if entry.is_file() && fs_utils::is_image(&entry) {
                        files.push(entry);
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: invalid glob pattern '{pattern}': {e}");
            }
        }
    }

    if files.is_empty() {
        eprintln!("Error: no matching image files found");
        std::process::exit(1);
    }

    // Sort for deterministic order
    files.sort();

    let format_override = cli.format.as_deref();
    let output_dir = cli.output.as_ref().map(PathBuf::from);

    let mut errors = 0;
    for file in &files {
        let output = output_dir
            .clone()
            .map(|dir| {
                let fname = file.file_name().unwrap_or_default();
                dir.join(fname)
            })
            .unwrap_or_else(|| recipe.output_path(file, format_override));

        // Use safe output path when no output dir override and no format change
        let safe_output = if output_dir.is_some() || format_override.is_some() {
            output
        } else {
            let ext = output
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            fs_utils::safe_output_path(file, ext, &recipe.name)
        };

        let args = recipe.resolved_args(format_override);
        let argv = magick::CommandBuilder::build_argv(file, &args, &[], &safe_output);

        if cli.dry_run {
            println!("{}", argv.join(" "));
            continue;
        }

        eprint!("[{}] {} → {} ... ", errors + 1, file.display(), safe_output.display());
        match std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                match child.wait() {
                    Ok(status) if status.success() => {
                        eprintln!("OK");
                    }
                    Ok(status) => {
                        eprintln!("FAIL (exit: {status})");
                        errors += 1;
                    }
                    Err(e) => {
                        eprintln!("FAIL (wait: {e})");
                        errors += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("FAIL (spawn: {e})");
                errors += 1;
            }
        }
    }

    if errors > 0 {
        eprintln!("\n{errors} file(s) failed");
        std::process::exit(1);
    }
    std::process::exit(0);
}
```

- [ ] **Step 4: Wire CLI parsing in `main()`**

Replace the current `main()` function:

```rust
fn main() -> Result<()> {
    color_eyre::install()?;

    // Parse CLI arguments
    let cli = cli::Cli::parse();

    // If a recipe is specified, run in headless batch mode
    if cli.recipe.is_some() {
        return run_headless(cli);
    }

    // Otherwise, enter TUI mode
    if !magick::CommandBuilder::check_available() {
        eprintln!("Error: 'magick' not found on $PATH. Install ImageMagick 7+ first.");
        std::process::exit(1);
    }

    let terminal = ratatui::init();
    let result = run_app(terminal);
    ratatui::restore();
    result
}
```

Remove the old `magick` check from the current main() since it's now inside the TUI path only.

- [ ] **Step 5: Add `mod cli;` declaration and import `clap::Parser` in `main.rs`**

Add at the top of main.rs alongside other `pub mod` declarations:

```rust
pub mod cli;
```

Also add at the top of main.rs imports:
```rust
use clap::Parser;
```

- [ ] **Step 6: Run tests and verify compilation**

```bash
cd /data/rust/lazymagick && cargo test 2>&1 | tail -10
cd /data/rust/lazymagick && cargo build 2>&1
```

Expected: 62 tests pass, build succeeds.

- [ ] **Step 7: Commit**

```bash
cd /data/rust/lazymagick && git add -A && git commit -m "feat(cli): add headless batch mode with clap — -r/--recipe, -f/--format, -o/--output, --dry-run"
```

---

### Task 2: Export Built-In Recipes (E Key)

**Files:**
- Modify: `src/recipe.rs` — add `export_builtins()`
- Modify: `src/app.rs` — add `E` key handler

**Interfaces:**
- Consumes: `config::user_recipes_dir()`, `include_str!("../recipes/builtins.toml")`
- Produces: `recipe::export_builtins() -> Result<usize, String>`

- [ ] **Step 1: Add `export_builtins()` to `src/recipe.rs`**

Add after `load_all()` (around line 292):

```rust
/// Export built-in recipes to the user config directory for editing.
///
/// Creates `~/.config/lazymagick/recipes/builtins.toml` containing
/// all built-in recipes. Existing files are overwritten.
///
/// Returns the number of recipes exported, or an error message.
pub fn export_builtins() -> Result<usize, String> {
    let dest_dir = crate::config::user_recipes_dir();
    std::fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Cannot create recipes dir: {e}"))?;

    let dest_path = dest_dir.join("builtins.toml");
    let content = include_str!("../recipes/builtins.toml");
    std::fs::write(&dest_path, content)
        .map_err(|e| format!("Cannot write recipes file: {e}"))?;

    // Count how many recipes were exported
    let count = load_builtin().len();
    Ok(count)
}
```

- [ ] **Step 2: Add `E` key handler in `src/app.rs`**

In `on_key()`, after the existing action keys block (around line 608, after `KeyCode::Char('e')` handling), add:

```rust
KeyCode::Char('E') => {
    match crate::recipe::export_builtins() {
        Ok(count) => {
            self.add_log(
                format!("Exported {count} built-in recipes to ~/.config/lazymagick/recipes/"),
                LogLevel::Success,
            );
            // Reload user recipes
            let user_recipes = crate::recipe::load_user();
            for user_recipe in user_recipes {
                if let Some(pos) = self.recipes.iter().position(|r| r.name == user_recipe.name) {
                    self.recipes[pos] = user_recipe;
                } else {
                    self.recipes.push(user_recipe);
                }
            }
            self.sort_recipes();
        }
        Err(e) => {
            self.add_log(format!("Failed to export recipes: {e}"), LogLevel::Error);
        }
    }
}
```

- [ ] **Step 3: Run tests and verify compilation**

```bash
cd /data/rust/lazymagick && cargo test 2>&1 | tail -10
cd /data/rust/lazymagick && cargo build 2>&1
```

Expected: 62+ tests pass, build succeeds.

- [ ] **Step 4: Commit**

```bash
cd /data/rust/lazymagick && git add -A && git commit -m "feat(recipe): add E key to export built-in recipes to ~/.config/lazymagick/recipes/"
```

---

### Task 3: Inline Recipe Search / Filter

**Files:**
- Modify: `src/app.rs` — add `recipe_filter`, `is_filtering`, filtered view methods
- Modify: `src/ui/recipe_panel.rs` — accept filter, highlight matches
- Modify: `src/ui/mod.rs` — pass filter state to widget

**Interfaces:**
- App gains: `recipe_filter: String`, `is_filtering: bool`
- App gains: `fn filtered_recipes(&self) -> Vec<&Recipe>`
- App gains: `fn filtered_recipe_index(&self, idx: usize) -> Option<usize>`
- RecipePanel gains: `filter: &'a str`

- [ ] **Step 1: Add new fields to `App` struct in `src/app.rs`**

Add after `recipe_sort` field (around line 198):

```rust
    /// Current recipe filter text (empty = no filter).
    pub recipe_filter: String,
    /// Whether the user is currently typing a filter.
    pub is_filtering: bool,
```

- [ ] **Step 2: Initialize new fields in `App::new()`**

In the constructor (around line 302), add after `recipe_sort: SortOrder::default(),`:

```rust
            recipe_filter: String::new(),
            is_filtering: false,
```

- [ ] **Step 3: Add `filtered_recipes()` and `filtered_recipe_index()` methods to `App`**

Add after `sort_recipes()` (around line 864):

```rust
    /// Return recipes matching the current filter.
    ///
    /// When `recipe_filter` is empty, returns all recipes (unfiltered).
    /// Otherwise, matches case-insensitively against name, category, and tags.
    pub fn filtered_recipes(&self) -> Vec<&Recipe> {
        if self.recipe_filter.is_empty() {
            return self.recipes.iter().collect();
        }

        let filter = self.recipe_filter.to_lowercase();
        self.recipes
            .iter()
            .filter(|r| {
                r.name.to_lowercase().contains(&filter)
                    || r.category
                        .as_deref()
                        .is_some_and(|c| c.to_lowercase().contains(&filter))
                    || r.tags.iter().any(|t| t.to_lowercase().contains(&filter))
            })
            .collect()
    }

    /// Translate a cursor position into the real recipe index based on the filter.
    ///
    /// Returns `None` if the cursor is out of range or no recipes match.
    pub fn filtered_recipe_index(&self, cursor: usize) -> Option<usize> {
        let filtered = self.filtered_recipes();
        filtered.get(cursor).and_then(|&r| {
            self.recipes.iter().position(|x| std::ptr::eq(x, r))
        })
    }
```

- [ ] **Step 4: Modify `handle_recipe_focus` for filter input**

Replace the current `handle_recipe_focus` method (lines 614-648) with:

```rust
    fn handle_recipe_focus(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if self.is_filtering {
                    self.is_filtering = false;
                    self.recipe_filter.clear();
                }
            }
            KeyCode::Enter if self.is_filtering && !self.recipe_filter.is_empty() => {
                // Select the first matching recipe
                let filtered = self.filtered_recipes();
                if let Some(&recipe) = filtered.first() {
                    self.selected_recipe_name = Some(recipe.name.clone());
                    self.update_available_formats();
                    self.generate_preview();
                }
                self.is_filtering = false;
            }
            KeyCode::Char(c) if !self.is_filtering && (c.is_alphanumeric() || c == '/' || c == '-' || c == ' ') => {
                self.is_filtering = true;
                self.recipe_filter.push(c);
                self.recipe_cursor = 0;
            }
            KeyCode::Backspace if self.is_filtering => {
                self.recipe_filter.pop();
                self.recipe_cursor = 0;
                if self.recipe_filter.is_empty() {
                    self.is_filtering = false;
                }
            }
            KeyCode::Char(c) if self.is_filtering => {
                self.recipe_filter.push(c);
                self.recipe_cursor = 0;
            }
            _ if !self.is_filtering => {
                // Existing navigation keys
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.recipe_cursor = self.recipe_cursor
                            .saturating_add(1)
                            .min(self.filtered_recipes().len().saturating_sub(1));
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        self.recipe_cursor = self.recipe_cursor.saturating_sub(1);
                    }
                    KeyCode::Enter if !self.recipes.is_empty() => {
                        // Use filtered index
                        if let Some(real_idx) = self.filtered_recipe_index(self.recipe_cursor) {
                            let name = self.recipes[real_idx].name.clone();
                            self.selected_recipe_name = Some(name);
                            self.update_available_formats();
                            self.generate_preview();
                        }
                    }
                    KeyCode::Char('g') => {
                        self.recipe_cursor = 0;
                    }
                    KeyCode::Char('G') => {
                        self.recipe_cursor = self.filtered_recipes().len().saturating_sub(1);
                    }
                    KeyCode::PageUp | KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                        self.recipe_cursor = self.recipe_cursor.saturating_sub(10);
                    }
                    KeyCode::PageDown | KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                        self.recipe_cursor = self.recipe_cursor
                            .saturating_add(10)
                            .min(self.filtered_recipes().len().saturating_sub(1));
                    }
                    _ => {}
                }
            }
        }
    }
```

Note: The `KeyCode::Enter` in the existing handler that selects a recipe is now gated on `!self.is_filtering`. When filtering, `Enter` selects the first matching recipe and exits filter mode.

- [ ] **Step 5: Update `RecipePanel` widget to accept filter and render filtered list**

Replace the `RecipePanel` struct and its `Widget` impl in `src/ui/recipe_panel.rs`:

```rust
pub struct RecipePanel<'a> {
    pub recipes: &'a [Recipe],
    pub cursor: usize,
    pub selected: Option<&'a str>,
    pub focused: bool,
    pub sort_order: SortOrder,
    pub dry_run: bool,
    pub filter: &'a str,
    pub is_filtering: bool,
}
```

Update the title line to show filter when active:

```rust
let filter_label = if self.is_filtering {
    format!("[/{}]", self.filter)
} else if !self.filter.is_empty() {
    format!("[/{}]", self.filter)
} else {
    String::new()
};
let block = Block::default()
    .title(format!(" 1: Recipes [{sort_label}]{dry_run_label}{filter_label} "))
    ...
```

When rendering entries, only show filtered recipes. Pass the filtered list and adjust cursor:

```rust
// Build the filtered view
let filtered: Vec<&Recipe> = if self.filter.is_empty() {
    self.recipes.iter().collect()
} else {
    let filter_lower = self.filter.to_lowercase();
    self.recipes.iter().filter(|r| {
        r.name.to_lowercase().contains(&filter_lower)
            || r.category.as_deref().is_some_and(|c| c.to_lowercase().contains(&filter_lower))
            || r.tags.iter().any(|t| t.to_lowercase().contains(&filter_lower))
    }).collect()
};

let available_height = inner.height as usize;
let scroll_offset = self.cursor.saturating_sub(available_height.saturating_sub(1));

for i in scroll_offset..filtered.len().min(scroll_offset + available_height) {
    let recipe = filtered[i];
    // ... same rendering logic as before but using `filtered[i]` instead of `self.recipes[i]`
}
```

Replace `self.recipes.len()` with `filtered.len()` for `G` key rendering (the cursor is now in filtered space, not raw recipe space).

- [ ] **Step 6: Update `render()` in `src/ui/mod.rs` to pass filter state**

In the `RecipePanel` construction (around line 30-38), add:

```rust
let filtered_count = app.filtered_recipes().len();
let recipe_widget = recipe_panel::RecipePanel {
    recipes: &app.recipes,
    cursor: app.recipe_cursor.min(filtered_count.saturating_sub(1)),
    selected: app.selected_recipe_name.as_deref(),
    focused: app.focus == Focus::Recipe,
    sort_order: app.recipe_sort,
    dry_run: app.dry_run,
    filter: &app.recipe_filter,
    is_filtering: app.is_filtering,
};
```

- [ ] **Step 7: Run tests and verify compilation**

```bash
cd /data/rust/lazymagick && cargo test 2>&1 | tail -10
cd /data/rust/lazymagick && cargo build 2>&1
```

Expected: 62+ tests pass, build succeeds.

- [ ] **Step 8: Commit**

```bash
cd /data/rust/lazymagick && git add -A && git commit -m "feat(ui): add inline recipe search/filter — type to filter recipes by name, category, or tags"
```