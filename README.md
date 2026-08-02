# lazymagick

[![Rust](https://img.shields.io/badge/rust-2024-edition?logo=rust&style=flat&color=orange)](https://doc.rust-lang.org/stable/edition-guide/rust-2024/index.html)
[![Crates.io](https://img.shields.io/crates/v/lazymagick?style=flat)](https://crates.io/crates/lazymagick)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat)](#license)

**A TUI for ImageMagick** — compose and run `magick` recipes interactively, or
batch-process images from the command line. Inspired by
[lazyrsync](https://github.com/westpoint-io/lazyrsync) and
[lazydocker](https://github.com/jesseduffield/lazydocker).

![Screenshot placeholder](https://img.shields.io/badge/screenshot-coming_soon-666?style=flat)
<!-- TODO: Replace with an actual screenshot or asciicast -->

---

## Prerequisites

- **ImageMagick 7+** — the `magick` binary must be on your `$PATH`.
  - Debian/Ubuntu: `sudo apt install imagemagick`
  - Fedora: `sudo dnf install ImageMagick`
  - macOS (Homebrew): `brew install imagemagick`
  - Verify: `magick --version`
- **Rust 1.85+** — required for the 2024 edition. Install via
  [rustup](https://rustup.rs/).

## Installation

```sh
# From source
git clone https://github.com/atareao/lazymagick.git
cd lazymagick
cargo install --path .

# From crates.io (once published)
cargo install lazymagick
```

## Quick Start

Launch the TUI from any directory containing images:

```sh
lazymagick
```

A four-panel interface appears:

1. **Recipes** (left) — browse 42 built-in recipes by category
2. **Files** (center) — navigate directories and select images
3. **Command** (top-right) — preview the exact `magick` command
4. **Log** (bottom-right) — watch progress and results

Basic workflow:

1. **`Tab`** to focus the recipe panel, then **`j`**/**`k`** to pick a recipe
2. **`Enter`** to select it
3. **`Tab`** to the file panel, navigate with **`j`**/**`k`** and **`l`**/**`h`**, press **`Space`** to multi-select files
4. Press **`r`** to run — the command preview updates live

## CLI Batch Mode

lazymagick also works headlessly from the shell:

```sh
# Apply a recipe to all PNGs, output as AVIF
lazymagick -r "weight medium" -f avif *.png

# Print commands without executing them
lazymagick -r "webp 85" --dry-run *.jpg

# Recursive processing with custom output directory
lazymagick -r "compress" -R -o ./optimised/ photos/

# Override recipe output extension and add per-format args
lazymagick -r "thumbnail" -f jpeg banner.png logo.png
```

| Flag | Long | Description |
|------|------|-------------|
| `-r` | `--recipe` | Recipe name to apply (activates headless mode) |
| `-f` | `--format` | Output format override (e.g. `webp`, `avif`, `jpg`) |
| `-o` | `--output` | Output directory (default: same as input) |
| `-R` | `--recursive` | Process files in subdirectories recursively |
| | `--dry-run` | Print commands without executing them |

Positional arguments are input file paths or glob patterns (`*.png`, `photo.jpg`).

## Keybindings

| Key | Action |
|-----|--------|
| `q` / `Ctrl+q` | Quit (Browse mode only) |
| `Tab` / `1`–`4` | Cycle / focus panel |
| `j`/`↓` / `k`/`↑` | Move cursor |
| `Enter` | Select recipe / confirm / enter directory |
| `Space` | Toggle file selection (multi-select) |
| `h`/`←` / `l`/`→` | Parent / enter directory |
| `.` | Toggle hidden files |
| `f` | Open format picker |
| `r` / `c` | Run / cancel current recipe |
| `e` | Edit popup (output dir, extra args) |
| `b` | Before/after comparison |
| `p` | Image preview (Kitty/Sixel/Halfblocks) |
| `x` | EXIF metadata panel |
| `u` | Undo / revert list |
| `E` | Export built-in recipes |
| `R` | Toggle recursive processing |
| `n` | Toggle dry-run mode |
| `s` | Cycle recipe sort (name / usage / category) |
| `?` / `Esc` | Toggle help / close popup |

## Configuration

lazymagick follows the XDG Base Directory Specification:

```
~/.config/lazymagick/
├── recipes/           # User-defined recipes (TOML)
├── settings.toml      # Application settings
└── usage.toml         # Recipe usage counters (auto-managed)
```

### Settings (`settings.toml`)

All fields are optional:

```toml
auto_suffix = "lazymagick"           # Suffix added when input == output format
skip_run_confirm = false             # Skip "run this command?" prompt
skip_overwrite_confirm = false       # Skip "overwrite file?" prompt
default_directory = "~/Pictures"     # Starting directory on launch (optional)
```

### Color Theme

Customise any of 16 colour tokens. Values can be named colours (`Green`,
`Cyan`, `DarkGray`) or hex codes (`"#FF8800"`, `"#abc"`):

```toml
[theme]
border_focused   = "Green"       # Active panel border
border_unfocused = "DarkGray"    # Inactive panel border
cursor_fg        = "Cyan"        # Highlighted line foreground
cursor_bg        = "DarkGray"    # Highlighted line background
selected_fg      = "Green"       # Selected item foreground
text_fg          = "White"       # Normal text
dim_text_fg      = "DarkGray"    # Secondary / muted text
accent_fg        = "Cyan"        # Popup borders, active fields
warning_fg       = "Yellow"      # Warning log level
error_fg         = "Red"         # Error log level
info_fg          = "Blue"        # Info log level
success_fg       = "Green"       # Success log level
progress_fg      = "Cyan"        # Progress bar
directory_fg     = "Blue"        # Directory names in file panel
background       = "Black"       # Popup background
title_fg         = "Yellow"      # Panel titles
```

## Recipe System

42 built-in recipes ship with lazymagick, organised into categories:

### Categories

| Category | Recipes | Description |
|----------|---------|-------------|
| **Adjust** | 2 | Auto-orient, normalize |
| **Resize** | 5 | Fit within, crop center, resize 1920w, resize 50%, thumbnail |
| **Convert** | 8 | Convert, avif 50, jpeg 75, jpeg 90, jpeg xl, webp 85, png optimize, png to jpg |
| **Optimize** | 4 | Compress, reduce palette, trim, strip |
| **Filter** | 15 | Sharpen, blur, sketch + 12 Instagram-style filters (Clarendon, Lo-Fi, Valencia, X-Pro II, Nashville, Walden, Toaster, Earlybird, Gingham, Willow, Amaro, Inkwell) |
| **Decor** | 3 | Border, shadow, watermark |
| **Color** | 2 | Sepia, grayscale |
| **Compress** | 3 | Weight light, weight medium, weight aggressive |

### Recipe format (TOML)

User recipes live in `~/.config/lazymagick/recipes/`. Each `.toml` file can
contain one or more `[[recipe]]` entries:

```toml
[[recipe]]
name = "webp 85"
description = "Convert to WebP with quality 85"
category = "Convert"
tags = ["webp", "convert", "lossy"]
output_ext = "webp"

  [[recipe.stages]]
  flags = ["-strip"]

  [recipe.formats]
  webp = ["-quality", "85"]
  avif = ["-quality", "50"]
```

Key fields:

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique identifier (lowercased on load) |
| `description` | Yes | Short description shown in the recipe panel |
| `output_ext` | Yes | `"auto"` (keep input extension) or a literal extension |
| `category` | No | Grouping category for the recipe list |
| `tags` | No | Filtering / search tags |
| `stages` | No | Processing stages (each stage is a set of flags) |
| `args` | No | Legacy fallback — flat argument list (used if `stages` is empty) |
| `formats` | No | Per-format argument overrides keyed by extension |

Use **`E`** in the TUI to export all built-in recipes as editable files to
`~/.config/lazymagick/recipes/`.

## Format Override

Press **`f`** in the TUI (or pass `-f <ext>` in CLI mode) to override the
output format. When active:

1. The output file extension changes (e.g. `.png` → `.webp`)
2. If the recipe defines per-format arguments under `[recipe.formats]`, those
   are appended to the command
3. If no per-format args exist, only the extension changes — the recipe's base
   `args` or `stages` still apply

This makes it trivial to adapt any recipe to any output format without editing
the recipe itself.

## Image Preview

Press **`p`** on a selected file to open an inline image preview. lazymagick
auto-detects the terminal protocol:

- **Kitty** protocol (kitty terminal)
- **Sixel** protocol (foot, WezTerm, mlterm)
- **Halfblocks** fallback (any terminal with Unicode support)

Requires the `ratatui-image` crate (bundled) and a terminal that supports one
of the above protocols.

## License

MIT — see [LICENSE](LICENSE).