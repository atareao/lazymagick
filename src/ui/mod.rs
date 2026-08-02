//! Top-level TUI rendering — lay out panels and render each widget.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Paragraph,
};

use crate::app::{App, Focus, Mode};

pub mod before_after_popup;
pub mod command_panel;
pub mod dir_browser_popup;
pub mod edit_popup;
pub mod exif_panel;
pub mod file_panel;
pub mod format_picker;
pub mod help_popup;
pub mod image_preview_popup;
pub mod layout;
pub mod log_panel;
pub mod recipe_panel;
pub mod undo_popup;

/// Render the entire TUI for the current frame.
pub fn render(frame: &mut Frame, app: &App) {
    let areas = layout::chunk_areas(frame.area());

    // ── Title bar ────────────────────────────────────────────────
    render_title_bar(frame, areas.title_bar, app);

    // ── Recipe panel ─────────────────────────────────────────────
    let filtered_count = app.filtered_recipes().len();
    let recipe_widget = recipe_panel::RecipePanel {
        recipes: &app.recipes,
        cursor: app.recipe_cursor.min(filtered_count.saturating_sub(1)),
        selected: app.selected_recipe_name.as_deref(),
        focused: app.focus == Focus::Recipe,
        sort_order: app.recipe_sort,
        dry_run: app.dry_run,
        filter: &app.recipe_filter,
        is_filtering: app.is_filtering,
        theme: &app.theme,
    };
    frame.render_widget(&recipe_widget, areas.recipe_panel);

    // ── File panel ───────────────────────────────────────────────
    let file_widget = file_panel::FilePanel {
        current_dir: &app.current_dir,
        parent: app.current_dir.parent(),
        listing: &app.dir_listing,
        cursor: app.file_cursor,
        selected_files: &app.selected_files,
        show_hidden: app.show_hidden,
        focused: app.focus == Focus::File,
        theme: &app.theme,
    };
    frame.render_widget(&file_widget, areas.file_panel);

    // ── Command preview panel (or image preview) ────────────────
    if app.show_image_preview
        && let Some(ref protocol) = app.image_protocol
    {
        image_preview_popup::render(frame, areas.command_panel, protocol, &app.theme);
    } else if app.show_image_preview {
        image_preview_popup::render_placeholder(frame, areas.command_panel, &app.theme);
    } else {
        let cursor_file = app.cursor_file();
        let command_widget = command_panel::CommandPanel {
            recipe: app.selected_recipe(),
            input_file: cursor_file.as_deref(),
            format_override: app.format_override.as_deref(),
            preview_info: app.preview_info.as_ref(),
            preview_error: app.preview_error.as_deref(),
            focused: app.focus == Focus::Command,
            selected_file_count: app.selected_files.len(),
            is_running: app.magick_handle.is_some(),
            progress_current: app.progress_current,
            progress_total: app.progress_total,
            progress_stage: app.progress_stage.clone(),
            theme: &app.theme,
        };
        frame.render_widget(&command_widget, areas.command_panel);
    }

    // ── Log panel ────────────────────────────────────────────────
    let log_widget = log_panel::LogPanel {
        entries: &app.log_entries,
        process_running: app.magick_handle.is_some(),
        process_output: &app.process_output,
        focused: app.focus == Focus::Log,
        spinner_char: app.spinner_char(),
        theme: &app.theme,
    };
    frame.render_widget(&log_widget, areas.log_panel);

    // ── Status bar ───────────────────────────────────────────────
    render_status_bar(frame, areas.status_bar, app);

    // ── Overlays ─────────────────────────────────────────────────
    if app.show_format_picker {
        let picker = format_picker::FormatPicker {
            formats: &app.available_formats,
            cursor: app.format_picker_cursor,
            current_format: app.format_override.as_deref(),
            theme: &app.theme,
        };
        frame.render_widget(&picker, frame.area());
    }

    if app.show_edit_popup {
        let edit = edit_popup::EditPopup {
            output_dir: app.edit_output_dir.as_deref(),
            extra_args: &app.edit_extra_args,
            output_buf: &app.edit_output_buf,
            args_buf: &app.edit_args_buf,
            edit_field: app.edit_field,
            theme: &app.theme,
        };
        frame.render_widget(&edit, frame.area());

        // Directory browser overlay (on top of edit popup)
        if app.show_dir_browser {
            dir_browser_popup::render(
                frame.area(),
                frame.buffer_mut(),
                &app.dir_browser_path,
                app.dir_browser_cursor,
                &app.theme,
            );
        }
    }

    // EXIF metadata overlay
    if app.show_exif
        && let Some(ref exif) = app.exif_info
    {
        exif_panel::render(frame.area(), frame.buffer_mut(), exif, &app.theme);
    }

    if app.show_before_after
        && let Some(ref info) = app.before_after_info
    {
        before_after_popup::render(frame.area(), frame.buffer_mut(), info, &app.theme);
    }

    if app.show_help {
        let help = help_popup::HelpPopup { theme: &app.theme };
        frame.render_widget(&help, frame.area());
    }

    // Undo list overlay
    if app.show_undo_list {
        undo_popup::render(
            frame.area(),
            frame.buffer_mut(),
            &app.generated_outputs,
            app.undo_cursor,
            &app.theme,
        );
    }
}

/// Render the top title bar.
fn render_title_bar(frame: &mut Frame, area: Rect, app: &App) {
    let mode_indicator = match app.mode {
        Mode::Browse => "",
        Mode::Edit => " [EDIT]",
        Mode::Run => " [RUN]",
        Mode::Help => " [HELP]",
    };

    let recipe_name = app.selected_recipe_name.as_deref().unwrap_or("no recipe");

    let format_info = app
        .format_override
        .as_deref()
        .map(|f| format!("[{f}]"))
        .unwrap_or_default();

    let left = format!(" lazymagick v0.1.0{format_info}{mode_indicator} ");
    let right = format!(" [recipe: {recipe_name}]  [Ctrl+Q Quit] ");
    let padding = " ".repeat(
        area.width
            .saturating_sub(left.len() as u16 + right.len() as u16) as usize,
    );
    let title_line = Line::from(vec![left.clone().into(), padding.into(), right.into()]);

    let paragraph = Paragraph::new(title_line).style(
        Style::default()
            .fg(Color::White)
            .bg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(paragraph, area);
}

/// Render the bottom status bar with keybinding hints.
fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let dry_run_hint = if app.dry_run { " [DRY-RUN]" } else { "" };
    let recursive_hint = if app.recursive { " [RECURSIVE]" } else { "" };
    let hints: String = match app.mode {
        Mode::Help => " [Help overlay]  ?/Esc: close ".into(),
        Mode::Edit => " [Edit popup]  Esc: cancel  Enter: confirm ".into(),
        Mode::Run => {
            if app.magick_handle.is_some() {
                " [Running…]  c: cancel ".into()
            } else {
                format!(
                    " [1-4/Tab] Focus  [j/k] Move  [Space] Select  [r] Run  [f] Format{dry_run_hint}{recursive_hint}  [?] Help "
                )
            }
        }
        Mode::Browse => {
            format!(
                " [1-4/Tab] Focus  [j/k] Move  [Space] Select  [r] Run  [f] Format{dry_run_hint}{recursive_hint}  [?] Help "
            )
        }
    };

    let paragraph = Paragraph::new(Line::from(hints))
        .style(Style::default().fg(Color::DarkGray).bg(Color::Black));
    frame.render_widget(paragraph, area);
}
