//! Slide decks: opening and rendering real PDF files inside a project.

use std::fs;
use std::path::{Path, PathBuf};

use qm_core::{export_note, CoreError, ExportFormat, Project};
use tempfile::TempDir;

fn new_project() -> (TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    (dir, project)
}

/// Writes a real PDF (exported from a note) to `path`.
fn write_pdf(path: &Path, source: &str) {
    let document = qm_format::parse_document(source);
    export_note(&document, ExportFormat::Pdf, path).unwrap();
}

fn write_one_page_deck(project: &Project, relative: &str) -> PathBuf {
    let path = project.root().join(relative);
    write_pdf(
        &path,
        "# Lecture 1\n\nThe limit ;;lim_(x->0) sin x / x = 1;;.",
    );
    path
}

#[test]
fn open_slides_counts_the_pages() {
    let (_dir, project) = new_project();
    let path = write_one_page_deck(&project, "lecture.pdf");

    let deck = project.open_slides(&path).unwrap();

    assert_eq!(deck.page_count(), 1);
}

#[test]
fn render_page_has_the_requested_width_and_the_page_aspect_ratio() {
    let (_dir, project) = new_project();
    let deck = project
        .open_slides(&write_one_page_deck(&project, "lecture.pdf"))
        .unwrap();

    let image = deck.render_page(0, 600).unwrap();

    // Exported notes are A4 portrait: about √2 times as tall as wide.
    assert!((599..=600).contains(&image.width), "{}", image.width);
    let aspect = image.height as f32 / image.width as f32;
    assert!((aspect - 1.414).abs() < 0.01, "{aspect}");
    assert_eq!(
        image.rgba_premultiplied.len(),
        (image.width * image.height * 4) as usize
    );
}

#[test]
fn render_page_draws_the_text_on_a_white_page() {
    let (_dir, project) = new_project();
    let deck = project
        .open_slides(&write_one_page_deck(&project, "lecture.pdf"))
        .unwrap();

    let image = deck.render_page(0, 400).unwrap();

    let pixels: Vec<&[u8]> = image.rgba_premultiplied.chunks_exact(4).collect();
    assert!(
        pixels.iter().all(|pixel| pixel[3] == 255),
        "pages are opaque"
    );
    assert!(
        pixels.contains(&[255, 255, 255, 255].as_slice()),
        "white paper"
    );
    assert!(
        pixels.iter().any(|pixel| pixel[0] < 100),
        "dark text is drawn"
    );
}

#[test]
fn render_page_refuses_pages_past_the_end() {
    let (_dir, project) = new_project();
    let deck = project
        .open_slides(&write_one_page_deck(&project, "lecture.pdf"))
        .unwrap();

    let result = deck.render_page(1, 400);

    let error = result.unwrap_err();
    assert!(matches!(error, CoreError::Slides { .. }));
    assert!(error.to_string().contains("has no page 2"), "{error}");
}

#[test]
fn open_slides_rejects_files_that_are_not_pdfs() {
    let (_dir, project) = new_project();
    let path = project.root().join("fake.pdf");
    fs::write(&path, "this is not a PDF").unwrap();

    let result = project.open_slides(&path);

    let error = result.err().unwrap();
    assert!(matches!(error, CoreError::Slides { .. }));
    assert!(error.to_string().contains("not a readable PDF"), "{error}");
}

#[test]
fn open_slides_reports_missing_files() {
    let (_dir, project) = new_project();

    let result = project.open_slides(Path::new("gone.pdf"));

    assert!(matches!(result, Err(CoreError::Io { .. })));
}

#[test]
fn open_slides_refuses_files_outside_the_project() {
    let (_dir, project) = new_project();
    let outside = tempfile::tempdir().unwrap();
    let path = outside.path().join("elsewhere.pdf");
    write_pdf(&path, "Elsewhere");

    let result = project.open_slides(&path);

    assert!(matches!(result, Err(CoreError::OutsideProject(_))));
}

#[test]
fn file_tree_lists_slide_decks_in_every_folder() {
    let (_dir, project) = new_project();
    let week1 = project.create_folder(project.root(), "week1").unwrap();
    fs::write(project.root().join("intro.PDF"), "").unwrap();
    fs::write(week1.join("limits.pdf"), "").unwrap();
    fs::write(week1.join("figure.png"), "").unwrap();
    project.create_note(&week1, "notes").unwrap();

    let decks = project.file_tree().unwrap().slide_decks();

    assert_eq!(
        decks,
        vec![week1.join("limits.pdf"), project.root().join("intro.PDF")]
    );
}
