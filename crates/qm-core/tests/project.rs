//! Project (vault) operations against real temporary directories.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use qm_core::{
    CoreError, FileKind, FileNode, NoteSlides, Project, WorkspaceState, PROJECT_DIR_NAME,
};
use tempfile::TempDir;

fn new_project() -> (TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    (dir, project)
}

fn touch(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "").unwrap();
}

fn child_names(node: &FileNode) -> Vec<&str> {
    match &node.kind {
        FileKind::Folder(children) => children.iter().map(|c| c.name.as_str()).collect(),
        _ => panic!("{} is not a folder", node.name),
    }
}

fn trash_entries(project: &Project) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(project.root().join(PROJECT_DIR_NAME).join("trash"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

// --- open -------------------------------------------------------------------

#[test]
fn open_creates_project_folder_with_trash() {
    let (_dir, project) = new_project();
    assert!(project.root().join(".quickmath/trash").is_dir());
    assert!(project.root().is_absolute());
}

#[test]
fn open_fails_for_missing_folder_and_for_files() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file.txt");
    fs::write(&file, "").unwrap();

    assert!(matches!(
        Project::open(&dir.path().join("missing")),
        Err(CoreError::Io { .. })
    ));
    assert!(matches!(Project::open(&file), Err(CoreError::Io { .. })));
}

#[test]
fn name_is_the_folder_name() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("Analysis I");
    fs::create_dir(&root).unwrap();
    assert_eq!(Project::open(&root).unwrap().name(), "Analysis I");
}

// --- file_tree ----------------------------------------------------------------

#[test]
fn file_tree_sorts_folders_first_then_case_insensitively_and_skips_hidden() {
    let (_dir, project) = new_project();
    let root = project.root();
    touch(&root.join("beta.qm"));
    touch(&root.join("Alpha.qm"));
    touch(&root.join("image.png"));
    touch(&root.join(".hidden.qm"));
    touch(&root.join("zeta/inner.qm"));
    touch(&root.join("Archive/old.qm"));
    touch(&root.join(".git/config"));

    let tree = project.file_tree().unwrap();

    assert_eq!(tree.path, root);
    assert_eq!(
        child_names(&tree),
        ["Archive", "zeta", "Alpha.qm", "beta.qm", "image.png"]
    );
    let FileKind::Folder(children) = &tree.kind else {
        unreachable!()
    };
    assert_eq!(children[2].kind, FileKind::Note);
    assert_eq!(children[4].kind, FileKind::Other);
    assert_eq!(children[1].path, root.join("zeta"));
    assert_eq!(child_names(&children[1]), ["inner.qm"]);
}

#[test]
fn file_tree_of_empty_project_is_an_empty_root() {
    let (_dir, project) = new_project();
    assert_eq!(project.file_tree().unwrap().kind, FileKind::Folder(vec![]));
}

#[cfg(unix)]
#[test]
fn file_tree_does_not_follow_symlinked_folders() {
    let (_dir, project) = new_project();
    std::os::unix::fs::symlink(project.root(), project.root().join("loop")).unwrap();
    touch(&project.root().join("a.qm"));

    assert_eq!(child_names(&project.file_tree().unwrap()), ["a.qm"]);
}

// --- create_note / create_folder ----------------------------------------------------

#[test]
fn create_note_appends_extension_and_creates_empty_file() {
    let (_dir, project) = new_project();

    let path = project.create_note(project.root(), "Lecture 1").unwrap();

    assert_eq!(path, project.root().join("Lecture 1.qm"));
    assert_eq!(fs::read_to_string(&path).unwrap(), "");
    let kept = project.create_note(Path::new(""), "kept.qm").unwrap();
    assert_eq!(kept, project.root().join("kept.qm"));
}

#[test]
fn create_note_accepts_folders_relative_to_the_root() {
    let (_dir, project) = new_project();
    fs::create_dir(project.root().join("week1")).unwrap();

    let path = project.create_note(Path::new("week1"), "intro").unwrap();

    assert_eq!(path, project.root().join("week1/intro.qm"));
}

#[test]
fn create_note_rejects_invalid_names() {
    let (_dir, project) = new_project();
    for name in ["", "  ", "a/b", "a\\b", "..", ".hidden", "nul\0"] {
        let result = project.create_note(project.root(), name);
        assert!(
            matches!(result, Err(CoreError::InvalidName(_))),
            "{name:?}: {result:?}"
        );
    }
}

#[test]
fn create_note_refuses_to_overwrite() {
    let (_dir, project) = new_project();
    project.create_note(project.root(), "a").unwrap();
    project
        .write_note(&project.root().join("a.qm"), "keep me")
        .unwrap();

    let result = project.create_note(project.root(), "a.qm");

    assert!(matches!(result, Err(CoreError::AlreadyExists(_))));
    assert_eq!(
        project.read_note(&project.root().join("a.qm")).unwrap(),
        "keep me"
    );
}

#[test]
fn create_note_refuses_folders_outside_the_project() {
    let (dir, project) = new_project();
    let outside = tempfile::tempdir().unwrap();
    for folder in [
        outside.path().to_path_buf(),
        dir.path().join(".."),
        PathBuf::from("../.."),
    ] {
        let result = project.create_note(&folder, "escape");
        assert!(
            matches!(result, Err(CoreError::OutsideProject(_))),
            "{folder:?}: {result:?}"
        );
    }
    let hidden = project.root().join(PROJECT_DIR_NAME);
    assert!(matches!(
        project.create_note(&hidden, "x"),
        Err(CoreError::OutsideProject(_))
    ));
}

#[test]
fn create_note_in_missing_folder_fails() {
    let (_dir, project) = new_project();
    let result = project.create_note(&project.root().join("missing"), "x");
    assert!(matches!(result, Err(CoreError::Io { .. })));
}

#[cfg(unix)]
#[test]
fn symlinks_cannot_escape_the_project() {
    let (_dir, project) = new_project();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.qm"), "secret").unwrap();
    let link = project.root().join("link");
    std::os::unix::fs::symlink(outside.path(), &link).unwrap();

    assert!(matches!(
        project.create_note(&link, "x"),
        Err(CoreError::OutsideProject(_))
    ));
    assert!(matches!(
        project.read_note(&link.join("secret.qm")),
        Err(CoreError::OutsideProject(_))
    ));
    assert!(matches!(
        project.write_note(&link.join("new.qm"), "x"),
        Err(CoreError::OutsideProject(_))
    ));
}

#[test]
fn create_folder_creates_and_rejects_duplicates_and_bad_names() {
    let (_dir, project) = new_project();

    let path = project.create_folder(project.root(), "week 1").unwrap();

    assert!(path.is_dir());
    assert!(matches!(
        project.create_folder(project.root(), "week 1"),
        Err(CoreError::AlreadyExists(_))
    ));
    assert!(matches!(
        project.create_folder(project.root(), "a/b"),
        Err(CoreError::InvalidName(_))
    ));
    assert!(matches!(
        project.create_folder(Path::new(".."), "x"),
        Err(CoreError::OutsideProject(_))
    ));
}

// --- rename ---------------------------------------------------------------------------

#[test]
fn rename_note_keeps_the_extension() {
    let (_dir, project) = new_project();
    let note = project.create_note(project.root(), "old").unwrap();

    let renamed = project.rename(&note, "new").unwrap();

    assert_eq!(renamed, project.root().join("new.qm"));
    assert!(renamed.is_file());
    assert!(!note.exists());
}

#[test]
fn rename_folder_and_other_files_use_the_name_verbatim() {
    let (_dir, project) = new_project();
    let folder = project.create_folder(project.root(), "a").unwrap();
    touch(&project.root().join("pic.png"));

    assert_eq!(
        project.rename(&folder, "b").unwrap(),
        project.root().join("b")
    );
    assert_eq!(
        project
            .rename(&project.root().join("pic.png"), "photo.jpg")
            .unwrap(),
        project.root().join("photo.jpg")
    );
}

#[test]
fn rename_to_same_name_is_a_no_op() {
    let (_dir, project) = new_project();
    let note = project.create_note(project.root(), "same").unwrap();
    assert_eq!(project.rename(&note, "same").unwrap(), note);
    assert!(note.is_file());
}

#[test]
fn rename_refuses_collisions_invalid_names_and_bad_targets() {
    let (_dir, project) = new_project();
    let a = project.create_note(project.root(), "a").unwrap();
    project.create_note(project.root(), "b").unwrap();

    assert!(matches!(
        project.rename(&a, "b"),
        Err(CoreError::AlreadyExists(_))
    ));
    assert!(matches!(
        project.rename(&a, "../b"),
        Err(CoreError::InvalidName(_))
    ));
    assert!(matches!(
        project.rename(project.root(), "x"),
        Err(CoreError::InvalidName(_))
    ));
    assert!(matches!(
        project.rename(Path::new("../a.qm"), "x"),
        Err(CoreError::OutsideProject(_))
    ));
    assert!(matches!(
        project.rename(Path::new("missing.qm"), "x"),
        Err(CoreError::Io { .. })
    ));
}

// --- trash ------------------------------------------------------------------------------

#[test]
fn trash_moves_entries_with_collision_safe_names() {
    let (_dir, project) = new_project();
    for _ in 0..3 {
        let note = project.create_note(project.root(), "draft").unwrap();
        project.trash(&note).unwrap();
        assert!(!note.exists());
    }
    let folder = project.create_folder(project.root(), "old.notes").unwrap();
    touch(&folder.join("inside.qm"));
    project.trash(&folder).unwrap();
    project.create_folder(project.root(), "old.notes").unwrap();
    project.trash(&folder).unwrap();

    assert_eq!(
        trash_entries(&project),
        [
            "draft 2.qm",
            "draft 3.qm",
            "draft.qm",
            "old.notes",
            "old.notes 2"
        ]
    );
    assert!(project
        .root()
        .join(".quickmath/trash/old.notes/inside.qm")
        .is_file());
}

#[test]
fn trash_refuses_root_outside_and_missing_paths() {
    let (_dir, project) = new_project();
    assert!(matches!(
        project.trash(project.root()),
        Err(CoreError::InvalidName(_))
    ));
    assert!(matches!(
        project.trash(Path::new("..")),
        Err(CoreError::OutsideProject(_))
    ));
    assert!(matches!(
        project.trash(&project.root().join(PROJECT_DIR_NAME)),
        Err(CoreError::OutsideProject(_))
    ));
    assert!(matches!(
        project.trash(Path::new("missing.qm")),
        Err(CoreError::Io { .. })
    ));
}

// --- read / write -------------------------------------------------------------------------

#[test]
fn write_then_read_round_trips_and_leaves_no_temp_files() {
    let (_dir, project) = new_project();
    let note = project.root().join("note.qm");

    project.write_note(&note, "first").unwrap();
    project.write_note(&note, "second ∫ unicode").unwrap();

    assert_eq!(project.read_note(&note).unwrap(), "second ∫ unicode");
    let entries: Vec<_> = fs::read_dir(project.root())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries.len(), 2, "{entries:?}"); // note.qm and .quickmath
}

#[test]
fn write_into_missing_folder_fails_without_creating_it() {
    let (_dir, project) = new_project();
    let result = project.write_note(&project.root().join("missing/note.qm"), "x");
    assert!(matches!(result, Err(CoreError::Io { .. })));
    assert!(!project.root().join("missing").exists());
}

#[test]
fn read_and_write_refuse_paths_outside_the_project() {
    let (dir, project) = new_project();
    let outside = dir.path().join("../outside.qm");
    assert!(matches!(
        project.write_note(&outside, "x"),
        Err(CoreError::OutsideProject(_))
    ));
    assert!(matches!(
        project.read_note(&outside),
        Err(CoreError::OutsideProject(_))
    ));
    assert!(matches!(
        project.write_note(&project.root().join(".quickmath/workspace.json"), "{}"),
        Err(CoreError::OutsideProject(_))
    ));
}

#[test]
fn read_missing_note_is_an_io_error() {
    let (_dir, project) = new_project();
    assert!(matches!(
        project.read_note(Path::new("missing.qm")),
        Err(CoreError::Io { .. })
    ));
}

// --- wikilinks ------------------------------------------------------------------------------

fn wikilink_project() -> (TempDir, Project) {
    let (dir, project) = new_project();
    for note in [
        "Topology.qm",
        "archive/2023/topology.qm",
        "week1/Intro.qm",
        "week2/intro.qm",
        "week2/deep/Groups.qm",
        "lecture.1.qm",
    ] {
        touch(&project.root().join(note));
    }
    touch(&project.root().join("diagram.png"));
    (dir, project)
}

#[test]
fn wikilink_by_name_matches_anywhere_case_insensitively() {
    let (_dir, project) = wikilink_project();
    let root = project.root();
    assert_eq!(
        project.resolve_wikilink("groups"),
        Some(root.join("week2/deep/Groups.qm"))
    );
    assert_eq!(
        project.resolve_wikilink("lecture.1"),
        Some(root.join("lecture.1.qm"))
    );
}

#[test]
fn wikilink_prefers_the_shortest_path() {
    let (_dir, project) = wikilink_project();
    let root = project.root();
    assert_eq!(
        project.resolve_wikilink("topology"),
        Some(root.join("Topology.qm"))
    );
    assert_eq!(
        project.resolve_wikilink("intro"),
        Some(root.join("week1/Intro.qm"))
    );
}

#[test]
fn wikilink_with_folder_matches_the_relative_path() {
    let (_dir, project) = wikilink_project();
    let root = project.root();
    assert_eq!(
        project.resolve_wikilink("week2/intro"),
        Some(root.join("week2/intro.qm"))
    );
    assert_eq!(
        project.resolve_wikilink("2023/Topology"),
        Some(root.join("archive/2023/topology.qm"))
    );
    assert_eq!(
        project.resolve_wikilink("deep/groups.qm"),
        Some(root.join("week2/deep/Groups.qm"))
    );
}

#[test]
fn wikilink_ignores_extension_and_heading() {
    let (_dir, project) = wikilink_project();
    let expected = Some(project.root().join("week1/Intro.qm"));
    assert_eq!(project.resolve_wikilink("week1/Intro.qm#Goals"), expected);
    assert_eq!(project.resolve_wikilink(" Intro#^block "), expected);
}

#[test]
fn wikilink_without_match_is_none() {
    let (_dir, project) = wikilink_project();
    for target in [
        "missing",
        "",
        "#heading",
        "diagram",
        "diagram.png",
        "eek2/intro",
        "../Topology",
    ] {
        assert_eq!(project.resolve_wikilink(target), None, "{target:?}");
    }
}

// --- workspace ---------------------------------------------------------------------------------

#[test]
fn workspace_round_trips_with_relative_paths_on_disk() {
    let (_dir, project) = new_project();
    let folder = project.create_folder(project.root(), "week1").unwrap();
    let note = project.create_note(&folder, "intro").unwrap();
    let state = WorkspaceState {
        open_note: Some(note.clone()),
        expanded_folders: vec![folder.clone()],
        note_slides: BTreeMap::new(),
    };

    project.save_workspace(&state).unwrap();

    assert_eq!(project.load_workspace(), state);
    let json = fs::read_to_string(project.root().join(".quickmath/workspace.json")).unwrap();
    assert!(json.contains("\"week1/intro.qm\""), "{json}");
    assert!(
        !json.contains(&project.root().to_string_lossy().into_owned()),
        "{json}"
    );
}

#[test]
fn workspace_drops_entries_that_no_longer_exist() {
    let (_dir, project) = new_project();
    let kept = project.create_folder(project.root(), "kept").unwrap();
    let gone = project.create_folder(project.root(), "gone").unwrap();
    let note = project.create_note(project.root(), "note").unwrap();
    project
        .save_workspace(&WorkspaceState {
            open_note: Some(note.clone()),
            expanded_folders: vec![kept.clone(), gone.clone()],
            ..WorkspaceState::default()
        })
        .unwrap();
    fs::remove_dir(&gone).unwrap();
    fs::remove_file(&note).unwrap();

    let state = project.load_workspace();

    assert_eq!(
        state,
        WorkspaceState {
            open_note: None,
            expanded_folders: vec![kept],
            ..WorkspaceState::default()
        }
    );
}

#[test]
fn workspace_ignores_corrupt_or_escaping_entries() {
    let (_dir, project) = new_project();
    let path = project.root().join(".quickmath/workspace.json");
    assert_eq!(project.load_workspace(), WorkspaceState::default());

    fs::write(&path, "not json").unwrap();
    assert_eq!(project.load_workspace(), WorkspaceState::default());

    fs::write(
        &path,
        r#"{"open_note": "../../etc/passwd", "expanded_folders": ["/", ".."]}"#,
    )
    .unwrap();
    assert_eq!(project.load_workspace(), WorkspaceState::default());
}

#[test]
fn save_workspace_refuses_paths_outside_the_project() {
    let (_dir, project) = new_project();
    let outside = tempfile::tempdir().unwrap();
    let state = WorkspaceState {
        open_note: Some(outside.path().join("x.qm")),
        ..WorkspaceState::default()
    };
    assert!(matches!(
        project.save_workspace(&state),
        Err(CoreError::OutsideProject(_))
    ));
}

#[test]
fn workspace_round_trips_slides_links_with_relative_paths() {
    let (_dir, project) = new_project();
    let note = project.create_note(project.root(), "lecture 1").unwrap();
    let deck = project.root().join("slides/week 1.pdf");
    let state = WorkspaceState {
        note_slides: BTreeMap::from([(note.clone(), NoteSlides { deck, page: 4 })]),
        ..WorkspaceState::default()
    };

    project.save_workspace(&state).unwrap();

    assert_eq!(project.load_workspace(), state);
    let json = fs::read_to_string(project.root().join(".quickmath/workspace.json")).unwrap();
    assert!(json.contains("\"lecture 1.qm\""), "{json}");
    assert!(json.contains("\"slides/week 1.pdf\""), "{json}");
}

#[test]
fn workspace_keeps_links_to_missing_decks_but_drops_links_of_missing_notes() {
    let (_dir, project) = new_project();
    let kept = project.create_note(project.root(), "kept").unwrap();
    let gone = project.create_note(project.root(), "gone").unwrap();
    let missing_deck = NoteSlides {
        deck: project.root().join("missing.pdf"),
        page: 2,
    };
    project
        .save_workspace(&WorkspaceState {
            note_slides: BTreeMap::from([
                (kept.clone(), missing_deck.clone()),
                (gone.clone(), missing_deck.clone()),
            ]),
            ..WorkspaceState::default()
        })
        .unwrap();
    fs::remove_file(&gone).unwrap();

    let state = project.load_workspace();

    assert_eq!(state.note_slides, BTreeMap::from([(kept, missing_deck)]));
}

#[test]
fn workspace_ignores_slides_links_that_escape_the_project() {
    let (_dir, project) = new_project();
    project.create_note(project.root(), "note").unwrap();
    let path = project.root().join(".quickmath/workspace.json");
    fs::write(
        &path,
        r#"{"note_slides": {"note.qm": {"deck": "../../secret.pdf", "page": 0}}}"#,
    )
    .unwrap();

    assert_eq!(project.load_workspace(), WorkspaceState::default());
}
