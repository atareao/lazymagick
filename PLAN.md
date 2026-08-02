# Phase 2 — P1 Features Implementation Plan

> **Status: ✅ COMPLETE (3 commits)**

```
0603838 feat(ui): add inline recipe search/filter — type to filter recipes by name, category, or tags
f06bfa0 feat(recipe): add E key to export built-in recipes to ~/.config/lazymagick/recipes/
69ed12f feat(cli): add headless batch mode with clap — -r/--recipe, -f/--format, -o/--output, --dry-run
```

- **62 tests**: all pass
- **clippy**: clean
- **fmt**: clean
- **build**: release-ready

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

### Task 1: CLI / Batch Headless Mode ✅

**Files:**
- Modify: `Cargo.toml` — add `clap` + `glob` deps
- Create: `src/cli.rs` — Clap derive struct
- Modify: `src/main.rs` — parse CLI, run headless or TUI

**Interfaces:**
- Consumes: `recipe::load_all()`, `magick::CommandBuilder::build_argv()`, `magick::CommandBuilder::check_available()`
- Produces: `cli::Cli` struct, `run_headless(cli: Cli)` function

**Usage:**
```
lazymagick -r "weight medium" -f avif *.png
lazymagick -r "strip" --dry-run photo.jpg
lazymagick -r "resize 50%" -o ./thumbnails/ *.jpg
```

---

### Task 2: Export Built-In Recipes (E Key) ✅

**Files:**
- Modify: `src/recipe.rs` — add `export_builtins()`
- Modify: `src/app.rs` — add `E` key handler

**Interfaces:**
- Consumes: `config::user_recipes_dir()`, `include_str!("../recipes/builtins.toml")`
- Produces: `recipe::export_builtins() -> Result<usize, String>`

Press `E` in browse mode to export 42 built-in recipes to `~/.config/lazymagick/recipes/builtins.toml`.

---

### Task 3: Inline Recipe Search / Filter ✅

**Files:**
- Modify: `src/app.rs` — add `recipe_filter`, `is_filtering`, filtered view methods
- Modify: `src/ui/recipe_panel.rs` — accept filter, highlight matches
- Modify: `src/ui/mod.rs` — pass filter state to widget

**Interfaces:**
- App gains: `recipe_filter: String`, `is_filtering: bool`
- App gains: `fn filtered_recipes(&self) -> Vec<&Recipe>`
- App gains: `fn filtered_recipe_index(&self, idx: usize) -> Option<usize>`
- RecipePanel gains: `filter: &'a str`, `is_filtering: bool`

Type any characters in the recipe panel to filter by name, category, or tags. `Esc` clears filter, `Enter` selects first match.