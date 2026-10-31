//! `export_note` / `export_project` against real temporary directories.

use std::fs;
use std::path::Path;

use qm_core::{export_note, export_project, CoreError, ExportFormat, Project};
use qm_format::{Block, BlockKind, Document, FrontMatter, Inline};
use tempfile::TempDir;

const ALL_FORMATS: [ExportFormat; 3] = [
    ExportFormat::Obsidian,
    ExportFormat::Latex,
    ExportFormat::Pdf,
];

fn sample_document() -> Document {
    Document {
        front_matter: FrontMatter {
            title: Some("Sample".into()),
            ..FrontMatter::default()
        },
        blocks: vec![Block {
            kind: BlockKind::Paragraph(vec![Inline::Text("Hello".into())]),
            line: 0,
        }],
    }
}

/// A project with nested notes, an image, a hidden folder and an empty folder.
fn sample_project() -> (TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    let root = project.root();
    fs::create_dir_all(root.join("week1/figures")).unwrap();
    fs::create_dir_all(root.join("empty")).unwrap();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(root.join("index.qm"), "See [[intro]].").unwrap();
    fs::write(root.join("week1/intro.qm"), "Welcome").unwrap();
    fs::write(
        root.join("week1/figures/plot.png"),
        [0x89, b'P', b'N', b'G'],
    )
    .unwrap();
    fs::write(root.join(".git/HEAD"), "ref").unwrap();
    (dir, project)
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn extensions_match_formats() {
    assert_eq!(ExportFormat::Obsidian.extension(), "md");
    assert_eq!(ExportFormat::Latex.extension(), "tex");
    assert_eq!(ExportFormat::Pdf.extension(), "pdf");
}

#[test]
fn export_note_writes_each_format_creating_parent_folders() {
    let dir = tempfile::tempdir().unwrap();
    let document = sample_document();

    for format in ALL_FORMATS {
        let destination = dir
            .path()
            .join("out/nested")
            .join(format!("sample.{}", format.extension()));
        export_note(&document, format, &destination).unwrap();
        assert!(destination.is_file());
    }

    let out = dir.path().join("out/nested");
    assert!(read(&out.join("sample.md")).contains("title: \"Sample\""));
    assert!(read(&out.join("sample.tex")).contains("\\title{Sample}"));
    assert!(fs::read(out.join("sample.pdf"))
        .unwrap()
        .starts_with(b"%PDF"));
}

#[test]
fn obsidian_project_export_is_a_mirrored_vault_with_attachments() {
    let (_dir, project) = sample_project();
    let out = tempfile::tempdir().unwrap();
    let vault = out.path().join("vault");

    export_project(&project, ExportFormat::Obsidian, &vault).unwrap();

    assert!(vault.join("index.md").is_file());
    assert!(vault.join("week1/intro.md").is_file());
    assert_eq!(
        fs::read(vault.join("week1/figures/plot.png")).unwrap(),
        [0x89, b'P', b'N', b'G']
    );
    assert!(vault.join("empty").is_dir());
    assert!(!vault.join(".git").exists());
    assert!(!vault.join(".quickmath").exists());
    assert!(!vault.join("index.qm").exists());
}

#[test]
fn latex_and_pdf_project_exports_produce_one_file_per_note() {
    let (_dir, project) = sample_project();
    let out = tempfile::tempdir().unwrap();

    for format in [ExportFormat::Latex, ExportFormat::Pdf] {
        let folder = out.path().join(format.extension());
        export_project(&project, format, &folder).unwrap();
        let extension = format.extension();
        assert!(folder.join(format!("index.{extension}")).is_file());
        assert!(folder.join(format!("week1/intro.{extension}")).is_file());
        assert!(!folder.join("week1/figures/plot.png").exists());
    }
    assert!(read(&out.path().join("tex/index.tex")).contains("\\begin{document}"));
    assert!(fs::read(out.path().join("pdf/week1/intro.pdf"))
        .unwrap()
        .starts_with(b"%PDF"));
}

#[test]
fn project_export_into_a_subfolder_skips_that_subfolder() {
    let (_dir, project) = sample_project();
    let destination = project.root().join("export");

    export_project(&project, ExportFormat::Obsidian, &destination).unwrap();
    export_project(&project, ExportFormat::Obsidian, &destination).unwrap();

    assert!(destination.join("index.md").is_file());
    assert!(!destination.join("export").exists());
}

#[test]
fn project_export_refuses_the_project_folder_and_its_parents() {
    let (_dir, project) = sample_project();
    for destination in [project.root(), project.root().parent().unwrap()] {
        let result = export_project(&project, ExportFormat::Obsidian, destination);
        assert!(
            matches!(result, Err(CoreError::Export(_))),
            "{destination:?}: {result:?}"
        );
    }
    assert_eq!(read(&project.root().join("index.qm")), "See [[intro]].");
}
