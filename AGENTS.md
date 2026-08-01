# lazymagick — AGENTS.md

## What this is

A TUI tool (ratatui) that wraps ImageMagick's `magick` command — similar to
lazyrsync / lazydocker but for image processing. Users compose "recipes"
(preset magick command chains) and run them interactively.

## Key facts

- **Language**: Rust, edition **2024** (requires rustc ≥ 1.85; CI must pin ≥ 1.85).
- **TUI framework**: `ratatui` (the canonical Rust TUI library).
- **ImageMagick**: shells out to `/usr/bin/magick` via `std::process::Command`.
  ImageMagick 7.1.2 is installed on the dev machine with full delegates (cairo,
  svg, heic, jxl, webp, etc.). The tool must work with whatever `magick` is on
  `$PATH`.
- **Recipes**: declarative config (TOML or JSON) that defines a magick command
  with input, output, options, and optional pipeline stages. Shipped with a set
  of built-in recipes; users can add their own.
- **Output format**: recipes must make it trivial to change the output format
  (e.g., PNG → WebP, JPEG → AVIF) — this is a first-class concern, not an
  afterthought.

## Architecture

```
src/
  main.rs                — entrypoint, TUI event loop, panic recovery
  app.rs                 — App state machine, Mode/Focus enums, key dispatch, process mgmt
  recipe.rs              — Recipe struct, TOML parsing, built-in recipes (include_str!), loader
  magick.rs              — CommandBuilder, shell-out to `magick`, identify, MagickError
  config.rs              — XDG path discovery, Settings struct, TOML persistence
  fs_utils.rs            — file listing, image detection, safe_output_path, formatting
  ui/
    mod.rs               — top-level render(), orchestrates all panels
    layout.rs            — 4-panel layout calculation (chunk_areas)
    recipe_panel.rs      — scrollable recipe list widget
    file_panel.rs        — directory browser with multi-select, hidden files toggle
    command_panel.rs     — magick command preview + image metadata
    log_panel.rs         — color-coded log with live output streaming
    format_picker.rs     — format override popup (j/k, Enter, Esc)
    help_popup.rs        — keybinding reference overlay
    edit_popup.rs        — output dir + extra args edit popup
recipes/                 — 10 built-in recipes as TOML files
```

### Keybindings

| Key | Action |
|---|---|
| `q` / `Ctrl+q` | Quit (Browse mode only) |
| `Tab` / `1`-`4` | Cycle / focus panel |
| `j`/`↓` / `k`/`↑` | Move cursor |
| `Enter` | Select recipe / confirm / enter directory |
| `Space` | Toggle file selection (multi-select) |
| `h`/`←` / `l`/`→` | Parent / enter directory |
| `.` | Toggle hidden files |
| `f` | Open format picker |
| `r` / `c` | Run / cancel current recipe |
| `e` | Edit popup (output dir, extra args) |
| `?` / `Esc` | Toggle help / close popup |

## Commands

```sh
cargo build               # debug build
cargo build --release     # release build
cargo run                 # run the TUI
cargo test                # all tests
cargo clippy              # lint (must pass before commits)
cargo fmt                 # format (must pass before commits)
```

No CI, no pre-commit hooks, no task runner configured yet. Add them as the
project matures.

## Recipe system essentials

- Built-in recipes live in `recipes/` at the project root (or embedded as
  `include_str!` / const strings).
- Each recipe has: name, description, input placeholder, output extension,
  and a list of magick operations/options.
- The output format must be overridable at runtime (e.g., `--format webp`
  replaces the output extension and adds the appropriate encoder options).
- User recipes go in `~/.config/lazymagick/recipes/` (XDG convention).

## Gotchas

- **Edition 2024**: `cargo fmt` uses the 2024 style. `gen` is a reserved
  keyword. `unsafe` blocks in `static`/`const` initializers are now allowed.
  `impl Trait` in return position is `use<..>` syntax. Run `cargo fmt` after
  any significant edit.
- **No git history yet**: the repo has zero commits. First meaningful commit
  should set up `.gitignore` (already has `/target`), `Cargo.lock` tracking,
  and a sensible initial structure.
- **`magick` must be on `$PATH`**: the tool does not bundle ImageMagick.
  Document this as a prerequisite in the README.
- **ratatui version**: pin a recent stable ratatui (≥ 0.28). Use
  `crossterm` as the backend (most common, well-supported).
- **Testing TUI apps**: prefer testing the business logic (recipe parsing,
  command building) with standard `#[test]`. TUI integration tests are
  low priority — use `assert_contains` on rendered strings if needed.

## References

- ImageMagick command-line tools: https://imagemagick.org/command-line-tools/
- ImageMagick options: https://imagemagick.org/command-line-processing/#option
- lazyrsync (inspiration): https://github.com/westpoint-io/lazyrsync