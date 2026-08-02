// src/magick.rs — Command builder, runner, and image info for ImageMagick.
//
// This module shells out to `/usr/bin/magick` (whatever is on $PATH). It does
// NOT bundle ImageMagick — the binary must be installed separately.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Information about an image file, extracted via `magick identify`.
#[derive(Debug, Clone)]
pub struct ImageInfo {
    /// Full path to the image file (as passed to `identify`).
    pub path: String,
    /// Image format, e.g. `"PNG"`, `"JPEG"`, `"WebP"`.
    pub format: String,
    /// Dimensions, e.g. `"1920x1080"`.
    pub dimensions: String,
    /// Bit depth, e.g. `"8-bit"`.
    pub bit_depth: String,
    /// Colour space, e.g. `"sRGB"`, `"Gray"`.
    pub color_space: String,
    /// Human-readable file size, e.g. `"2.1MB"`.
    pub file_size: String,
}

/// Parsed EXIF metadata from `magick identify -verbose`.
#[derive(Debug, Clone, Default)]
pub struct ExifInfo {
    /// Camera make, e.g. `"Canon"`.
    pub make: String,
    /// Camera model, e.g. `"EOS R5"`.
    pub model: String,
    /// ISO speed, e.g. `"400"`.
    pub iso: String,
    /// Exposure time, e.g. `"1/125"`.
    pub exposure: String,
    /// Aperture, e.g. `"f/2.8"`.
    pub aperture: String,
    /// Focal length, e.g. `"50mm"`.
    pub focal_length: String,
    /// Date taken, e.g. `"2024:01:01 12:00:00"`.
    pub date_taken: String,
    /// GPS latitude, e.g. `"40.7128 N"`.
    pub gps_latitude: String,
    /// GPS longitude, e.g. `"74.0060 W"`.
    pub gps_longitude: String,
    /// Software used, e.g. `"Adobe Lightroom"`.
    pub software: String,
    /// Orientation, e.g. `"Top-left"`.
    pub orientation: String,
    /// Raw key-value pairs for any other EXIF data.
    pub raw: Vec<(String, String)>,
}

/// Errors that can occur during magick operations.
#[derive(Debug)]
pub enum MagickError {
    /// The `magick` binary was not found on `$PATH`.
    NotFound,
    /// `magick identify` failed (stderr included).
    IdentifyFailed(String),
    /// A `magick` command failed (stderr included).
    RunFailed(String),
    /// An I/O error occurred (e.g. spawning the process).
    Io(std::io::Error),
}

impl fmt::Display for MagickError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MagickError::NotFound => {
                write!(f, "magick binary not found on $PATH")
            }
            MagickError::IdentifyFailed(stderr) => {
                write!(f, "magick identify failed: {stderr}")
            }
            MagickError::RunFailed(stderr) => {
                write!(f, "magick command failed: {stderr}")
            }
            MagickError::Io(err) => {
                write!(f, "I/O error: {err}")
            }
        }
    }
}

impl std::error::Error for MagickError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MagickError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for MagickError {
    fn from(err: std::io::Error) -> Self {
        MagickError::Io(err)
    }
}

/// Builds and runs `magick` commands.
///
/// All methods are static — the struct serves as a namespace.
pub struct CommandBuilder;

impl CommandBuilder {
    /// Build the full argument vector for a `magick` command.
    ///
    /// Pattern: `magick {input} {recipe_args} {format_args} {output}`
    ///
    /// The first element in the returned vector is `"magick"` (the binary name,
    /// to be passed to [`Command::new`]).
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::path::Path;
    /// let argv = crate::magick::CommandBuilder::build_argv(
    ///     Path::new("in.png"),
    ///     &["-strip".to_string()],
    ///     &["-quality".to_string(), "85".to_string()],
    ///     Path::new("out.webp"),
    /// );
    /// assert_eq!(argv[0], "magick");
    /// assert!(argv.contains(&"-strip".to_string()));
    /// ```
    pub fn build_argv(
        input: &Path,
        recipe_args: &[String],
        format_args: &[String],
        output: &Path,
    ) -> Vec<String> {
        let mut argv = Vec::with_capacity(3 + recipe_args.len() + format_args.len());

        argv.push("magick".to_string());
        argv.push(input.to_string_lossy().into_owned());
        argv.extend(recipe_args.iter().cloned());
        argv.extend(format_args.iter().cloned());
        argv.push(output.to_string_lossy().into_owned());

        argv
    }

    /// Run a `magick` command.
    ///
    /// `argv` must start with `"magick"` as the first element (the binary name,
    /// passed to [`Command::new`]). Returns the full [`Output`] on success, or
    /// a [`MagickError`] if the binary is missing or the command exits with a
    /// non-zero status.
    ///
    /// # Errors
    ///
    /// Returns [`MagickError::NotFound`] if `magick` is not on `$PATH`.
    /// Returns [`MagickError::RunFailed`] if the command exits with non-zero.
    pub fn run(argv: &[String]) -> Result<Output, MagickError> {
        if !Self::check_available() {
            return Err(MagickError::NotFound);
        }

        debug_assert!(!argv.is_empty(), "argv must contain at least 'magick'");
        let output = Command::new(&argv[0]).args(&argv[1..]).output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(MagickError::RunFailed(stderr));
        }

        Ok(output)
    }

    /// Run `magick identify` on an image and parse the output.
    ///
    /// Uses `magick identify -format '%m|%wx%h|%z-bit|%[colorspace]|%b'` under
    /// the hood for machine-parseable output.
    ///
    /// # Errors
    ///
    /// Returns [`MagickError::NotFound`] if `magick` is not on `$PATH`.
    /// Returns [`MagickError::IdentifyFailed`] if identify fails (e.g. the file
    /// does not exist or is not a valid image).
    pub fn identify(input: &Path) -> Result<ImageInfo, MagickError> {
        if !Self::check_available() {
            return Err(MagickError::NotFound);
        }

        let input_str = input.to_string_lossy();
        let output = Command::new("magick")
            .args(["identify", "-format", "%m|%wx%h|%z-bit|%[colorspace]|%b"])
            .arg(input.as_os_str())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(MagickError::IdentifyFailed(stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Parse pipe-delimited fields: format|dimensions|bit_depth|colorspace|file_size
        let fields: Vec<&str> = stdout.split('|').collect();
        if fields.len() < 5 {
            let stderr = format!("unexpected identify output format: {stdout:?}");
            return Err(MagickError::IdentifyFailed(stderr));
        }

        Ok(ImageInfo {
            path: input_str.into_owned(),
            format: fields[0].to_string(),
            dimensions: fields[1].to_string(),
            bit_depth: fields[2].to_string(),
            color_space: fields[3].to_string(),
            file_size: fields[4].to_string(),
        })
    }

    /// Run `magick identify -verbose` and parse EXIF metadata.
    ///
    /// Returns an `ExifInfo` with camera settings, GPS, and other properties.
    /// Fields that are not present in the image are left empty.
    pub fn identify_exif(input: &Path) -> Result<ExifInfo, MagickError> {
        if !Self::check_available() {
            return Err(MagickError::NotFound);
        }

        let output = Command::new("magick")
            .args(["identify", "-verbose"])
            .arg(input.as_os_str())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(MagickError::IdentifyFailed(stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut exif = ExifInfo::default();

        // Parse properties section (after "Properties:" line)
        let mut in_properties = false;
        for line in stdout.lines() {
            let trimmed = line.trim();

            if trimmed == "Properties:" {
                in_properties = true;
                continue;
            }

            if in_properties {
                // Stop at next section (indentation ends or blank line followed by non-indented)
                if trimmed.is_empty()
                    || (!trimmed.starts_with("exif:")
                        && !trimmed.starts_with("date:")
                        && !trimmed.starts_with("xmp:")
                        && !trimmed.starts_with("png:")
                        && !trimmed.starts_with("jpeg:")
                        && !trimmed.starts_with("signature"))
                {
                    // Check if this is a new section header (no leading space)
                    if !line.starts_with(' ') && trimmed.ends_with(':') {
                        in_properties = false;
                        continue;
                    }
                    // Skip non-EXIF lines but stay in properties
                    continue;
                }

                // Parse "key: value"
                if let Some((key, value)) = trimmed.split_once(": ") {
                    let val = value.trim().to_string();
                    match key {
                        "exif:Make" => exif.make = val,
                        "exif:Model" => exif.model = val,
                        "exif:ISOSpeedRatings" | "exif:ISOSpeed" => exif.iso = val,
                        "exif:ExposureTime" => exif.exposure = val,
                        "exif:FNumber" => exif.aperture = val,
                        "exif:FocalLength" => exif.focal_length = val,
                        "exif:DateTimeOriginal" | "exif:DateTimeDigitized" => {
                            if exif.date_taken.is_empty() {
                                exif.date_taken = val;
                            }
                        }
                        "exif:GPSLatitude" => exif.gps_latitude = val,
                        "exif:GPSLongitude" => exif.gps_longitude = val,
                        "exif:Software" => exif.software = val,
                        "exif:Orientation" => exif.orientation = val,
                        _ => {
                            // Store any other exif: or other property
                            if key.starts_with("exif:") || key.starts_with("date:") {
                                exif.raw.push((key.to_string(), val));
                            }
                        }
                    }
                }
            }
        }

        Ok(exif)
    }

    /// Check whether `magick` is available on `$PATH`.
    ///
    /// Returns `true` if a `magick` binary was found.
    pub fn check_available() -> bool {
        which_magick().is_some()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Resolve the path to the `magick` binary using `Command` (cross-platform).
fn which_magick() -> Option<PathBuf> {
    // `std::process::Command` with a bare command name will search PATH on
    // all platforms.  We just try spawning it (with --version) and see if
    // the process starts.
    Command::new("magick")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()
        .filter(|s| s.success())
        .map(|_| PathBuf::from("magick"))
}

// ---------------------------------------------------------------------------
// Output path safety
// ---------------------------------------------------------------------------

/// Generate an output path that never overwrites the source.
///
/// Rules:
/// - If the output extension differs from the input extension → use the output
///   path as-is (different format, no collision risk).
/// - If the output extension matches the input extension (or both paths have
///   no extension, or the same extension) → insert a suffix before the
///   extension: `{stem}_{suffix}.{ext}`.
///
/// The suffix defaults to the recipe name lowercased with all whitespace
/// removed.
///
/// # Examples
///
/// ```
/// # use std::path::Path;
/// // Different extensions → as-is.
/// let out = crate::magick::safe_output_path(
///     Path::new("photo.png"),
///     Path::new("photo.webp"),
///     "convert to webp",
/// );
/// assert_eq!(out.to_string_lossy(), "photo.webp");
///
/// // Same extension → suffix added.
/// let out = crate::magick::safe_output_path(
///     Path::new("photo.png"),
///     Path::new("photo.png"),
///     "optimize",
/// );
/// assert_eq!(out.to_string_lossy(), "photo_optimize.png");
/// ```
pub fn safe_output_path(input: &Path, output: &Path, recipe_name: &str) -> PathBuf {
    let input_ext = input.extension().map(|e| e.to_ascii_lowercase());
    let output_ext = output.extension().map(|e| e.to_ascii_lowercase());

    // If extensions differ (or only one has an extension) → safe to use as-is.
    if input_ext != output_ext {
        return output.to_path_buf();
    }

    // Same extension (or both lack one) → insert suffix.
    let suffix: String = recipe_name
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_lowercase())
        .collect();

    let stem = output
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut result = PathBuf::from(output.parent().unwrap_or_else(|| Path::new("")));
    let new_filename = if let Some(ext) = output.extension() {
        format!("{stem}_{suffix}.{}", ext.to_string_lossy())
    } else {
        format!("{stem}_{suffix}")
    };
    result.push(new_filename);
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- build_argv ---------------------------------------------------------

    #[test]
    fn build_argv_basic_structure() {
        let argv = CommandBuilder::build_argv(
            Path::new("in.png"),
            &["-strip".to_string()],
            &["-quality".to_string(), "85".to_string()],
            Path::new("out.webp"),
        );

        assert_eq!(argv[0], "magick");
        assert_eq!(argv[1], "in.png");
        assert_eq!(argv[2], "-strip");
        assert_eq!(argv[3], "-quality");
        assert_eq!(argv[4], "85");
        assert_eq!(argv[5], "out.webp");
    }

    #[test]
    fn build_argv_empty_args() {
        let argv = CommandBuilder::build_argv(Path::new("a.jpg"), &[], &[], Path::new("b.png"));

        assert_eq!(argv.len(), 3);
        assert_eq!(argv[0], "magick");
        assert_eq!(argv[1], "a.jpg");
        assert_eq!(argv[2], "b.png");
    }

    #[test]
    fn build_argv_multiple_recipe_args() {
        let recipe_args = vec![
            "-strip".to_string(),
            "-flatten".to_string(),
            "-resize".to_string(),
            "800x600".to_string(),
        ];
        let argv = CommandBuilder::build_argv(
            Path::new("input.tif"),
            &recipe_args,
            &["-quality".to_string(), "90".to_string()],
            Path::new("output.jpg"),
        );

        assert_eq!(argv[0], "magick");
        assert_eq!(argv[1], "input.tif");
        // recipe args come after input
        assert_eq!(argv[2], "-strip");
        assert_eq!(argv[3], "-flatten");
        assert_eq!(argv[4], "-resize");
        assert_eq!(argv[5], "800x600");
        // then format args
        assert_eq!(argv[6], "-quality");
        assert_eq!(argv[7], "90");
        // then output
        assert_eq!(argv[8], "output.jpg");
    }

    // -- check_available ----------------------------------------------------

    #[test]
    fn check_available_returns_true_when_magick_installed() {
        assert!(
            CommandBuilder::check_available(),
            "magick 7.1.2 should be installed on this system"
        );
    }

    // -- identify -----------------------------------------------------------

    #[test]
    fn identify_parses_real_image() {
        let dir = tempfile::tempdir().unwrap();
        let img_path = dir.path().join("test_identify.png");

        let status = Command::new("magick")
            .args(["-size", "100x100", "xc:red", &img_path.to_string_lossy()])
            .status()
            .unwrap();
        assert!(status.success(), "failed to create test image");

        let info = CommandBuilder::identify(&img_path).unwrap();

        assert_eq!(info.path, img_path.to_string_lossy());
        assert_eq!(info.format, "PNG");
        assert_eq!(info.dimensions, "100x100");
        assert_eq!(info.bit_depth, "1-bit");
        assert_eq!(info.file_size, "321B");
    }

    #[test]
    fn identify_fails_on_missing_file() {
        let missing = Path::new("/tmp/this_file_does_not_exist_42.png");
        let err = CommandBuilder::identify(missing).unwrap_err();

        assert!(
            matches!(err, MagickError::IdentifyFailed(_)),
            "expected IdentifyFailed, got {err:?}"
        );
    }

    // -- safe_output_path ---------------------------------------------------

    #[test]
    fn safe_output_different_extensions() {
        let out = safe_output_path(Path::new("photo.png"), Path::new("photo.webp"), "to webp");
        // Extensions differ (.png vs .webp) → as-is.
        assert_eq!(out.to_string_lossy(), "photo.webp");
    }

    #[test]
    fn safe_output_same_extension_adds_suffix() {
        let out = safe_output_path(Path::new("photo.png"), Path::new("photo.png"), "optimize");
        // Same extension → adds suffix.
        assert_eq!(out.to_string_lossy(), "photo_optimize.png");
    }

    #[test]
    fn safe_output_same_extension_custom_suffix() {
        let out = safe_output_path(
            Path::new("input.jpg"),
            Path::new("input.jpg"),
            "Convert To Small",
        );
        // Whitespace stripped, lowercased.
        assert_eq!(out.to_string_lossy(), "input_converttosmall.jpg");
    }

    #[test]
    fn safe_output_no_input_extension() {
        let out = safe_output_path(
            Path::new("input"),      // no extension
            Path::new("input.webp"), // has extension
            "test",
        );
        // input has no ext, output does → different → as-is.
        assert_eq!(out.to_string_lossy(), "input.webp");
    }

    #[test]
    fn safe_output_both_no_extension() {
        let out = safe_output_path(Path::new("input"), Path::new("output"), "process");
        // both lack extension → suffix appended.
        assert_eq!(out.to_string_lossy(), "output_process");
    }

    #[test]
    fn safe_output_preserves_directory() {
        let out = safe_output_path(
            Path::new("sub/photo.png"),
            Path::new("sub/photo.png"),
            "opt",
        );
        assert_eq!(out.to_string_lossy(), "sub/photo_opt.png");
    }
}
