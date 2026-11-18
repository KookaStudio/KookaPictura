use std::fs;
use std::path::{Path, PathBuf};

use super::naming::{sanitize, BatchDate, FileNaming};
use super::*;
use crate::action::{Action, ActionSet, ActionStep, StepKind};
use crate::command::CommandParams;
use crate::library::ActionLibrary;

const DATE: BatchDate = BatchDate {
    year: 2026,
    month: 3,
    day: 4,
};

fn naming(fields: &[&str]) -> FileNaming {
    FileNaming {
        fields: fields.iter().map(|f| f.to_string()).collect(),
        ..FileNaming::default()
    }
}

struct TempTree(PathBuf);

impl TempTree {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("pictura-batch-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn file(&self, rel: &str) -> PathBuf {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"x").unwrap();
        path
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn exts() -> Vec<String> {
    ["png", "jpg", "psd"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn relative(root: &Path, paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|p| {
            p.strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

#[test]
fn naming_tokens_evaluate_in_order() {
    let scheme = naming(&["document name", "_v", "3 Digit Serial Number", "EXTENSION"]);
    // serial 7 = starting serial 1 + position 6
    assert_eq!(scheme.render("Beach.PSD", "jpg", 6, DATE), "beach_v007.JPG");
}

#[test]
fn naming_serial_letters_continue_past_z() {
    let scheme = naming(&["Serial Letter (A, B, C...)"]);
    let names: Vec<String> = [0, 25, 26]
        .iter()
        .map(|&p| scheme.render("x.png", "png", p, DATE))
        .collect();
    assert_eq!(names, ["A.png", "Z.png", "AA.png"]);
    assert_eq!(
        naming(&["Serial Letter (a, b, c...)"]).render("x", "png", 27, DATE),
        "ab.png"
    );
}

#[test]
fn naming_date_tokens_use_the_start_date() {
    let expected = [
        ("mmddyy (date)", "030426"),
        ("mmdd (date)", "0304"),
        ("yyyymmdd (date)", "20260304"),
        ("yymmdd (date)", "260304"),
        ("yyddmm (date)", "260403"),
        ("ddmmyy (date)", "040326"),
        ("ddmm (date)", "0403"),
    ];
    for (token, value) in expected {
        assert_eq!(
            naming(&[token, "extension"]).render("a.png", "png", 0, DATE),
            format!("{value}.png"),
            "{token}"
        );
    }
}

#[test]
fn naming_appends_a_missing_extension() {
    assert_eq!(
        naming(&["Document Name"]).render("a.tif", "tif", 0, DATE),
        "a.tif"
    );
    assert_eq!(
        naming(&["Document Name"]).render("a.png", "JPG", 0, DATE),
        "a.jpg"
    );
}

#[test]
fn naming_serials_count_from_the_start_value() {
    let mut scheme = naming(&["2 Digit Serial Number"]);
    scheme.starting_serial = 9;
    let names: Vec<String> = (0..3)
        .map(|p| scheme.render("a.png", "png", p, DATE))
        .collect();
    assert_eq!(names, ["09.png", "10.png", "11.png"]);
    assert_eq!(
        naming(&["1 Digit Serial Number"]).render("a", "png", 11, DATE),
        "12.png"
    );
}

#[test]
fn naming_windows_compatibility_sanitizes() {
    assert_eq!(sanitize("a:b?c. ", true, false), "a_b_c");
    let mut scheme = naming(&["a:b?c. ", "extension"]);
    scheme.windows = true;
    assert_eq!(scheme.render("in.png", "png", 0, DATE), "a_b_c.png");
    assert_eq!(sanitize("a:b", false, true), "a_b");
}

#[test]
fn naming_unix_compatibility_always_applies() {
    assert_eq!(naming(&["x/y"]).render("in.png", "png", 0, DATE), "x_y.png");
    assert_eq!(sanitize("a:b?", false, false), "a:b?");
}

fn library_with(actions: Vec<Action>) -> ActionLibrary {
    let mut set = ActionSet::new("Set");
    set.actions = actions;
    ActionLibrary::from_sets(vec![set])
}

fn action(name: &str, steps: Vec<ActionStep>) -> Action {
    let mut action = Action::new(name);
    action.steps = steps;
    action
}

fn save_as_step() -> ActionStep {
    ActionStep::command(
        SAVE_AS_COMMAND,
        CommandParams::new()
            .with("as", "JPG")
            .with("in", "/tmp/out.jpg"),
    )
}

fn play_step(target: &str) -> ActionStep {
    let mut step = ActionStep::command("layer.flatten.image", CommandParams::new());
    step.kind = StepKind::Play {
        action: target.to_string(),
    };
    step
}

fn options_for(source: &Path, action: &str) -> BatchOptions {
    BatchOptions {
        set: "Set".into(),
        action: action.into(),
        source_folder: source.to_string_lossy().into_owned(),
        ..BatchOptions::default()
    }
}

#[test]
fn validate_rejects_a_scheme_without_a_unique_field() {
    let tree = TempTree::new("unique");
    let library = library_with(vec![action("A", vec![])]);
    let mut options = options_for(&tree.0, "A");
    options.destination = BatchDestination::Folder;
    options.destination_folder = options.source_folder.clone();
    options.naming = naming(&["yyyymmdd (date)", "extension"]);
    assert_eq!(validate(&options, &library), Err(BatchError::NoUniqueField));
    options.naming = naming(&["yyyymmdd (date)", "Serial Letter (a, b, c...)"]);
    assert_eq!(validate(&options, &library), Ok(()));
}

#[test]
fn validate_rejects_an_extension_before_other_fields() {
    let tree = TempTree::new("extension-last");
    let library = library_with(vec![action("A", vec![])]);
    let mut options = options_for(&tree.0, "A");
    options.destination = BatchDestination::Folder;
    options.destination_folder = options.source_folder.clone();
    options.naming = naming(&["Document Name", "extension", "3 Digit Serial Number"]);
    assert_eq!(
        validate(&options, &library),
        Err(BatchError::ExtensionNotLast)
    );
    options.naming = naming(&[
        "Document Name",
        "3 Digit Serial Number",
        "",
        "extension",
        "",
    ]);
    assert_eq!(validate(&options, &library), Ok(()));
    options.naming = naming(&["Document Name", "3 Digit Serial Number"]);
    assert_eq!(validate(&options, &library), Ok(()));
}

#[test]
fn validate_rejects_a_missing_action() {
    let tree = TempTree::new("action");
    let library = library_with(vec![action("A", vec![])]);
    let err = validate(&options_for(&tree.0, "Missing"), &library).unwrap_err();
    assert!(matches!(err, BatchError::MissingAction { ref action, .. } if action == "Missing"));
    assert!(err.to_string().contains("Missing"));
}

#[test]
fn validate_override_save_needs_a_save_as_step() {
    let tree = TempTree::new("savestep");
    let mut disabled = save_as_step();
    disabled.flags.enabled = false;
    let library = library_with(vec![action("A", vec![disabled])]);
    let mut options = options_for(&tree.0, "A");
    options.destination = BatchDestination::SaveAndClose;
    options.override_save = true;
    assert_eq!(validate(&options, &library), Err(BatchError::NoSaveAsStep));
    options.override_save = false;
    assert_eq!(validate(&options, &library), Ok(()));
}

#[test]
fn validate_accepts_a_nested_save_as_step() {
    let tree = TempTree::new("nested");
    let library = library_with(vec![
        action("Outer", vec![play_step("Inner"), play_step("Outer")]),
        action("Inner", vec![save_as_step()]),
    ]);
    let mut options = options_for(&tree.0, "Outer");
    options.destination = BatchDestination::SaveAndClose;
    options.override_save = true;
    assert_eq!(validate(&options, &library), Ok(()));
}

#[test]
fn validate_rejects_missing_folders() {
    let tree = TempTree::new("folders");
    let library = library_with(vec![action("A", vec![])]);
    let missing = tree.0.join("nope").to_string_lossy().into_owned();
    let mut options = options_for(&tree.0, "A");
    options.source_folder = missing.clone();
    let err = validate(&options, &library).unwrap_err();
    assert_eq!(err, BatchError::MissingSourceFolder(missing.clone()));
    assert!(err.to_string().contains(&missing));

    let mut options = options_for(&tree.0, "A");
    options.destination = BatchDestination::Folder;
    options.destination_folder = missing.clone();
    assert_eq!(
        validate(&options, &library),
        Err(BatchError::MissingDestinationFolder(missing))
    );

    let mut options = options_for(&tree.0, "A");
    options.errors = ErrorPolicy::Log;
    assert_eq!(
        validate(&options, &library),
        Err(BatchError::MissingErrorLog)
    );
}

#[test]
fn folder_source_lists_top_level_images_in_order() {
    let tree = TempTree::new("top");
    for f in ["b.jpg", "A.png", "notes.txt", ".hidden.jpg", "sub/c.jpg"] {
        tree.file(f);
    }
    let found = resolve_folder(&tree.0, false, &exts());
    assert_eq!(relative(&tree.0, &found), ["A.png", "b.jpg"]);
}

#[test]
fn folder_source_includes_subfolders() {
    let tree = TempTree::new("recurse");
    for f in [
        "b.jpg",
        "A.png",
        "notes.txt",
        ".hidden.jpg",
        "sub/c.jpg",
        ".git/d.png",
    ] {
        tree.file(f);
    }
    let found = resolve_folder(&tree.0, true, &exts());
    assert_eq!(relative(&tree.0, &found), ["A.png", "b.jpg", "sub/c.jpg"]);
}

#[cfg(unix)]
#[test]
fn folder_source_survives_a_link_cycle() {
    let tree = TempTree::new("cycle");
    tree.file("a.png");
    tree.file("sub/b.png");
    std::os::unix::fs::symlink(&tree.0, tree.0.join("sub/loop")).unwrap();
    let found = resolve_folder(&tree.0, true, &exts());
    assert_eq!(relative(&tree.0, &found), ["a.png", "sub/b.png"]);
}

#[test]
fn error_log_line_names_the_input() {
    assert_eq!(
        error_log_line("/in/b.png", "cannot read\nfile"),
        "/in/b.png: cannot read file\n"
    );
}

#[test]
fn override_save_path_follows_the_destination() {
    let mut options = BatchOptions {
        destination: BatchDestination::Folder,
        destination_folder: "/out".into(),
        naming: naming(&["Document Name", "2 Digit Serial Number", "extension"]),
        ..BatchOptions::default()
    };
    let input = Path::new("/in/Beach.png");
    assert_eq!(
        override_save_path(&options, input, "JPG", 1, DATE),
        Some(PathBuf::from("/out/Beach02.jpg"))
    );
    options.destination = BatchDestination::SaveAndClose;
    assert_eq!(
        override_save_path(&options, input, "JPG", 1, DATE),
        Some(PathBuf::from("/in/Beach.jpg"))
    );
    options.destination = BatchDestination::None;
    assert_eq!(override_save_path(&options, input, "JPG", 1, DATE), None);
}

#[test]
fn batch_options_round_trip_through_json() {
    let options = BatchOptions {
        set: "S".into(),
        source: BatchSource::OpenedFiles,
        destination: BatchDestination::SaveAndClose,
        errors: ErrorPolicy::Log,
        ..BatchOptions::default()
    };
    let text = serde_json::to_string(&options).unwrap();
    assert!(text.contains("\"openedFiles\"") && text.contains("\"saveAndClose\""));
    assert_eq!(
        serde_json::from_str::<BatchOptions>(&text).unwrap(),
        options
    );
    let defaults: BatchOptions = serde_json::from_str("{}").unwrap();
    assert_eq!(defaults.naming.fields, ["Document Name", "extension"]);
}
