//! App state, event loop, and key dispatch for the lazymagick TUI.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use ratatui_image::picker::Picker;

use crate::config;
use crate::fs_utils;
use crate::magick::{self, CommandBuilder};
use crate::recipe::{self, Recipe};

// ---------------------------------------------------------------------------
// Spinner
// ---------------------------------------------------------------------------

const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

// ---------------------------------------------------------------------------
// SortOrder
// ---------------------------------------------------------------------------

/// How to sort recipes in the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    /// Alphabetical by name.
    #[default]
    Name,
    /// By usage count (descending).
    Usage,
    /// By category, then name.
    Category,
}

// ---------------------------------------------------------------------------
// LogLevel
// ---------------------------------------------------------------------------

/// Severity / kind of a log entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Informational message.
    Info,
    /// Successful operation.
    Success,
    /// Error / failure.
    Error,
    /// Raw magick stderr line.
    Magick,
}

// ---------------------------------------------------------------------------
// EditField
// ---------------------------------------------------------------------------

/// Which field is currently being edited in the edit popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditField {
    OutputDir,
    ExtraArgs,
}

// ---------------------------------------------------------------------------
// Mode
// ---------------------------------------------------------------------------

/// Application mode — determines which set of keybindings is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Default mode: browse recipes, files, and command preview.
    #[default]
    Browse,
    /// Edit parameters popup is open.
    Edit,
    /// A recipe is running (process spawned).
    Run,
    /// Help overlay is open.
    Help,
}

// ---------------------------------------------------------------------------
// Focus
// ---------------------------------------------------------------------------

/// Which panel has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// Left panel — recipe list.
    Recipe,
    /// Middle-top panel — file browser.
    File,
    /// Right panel — command preview.
    Command,
    /// Bottom panel — log / output.
    Log,
}

impl Focus {
    /// Cycle to the next panel (Recipe → File → Command → Log → Recipe).
    pub fn next(self) -> Self {
        match self {
            Focus::Recipe => Focus::File,
            Focus::File => Focus::Command,
            Focus::Command => Focus::Log,
            Focus::Log => Focus::Recipe,
        }
    }
}

// ---------------------------------------------------------------------------
// LogEntry
// ---------------------------------------------------------------------------

/// A single log line displayed in the log panel.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// The log message text.
    pub message: String,
    /// Severity / kind of the entry.
    pub level: LogLevel,
    /// Human-readable timestamp (e.g. `"12:34:56"`).
    pub timestamp: String,
}

// ---------------------------------------------------------------------------
// ProcessingJob
// ---------------------------------------------------------------------------

/// A single queued image-processing job (one input file).
#[derive(Debug, Clone)]
pub struct ProcessingJob {
    /// Name of the recipe to apply.
    pub recipe_name: String,
    /// Path to the input image file.
    pub input: PathBuf,
    /// Path to the intended output file.
    pub output: PathBuf,
    /// Full argument vector including `"magick"` as argv[0].
    pub argv: Vec<String>,
}

// ---------------------------------------------------------------------------
// MagickHandle
// ---------------------------------------------------------------------------

/// Handle to a running `magick` child process and its output reader.
pub struct MagickHandle {
    /// The child process.
    pub child: Child,
    /// Receiver for live stderr lines.
    pub rx: mpsc::Receiver<String>,
    /// Shared cancel flag for the reader thread.
    pub cancel: Arc<AtomicBool>,
    /// Handle to the stderr reader thread (taken on kill for join).
    pub thread_handle: Option<thread::JoinHandle<()>>,
}

impl MagickHandle {
    /// Kill the child process and join the reader thread.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(h) = self.thread_handle.take() {
            let _ = h.join();
        }
    }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

/// Central application state.
pub struct App {
    // -- Mode & lifecycle -------------------------------------------
    /// Current application mode.
    pub mode: Mode,
    /// Whether the event loop should exit on the next tick.
    pub should_quit: bool,
    /// Last tick timestamp (used for rate-limiting).
    pub last_tick: Instant,
    /// Which panel currently has keyboard focus.
    pub focus: Focus,

    // -- Recipes ----------------------------------------------------
    /// All loaded recipes (built-in + user).
    pub recipes: Vec<Recipe>,
    /// Cursor position in the recipe list.
    pub recipe_cursor: usize,
    /// Name of the currently selected recipe, if any.
    pub selected_recipe_name: Option<String>,
    /// Current sort order for the recipe list.
    pub recipe_sort: SortOrder,
    /// Current recipe filter text (empty = no filter).
    pub recipe_filter: String,
    /// Whether the user is currently typing a filter.
    pub is_filtering: bool,

    // -- File browser -----------------------------------------------
    /// Current working directory in the file panel.
    pub current_dir: PathBuf,
    /// Cached directory listing for `current_dir`.
    pub dir_listing: fs_utils::DirListing,
    /// Cursor position in the file panel (into the visible entries list).
    pub file_cursor: usize,
    /// Files that have been marked for processing.
    pub selected_files: Vec<PathBuf>,
    /// Whether to show hidden files (names starting with `.`).
    pub show_hidden: bool,

    // -- Command preview & format -----------------------------------
    /// Active format override extension (e.g. `"webp"`).
    pub format_override: Option<String>,
    /// Format names available in the current recipe.
    pub available_formats: Vec<String>,

    // -- Format picker ----------------------------------------------
    /// Whether the format picker popup is visible.
    pub show_format_picker: bool,
    /// Cursor position in the format picker.
    pub format_picker_cursor: usize,

    // -- Edit popup -------------------------------------------------
    /// Whether the edit parameters popup is visible.
    /// Whether the directory browser popup is open (inside edit mode).
    pub show_dir_browser: bool,
    /// Current path being browsed in the directory picker.
    pub dir_browser_path: PathBuf,
    /// Cursor position in the directory browser listing.
    pub dir_browser_cursor: usize,
    /// Current directory listing for the directory browser.
    pub dir_browser_listing: Vec<PathBuf>,

    pub show_edit_popup: bool,
    /// Output directory override (None = same dir as input).
    pub edit_output_dir: Option<PathBuf>,
    /// Extra ImageMagick arguments as a space-separated string.
    pub edit_extra_args: String,
    /// Which edit field is currently active.
    pub edit_field: EditField,
    /// Current text buffer for the output dir field.
    pub edit_output_buf: String,
    /// Current text buffer for the extra args field.
    pub edit_args_buf: String,

    // -- Log --------------------------------------------------------
    /// All log entries (oldest first).
    pub log_entries: Vec<LogEntry>,

    // -- Help -------------------------------------------------------
    /// Whether the help overlay is visible.
    pub show_help: bool,

    // -- Process management -----------------------------------------
    /// Handle to a running magick process (if any).
    pub magick_handle: Option<MagickHandle>,
    /// Current spinner frame index.
    pub spinner_frame: usize,
    /// Whether the spinner animation is active.
    pub spinner_active: bool,
    /// Accumulated process stderr output.
    /// Parsed progress from magick `-monitor` stderr.
    pub progress_current: u64,
    /// Total units for progress bar.
    pub progress_total: u64,
    /// Name of the current processing stage.
    pub progress_stage: String,
    pub process_output: String,

    // -- Processing queue -------------------------------------------
    /// Queued processing jobs for batch execution.
    pub processing_queue: Vec<ProcessingJob>,
    /// Index of the next job to execute in the queue.
    pub processing_queue_index: usize,

    /// Whether to perform a dry run (log commands only, no execution).
    pub dry_run: bool,
    /// Whether to process files in subdirectories recursively.
    pub recursive: bool,

    // -- Preview / image info ---------------------------------------
    /// Parsed image metadata from `magick identify`.
    pub preview_info: Option<magick::ImageInfo>,
    /// Error message from the last `identify` attempt.
    pub preview_error: Option<String>,
    /// Parsed EXIF metadata (fetched on demand).
    pub exif_info: Option<magick::ExifInfo>,
    /// Whether to show the EXIF panel overlay.
    pub show_exif: bool,

    // -- Undo / Revert ---------------------------------------------
    /// List of previously generated output files (for undo).
    pub generated_outputs: Vec<PathBuf>,
    /// Whether the undo list popup is visible.
    pub show_undo_list: bool,
    /// Cursor in the undo list.
    pub undo_cursor: usize,

    // -- Before/After comparison -----------------------------------
    /// Whether the before/after comparison popup is visible.
    pub show_before_after: bool,
    /// Comparison info (if any) from a processed file.
    pub before_after_info: Option<magick::BeforeAfterInfo>,

    // -- Image preview ----------------------------------------------
    /// Whether the image preview popup is visible.
    pub show_image_preview: bool,
    /// Encoded image protocol for the current preview (if any).
    pub image_protocol: Option<ratatui_image::protocol::Protocol>,
    /// Terminal capability detector (must outlive `image_protocol`).
    pub image_picker: Option<Picker>,

    /// Parsed theme colors for the UI.
    pub theme: config::ThemeColors,
}

impl App {
    /// Create a new `App` with all initial state loaded.
    pub fn new() -> Self {
        let mut recipes = recipe::load_all();
        recipes.sort_by_key(|a| a.name.to_lowercase());

        // Initial available formats from the first recipe, if any
        let available_formats: Vec<String> = recipes
            .first()
            .map(|r| {
                let mut keys: Vec<String> = r.formats.keys().cloned().collect();
                keys.sort();
                keys
            })
            .unwrap_or_default();

        let current_dir = match std::env::current_dir() {
            Ok(d) => d,
            Err(_) => PathBuf::from("/"),
        };
        let dir_listing = fs_utils::list_directory(&current_dir).unwrap_or_default();

        let settings = crate::config::Settings::load();

        Self {
            mode: Mode::default(),
            should_quit: false,
            last_tick: Instant::now(),
            focus: Focus::Recipe,

            recipes,
            recipe_cursor: 0,
            selected_recipe_name: None,
            recipe_sort: SortOrder::default(),
            recipe_filter: String::new(),
            is_filtering: false,

            current_dir,
            dir_listing,
            file_cursor: 0,
            selected_files: Vec::new(),
            show_hidden: false,

            format_override: None,
            available_formats,

            show_format_picker: false,
            format_picker_cursor: 0,

            show_dir_browser: false,
            dir_browser_path: PathBuf::from("/"),
            dir_browser_cursor: 0,
            dir_browser_listing: Vec::new(),

            show_edit_popup: false,
            edit_output_dir: None,
            edit_extra_args: String::new(),
            edit_field: EditField::OutputDir,
            edit_output_buf: String::new(),
            edit_args_buf: String::new(),

            log_entries: Vec::new(),

            show_help: false,

            magick_handle: None,
            spinner_frame: 0,
            spinner_active: false,
            progress_current: 0,
            progress_total: 0,
            progress_stage: String::new(),
            process_output: String::new(),

            processing_queue: Vec::new(),
            processing_queue_index: 0,

            dry_run: false,
            recursive: false,

            preview_info: None,
            preview_error: None,
            exif_info: None,
            show_exif: false,
            generated_outputs: Vec::new(),
            show_undo_list: false,
            undo_cursor: 0,
            show_before_after: false,
            before_after_info: None,
            show_image_preview: false,
            image_protocol: None,
            image_picker: Some(Picker::halfblocks()),
            theme: config::ThemeColors::from(&settings.theme),
        }
    }

    // ── Tick ──────────────────────────────────────────────────────

    /// Called every tick interval (≈100ms). Advances the spinner and checks
    /// on any running child process.
    pub fn on_tick(&mut self) {
        if self.spinner_active {
            self.spinner_frame = (self.spinner_frame + 1) % SPINNER_CHARS.len();
        }

        // Check running magick process
        if let Some(ref mut handle) = self.magick_handle {
            // Drain any available stderr output directly via handle
            while let Ok(line) = handle.rx.try_recv() {
                // Parse progress inline to avoid borrow conflict
                if let (Some(data), Some(op)) = (
                    line.split("]: ").nth(1),
                    line.split('[').next().filter(|s| !s.is_empty()),
                ) {
                    if let Some(frac) = data.split_whitespace().next()
                        && let Some((cur_str, tot_str)) = frac.split_once('/')
                    {
                        let cur = cur_str.parse::<f64>().unwrap_or(0.0);
                        let tot = tot_str.parse::<f64>().unwrap_or(1.0);
                        self.progress_current = (cur * 100.0) as u64;
                        self.progress_total = (tot * 100.0) as u64;
                    }
                    self.progress_stage = op.trim().to_string();
                }
                if !self.process_output.is_empty() {
                    self.process_output.push('\n');
                }
                self.process_output.push_str(&line);
            }

            match handle.child.try_wait() {
                Ok(Some(status)) => {
                    // Process finished
                    let handle = self.magick_handle.take().unwrap();
                    // Signal reader thread to exit
                    handle.cancel.store(true, Ordering::Relaxed);

                    // Drain any remaining output directly
                    while let Ok(line) = handle.rx.try_recv() {
                        if let (Some(data), Some(op)) = (
                            line.split("]: ").nth(1),
                            line.split('[').next().filter(|s| !s.is_empty()),
                        ) {
                            if let Some(frac) = data.split_whitespace().next()
                                && let Some((cur_str, tot_str)) = frac.split_once('/')
                            {
                                let cur = cur_str.parse::<f64>().unwrap_or(0.0);
                                let tot = tot_str.parse::<f64>().unwrap_or(1.0);
                                self.progress_current = (cur * 100.0) as u64;
                                self.progress_total = (tot * 100.0) as u64;
                            }
                            self.progress_stage = op.trim().to_string();
                        }
                        if !self.process_output.is_empty() {
                            self.process_output.push('\n');
                        }
                        self.process_output.push_str(&line);
                    }

                    self.spinner_active = false;

                    if status.success() {
                        // Track output for undo
                        if let Some(job) = self.processing_queue.get(self.processing_queue_index) {
                            self.generated_outputs.push(job.output.clone());
                        }
                        self.add_log(
                            format!(
                                "[{}] {} completed successfully",
                                self.processing_queue_index,
                                handle_to_job_name(
                                    &self.processing_queue,
                                    self.processing_queue_index
                                )
                            ),
                            LogLevel::Success,
                        );
                    } else {
                        self.add_log(
                            format!(
                                "[{}] {} failed with status: {status}",
                                self.processing_queue_index,
                                handle_to_job_name(
                                    &self.processing_queue,
                                    self.processing_queue_index
                                )
                            ),
                            LogLevel::Error,
                        );
                    }

                    // Advance to next job in queue
                    self.processing_queue_index += 1;
                    if self.processing_queue_index < self.processing_queue.len() {
                        self.run_next_in_queue();
                    } else {
                        let count = self.processing_queue.len();
                        self.processing_queue.clear();
                        self.processing_queue_index = 0;
                        self.add_log(
                            format!("Batch complete: {count} file(s) processed"),
                            LogLevel::Success,
                        );
                        self.mode = Mode::Browse;
                    }
                }
                Ok(None) => {
                    // Still running — already drained output above
                }
                Err(e) => {
                    self.magick_handle = None;
                    self.spinner_active = false;
                    self.add_log(format!("Error checking process: {e}"), LogLevel::Error);
                    self.mode = Mode::Browse;
                }
            }
        }
    }

    // ── Key dispatch ──────────────────────────────────────────────

    /// Handle a key event. Dispatches based on `self.mode` and `self.focus`.
    pub fn on_key(&mut self, key: KeyEvent) {
        // Undo list popup — handles keys before any other dispatch
        if self.show_undo_list {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.undo_cursor = self
                        .undo_cursor
                        .saturating_add(1)
                        .min(self.generated_outputs.len().saturating_sub(1));
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.undo_cursor = self.undo_cursor.saturating_sub(1);
                }
                KeyCode::Enter | KeyCode::Char('d') => {
                    if !self.generated_outputs.is_empty() {
                        let idx = self
                            .undo_cursor
                            .min(self.generated_outputs.len().saturating_sub(1));
                        let path = &self.generated_outputs[idx];
                        match std::fs::remove_file(path) {
                            Ok(_) => {
                                self.add_log(
                                    format!("Deleted: {}", path.display()),
                                    LogLevel::Info,
                                );
                                self.generated_outputs.remove(idx);
                                self.undo_cursor = self
                                    .undo_cursor
                                    .min(self.generated_outputs.len().saturating_sub(1));
                            }
                            Err(e) => {
                                self.add_log(
                                    format!("Failed to delete {}: {e}", path.display()),
                                    LogLevel::Error,
                                );
                            }
                        }
                    }
                }
                KeyCode::Char('c') => {
                    self.generated_outputs.clear();
                    self.undo_cursor = 0;
                    self.add_log("Undo list cleared".into(), LogLevel::Info);
                }
                KeyCode::Esc => {
                    self.show_undo_list = false;
                }
                _ => {}
            }
            return;
        }

        // Mode-specific dispatch
        match self.mode {
            Mode::Help => {
                match key.code {
                    KeyCode::Char('?') | KeyCode::Esc => {
                        self.toggle_help();
                    }
                    _ => {}
                }
                return;
            }
            Mode::Edit => {
                // If dir browser is open, handle its keys
                if self.show_dir_browser {
                    match key.code {
                        KeyCode::Esc => {
                            self.show_dir_browser = false;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            self.dir_browser_cursor = self
                                .dir_browser_cursor
                                .saturating_add(1)
                                .min(self.dir_browser_listing.len().saturating_sub(1));
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            self.dir_browser_cursor = self.dir_browser_cursor.saturating_sub(1);
                        }
                        KeyCode::Enter | KeyCode::Right => {
                            // Enter subdirectory
                            self.enter_dir_browser_subdir();
                        }
                        KeyCode::Backspace | KeyCode::Left => {
                            // Go to parent
                            if let Some(parent) = self.dir_browser_path.parent() {
                                self.dir_browser_path = parent.to_path_buf();
                                self.refresh_dir_browser_listing();
                                self.dir_browser_cursor = 0;
                            }
                        }
                        KeyCode::Char(' ') | KeyCode::Char('s')
                            if self.dir_browser_path.is_dir() =>
                        {
                            // Select the current directory
                            self.edit_output_buf =
                                self.dir_browser_path.to_string_lossy().to_string();
                            self.show_dir_browser = false;
                        }
                        _ => {}
                    }
                    return;
                }

                match key.code {
                    KeyCode::Esc => {
                        self.show_edit_popup = false;
                        self.mode = Mode::Browse;
                    }
                    KeyCode::Enter => {
                        // Apply edits
                        self.edit_output_dir = if self.edit_output_buf.is_empty() {
                            None
                        } else {
                            Some(PathBuf::from(&self.edit_output_buf))
                        };
                        self.edit_extra_args = self.edit_args_buf.clone();
                        self.show_edit_popup = false;
                        self.mode = Mode::Browse;
                        self.add_log("Applied edit parameters".into(), LogLevel::Info);
                    }
                    KeyCode::Tab => {
                        self.edit_field = match self.edit_field {
                            EditField::OutputDir => EditField::ExtraArgs,
                            EditField::ExtraArgs => EditField::OutputDir,
                        };
                    }
                    KeyCode::Backspace => {
                        let buf = match self.edit_field {
                            EditField::OutputDir => &mut self.edit_output_buf,
                            EditField::ExtraArgs => &mut self.edit_args_buf,
                        };
                        buf.pop();
                    }
                    KeyCode::Char(c) => {
                        let buf = match self.edit_field {
                            EditField::OutputDir => &mut self.edit_output_buf,
                            EditField::ExtraArgs => &mut self.edit_args_buf,
                        };
                        buf.push(c);
                    }
                    _ => {}
                }

                // Check for Ctrl+O to open directory browser
                if key.modifiers == KeyModifiers::CONTROL
                    && matches!(key.code, KeyCode::Char('o') | KeyCode::Char('O'))
                    && self.edit_field == EditField::OutputDir
                {
                    self.open_dir_browser();
                }
                return;
            }
            _ => {}
        }

        // Global keys
        match key.code {
            KeyCode::Char('q')
                if self.mode == Mode::Browse && key.modifiers == KeyModifiers::NONE =>
            {
                self.should_quit = true;
            }
            KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => {
                self.should_quit = true;
            }
            KeyCode::Char('?') => {
                self.toggle_help();
            }
            KeyCode::Esc => {
                if self.show_format_picker {
                    self.show_format_picker = false;
                    return;
                }
                if self.show_edit_popup {
                    self.show_edit_popup = false;
                    self.mode = Mode::Browse;
                    return;
                }
                if self.show_before_after {
                    self.show_before_after = false;
                    self.before_after_info = None;
                    return;
                }
                if self.show_image_preview {
                    self.show_image_preview = false;
                    self.image_protocol = None;
                    return;
                }
            }
            KeyCode::Tab => {
                self.focus = self.focus.next();
            }
            KeyCode::BackTab => {
                self.focus = match self.focus {
                    Focus::Recipe => Focus::Log,
                    Focus::File => Focus::Recipe,
                    Focus::Command => Focus::File,
                    Focus::Log => Focus::Command,
                };
            }
            KeyCode::Char('1') => self.focus = Focus::Recipe,
            KeyCode::Char('2') => self.focus = Focus::File,
            KeyCode::Char('3') => self.focus = Focus::Command,
            KeyCode::Char('4') => self.focus = Focus::Log,
            KeyCode::Char('s') => {
                self.recipe_sort = match self.recipe_sort {
                    SortOrder::Name => SortOrder::Usage,
                    SortOrder::Usage => SortOrder::Category,
                    SortOrder::Category => SortOrder::Name,
                };
                self.sort_recipes();
            }
            KeyCode::Char('n') => {
                self.dry_run = !self.dry_run;
                let msg = if self.dry_run {
                    "Dry-run mode enabled"
                } else {
                    "Dry-run mode disabled"
                };
                self.add_log(msg.into(), LogLevel::Info);
            }
            KeyCode::Char('R') => {
                self.recursive = !self.recursive;
                let msg = if self.recursive {
                    "Recursive mode enabled — processing subdirectories"
                } else {
                    "Recursive mode disabled"
                };
                self.add_log(msg.into(), LogLevel::Info);
            }
            KeyCode::Char('x') => {
                if self.show_exif {
                    self.show_exif = false;
                } else if let Some(file) = self.visible_entry_at(self.file_cursor)
                    && file.is_file()
                    && fs_utils::is_image(&file)
                {
                    self.exif_info = magick::CommandBuilder::identify_exif(&file).ok();
                    self.show_exif = true;
                }
            }
            KeyCode::Char('u') => {
                self.show_undo_list = !self.show_undo_list;
                self.undo_cursor = 0;
            }
            KeyCode::Char('b')
                if self.mode == Mode::Browse
                    && !self.show_format_picker
                    && !self.show_edit_popup
                    && !self.show_undo_list =>
            {
                self.toggle_before_after();
                return;
            }
            KeyCode::Char('p')
                if self.mode == Mode::Browse
                    && !self.show_format_picker
                    && !self.show_edit_popup
                    && !self.show_undo_list =>
            {
                self.toggle_image_preview();
                return;
            }
            _ => {}
        }

        // If a popup is open, route keys to it
        if self.show_format_picker {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.format_picker_cursor = self
                        .format_picker_cursor
                        .saturating_add(1)
                        .min(self.available_formats.len().saturating_sub(1));
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.format_picker_cursor = self.format_picker_cursor.saturating_sub(1);
                }
                KeyCode::Enter => {
                    let fmt = &self.available_formats[self.format_picker_cursor];
                    self.select_format(fmt.clone());
                    self.show_format_picker = false;
                }
                KeyCode::Esc => {
                    self.show_format_picker = false;
                }
                _ => {}
            }
            return;
        }

        if self.show_edit_popup {
            // Edit mode is handled above in the Mode::Edit dispatch
            if let KeyCode::Esc = key.code {
                self.show_edit_popup = false;
                self.mode = Mode::Browse;
            }
            return;
        }

        // Focus-specific keys
        match self.focus {
            Focus::Recipe => self.handle_recipe_focus(key),
            Focus::File => self.handle_file_focus(key),
            Focus::Command => self.handle_command_focus(key),
            Focus::Log => self.handle_log_focus(key),
        }

        // Action keys (work from any focus)
        match key.code {
            KeyCode::Char('f') => {
                self.toggle_format_picker();
            }
            KeyCode::Char('r') if self.selected_recipe_name.is_some() => {
                self.run_current();
            }
            KeyCode::Char('c') => {
                self.cancel_run();
            }
            KeyCode::Char('e') => {
                self.open_edit();
            }
            KeyCode::Char('E') => {
                match crate::recipe::export_builtins() {
                    Ok(count) => {
                        self.add_log(
                            format!(
                                "Exported {count} built-in recipes to ~/.config/lazymagick/recipes/"
                            ),
                            LogLevel::Success,
                        );
                        // Reload user recipes
                        let user_recipes = crate::recipe::load_user();
                        for user_recipe in user_recipes {
                            if let Some(pos) =
                                self.recipes.iter().position(|r| r.name == user_recipe.name)
                            {
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
            _ => {}
        }
    }

    // ── Focus-specific key handlers ───────────────────────────────

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
            KeyCode::Char(c)
                if !self.is_filtering
                    && (c.is_alphanumeric() || c == '/' || c == '-' || c == ' ' || c == '.') =>
            {
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
                let filtered_len = self.filtered_recipes().len();
                let max_idx = filtered_len.saturating_sub(1);
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.recipe_cursor = self.recipe_cursor.saturating_add(1).min(max_idx);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        self.recipe_cursor = self.recipe_cursor.saturating_sub(1);
                    }
                    KeyCode::Enter if !self.recipes.is_empty() => {
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
                        self.recipe_cursor = max_idx;
                    }
                    KeyCode::PageUp | KeyCode::Char('u')
                        if key.modifiers == KeyModifiers::CONTROL =>
                    {
                        self.recipe_cursor = self.recipe_cursor.saturating_sub(10);
                    }
                    KeyCode::PageDown | KeyCode::Char('d')
                        if key.modifiers == KeyModifiers::CONTROL =>
                    {
                        self.recipe_cursor = self.recipe_cursor.saturating_add(10).min(max_idx);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn handle_file_focus(&mut self, key: KeyEvent) {
        let visible_count = self.file_visible_count();

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.file_cursor = self
                    .file_cursor
                    .saturating_add(1)
                    .min(visible_count.saturating_sub(1));
                self.generate_preview();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.file_cursor = self.file_cursor.saturating_sub(1);
                self.generate_preview();
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                self.activate_file_cursor();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.parent_directory();
            }
            KeyCode::Char(' ') => {
                self.toggle_file_selection();
            }
            KeyCode::Char('.') => {
                self.toggle_hidden();
            }
            _ => {}
        }
    }

    fn handle_command_focus(&mut self, _key: KeyEvent) {
        // Command panel is display-only — no cursor-based interactions
    }

    fn handle_log_focus(&mut self, _key: KeyEvent) {
        // Log panel is display-only in v1 (no scrolling yet)
    }

    // ── File helpers ──────────────────────────────────────────────

    /// Return the number of visible entries in the file panel.
    fn file_visible_count(&self) -> usize {
        Self::count_visible(&self.dir_listing, self.show_hidden)
    }

    fn count_visible(listing: &fs_utils::DirListing, show_hidden: bool) -> usize {
        let is_hidden = |p: &PathBuf| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        };

        let dirs: usize = listing
            .directories
            .iter()
            .filter(|d| show_hidden || !is_hidden(d))
            .count();
        let imgs: usize = listing
            .image_files
            .iter()
            .filter(|f| show_hidden || !is_hidden(f))
            .count();
        let others: usize = listing
            .other_files
            .iter()
            .filter(|f| show_hidden || !is_hidden(f))
            .count();

        dirs + imgs + others
    }

    /// Get the path at the given visible-index cursor.
    fn visible_entry_at(&self, cursor: usize) -> Option<PathBuf> {
        let is_hidden = |p: &PathBuf| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        };

        let mut idx = 0;
        for dir in &self.dir_listing.directories {
            if self.show_hidden || !is_hidden(dir) {
                if idx == cursor {
                    return Some(dir.clone());
                }
                idx += 1;
            }
        }
        for img in &self.dir_listing.image_files {
            if self.show_hidden || !is_hidden(img) {
                if idx == cursor {
                    return Some(img.clone());
                }
                idx += 1;
            }
        }
        for other in &self.dir_listing.other_files {
            if self.show_hidden || !is_hidden(other) {
                if idx == cursor {
                    return Some(other.clone());
                }
                idx += 1;
            }
        }
        None
    }

    /// Activate the entry at the current file cursor.
    fn activate_file_cursor(&mut self) {
        let Some(path) = self.visible_entry_at(self.file_cursor) else {
            return;
        };

        if path.is_dir() {
            self.enter_directory(&path);
        } else if fs_utils::is_image(&path) {
            self.toggle_file_selection_for(path);
        }
    }

    /// Navigate into a directory.
    pub fn enter_directory(&mut self, path: &Path) {
        if path.is_dir() {
            let listing = fs_utils::list_directory(path).unwrap_or_default();
            self.current_dir = path.to_path_buf();
            self.dir_listing = listing;
            self.file_cursor = 0;
            self.selected_files.clear();
            self.generate_preview();
        }
    }

    /// Navigate to the parent directory.
    pub fn parent_directory(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            let target = if parent.as_os_str().is_empty() {
                PathBuf::from("/")
            } else {
                parent.to_path_buf()
            };
            let listing = fs_utils::list_directory(&target).unwrap_or_default();
            self.current_dir = target;
            self.dir_listing = listing;
            self.file_cursor = 0;
            self.selected_files.clear();
            self.generate_preview();
        }
    }

    /// Toggle the selected state of a file.
    fn toggle_file_selection_for(&mut self, path: PathBuf) {
        if let Some(pos) = self.selected_files.iter().position(|p| p == &path) {
            self.selected_files.remove(pos);
        } else {
            self.selected_files.push(path);
        }
    }

    /// Toggle the selection state of the file under the cursor.
    pub fn toggle_file_selection(&mut self) {
        if let Some(path) = self.visible_entry_at(self.file_cursor) {
            self.toggle_file_selection_for(path);
        }
    }

    // ── Recipe helpers ────────────────────────────────────────────

    /// Focus on a specific recipe by name.
    pub fn focus_recipe(&mut self, name: &str) {
        if let Some(pos) = self.recipes.iter().position(|r| r.name == name) {
            self.recipe_cursor = pos;
            self.selected_recipe_name = Some(name.to_string());
            self.update_available_formats();
            self.generate_preview();
        }
    }

    /// Focus on a specific file by path.
    pub fn focus_file(&mut self, path: &Path) {
        let visible_count = self.file_visible_count();
        for i in 0..visible_count {
            if let Some(entry) = self.visible_entry_at(i)
                && entry == path
            {
                self.file_cursor = i;
                self.generate_preview();
                return;
            }
        }
    }

    // ── Sort ──────────────────────────────────────────────────────

    /// Re-sort the recipe list according to `self.recipe_sort` and clamp the cursor.
    pub fn sort_recipes(&mut self) {
        match self.recipe_sort {
            SortOrder::Name => {
                self.recipes.sort_by_key(|a| a.name.to_lowercase());
            }
            SortOrder::Usage => {
                self.recipes
                    .sort_by_key(|b| std::cmp::Reverse(b.usage_count));
            }
            SortOrder::Category => {
                self.recipes.sort_by(|a, b| {
                    let cat_a = a.category.as_deref().unwrap_or("");
                    let cat_b = b.category.as_deref().unwrap_or("");
                    cat_a
                        .cmp(cat_b)
                        .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });
            }
        }
        // Re-clamp cursor
        self.recipe_cursor = self.recipe_cursor.min(self.recipes.len().saturating_sub(1));
    }

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
        filtered
            .get(cursor)
            .and_then(|&r| self.recipes.iter().position(|x| std::ptr::eq(x, r)))
    }

    // ── Format picker ─────────────────────────────────────────────

    /// Toggle the format picker popup on/off.
    pub fn toggle_format_picker(&mut self) {
        if self.available_formats.is_empty() {
            return;
        }
        self.show_format_picker = !self.show_format_picker;
        if self.show_format_picker {
            // Set cursor to current format if set
            self.format_picker_cursor = self
                .format_override
                .as_ref()
                .and_then(|cur| self.available_formats.iter().position(|f| f == cur))
                .unwrap_or(0);
        }
    }

    /// Apply a format override by name.
    pub fn select_format(&mut self, format: String) {
        self.format_override = Some(format);
        self.show_format_picker = false;
        self.generate_preview();
    }

    // ── Help ──────────────────────────────────────────────────────

    /// Toggle the help overlay.
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        self.mode = if self.show_help {
            Mode::Help
        } else {
            Mode::Browse
        };
    }

    // ── Hidden files ──────────────────────────────────────────────

    /// Toggle showing hidden files.
    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        // Clamp cursor to visible count
        let count = self.file_visible_count();
        if count == 0 {
            self.file_cursor = 0;
        } else {
            self.file_cursor = self.file_cursor.min(count.saturating_sub(1));
        }
    }

    // ── Run / Cancel ──────────────────────────────────────────────

    /// Build the command vector for a single input file.
    fn build_command_for_file(&self, recipe: &Recipe, input: &Path) -> Option<ProcessingJob> {
        let format_override = self.format_override.as_deref();
        let output_path = recipe.output_path(input, format_override);

        // Apply output dir override
        let output = if let Some(ref dir) = self.edit_output_dir {
            let fname = output_path.file_name()?;
            dir.join(fname)
        } else {
            output_path
        };

        // Apply safe output path if same directory as input
        let safe_output = if self.edit_output_dir.is_some() || format_override.is_some() {
            output
        } else {
            fs_utils::safe_output_path(
                input,
                output.extension().and_then(|e| e.to_str()).unwrap_or(""),
                &recipe.name,
            )
        };

        let recipe_args = recipe.resolved_args(format_override);
        let extra_args: Vec<String> = self
            .edit_extra_args
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let mut argv = CommandBuilder::build_argv(input, &recipe_args, &[], &safe_output);
        argv.insert(1, "-monitor".to_string());
        argv.extend(extra_args);

        Some(ProcessingJob {
            recipe_name: recipe.name.clone(),
            input: input.to_path_buf(),
            output: safe_output,
            argv,
        })
    }

    /// Build, check, and queue the magick command for all selected files.
    pub fn run_current(&mut self) {
        if self.magick_handle.is_some() {
            self.add_log("A process is already running".to_string(), LogLevel::Error);
            return;
        }

        let recipe_name = match self.selected_recipe_name.clone() {
            Some(n) => n,
            None => {
                self.add_log("No recipe selected".to_string(), LogLevel::Error);
                return;
            }
        };
        let Some(recipe_idx) = self.recipes.iter().position(|r| r.name == recipe_name) else {
            self.add_log(format!("Recipe '{recipe_name}' not found"), LogLevel::Error);
            return;
        };

        if self.selected_files.is_empty() {
            self.add_log(
                "No files selected (use Space to select)".to_string(),
                LogLevel::Error,
            );
            return;
        }

        if !CommandBuilder::check_available() {
            self.add_log("'magick' not found on $PATH".to_string(), LogLevel::Error);
            return;
        }

        // Build queue for ALL selected files (borrow recipe immutably)
        let mut queue: Vec<ProcessingJob> = Vec::new();
        let mut skipped: Vec<String> = Vec::new();
        {
            let recipe = &self.recipes[recipe_idx];
            for file in &self.selected_files {
                if self.recursive && file.is_dir() {
                    // Walk subdirectories recursively
                    for entry in walkdir::WalkDir::new(file)
                        .follow_links(true)
                        .into_iter()
                        .filter_map(|e| e.ok())
                    {
                        let path = entry.path().to_path_buf();
                        if path.is_file() && fs_utils::is_image(&path) {
                            match self.build_command_for_file(recipe, &path) {
                                Some(job) => queue.push(job),
                                None => {
                                    skipped.push(format!(
                                        "Skipping (could not build command): {}",
                                        path.display()
                                    ));
                                }
                            }
                        }
                    }
                } else if file.is_file() && fs_utils::is_image(file) {
                    match self.build_command_for_file(recipe, file) {
                        Some(job) => queue.push(job),
                        None => {
                            skipped.push(format!(
                                "Skipping file (could not build command): {}",
                                file.display()
                            ));
                        }
                    }
                } else {
                    skipped.push(format!("Skipping (not an image): {}", file.display()));
                }
            }
        }
        for msg in skipped {
            self.add_log(msg, LogLevel::Error);
        }

        if queue.is_empty() {
            self.add_log("No valid jobs to run".to_string(), LogLevel::Error);
            return;
        }

        // Dry-run mode: just log commands and return
        if self.dry_run {
            for job in &queue {
                self.add_log(format!("[DRY-RUN] {}", job.argv.join(" ")), LogLevel::Info);
            }
            return;
        }

        // Increment recipe usage count
        if let Some(recipe) = self.recipes.get_mut(recipe_idx) {
            recipe.usage_count += 1;
        }

        // Set up queue and start processing
        self.processing_queue = queue;
        self.processing_queue_index = 0;
        self.run_next_in_queue();
    }

    /// Start (or continue) processing the next job in the queue.
    fn run_next_in_queue(&mut self) {
        let idx = self.processing_queue_index;
        if idx >= self.processing_queue.len() {
            let count = self.processing_queue.len();
            self.processing_queue.clear();
            self.processing_queue_index = 0;
            self.add_log(
                format!("Batch complete: {count} file(s) processed"),
                LogLevel::Success,
            );
            self.mode = Mode::Browse;
            return;
        }

        // Clone what we need from the job to avoid borrow conflicts
        let argv = self.processing_queue[idx].argv.clone();

        self.add_log(
            format!("[{}] Running: {}", idx, argv.join(" ")),
            LogLevel::Info,
        );

        self.mode = Mode::Run;
        self.spinner_active = true;
        self.spinner_frame = 0;
        self.process_output.clear();
        self.progress_current = 0;
        self.progress_total = 0;
        self.progress_stage.clear();

        match self.spawn_magick(&argv) {
            Ok(handle) => {
                self.magick_handle = Some(handle);
            }
            Err(e) => {
                self.add_log(
                    format!("[{}] Failed to spawn process: {e}", idx),
                    LogLevel::Error,
                );
                self.spinner_active = false;
                self.mode = Mode::Browse;
                // Try the next job
                self.processing_queue_index += 1;
                if self.processing_queue_index < self.processing_queue.len() {
                    self.run_next_in_queue();
                }
            }
        }
    }

    /// Spawn a `magick` process and return a [`MagickHandle`].
    fn spawn_magick(&mut self, argv: &[String]) -> Result<MagickHandle, String> {
        debug_assert!(!argv.is_empty(), "argv must contain at least 'magick'");
        debug_assert_eq!(argv[0], "magick", "argv[0] must be 'magick'");

        let mut child = Command::new(&argv[0])
            .args(&argv[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Cannot spawn magick: {e}"))?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Cannot capture stderr".to_string())?;

        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_clone = cancel.clone();

        let thread_handle = thread::spawn(move || {
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                if cancel_clone.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(line) = line
                    && tx.send(line).is_err()
                {
                    break;
                }
            }
        });

        Ok(MagickHandle {
            child,
            rx,
            cancel,
            thread_handle: Some(thread_handle),
        })
    }

    /// Cancel the running process (SIGTERM).
    pub fn cancel_run(&mut self) {
        if let Some(mut handle) = self.magick_handle.take() {
            handle.kill();
            self.add_log("Process cancelled".into(), LogLevel::Error);
            self.spinner_active = false;
            self.mode = Mode::Browse;
            // Clear remaining queue
            self.processing_queue.clear();
            self.processing_queue_index = 0;
        }
    }

    // ── Edit popup ────────────────────────────────────────────────

    /// Open the edit parameters popup with current values.
    pub fn open_edit(&mut self) {
        self.edit_output_buf = self
            .edit_output_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        self.edit_args_buf = self.edit_extra_args.clone();
        self.edit_field = EditField::OutputDir;
        self.show_edit_popup = true;
        self.mode = Mode::Edit;
    }

    /// Open the directory browser popup starting from the current output dir.
    pub fn open_dir_browser(&mut self) {
        let start = if self.edit_output_buf.is_empty() {
            PathBuf::from("/")
        } else {
            let p = PathBuf::from(&self.edit_output_buf);
            if p.is_dir() {
                p
            } else if let Some(parent) = p.parent() {
                parent.to_path_buf()
            } else {
                PathBuf::from("/")
            }
        };
        self.dir_browser_path = start;
        self.dir_browser_cursor = 0;
        self.refresh_dir_browser_listing();
        self.show_dir_browser = true;
    }

    /// Refresh the directory listing for the browser.
    pub fn refresh_dir_browser_listing(&mut self) {
        self.dir_browser_listing.clear();
        if let Ok(read_dir) = std::fs::read_dir(&self.dir_browser_path) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && !name.starts_with('.')
                {
                    self.dir_browser_listing.push(path);
                }
            }
        }
        self.dir_browser_listing.sort_by_key(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_lowercase())
        });
    }

    /// Enter the subdirectory at the current cursor position in the browser.
    pub fn enter_dir_browser_subdir(&mut self) {
        if self.dir_browser_listing.is_empty() {
            return;
        }
        let idx = self.dir_browser_cursor;
        if idx < self.dir_browser_listing.len() {
            let subdir = self.dir_browser_listing[idx].clone();
            if subdir.is_dir() {
                self.dir_browser_path = subdir;
                self.dir_browser_cursor = 0;
                self.refresh_dir_browser_listing();
            }
        }
    }

    // ── Log ───────────────────────────────────────────────────────

    /// Add a log entry with the current timestamp.
    pub fn add_log(&mut self, message: String, level: LogLevel) {
        let now = chrono_or_fallback();
        let entry = LogEntry {
            message,
            level,
            timestamp: now,
        };
        self.log_entries.push(entry);
    }

    /// Toggle the before/after comparison popup.
    pub fn toggle_before_after(&mut self) {
        if self.show_before_after {
            self.show_before_after = false;
            self.before_after_info = None;
            return;
        }

        let file = self.visible_entry_at(self.file_cursor);
        let Some(ref path) = file else { return };
        if !path.is_file() || !crate::fs_utils::is_image(path) {
            self.add_log(
                "No valid image file selected for comparison".into(),
                LogLevel::Error,
            );
            return;
        }

        // Build argv from current recipe + format + extra args
        let recipe_idx = self
            .selected_recipe_name
            .as_ref()
            .and_then(|name| self.recipes.iter().position(|r| r.name == *name));
        let Some(recipe_idx) = recipe_idx else { return };
        let recipe = &self.recipes[recipe_idx];
        let format_override = self.format_override.as_deref();

        let output_path = recipe.output_path(path, format_override);
        let safe_output = if self.edit_output_dir.is_some() || format_override.is_some() {
            output_path
        } else {
            crate::fs_utils::safe_output_path(
                path,
                output_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or(""),
                &recipe.name,
            )
        };
        let recipe_args = recipe.resolved_args(format_override);
        let extra_args: Vec<String> = self
            .edit_extra_args
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let mut argv =
            crate::magick::CommandBuilder::build_argv(path, &recipe_args, &[], &safe_output);
        argv.extend(extra_args);

        match crate::magick::BeforeAfterInfo::new(path, &argv, &safe_output) {
            Ok(info) => {
                self.before_after_info = Some(info);
                self.show_before_after = true;
            }
            Err(e) => {
                self.add_log(
                    format!("Before/after comparison failed: {e}"),
                    LogLevel::Error,
                );
            }
        }
    }

    /// Toggle the image preview popup for the selected file.
    pub fn toggle_image_preview(&mut self) {
        if self.show_image_preview {
            self.show_image_preview = false;
            self.image_protocol = None;
            return;
        }

        let file = self.visible_entry_at(self.file_cursor);
        let Some(ref path) = file else { return };
        if !path.is_file() || !crate::fs_utils::is_image(path) {
            self.add_log(
                "No valid image file selected for preview".into(),
                LogLevel::Error,
            );
            return;
        }

        let Some(ref picker) = self.image_picker else {
            self.add_log(
                "Terminal does not support image preview (try Kitty, WezTerm, or a sixel-capable terminal)"
                    .into(),
                LogLevel::Error,
            );
            return;
        };

        match image::ImageReader::open(path) {
            Ok(reader) => match reader.decode() {
                Ok(img) => {
                    // ~40×20 cells to fit the command panel area (~35% width)
                    let cell_size = ratatui::layout::Size::new(40, 20);
                    match picker.new_protocol(img, cell_size, ratatui_image::Resize::Fit(None)) {
                        Ok(protocol) => {
                            self.image_protocol = Some(protocol);
                            self.show_image_preview = true;
                        }
                        Err(e) => {
                            self.add_log(format!("Image encoding failed: {e}"), LogLevel::Error);
                        }
                    }
                }
                Err(e) => {
                    self.add_log(format!("Failed to decode image: {e}"), LogLevel::Error);
                }
            },
            Err(e) => {
                self.add_log(format!("Failed to open image: {e}"), LogLevel::Error);
            }
        }
    }

    // ── Preview ───────────────────────────────────────────────────

    /// Generate a preview by running `magick identify` on the selected file.
    pub fn generate_preview(&mut self) {
        self.preview_info = None;
        self.preview_error = None;

        let file = self.visible_entry_at(self.file_cursor);
        let Some(ref path) = file else {
            return;
        };

        if !path.is_file() || !fs_utils::is_image(path) {
            return;
        }

        match CommandBuilder::identify(path) {
            Ok(info) => {
                self.preview_info = Some(info);
                self.preview_error = None;
            }
            Err(e) => {
                self.preview_error = Some(e.to_string());
            }
        }
    }

    // ── Internal helpers ──────────────────────────────────────────

    fn update_available_formats(&mut self) {
        self.available_formats = self
            .selected_recipe_name
            .as_ref()
            .and_then(|name| self.recipes.iter().find(|r| r.name == *name))
            .map(|r| {
                let mut keys: Vec<String> = r.formats.keys().cloned().collect();
                keys.sort();
                keys
            })
            .unwrap_or_default();
    }

    /// Return the current spinner character.
    pub fn spinner_char(&self) -> char {
        SPINNER_CHARS[self.spinner_frame % SPINNER_CHARS.len()]
    }

    /// Get the currently selected recipe, if any.
    pub fn selected_recipe(&self) -> Option<&Recipe> {
        self.selected_recipe_name
            .as_ref()
            .and_then(|name| self.recipes.iter().find(|r| r.name == *name))
    }

    /// Get the file path at the current file cursor, if any.
    pub fn cursor_file(&self) -> Option<PathBuf> {
        self.visible_entry_at(self.file_cursor)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

// ── helpers ──────────────────────────────────────────────────────

/// Return a formatted timestamp string (HH:MM:SS).
///
/// Uses `chrono` if available, otherwise falls back to a manual calculation
/// from `SystemTime`.
fn chrono_or_fallback() -> String {
    // Simple manual timestamp using std::time
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    let seconds = secs % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

/// Get the recipe/job name from the queue at the given index (for log messages).
fn handle_to_job_name(queue: &[ProcessingJob], idx: usize) -> String {
    queue
        .get(idx)
        .map(|j| format!("{} → {}", j.recipe_name, j.output.display()))
        .unwrap_or_else(|| format!("job {idx}"))
}
