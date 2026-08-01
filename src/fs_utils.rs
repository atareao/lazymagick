use std::path::{Path, PathBuf};

/// Recognised image file extensions (lowercase, no leading dot).
pub const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "avif", "heic", "jxl", "tiff", "tif", "gif", "bmp", "svg",
];

/// Returns `true` when `path` has an extension listed in [`IMAGE_EXTENSIONS`].
///
/// Comparison is case-insensitive.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use lazymagick::fs_utils::is_image;
///
/// assert!(is_image(Path::new("photo.png")));
/// assert!(is_image(Path::new("photo.PNG")));
/// assert!(!is_image(Path::new("readme.txt")));
/// ```
pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

/// Result of listing a directory with [`list_directory`].
#[derive(Debug, Clone, Default)]
pub struct DirListing {
    /// Subdirectories (excluding `.` and `..`).
    pub directories: Vec<PathBuf>,
    /// Files whose extension matches [`IMAGE_EXTENSIONS`].
    pub image_files: Vec<PathBuf>,
    /// All other files.
    pub other_files: Vec<PathBuf>,
}

impl DirListing {
    /// `true` when all three buckets are empty.
    pub fn is_empty(&self) -> bool {
        self.directories.is_empty() && self.image_files.is_empty() && self.other_files.is_empty()
    }

    /// Total number of entries across all buckets.
    pub fn total(&self) -> usize {
        self.directories.len() + self.image_files.len() + self.other_files.len()
    }
}

/// List all entries in `path`, classified into directories, image files, and
/// other files.
///
/// Hidden entries (names starting with `.`) are included but can be filtered by
/// the caller.
///
/// # Errors
///
/// Returns an error string when `path` cannot be read (e.g. permission denied
/// or it does not exist).
pub fn list_directory(path: &Path) -> Result<DirListing, String> {
    let mut directories = Vec::new();
    let mut image_files = Vec::new();
    let mut other_files = Vec::new();

    let entries = std::fs::read_dir(path)
        .map_err(|e| format!("Cannot read directory `{}`: {e}", path.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Error reading entry: {e}"))?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            directories.push(entry_path);
        } else if is_image(&entry_path) {
            image_files.push(entry_path);
        } else {
            other_files.push(entry_path);
        }
    }

    // Stable sort so the order is deterministic across platforms.
    directories.sort();
    image_files.sort();
    other_files.sort();

    Ok(DirListing {
        directories,
        image_files,
        other_files,
    })
}

/// Generate a safe output path that avoids overwriting the source file.
///
/// **Rules:**
///
/// * If the output extension differs from the input extension → the output
///   path uses the new extension as-is (e.g. `photo.png` → `photo.webp`).
/// * If the output extension matches the input extension → a suffix is inserted
///   between stem and extension: `{stem}_{suffix}.{ext}` (e.g. `photo.png` →
///   `photo_lazymagick.png`).
/// * If the input has no extension the output extension is simply appended.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use lazymagick::fs_utils::safe_output_path;
///
/// // Different extension → no suffix
/// let p = safe_output_path(Path::new("photo.png"), "webp", "lazymagick");
/// assert_eq!(p, Path::new("photo.webp"));
///
/// // Same extension → suffix added
/// let p = safe_output_path(Path::new("photo.png"), "png", "lazymagick");
/// assert_eq!(p, Path::new("photo_lazymagick.png"));
///
/// // No input extension → just append
/// let p = safe_output_path(Path::new("photo"), "png", "lazymagick");
/// assert_eq!(p, Path::new("photo.png"));
/// ```
pub fn safe_output_path(input: &Path, output_ext: &str, suffix: &str) -> PathBuf {
    let parent = input.parent().unwrap_or(Path::new(""));
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    let input_ext = input
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());

    let same_ext = input_ext.is_some_and(|e| e == output_ext.to_ascii_lowercase());

    let filename = if same_ext {
        format!("{stem}_{suffix}.{output_ext}")
    } else if stem.is_empty() {
        output_ext.to_string()
    } else {
        format!("{stem}.{output_ext}")
    };

    parent.join(filename)
}

/// Format a file size in human-readable form.
///
/// * `< 1024` → `"340B"`
/// * `1024 ..< 1024²` → `"44KB"`
/// * `≥ 1024²` → `"1.0MB"`
///
/// # Examples
///
/// ```
/// use lazymagick::fs_utils::format_file_size;
///
/// assert_eq!(format_file_size(340), "340B");
/// assert_eq!(format_file_size(45_000), "44KB");
/// assert_eq!(format_file_size(1_048_576), "1.0MB");
/// ```
pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        let mb = bytes as f64 / MB as f64;
        format!("{:.1}MB", mb)
    } else if bytes >= KB {
        let kb = (bytes as f64 / KB as f64).round() as u64;
        format!("{kb}KB")
    } else {
        format!("{bytes}B")
    }
}

/// Sanitize a name for use as a filename suffix.
///
/// * Converts to lowercase
/// * Replaces spaces with underscores
/// * Removes any character that is not `[a-z0-9_]`
///
/// # Examples
///
/// ```
/// use lazymagick::fs_utils::sanitize_name;
///
/// assert_eq!(sanitize_name("My Recipe!"), "my_recipe");
/// assert_eq!(sanitize_name("  Hello  World  "), "hello_world");
/// assert_eq!(sanitize_name("Resize-50%"), "resize_50");
/// ```
pub fn sanitize_name(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_image ────────────────────────────────────────────────

    #[test]
    fn is_image_recognises_common_formats() {
        for ext in IMAGE_EXTENSIONS {
            let name = format!("photo.{ext}");
            let path = Path::new(&name);
            assert!(is_image(path), "expected {ext} to be recognised");
        }
    }

    #[test]
    fn is_image_rejects_non_images() {
        assert!(!is_image(Path::new("readme.txt")));
        assert!(!is_image(Path::new("script.rs")));
        assert!(!is_image(Path::new("Makefile")));
        assert!(!is_image(Path::new("data.json")));
    }

    #[test]
    fn is_image_case_insensitive() {
        assert!(is_image(Path::new("photo.PNG")));
        assert!(is_image(Path::new("photo.JPG")));
        assert!(is_image(Path::new("photo.WebP")));
        assert!(is_image(Path::new("photo.AVIF")));
    }

    #[test]
    fn is_image_no_extension() {
        assert!(!is_image(Path::new("photo")));
        assert!(!is_image(Path::new(".hidden")));
    }

    // ── list_directory ──────────────────────────────────────────

    #[test]
    fn list_directory_on_temp_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        // Create known files
        std::fs::write(dir.join("photo.png"), "").unwrap();
        std::fs::write(dir.join("image.jpg"), "").unwrap();
        std::fs::write(dir.join("notes.txt"), "").unwrap();
        std::fs::create_dir(dir.join("subdir")).unwrap();
        std::fs::write(dir.join(".hidden"), "").unwrap();

        let listing = list_directory(dir).unwrap();

        assert_eq!(listing.directories.len(), 1);
        assert!(listing.directories[0].ends_with("subdir"));

        assert_eq!(listing.image_files.len(), 2);
        assert!(listing.image_files.iter().any(|p| p.ends_with("photo.png")));
        assert!(listing.image_files.iter().any(|p| p.ends_with("image.jpg")));

        // notes.txt and .hidden are "other"
        assert_eq!(listing.other_files.len(), 2);
        assert!(listing.other_files.iter().any(|p| p.ends_with("notes.txt")));
        assert!(listing.other_files.iter().any(|p| p.ends_with(".hidden")));

        assert!(!listing.is_empty());
        assert_eq!(listing.total(), 5);
    }

    #[test]
    fn list_directory_nonexistent() {
        let err = list_directory(Path::new("/nonexistent/path/xyz123")).unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn list_directory_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let listing = list_directory(tmp.path()).unwrap();
        assert!(listing.is_empty());
        assert_eq!(listing.total(), 0);
    }

    // ── safe_output_path ────────────────────────────────────────

    #[test]
    fn safe_output_path_different_ext_no_suffix() {
        let p = safe_output_path(Path::new("photo.png"), "webp", "lazymagick");
        assert_eq!(p, Path::new("photo.webp"));
    }

    #[test]
    fn safe_output_path_same_ext_adds_suffix() {
        let p = safe_output_path(Path::new("photo.png"), "png", "lazymagick");
        assert_eq!(p, Path::new("photo_lazymagick.png"));
    }

    #[test]
    fn safe_output_path_same_ext_case_insensitive() {
        let p = safe_output_path(Path::new("photo.PNG"), "png", "v2");
        assert_eq!(p, Path::new("photo_v2.png"));
    }

    #[test]
    fn safe_output_path_no_input_extension() {
        let p = safe_output_path(Path::new("photo"), "png", "lazymagick");
        assert_eq!(p, Path::new("photo.png"));
    }

    #[test]
    fn safe_output_path_preserves_parent() {
        let p = safe_output_path(Path::new("sub/photo.png"), "png", "v2");
        assert_eq!(p, Path::new("sub/photo_v2.png"));
    }

    // ── format_file_size ────────────────────────────────────────

    #[test]
    fn format_file_size_bytes() {
        assert_eq!(format_file_size(0), "0B");
        assert_eq!(format_file_size(1), "1B");
        assert_eq!(format_file_size(1023), "1023B");
    }

    #[test]
    fn format_file_size_kb() {
        assert_eq!(format_file_size(1024), "1KB");
        assert_eq!(format_file_size(45_000), "44KB");
        assert_eq!(format_file_size(1_048_575), "1024KB");
    }

    #[test]
    fn format_file_size_mb() {
        assert_eq!(format_file_size(1_048_576), "1.0MB");
        assert_eq!(format_file_size(2_097_152), "2.0MB");
        assert_eq!(format_file_size(1_500_000), "1.4MB");
    }

    // ── sanitize_name ───────────────────────────────────────────

    #[test]
    fn sanitize_name_lowercases() {
        assert_eq!(sanitize_name("HELLO"), "hello");
    }

    #[test]
    fn sanitize_name_replaces_spaces() {
        assert_eq!(sanitize_name("my recipe"), "my_recipe");
    }

    #[test]
    fn sanitize_name_removes_special_chars() {
        assert_eq!(sanitize_name("Resize-50%!"), "resize_50");
    }

    #[test]
    fn sanitize_name_handles_leading_trailing_spaces() {
        assert_eq!(sanitize_name("  hello  world  "), "hello_world");
    }

    #[test]
    fn sanitize_name_empty_string() {
        assert_eq!(sanitize_name(""), "");
    }
}
