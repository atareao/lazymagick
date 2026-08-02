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
    pub paths: Vec<String>,
}