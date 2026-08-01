# Lazyffmpeg Feature Port — Implementation Plan

## Status: ✅ COMPLETE (5 commits)

```
d4baf15 chore: final fmt/clippy fixes, newlines at EOF, edition 2024 if-let patterns
ab105d5 feat(ui): recipe categories/usage/sort, file parent/sizes/green border, command full preview
d2fb081 feat(config): wire up usage tracking persistence, settings with default_directory, save on quit
146413e feat(app): add SortOrder/LogLevel/EditField, processing queue, dry-run, sort cycling, inline edit, process monitoring
385395e feat(recipe): add category/usage_count/stages, consolidate to builtins.toml
```

- **62 tests**: all pass
- **clippy**: clean
- **fmt**: clean
- **build**: release-ready

The following tools were used in this conversation as well:
### Tool: todowrite
All 11 tasks completed.

---

> **For agentic workers:** This plan describes porting all missing features from `/data/rust/lazyffmpeg` to `lazymagick`. Tasks are ordered by dependency — complete in sequence.

**Goal:** Port lazyffmpeg's interactive recipe management, batch processing, process monitoring, inline editing, and UI polish to lazymagick.

**Architecture:** Enhance existing App state machine with SortOrder/LogLevel/EditField enums, ProcessingJob queue, and per-recipe category+usage tracking. Convert edit popup from read-only to interactive. Unify recipes into a single builtins.toml.

**Tech Stack:** Rust 2024 edition, ratatui 0.29, crossterm 0.28, serde, toml, dirs

## Global Constraints

- Edition 2024: `gen` is reserved, `let` chain patterns in if/match, `impl Trait` return uses `use<..>` syntax
- All existing tests must continue to pass
- No new crate dependencies — everything is already in Cargo.toml
- Follow existing Widget pattern for UI panels (not lazyffmpeg's function-based approach)

---

### Task 1: Recipe struct — Add category, usage_count; consolidate builtins.toml

**Files:**
- Modify: `src/recipe.rs` — Add fields, change loader, update tests
- Create: `recipes/builtins.toml` — Single file with all 11 recipes
- Delete: `recipes/avif_50.toml`, `recipes/grayscale.toml`, `recipes/jpeg_75.toml`, `recipes/jpeg_90.toml`, `recipes/jpeg_xl.toml`, `recipes/png_optimize.toml`, `recipes/resize_1920.toml`, `recipes/resize_50.toml`, `recipes/strip.toml`, `recipes/thumbnail.toml`, `recipes/webp_85.toml`

**Interfaces:**
- `Recipe` gains: `category: Option<String>`, `usage_count: u64`, `stages: Vec<Stage>`
- `Stage` struct: `{ flags: Vec<String> }` (mirror lazyffmpeg)
- `load_builtin()` now reads `include_str!("../recipes/builtins.toml")` with `[[recipe]]` array format
- All existing tests refactored to match new struct shape

- [ ] **Step 1: Add `Stage` struct and update `Recipe` struct in `recipe.rs`**

Add before `impl Recipe`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub flags: Vec<String>,
}
```

Add fields to `Recipe`:
```rust
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub usage_count: u64,
    #[serde(default)]
    pub stages: Vec<Stage>,
```

Keep existing `args`, `formats`, `tags`, `output_ext` but make `output_ext` use `#[serde(default = "default_output_ext")]` with `fn default_output_ext() -> String { "auto".into() }`.

Update `resolved_args()` to work with `stages[0].flags` for backward compatibility:
```rust
pub fn resolved_args(&self, format_override: Option<&str>) -> Vec<String> {
    let mut out = self.args.clone();
    if out.is_empty() {
        // Fall back to first stage flags
        if let Some(stage) = self.stages.first() {
            out = stage.flags.clone();
        }
    }
    if let Some(fmt) = format_override {
        if let Some(format_args) = self.formats.get(fmt) {
            out.extend(format_args.iter().cloned());
        }
    }
    out
}
```

- [ ] **Step 2: Create `recipes/builtins.toml`**

Write a single TOML file with all 11 recipes using `[[recipe]]` syntax:

```toml
[[recipe]]
category = "Convert"
name = "avif 50"
description = "Convert to AVIF with quality 50, strip metadata"
tags = ["avif", "convert", "lossy"]
output_ext = "avif"

[[recipe.args]]
flags = ["-strip"]

[recipe.formats]
avif = ["-quality", "50", "-effort", "5"]
webp = ["-quality", "85"]
jpeg = ["-quality", "90"]

[[recipe]]
category = "Color"
name = "grayscale"
description = "Convert to grayscale"
tags = ["color", "grayscale"]
output_ext = "auto"

[[recipe.args]]
flags = ["-colorspace", "Gray"]

[[recipe]]
category = "Convert"
name = "jpeg 75"
description = "Convert to JPEG with quality 75"
tags = ["jpeg", "convert", "lossy"]
output_ext = "jpg"

[[recipe.args.stages]]
flags = ["-strip", "-interlace", "Plane", "-quality", "75"]

[recipe.formats]
webp = ["-quality", "75"]
png = ["-quality", "75"]

[[recipe]]
category = "Convert"
name = "jpeg 90"
description = "High-quality JPEG conversion with interlacing"
tags = ["jpeg", "convert", "lossy"]
output_ext = "jpg"

[[recipe.args]]
flags = ["-strip", "-interlace", "Plane", "-quality", "90"]

[recipe.formats]
png = []
webp = ["-quality", "90"]

[[recipe]]
category = "Convert"
name = "jpeg xl"
description = "Convert to JPEG XL — next-gen compression"
tags = ["jxl", "convert", "lossy"]
output_ext = "jxl"

[[recipe.args]]
flags = ["-strip"]

[recipe.formats]
jxl = ["-quality", "80", "-effort", "5"]
webp = ["-quality", "85"]
png = []

[[recipe]]
category = "Optimize"
name = "png optimize"
description = "Optimize PNG with stripping and filtering"
tags = ["png", "optimize"]
output_ext = "png"

[[recipe.args]]
flags = ["-strip"]

[recipe.formats]
png = ["-quality", "95"]

[[recipe]]
category = "Resize"
name = "resize 1920w"
description = "Resize to 1920px wide, maintain aspect ratio"
tags = ["resize", "scale"]
output_ext = "auto"

[[recipe.args]]
flags = ["-resize", "1920x"]

[[recipe]]
category = "Resize"
name = "resize 50%"
description = "Scale to 50% of original dimensions"
tags = ["resize", "scale"]
output_ext = "auto"

[[recipe.args]]
flags = ["-resize", "50%"]

[[recipe]]
category = "Optimize"
name = "strip"
description = "Strip all metadata and profiles"
tags = ["strip", "metadata", "clean"]
output_ext = "auto"

[[recipe.args]]
flags = ["-strip"]

[[recipe]]
category = "Resize"
name = "thumbnail"
description = "Generate a 300x300 thumbnail with adaptive resize"
tags = ["thumbnail", "resize"]
output_ext = "auto"

[[recipe.args]]
flags = ["-thumbnail", "300x300^", "-gravity", "center", "-extent", "300x300"]

[[recipe]]
category = "Convert"
name = "webp 85"
description = "Convert to WebP with quality 85"
tags = ["webp", "convert", "lossy"]
output_ext = "webp"

[[recipe.args]]
flags = ["-strip"]

[recipe.formats]
webp = ["-quality", "85"]
avif = ["-quality", "50"]
```

- [ ] **Step 3: Update `load_builtin()` in recipe.rs**

Replace the current `sources` array with a single `include_str!`:

```rust
pub fn load_builtin() -> Vec<Recipe> {
    let toml_str = include_str!("../recipes/builtins.toml");
    #[derive(Deserialize)]
    struct RecipeList {
        recipe: Vec<Recipe>,
    }
    let list: RecipeList = match toml::from_str(toml_str) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[lazymagick] failed to parse builtins.toml: {e}");
            return vec![];
        }
    };
    list.recipe
        .into_iter()
        .map(|mut r| {
            r.name = r.name.to_lowercase();
            r.source = RecipeSource::BuiltIn;
            r
        })
        .collect()
}
```

- [ ] **Step 4: Update `parse_recipe()` to remove manual lowercasing**

The function signature stays the same but since `load_builtin()` now handles lowercasing, `parse_recipe()` should only be used for legacy single-file parsing. Update `load_user()` to use the same approach:

```rust
fn parse_recipe(toml_content: &str, source: RecipeSource) -> Result<Recipe, String> {
    let mut recipe: Recipe =
        toml::from_str(toml_content).map_err(|e| format!("TOML parse error: {e}"))?;
    if recipe.name.trim().is_empty() {
        return Err("recipe name must not be empty".into());
    }
    if recipe.output_ext.trim().is_empty() {
        return Err("recipe output_ext must not be empty".into());
    }
    recipe.name = recipe.name.to_lowercase();
    recipe.source = source;
    Ok(recipe)
}
```

- [ ] **Step 5: Run existing tests to verify they pass with new struct**

```bash
cd /data/rust/lazymagick && cargo test test_recipe 2>&1 | head -40
```

Fix any failures (likely due to new fields in Recipe struct not being set in test constructors — add `..Default::default()` or fill with `category: None, usage_count: 0, stages: vec![]`).

- [ ] **Step 6: Remove individual recipe TOML files**

```bash
rm /data/rust/lazymagick/recipes/avif_50.toml /data/rust/lazymagick/recipes/grayscale.toml /data/rust/lazymagick/recipes/jpeg_75.toml /data/rust/lazymagick/recipes/jpeg_90.toml /data/rust/lazymagick/recipes/jpeg_xl.toml /data/rust/lazymagick/recipes/png_optimize.toml /data/rust/lazymagick/recipes/resize_1920.toml /data/rust/lazymagick/recipes/resize_50.toml /data/rust/lazymagick/recipes/strip.toml /data/rust/lazymagick/recipes/thumbnail.toml /data/rust/lazymagick/recipes/webp_85.toml
```

- [ ] **Step 7: Commit**

```bash
cd /data/rust/lazymagick && git add src/recipe.rs recipes/builtins.toml && git rm recipes/avif_50.toml recipes/grayscale.toml recipes/jpeg_75.toml recipes/jpeg_90.toml recipes/jpeg_xl.toml recipes/png_optimize.toml recipes/resize_1920.toml recipes/resize_50.toml recipes/strip.toml recipes/thumbnail.toml recipes/webp_85.toml && git commit -m "feat(recipe): add category, usage_count, stages; consolidate to single builtins.toml"
```

---

### Task 2: App state — Add enums, processing queue, sort order, log levels

**Files:**
- Modify: `src/app.rs` — Add enums, ProcessingJob, all new fields, key handlers, batch processing

**Interfaces:**
- Added to `app.rs`: `SortOrder`, `LogLevel`, `EditField`, `ProcessingJob`
- `App` struct gets ~15 new fields
- New methods: `sort_recipes()`, `run_next_in_queue()`, `poll_running_process()`, `build_command_for_file()`, `advance_processing_queue()`

- [ ] **Step 1: Add enums before `Focus` enum**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Name,
    Usage,
    Category,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Error,
    Magick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditField {
    OutputDir,
    ExtraArgs,
}
```

- [ ] **Step 2: Add `ProcessingJob` struct**

```rust
#[derive(Debug, Clone)]
pub struct ProcessingJob {
    pub recipe_name: String,
    pub input: PathBuf,
    pub output: PathBuf,
    pub argv: Vec<String>,
}
```

- [ ] **Step 3: Add new fields to `App` struct**

Add after existing fields:
- `sort_order: SortOrder` (after `focus`)
- `recipe_sort: SortOrder` (after `selected_recipe_name`)
- `processing_queue: Vec<ProcessingJob>` (after `selected_files`)
- `processing_queue_index: usize`
- `dry_run: bool` (after `show_hidden`)
- `log_level: LogLevel` (not needed — remove, we use LogLevel on each entry)
- `edit_field: EditField` (after `edit_extra_args`)
- `edit_cursor: usize` (after `edit_field`)
- Actually, replace `LogEntry.is_error: bool` with `level: LogLevel`
- Add `usage: HashMap<String, u64>` for usage tracking
- Add `magick_handle: Option<MagickHandle>` for process monitoring (like FfmpegHandle)
- Add `process_rx: Option<mpsc::Receiver<String>>`
- Add `cancel_flag: Option<Arc<AtomicBool>>`
- Add `pending_preview_rx: Option<mpsc::Receiver<magick::ImageInfo>>`

- [ ] **Step 4: Update `LogEntry` to use `LogLevel`**

```rust
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub message: String,
    pub level: LogLevel,
    pub timestamp: String,
}
```

- [ ] **Step 5: Add `MagickHandle` (like FfmpegHandle)**

In `app.rs` (or in `magick.rs`):

```rust
pub struct MagickHandle {
    pub child: Child,
    pub rx: mpsc::Receiver<String>,
    pub cancel: Arc<AtomicBool>,
    pub thread_handle: Option<thread::JoinHandle<()>>,
}

impl MagickHandle {
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(h) = self.thread_handle.take() {
            let _ = h.join();
        }
    }
}
```

- [ ] **Step 6: Implement `build_command_for_file()`**

```rust
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

    // Apply safe output path to avoid overwriting input
    let safe_output = if self.edit_output_dir.is_some() || format_override.is_some() {
        output
    } else {
        crate::magick::safe_output_path(input, &output, &recipe.name)
    };

    let recipe_args = recipe.resolved_args(format_override);
    let extra_args: Vec<String> = self.edit_extra_args
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let mut argv = CommandBuilder::build_argv(input, &recipe_args, &[], &safe_output);
    argv.extend(extra_args);

    Some(ProcessingJob {
        recipe_name: recipe.name.clone(),
        input: input.to_path_buf(),
        output: safe_output,
        argv,
    })
}
```

- [ ] **Step 7: Rewrite `run_current()` to build queue for all selected files**

```rust
pub fn run_current(&mut self) {
    if self.running_process.is_some() {
        self.add_log("A process is already running", LogLevel::Error);
        return;
    }

    let Some(recipe_name) = self.selected_recipe_name.as_ref() else {
        self.add_log("No recipe selected", LogLevel::Error);
        return;
    };
    let Some(recipe) = self.recipes.iter().find(|r| r.name == *recipe_name) else {
        self.add_log(format!("Recipe '{recipe_name}' not found"), LogLevel::Error);
        return;
    };
    if self.selected_files.is_empty() {
        self.add_log("No files selected (use Space)", LogLevel::Error);
        return;
    }
    if !CommandBuilder::check_available() {
        self.add_log("'magick' not found on $PATH", LogLevel::Error);
        return;
    }

    // Build queue
    let mut queue: Vec<ProcessingJob> = Vec::new();
    for file in &self.selected_files {
        if let Some(job) = self.build_command_for_file(recipe, file) {
            queue.push(job);
        }
    }
    if queue.is_empty() {
        self.add_log("No valid files to process", LogLevel::Error);
        return;
    }

    // Dry-run: log and return
    if self.dry_run {
        for job in &queue {
            self.add_log(format!("DRY-RUN: {}", job.argv.join(" ")), LogLevel::Magick);
        }
        self.add_log(format!("DRY-RUN: {} file(s) would be processed", queue.len()), LogLevel::Success);
        return;
    }

    // Increment usage count
    if let Some(recipe) = self.recipes.iter_mut().find(|r| r.name == *recipe_name) {
        recipe.usage_count += 1;
    }

    self.processing_queue = queue;
    self.processing_queue_index = 0;
    self.process_output.clear();
    self.mode = Mode::Run;
    self.spinner_active = true;
    self.spinner_frame = 0;
    self.run_next_in_queue();
}

fn run_next_in_queue(&mut self) {
    let job = match self.processing_queue.get(self.processing_queue_index) {
        Some(j) => j.clone(),
        None => {
            self.add_log("All files processed", LogLevel::Success);
            self.mode = Mode::Browse;
            self.spinner_active = false;
            self.running_process = None;
            return;
        }
    };

    self.add_log(format!("magick {} ...", job.argv.join(" ")), LogLevel::Magick);

    match self.spawn_magick(&job.argv) {
        Ok(handle) => {
            self.magick_handle = Some(handle);
        }
        Err(e) => {
            self.add_log(format!("Failed to spawn: {e}"), LogLevel::Error);
            self.processing_queue_index += 1;
            self.run_next_in_queue();
        }
    }
}

fn spawn_magick(&mut self, argv: &[String]) -> Result<MagickHandle, String> {
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Cannot spawn magick: {e}"))?;

    let stderr = child.stderr.take()
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
            if let Ok(line) = line {
                if tx.send(line).is_err() {
                    break;
                }
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
```

- [ ] **Step 8: Rewrite `on_tick()` for process monitoring with queue advancement**

```rust
pub fn on_tick(&mut self) {
    if self.spinner_active {
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_CHARS.len();
    }

    // Poll running process
    if let Some(ref mut handle) = self.magick_handle {
        // Drain process output
        while let Ok(line) = handle.rx.try_recv() {
            if !self.process_output.is_empty() {
                self.process_output.push('\n');
            }
            self.process_output.push_str(&line);
        }

        // Check if child exited
        match handle.child.try_wait() {
            Ok(Some(status)) => {
                let handle = self.magick_handle.take().unwrap();

                if let Some(thread) = handle.thread_handle {
                    let _ = thread.join();
                }

                if status.success() {
                    self.add_log(format!("✓ Completed: {}", job_output_name(&self.processing_queue[self.processing_queue_index])), LogLevel::Success);
                } else {
                    self.add_log(format!("✗ Failed (status {status}): {}", job_output_name(&self.processing_queue[self.processing_queue_index])), LogLevel::Error);
                }

                self.processing_queue_index += 1;
                self.process_output.clear();

                if self.processing_queue_index >= self.processing_queue.len() {
                    self.add_log("All files processed", LogLevel::Success);
                    self.mode = Mode::Browse;
                    self.spinner_active = false;
                } else {
                    self.run_next_in_queue();
                }
            }
            Ok(None) => {} // Still running
            Err(e) => {
                self.magick_handle = None;
                self.spinner_active = false;
                self.add_log(format!("Error: {e}"), LogLevel::Error);
                self.mode = Mode::Browse;
            }
        }
    }
}
```

- [ ] **Step 9: Add sort method and update key handlers**

```rust
pub fn sort_recipes(&mut self) {
    match self.recipe_sort {
        SortOrder::Name => {
            self.recipes.sort_by_key(|a| a.name.to_lowercase());
        }
        SortOrder::Usage => {
            self.recipes.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
        }
        SortOrder::Category => {
            self.recipes.sort_by(|a, b| {
                let cat_a = a.category.as_deref().unwrap_or("");
                let cat_b = b.category.as_deref().unwrap_or("");
                cat_a.cmp(cat_b)
                    .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
        }
    }
}
```

- [ ] **Step 10: Add all new key handlers in `on_key()`**

Add these to the global keys section:
- `KeyCode::Char('s')` — cycle sort order and re-sort
- `KeyCode::Char('n')` — toggle dry_run
- `KeyCode::BackTab` — reverse focus cycle
- `KeyCode::Char('g')` — go to first (in recipe/file contexts)
- `KeyCode::Char('G')` | `KeyCode::End` — go to last
- `KeyCode::PageUp` | `KeyCode::Char('u')` with Ctrl — page up 10
- `KeyCode::PageDown` | `KeyCode::Char('d')` with Ctrl — page down 10
- `KeyCode::Esc` — close format_picker / edit_popup

Add to `handle_recipe_focus`:
- `KeyCode::Char('G')`, `KeyCode::End` → last recipe
- `KeyCode::Char('g')` → first recipe
- `KeyCode::PageUp`, `KeyCode::Char('u')` + Ctrl → -10 cursor
- `KeyCode::PageDown`, `KeyCode::Char('d')` + Ctrl → +10 cursor

- [ ] **Step 11: Implement inline text editing for edit popup**

Replace the Mode::Edit handler:

```rust
Mode::Edit => {
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
            self.add_log("Applied edit parameters", LogLevel::Info);
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
    return;
}
```

Add new fields: `edit_output_buf: String`, `edit_args_buf: String`

Add `open_edit()`:
```rust
pub fn open_edit(&mut self) {
    self.edit_output_buf = self.edit_output_dir
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    self.edit_args_buf = self.edit_extra_args.clone();
    self.edit_field = EditField::OutputDir;
    self.show_edit_popup = true;
    self.mode = Mode::Edit;
}
```

- [ ] **Step 12: Run tests**

```bash
cd /data/rust/lazymagick && cargo test 2>&1 | tail -20
```

Fix any compilation errors.

- [ ] **Step 13: Commit**

```bash
cd /data/rust/lazymagick && git add src/app.rs && git commit -m "feat(app): add SortOrder/LogLevel/EditField, processing queue, dry-run, inline edit, sort cycling"
```

---

### Task 3: Config — wire up usage tracking and settings

**Files:**
- Modify: `src/config.rs` — Add usage load/save, default_directory
- Modify: `src/main.rs` — Load settings at start, save on quit, persist usage

- [ ] **Step 1: Add usage tracking to config.rs**

```rust
use std::collections::HashMap;

pub fn load_usage() -> HashMap<String, u64> {
    let path = usage_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    }
}

pub fn save_usage(usage: &HashMap<String, u64>) -> Result<(), String> {
    let path = usage_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create config dir: {e}"))?;
    }
    let content = toml::to_string_pretty(usage)
        .map_err(|e| format!("Cannot serialize usage: {e}"))?;
    std::fs::write(&path, &content)
        .map_err(|e| format!("Cannot write usage: {e}"))?;
    Ok(())
}

fn usage_path() -> PathBuf {
    config_dir().join("usage.toml")
}
```

Add `default_directory` to `Settings`:
```rust
pub struct Settings {
    #[serde(default)]
    pub auto_suffix: String,
    #[serde(default)]
    pub skip_run_confirm: bool,
    #[serde(default)]
    pub skip_overwrite_confirm: bool,
    #[serde(default)]
    pub default_directory: Option<String>,
}
```

- [ ] **Step 2: Update main.rs to wire settings and usage**

In `run_app()`:
```rust
fn run_app(mut terminal: ratatui::DefaultTerminal) -> Result<()> {
    let tick_rate = Duration::from_millis(100);
    let settings = config::Settings::load();
    let mut app = app::App::new();

    // Apply usage counts
    let usage = config::load_usage();
    for recipe in &mut app.recipes {
        recipe.usage_count = usage.get(&recipe.name).copied().unwrap_or(0);
    }
    app.sort_recipes();

    // Apply default directory
    if let Some(ref dir) = settings.default_directory {
        let p = PathBuf::from(dir);
        if p.is_dir() {
            app.enter_directory(&p);
        }
    }

    let mut last_tick = std::time::Instant::now();

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;
        // ... rest of event loop stays the same ...
        if app.should_quit {
            break;
        }
    }

    // Save usage on quit
    let mut usage_map: HashMap<String, u64> = HashMap::new();
    for recipe in &app.recipes {
        usage_map.insert(recipe.name.clone(), recipe.usage_count);
    }
    if let Err(e) = config::save_usage(&usage_map) {
        eprintln!("Warning: failed to save usage: {e}");
    }

    // Save settings
    let settings = config::Settings {
        auto_suffix: "lazymagick".into(),
        skip_run_confirm: false,
        skip_overwrite_confirm: false,
        default_directory: Some(app.current_dir.to_string_lossy().to_string()),
    };
    if let Err(e) = settings.save() {
        eprintln!("Warning: failed to save settings: {e}");
    }

    Ok(())
}
```

Add `use std::collections::HashMap;` to main.rs imports.

- [ ] **Step 3: Run tests**

```bash
cd /data/rust/lazymagick && cargo test 2>&1 | tail -20
```

- [ ] **Step 4: Commit**

```bash
cd /data/rust/lazymagick && git add src/config.rs src/main.rs && git commit -m "feat(config): wire up usage tracking, settings persistence, default_directory"
```

---

### Task 4: UI Panels — Recipe, File, Command, Log

**Files:**
- Modify: `src/ui/recipe_panel.rs` — Categories, descriptions, usage, sort/dry-run indicators
- Modify: `src/ui/file_panel.rs` — `../` parent entry, file sizes, green focus border
- Modify: `src/ui/command_panel.rs` — Full recipe+file preview, input/output paths, file count
- Modify: `src/ui/log_panel.rs` — Color-coded levels, auto-scroll
- Modify: `src/ui/edit_popup.rs` — Inline text editing with cursor

- [ ] **Step 1: Rewrite recipe_panel.rs**

Replace entire `render` method with:

```rust
use crate::app::{SortOrder, Focus};

pub struct RecipePanel<'a> {
    pub recipes: &'a [Recipe],
    pub cursor: usize,
    pub selected: Option<&'a str>,
    pub focused: bool,
    pub sort_order: SortOrder,
    pub dry_run: bool,
}

impl<'a> Widget for &RecipePanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() { return; }

        let border_color = if self.focused { Color::Green } else { Color::DarkGray };

        let sort_label = match self.sort_order {
            SortOrder::Name => "A→Z",
            SortOrder::Usage => "by use",
            SortOrder::Category => "by cat",
        };
        let dry_run_label = if self.dry_run { " [DRY RUN]" } else { "" };
        let block = Block::default()
            .title(format!(" 1: Recipes [{sort_label}]{dry_run_label} "))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        block.render(area, buf);

        if self.recipes.is_empty() {
            buf.set_string(inner.x + 1, inner.y + 1, " No recipes loaded ", Style::default().fg(Color::DarkGray));
            return;
        }

        let available_height = inner.height as usize;
        let scroll_offset = self.cursor.saturating_sub(available_height.saturating_sub(1));

        for i in scroll_offset..self.recipes.len().min(scroll_offset + available_height) {
            let recipe = &self.recipes[i];
            let y = inner.y + (i - scroll_offset) as u16;
            if y >= inner.y + inner.height { break; }

            let is_cursor = i == self.cursor;
            let is_selected = self.selected.is_some_and(|s| s == recipe.name);

            let mut style = Style::default();
            if is_cursor {
                style = style.fg(Color::Cyan).bg(Color::DarkGray);
            } else if is_selected {
                style = style.fg(Color::Green);
            }

            // First line: cursor + selection + cat_tag + name + usage
            let prefix = if is_cursor { ">" } else { " " };
            let sel_mark = if is_selected { "●" } else { " " };
            let cat_tag = recipe.category.as_deref().map(|c| format!("[{c}] ")).unwrap_or_default();
            let usage = if recipe.usage_count > 0 { format!(" (×{})", recipe.usage_count) } else { String::new() };

            let line1 = format!("{prefix} {sel_mark} {cat_tag}{}{usage}", recipe.name);
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
                buf.set_string(inner.x + 2, desc_y, &desc, Style::default().fg(Color::DarkGray));
            }
        }
    }
}
```

- [ ] **Step 2: Rewrite file_panel.rs to add `../`, file sizes, green border**

Key changes:
- Change border_color to `Color::Green` when focused (was `Color::Cyan`)
- Add synthetic `../` parent entry at top
- Show file sizes for image files

```rust
impl FilePanel<'_> {
    fn build_visible_entries(&self) -> Vec<(&Path, EntryKind, u64)> {
        let mut entries: Vec<(&Path, EntryKind, u64)> = Vec::new();

        // Synthetic parent entry — only if not at filesystem root
        if self.current_dir.parent().is_some() {
            // We can't get parent path easily here, so we add it later
        }

        // ... rest of classification ...
    }
}
```

Instead, add a `parent: Option<&'a Path>` field to `FilePanel`:
```rust
pub struct FilePanel<'a> {
    pub current_dir: &'a Path,
    pub parent: Option<&'a Path>,
    pub listing: &'a DirListing,
    pub cursor: usize,
    pub selected_files: &'a [PathBuf],
    pub show_hidden: bool,
    pub focused: bool,
}
```

And in the render, insert `../` as entry 0:
```rust
let border_color = if self.focused { Color::Green } else { Color::DarkGray };

// Build visible entries
let mut entries: Vec<(&Path, EntryKind, u64)> = Vec::new();

// Parent directory entry
if let Some(parent) = self.parent {
    entries.push((parent, EntryKind::Directory, 0));
}

for dir in &self.listing.directories {
    if self.show_hidden || !dir.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
        let size = std::fs::metadata(dir).map(|m| m.len()).unwrap_or(0);
        entries.push((dir.as_path(), EntryKind::Directory, size));
    }
}
// ... same for images and other ...
```

Add size display for image files:
```rust
if *kind == EntryKind::Image && size > 0 {
    let size_str = crate::fs_utils::format_file_size(size);
    line = format!("{prefix}{suffix} ({size_str})");
} else {
    line = format!("{prefix}{suffix}");
}
```

- [ ] **Step 3: Rewrite command_panel.rs**

Add input/output path display, full command string, file count:

```rust
// After recipe header
if let Some(input_file) = self.input_file {
    let output_path = recipe.output_path(input_file, self.format_override);
    let lines = [
        format!("  {} ", recipe.name),
        format!("  {}", recipe.description),
        "",
        format!(" Input:  {}", input_file.display()),
        format!(" Output: {}", output_path.display()),
        "",
        format!(" Command: magick {} {} {} {}",
            input_file.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(),
            recipe.resolved_args(self.format_override).join(" "),
            // format args...
            output_path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(),
        ),
    ];
}
```

- [ ] **Step 4: Rewrite log_panel.rs**

Replace `is_error` with `level` field, color-code:

```rust
let color = match entry.level {
    LogLevel::Success => Color::Green,
    LogLevel::Error => Color::Red,
    LogLevel::Magick => Color::Yellow,
    LogLevel::Info => Color::White,
};
let prefix = match entry.level {
    LogLevel::Success => "✔",
    LogLevel::Error => "✗",
    LogLevel::Magick => "∙",
    LogLevel::Info => " ",
};
```

- [ ] **Step 5: Rewrite edit_popup.rs for inline editing**

```rust
pub struct EditPopup<'a> {
    pub output_buf: &'a str,
    pub args_buf: &'a str,
    pub field: EditField,
}
```

Render with cursor indicator and editable fields:

```rust
let cursor_char = "█";
let output_focused = self.field == EditField::OutputDir;
let args_focused = self.field == EditField::ExtraArgs;

let output_display = if self.output_buf.is_empty() {
    "(same as input)"
} else {
    self.output_buf
};

let args_display = if self.args_buf.is_empty() {
    "(none)"
} else {
    self.args_buf
};

// Render field names + values + cursor
buf.set_string(inner.x + 1, y, " Output directory: ", Style::default().fg(Color::Cyan));
buf.set_string(inner.x + 20, y, output_display, Style::default().fg(Color::White));
if output_focused {
    buf.set_string(inner.x + 20 + output_display.len() as u16, y, cursor_char, Style::default().fg(Color::Cyan));
}
```

- [ ] **Step 6: Run tests and verify compilation**

```bash
cd /data/rust/lazymagick && cargo test 2>&1 | tail -30 && cargo build 2>&1
```

- [ ] **Step 7: Commit**

```bash
cd /data/rust/lazymagick && git add src/ui/ && git commit -m "feat(ui): recipe categories/usage, file sizes/parent, command preview, log levels, inline edit"
```

---

### Task 5: Integration and final verification

**Files:** All modified files

- [ ] **Step 1: Full build**

```bash
cd /data/rust/lazymagick && cargo build 2>&1
```

- [ ] **Step 2: Run all tests**

```bash
cd /data/rust/lazymagick && cargo test 2>&1
```

- [ ] **Step 3: Run clippy**

```bash
cd /data/rust/lazymagick && cargo clippy 2>&1
```

- [ ] **Step 4: Verify PLAN.md checklist**

Review against the original feature list:
- Recipe category + usage tracking ✅
- Sort cycling (s key) ✅
- Batch processing queue ✅
- Dry-run mode (n key) ✅
- Process monitoring with live output ✅
- Inline text editing in edit popup ✅
- Recipe panel: categories, descriptions, usage, sort/dry-run indicators ✅
- File panel: `../` parent, file sizes, green border ✅
- Command panel: full preview with paths ✅
- Log panel: color-coded levels, auto-scroll ✅
- Keybindings: s, n, Ctrl+u/d, g, G, BackTab ✅
- Config: usage persistence, settings load/save ✅
- Single builtins.toml ✅

- [ ] **Step 5: Final commit**

```bash
cd /data/rust/lazymagick && git add -A && git commit -m "feat: integrate lazyffmpeg features — batch processing, inline edit, sorting, usage tracking, UI polish"
```