// lazymagick — A TUI for ImageMagick

pub mod app;
pub mod cli;
pub mod config;
pub mod fs_utils;
pub mod magick;
pub mod recipe;
pub mod ui;

use clap::Parser;
use color_eyre::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

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

fn run_app(mut terminal: ratatui::DefaultTerminal) -> Result<()> {
    let tick_rate = Duration::from_millis(100);
    let mut app = app::App::new();

    // ── Load persisted usage counts ──────────────────────────
    let usage = config::load_usage();
    for recipe in &mut app.recipes {
        if let Some(&count) = usage.get(&recipe.name) {
            recipe.usage_count = count;
        }
    }
    app.sort_recipes();

    // ── Load settings ───────────────────────────────────────
    let settings = config::Settings::load();
    if let Some(ref dir) = settings.default_directory {
        let p = PathBuf::from(dir);
        if p.is_dir() {
            app.enter_directory(&p);
        }
    }

    let mut last_tick = std::time::Instant::now();

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        let timeout = tick_rate
            .saturating_sub(last_tick.elapsed())
            .max(Duration::from_millis(10));

        if crossterm::event::poll(timeout)? {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(key) => {
                    app.on_key(key);
                }
                crossterm::event::Event::Resize(_w, _h) => {
                    // Force a redraw on the next frame; no state change needed.
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = std::time::Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    // ── Save usage counts on quit ────────────────────────────
    let mut usage_map: HashMap<String, u64> = HashMap::new();
    for recipe in &app.recipes {
        usage_map.insert(recipe.name.clone(), recipe.usage_count);
    }
    if let Err(e) = config::save_usage(&usage_map) {
        eprintln!("Warning: failed to save usage: {e}");
    }

    // ── Save settings on quit ────────────────────────────────
    let current_dir = app.current_dir.to_string_lossy().to_string();
    let settings = config::Settings {
        auto_suffix: "lazymagick".into(),
        skip_run_confirm: false,
        skip_overwrite_confirm: false,
        default_directory: Some(current_dir),
        ..Default::default()
    };
    if let Err(e) = settings.save() {
        eprintln!("Warning: failed to save settings: {e}");
    }

    Ok(())
}

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
                    if cli.recursive && entry.is_dir() {
                        // Walk subdirectories recursively
                        for sub in walkdir::WalkDir::new(&entry)
                            .follow_links(true)
                            .into_iter()
                            .filter_map(|e| e.ok())
                        {
                            let path = sub.path().to_path_buf();
                            if path.is_file() && fs_utils::is_image(&path) {
                                files.push(path);
                            }
                        }
                    } else if entry.is_file() && fs_utils::is_image(&entry) {
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
            let ext = output.extension().and_then(|e| e.to_str()).unwrap_or("");
            fs_utils::safe_output_path(file, ext, &recipe.name)
        };

        let args = recipe.resolved_args(format_override);
        let argv = magick::CommandBuilder::build_argv(file, &args, &[], &safe_output);

        if cli.dry_run {
            println!("{}", argv.join(" "));
            continue;
        }

        eprint!(
            "[{}] {} → {} ... ",
            errors + 1,
            file.display(),
            safe_output.display()
        );
        match std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(mut child) => match child.wait() {
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
            },
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
