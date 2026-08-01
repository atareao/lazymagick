// lazymagick — A TUI for ImageMagick

pub mod app;
pub mod config;
pub mod fs_utils;
pub mod magick;
pub mod recipe;
pub mod ui;

use color_eyre::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

fn main() -> Result<()> {
    color_eyre::install()?;

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
    };
    if let Err(e) = settings.save() {
        eprintln!("Warning: failed to save settings: {e}");
    }

    Ok(())
}