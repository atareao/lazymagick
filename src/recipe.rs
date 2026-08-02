//! Recipe system for lazymagick.
//!
//! A recipe is a declarative preset that describes a single ImageMagick
//! processing operation: input → args → output.  Recipes are defined in
//! TOML files and can be extended with per-format options so the user
//! can switch output formats at runtime (e.g. WebP → AVIF) without
//! editing the recipe.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// RecipeSource
// ---------------------------------------------------------------------------

/// Where a recipe was loaded from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RecipeSource {
    /// Recipe shipped with the application.
    #[default]
    BuiltIn,
    /// Recipe loaded from a user-defined file.
    User(PathBuf),
}

// ---------------------------------------------------------------------------
// Stage
// ---------------------------------------------------------------------------

/// A single processing stage — a set of ImageMagick flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    /// ImageMagick flags for this stage (e.g. `["-strip"]`).
    pub flags: Vec<String>,
}

// ---------------------------------------------------------------------------
// Recipe
// ---------------------------------------------------------------------------

/// A single image-processing recipe — a named preset of `magick` arguments.
///
/// # Format override
///
/// Every recipe has a *default* output extension (`output_ext`) and a set of
/// always-applied `args`.  In addition, the recipe can define per-format
/// argument overrides under `formats`.  When the user picks a format override
/// (e.g. `"webp"`) the recipe swaps the output extension and appends any
/// format-specific arguments to the base args.
#[derive(Debug, Clone, Deserialize)]
pub struct Recipe {
    /// Human-readable name (lowercased on load).
    pub name: String,

    /// Short description of what the recipe does.
    pub description: String,

    /// Optional category for grouping (e.g. "Resize", "Convert", "Optimize").
    #[serde(default)]
    pub category: Option<String>,

    /// How many times this recipe has been used (persisted).
    #[serde(default)]
    pub usage_count: u64,

    /// Optional tags for filtering / grouping.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Processing stages — each stage is a set of flags applied in sequence.
    /// When empty, falls back to `args` (legacy).
    #[serde(default)]
    pub stages: Vec<Stage>,

    /// Output extension — either `"auto"` (keep input extension) or a literal
    /// extension such as `"webp"`, `"jpg"`, `"png"`, etc.
    pub output_ext: String,

    /// Always-applied ImageMagick arguments (cloned and used as the base).
    #[serde(default)]
    pub args: Vec<String>,

    /// Per-format arguments keyed by format name (extension).
    /// When a format override is active the corresponding args are appended.
    #[serde(default)]
    pub formats: HashMap<String, Vec<String>>,

    /// Origin of the recipe (filled in by the loader, not serialised).
    #[serde(skip)]
    pub source: RecipeSource,
}

impl Recipe {
    /// Resolve the output path for a given input file and optional format
    /// override.
    ///
    /// Resolution rules (first match wins):
    /// 1. If `format_override` is `Some`, use it as the extension.
    /// 2. Otherwise, if `self.output_ext` is `"auto"`, keep the input's
    ///    extension.
    /// 3. Otherwise, use `self.output_ext`.
    ///
    /// Returns the new path with the resolved extension.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::path::Path;
    ///
    /// let recipe = Recipe {
    ///     name: "test".into(),
    ///     description: String::new(),
    ///     tags: vec![],
    ///     output_ext: "webp".into(),
    ///     args: vec![],
    ///     formats: std::collections::HashMap::new(),
    ///     source: RecipeSource::BuiltIn,
    /// };
    ///
    /// let input = Path::new("photo.png");
    /// assert_eq!(recipe.output_path(input, None),   Path::new("photo.webp"));
    /// assert_eq!(recipe.output_path(input, Some("avif")), Path::new("photo.avif"));
    /// ```
    pub fn output_path(&self, input: &Path, format_override: Option<&str>) -> PathBuf {
        let ext = match format_override {
            Some(fmt) => fmt.to_string(),
            None => {
                if self.output_ext == "auto" {
                    input
                        .extension()
                        .map(|e| e.to_string_lossy().to_string())
                        .unwrap_or_default()
                } else {
                    self.output_ext.clone()
                }
            }
        };
        input.with_extension(ext)
    }

    /// Build the full list of ImageMagick arguments for this recipe.
    ///
    /// Returns the base `args` (always applied).  If `args` is empty, falls
    /// back to the first stage's flags (if any).  If `format_override` is
    /// `Some` **and** that format exists in `self.formats`, the
    /// format-specific arguments are appended.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::collections::HashMap;
    ///
    /// let mut formats = HashMap::new();
    /// formats.insert("webp".into(), vec!["-quality".into(), "85".into()]);
    ///
    /// let recipe = Recipe {
    ///     name: "test".into(),
    ///     description: String::new(),
    ///     category: None,
    ///     usage_count: 0,
    ///     tags: vec![],
    ///     stages: vec![],
    ///     output_ext: "webp".into(),
    ///     args: vec!["-strip".into()],
    ///     formats,
    ///     source: RecipeSource::BuiltIn,
    /// };
    ///
    /// assert_eq!(recipe.resolved_args(None),       vec!["-strip"]);
    /// assert_eq!(recipe.resolved_args(Some("webp")),
    ///            vec!["-strip", "-quality", "85"]);
    /// assert_eq!(recipe.resolved_args(Some("avif")),
    ///            vec!["-strip"]);
    /// ```
    pub fn resolved_args(&self, format_override: Option<&str>) -> Vec<String> {
        let mut out = self.args.clone();
        if out.is_empty()
            && let Some(stage) = self.stages.first()
        {
            out = stage.flags.clone();
        }
        if let Some(fmt) = format_override
            && let Some(format_args) = self.formats.get(fmt)
        {
            out.extend(format_args.iter().cloned());
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Loaders
// ---------------------------------------------------------------------------

/// Load all built-in recipes shipped with the application.
///
/// Recipes are defined in `recipes/builtins.toml` using the `[[recipe]]`
/// TOML array-of-tables syntax (like lazyffmpeg).
pub fn load_builtin() -> Vec<Recipe> {
    #[derive(Deserialize)]
    struct RecipeList {
        recipe: Vec<Recipe>,
    }

    let toml_str = include_str!("../recipes/builtins.toml");
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

/// Load user-defined recipes from `~/.config/lazymagick/recipes/`.
///
/// Returns an empty vec when the directory does not exist or cannot be read.
pub fn load_user() -> Vec<Recipe> {
    let Some(config_dir) = dirs::config_dir() else {
        return vec![];
    };
    load_user_from(&config_dir.join("lazymagick").join("recipes"))
}

/// Load user-defined recipes from an explicit directory path.
///
/// This variant exists so tests can point at a temporary directory without
/// touching the real `~/.config`.
fn load_user_from(dir: &Path) -> Vec<Recipe> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    let mut recipes = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "toml") {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[lazymagick] skipping user recipe {path:?}: {e}");
                    continue;
                }
            };
            match parse_recipe(&content, RecipeSource::User(path.clone())) {
                Ok(r) => recipes.push(r),
                Err(e) => {
                    eprintln!("[lazymagick] skipping user recipe {path:?}: {e}");
                }
            }
        }
    }
    recipes
}

/// Load all recipes (built-in + user).
///
/// User recipes **override** built-ins of the same name (comparison is
/// case-sensitive after the name has been lowercased).
pub fn load_all() -> Vec<Recipe> {
    let mut builtin = load_builtin();
    let user = load_user();

    // Index built-ins by name (owned keys to avoid borrow-checker conflicts).
    let mut map: HashMap<String, usize> = HashMap::new();
    for (i, r) in builtin.iter().enumerate() {
        map.insert(r.name.clone(), i);
    }

    // Insert or override.
    for recipe in user {
        if let Some(&idx) = map.get(&recipe.name) {
            builtin[idx] = recipe;
        } else {
            map.insert(recipe.name.clone(), builtin.len());
            builtin.push(recipe);
        }
    }

    builtin
}

/// Export built-in recipes to the user config directory for editing.
///
/// Creates `~/.config/lazymagick/recipes/builtins.toml` containing
/// all built-in recipes. Existing files are overwritten.
///
/// Returns the number of recipes exported, or an error message.
pub fn export_builtins() -> Result<usize, String> {
    let dest_dir = crate::config::user_recipes_dir();
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("Cannot create recipes dir: {e}"))?;

    let dest_path = dest_dir.join("builtins.toml");
    let content = include_str!("../recipes/builtins.toml");
    std::fs::write(&dest_path, content).map_err(|e| format!("Cannot write recipes file: {e}"))?;

    // Count how many recipes were exported
    let count = load_builtin().len();
    Ok(count)
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Parse a single recipe from its TOML source string.
///
/// # Errors
///
/// Returns an error message if the TOML is malformed, the name is empty, or
/// `output_ext` is empty.
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ----- Parsing & validation -------------------------------------------

    fn make_webp_85() -> String {
        r###"
name = "webp 85"
description = "Convert to WebP with quality 85"
output_ext = "webp"
stages = [{ flags = ["-strip"] }]

[formats]
webp = ["-quality", "85"]
"###
        .into()
    }

    fn make_avif_50() -> String {
        r###"
name = "avif 50"
description = "Convert to AVIF with quality 50"
output_ext = "avif"
stages = [{ flags = ["-strip"] }]

[formats]
avif = ["-quality", "50", "-effort", "5"]
webp = ["-quality", "85"]
"###
        .into()
    }

    fn make_jpeg_90() -> String {
        r###"
name = "jpeg 90"
description = "High-quality JPEG"
output_ext = "jpg"
stages = [{ flags = ["-strip", "-interlace", "Plane", "-quality", "90"] }]
"###
        .into()
    }

    fn make_resize_50() -> String {
        r###"
name = "resize 50%"
description = "Scale to 50%"
output_ext = "auto"
stages = [{ flags = ["-resize", "50%"] }]
"###
        .into()
    }

    fn make_grayscale() -> String {
        r###"
name = "grayscale"
description = "Convert to grayscale"
output_ext = "auto"
stages = [{ flags = ["-colorspace", "Gray"] }]
"###
        .into()
    }

    fn make_png_optimize() -> String {
        r###"
name = "png optimize"
description = "Optimize PNG"
output_ext = "png"
stages = [{ flags = ["-strip"] }]

[formats]
png = ["-quality", "95"]
"###
        .into()
    }

    #[test]
    fn parse_webp_85() {
        let recipe = parse_recipe(&make_webp_85(), RecipeSource::BuiltIn).unwrap();
        assert_eq!(recipe.name, "webp 85");
        assert_eq!(recipe.output_ext, "webp");
        assert_eq!(recipe.stages[0].flags, vec!["-strip"]);
        assert_eq!(recipe.source, RecipeSource::BuiltIn);
    }

    #[test]
    fn parse_avif_50() {
        let recipe = parse_recipe(&make_avif_50(), RecipeSource::BuiltIn).unwrap();
        assert_eq!(recipe.name, "avif 50");
        assert_eq!(recipe.output_ext, "avif");
        assert!(recipe.formats.contains_key("webp"));
    }

    #[test]
    fn parse_jpeg_90() {
        let recipe = parse_recipe(&make_jpeg_90(), RecipeSource::BuiltIn).unwrap();
        assert_eq!(recipe.name, "jpeg 90");
        assert_eq!(recipe.output_ext, "jpg");
        assert!(recipe.stages[0].flags.contains(&"-interlace".to_string()));
    }

    #[test]
    fn parse_resize_50() {
        let recipe = parse_recipe(&make_resize_50(), RecipeSource::BuiltIn).unwrap();
        assert_eq!(recipe.name, "resize 50%");
        assert_eq!(recipe.output_ext, "auto");
        assert_eq!(recipe.stages[0].flags, vec!["-resize", "50%"]);
    }

    #[test]
    fn parse_grayscale() {
        let recipe = parse_recipe(&make_grayscale(), RecipeSource::BuiltIn).unwrap();
        assert_eq!(recipe.name, "grayscale");
        assert_eq!(recipe.stages[0].flags, vec!["-colorspace", "Gray"]);
    }

    #[test]
    fn parse_png_optimize() {
        let recipe = parse_recipe(&make_png_optimize(), RecipeSource::BuiltIn).unwrap();
        assert_eq!(recipe.name, "png optimize");
        assert_eq!(recipe.output_ext, "png");
        assert!(recipe.formats.contains_key("png"));
    }

    // ----- resolved_args --------------------------------------------------

    #[test]
    fn resolved_args_no_override() {
        let recipe = parse_recipe(&make_webp_85(), RecipeSource::BuiltIn).unwrap();
        assert_eq!(recipe.resolved_args(None), vec!["-strip"]);
    }

    #[test]
    fn resolved_args_with_matching_override() {
        let recipe = parse_recipe(&make_webp_85(), RecipeSource::BuiltIn).unwrap();
        assert_eq!(
            recipe.resolved_args(Some("webp")),
            vec!["-strip", "-quality", "85"]
        );
    }

    #[test]
    fn resolved_args_with_mismatched_override() {
        let recipe = parse_recipe(&make_webp_85(), RecipeSource::BuiltIn).unwrap();
        assert_eq!(recipe.resolved_args(Some("png")), vec!["-strip"]);
    }

    #[test]
    fn resolved_args_avif_format_override() {
        let recipe = parse_recipe(&make_avif_50(), RecipeSource::BuiltIn).unwrap();
        assert_eq!(
            recipe.resolved_args(Some("avif")),
            vec!["-strip", "-quality", "50", "-effort", "5"]
        );
    }

    #[test]
    fn resolved_args_from_stages() {
        // When args is empty but stages exists, use stages[0].flags
        let toml = r#"
name = "from-stages"
description = "test"
output_ext = "auto"
stages = [{ flags = ["-resize", "50%"] }]
"#;
        let recipe = parse_recipe(toml, RecipeSource::BuiltIn).unwrap();
        assert!(recipe.args.is_empty());
        assert_eq!(recipe.resolved_args(None), vec!["-resize", "50%"]);
    }

    // ----- output_path ----------------------------------------------------

    #[test]
    fn output_path_with_explicit_ext() {
        let recipe = Recipe {
            name: "x".into(),
            description: String::new(),
            category: None,
            usage_count: 0,
            tags: vec![],
            stages: vec![],
            output_ext: "webp".into(),
            args: vec![],
            formats: HashMap::new(),
            source: RecipeSource::BuiltIn,
        };
        assert_eq!(
            recipe.output_path(Path::new("photo.png"), None),
            Path::new("photo.webp")
        );
    }

    #[test]
    fn output_path_with_auto_ext() {
        let recipe = Recipe {
            name: "x".into(),
            description: String::new(),
            category: None,
            usage_count: 0,
            tags: vec![],
            stages: vec![],
            output_ext: "auto".into(),
            args: vec![],
            formats: HashMap::new(),
            source: RecipeSource::BuiltIn,
        };
        assert_eq!(
            recipe.output_path(Path::new("photo.png"), None),
            Path::new("photo.png")
        );
        assert_eq!(
            recipe.output_path(Path::new("photo.jpeg"), None),
            Path::new("photo.jpeg")
        );
    }

    #[test]
    fn output_path_with_format_override() {
        let recipe = Recipe {
            name: "x".into(),
            description: String::new(),
            category: None,
            usage_count: 0,
            tags: vec![],
            stages: vec![],
            output_ext: "auto".into(),
            args: vec![],
            formats: HashMap::new(),
            source: RecipeSource::BuiltIn,
        };
        assert_eq!(
            recipe.output_path(Path::new("photo.png"), Some("webp")),
            Path::new("photo.webp")
        );
        assert_eq!(
            recipe.output_path(Path::new("photo.png"), Some("avif")),
            Path::new("photo.avif")
        );
    }

    // ----- load_builtin ---------------------------------------------------

    #[test]
    fn load_builtin_returns_all_42_recipes() {
        let recipes = load_builtin();
        assert_eq!(
            recipes.len(),
            42,
            "expected 42 built-in recipes (38 + 4 weight reduction)"
        );
    }

    #[test]
    fn load_builtin_names_are_lowercase() {
        let recipes = load_builtin();
        for r in &recipes {
            assert_eq!(
                r.name,
                r.name.to_lowercase(),
                "recipe name '{}' is not lowercased",
                r.name
            );
        }
    }

    #[test]
    fn load_builtin_all_have_non_empty_output_ext() {
        let recipes = load_builtin();
        for r in &recipes {
            assert!(
                !r.output_ext.is_empty(),
                "recipe '{}' has empty output_ext",
                r.name
            );
        }
    }

    #[test]
    fn load_builtin_recipes_have_categories() {
        let recipes = load_builtin();
        for r in &recipes {
            assert!(
                r.category.is_some(),
                "recipe '{}' should have a category",
                r.name
            );
        }
    }

    // ----- load_all / override --------------------------------------------

    #[test]
    fn load_all_user_overrides_builtin() {
        let dir = tempfile::tempdir().unwrap();
        let recipes_dir = dir.path().join("lazymagick").join("recipes");
        std::fs::create_dir_all(&recipes_dir).unwrap();

        // A user recipe that overrides "webp 85" with a different description.
        let override_toml = r#"
name = "webp 85"
description = "OVERRIDDEN"
output_ext = "webp"
stages = [{ flags = ["-strip"] }]
"#;
        std::fs::write(recipes_dir.join("override.toml"), override_toml).unwrap();

        let mut builtin = load_builtin();
        let user = load_user_from(&recipes_dir);

        // Manual merge (same logic as load_all).
        let mut map: HashMap<String, usize> = HashMap::new();
        for (i, r) in builtin.iter().enumerate() {
            map.insert(r.name.clone(), i);
        }
        for recipe in user {
            if let Some(&idx) = map.get(&recipe.name) {
                builtin[idx] = recipe;
            }
        }

        let overridden = builtin.iter().find(|r| r.name == "webp 85").unwrap();
        assert_eq!(overridden.description, "OVERRIDDEN");
    }

    #[test]
    fn load_all_merges_new_user_recipes() {
        let dir = tempfile::tempdir().unwrap();
        let recipes_dir = dir.path().join("lazymagick").join("recipes");
        std::fs::create_dir_all(&recipes_dir).unwrap();

        let custom_toml = r#"
name = "my custom recipe"
description = "A user-defined recipe"
output_ext = "png"
stages = [{ flags = ["-strip"] }]
"#;
        std::fs::write(recipes_dir.join("custom.toml"), custom_toml).unwrap();

        let user = load_user_from(&recipes_dir);
        assert_eq!(user.len(), 1);
        assert_eq!(user[0].name, "my custom recipe");
        assert_eq!(
            user[0].source,
            RecipeSource::User(recipes_dir.join("custom.toml"))
        );
    }

    // ----- edge cases -----------------------------------------------------

    #[test]
    fn load_user_nonexistent_dir_returns_empty() {
        let recipes = load_user_from(Path::new("/nonexistent/path"));
        assert!(recipes.is_empty());
    }

    #[test]
    fn parse_recipe_rejects_empty_name() {
        let result = parse_recipe(
            r#"name = ""
description = "empty"
output_ext = "png"
"#,
            RecipeSource::BuiltIn,
        );
        assert!(result.is_err());
    }

    #[test]
    fn parse_recipe_rejects_empty_output_ext() {
        let result = parse_recipe(
            r#"name = "test"
description = "empty ext"
output_ext = ""
"#,
            RecipeSource::BuiltIn,
        );
        assert!(result.is_err());
    }

    #[test]
    fn parse_recipe_rejects_malformed_toml() {
        let result = parse_recipe("not valid toml {{{", RecipeSource::BuiltIn);
        assert!(result.is_err());
    }
}
