//! CLI argument parsing for headless batch mode.

use clap::Parser;

/// A TUI for ImageMagick — compose and run magick recipes interactively
/// or batch-process images from the command line.
///
/// Usage:
///   lazymagick                    # Launch the TUI
///   lazymagick -r "weight medium" -f avif *.png   # Batch mode
///   lazymagick -r "strip" --dry-run photo.jpg     # Dry run
///   lazymagick -r "resize 50%" -o ./thumbs/ *.jpg # Custom output dir
///   lazymagick --completions bash > completions    # Generate shell completions
///
/// Recipes are defined in TOML under ~/.config/lazymagick/recipes/
/// or shipped as 42 built-in recipes (Basic, Advanced, Instagram, Weight, Utility).
#[derive(Debug, Parser)]
#[command(name = "lazymagick", version, about, long_about = None)]
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

    /// Process files in subdirectories recursively.
    #[arg(short = 'R', long = "recursive")]
    pub recursive: bool,

    /// Generate shell completion script (bash, zsh, or fish).
    #[arg(long = "completions")]
    pub completions: Option<clap_complete::Shell>,

    /// Input file paths / glob patterns (e.g. `*.png`, `photo.jpg`).
    pub paths: Vec<String>,
}
