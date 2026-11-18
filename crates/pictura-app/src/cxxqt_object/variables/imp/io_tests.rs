use super::tests::{bind, test_pixels, with_file};
use super::*;
use std::path::{Path, PathBuf};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "pictura-variables-{:x}",
            u64::from_le_bytes(pictura_codec::os_random())
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str, contents: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn variable_import_reads_delimited_encodings() {
    let dir = Scratch::new();
    for (extension, separator) in [("csv", ','), ("tsv", '\t')] {
        for newline in ["\r", "\n", "\r\n"] {
            let input = format!("show{separator}photo{newline}false{separator} \" café{separator}two\"\" words{newline} here \" ");
            let utf16: Vec<u8> = [0xff, 0xfe]
                .into_iter()
                .chain(input.encode_utf16().flat_map(u16::to_le_bytes))
                .collect();
            let windows: Vec<u8> = input.chars().map(|c| c as u8).collect();
            for (bytes, encoding) in [
                (input.as_bytes(), Encoding::Utf8),
                (utf16.as_slice(), Encoding::Automatic),
                (windows.as_slice(), Encoding::Windows1252),
            ] {
                let path = dir.file(&format!("data.{extension}"), bytes);
                let mut doc = with_file();
                bind(&mut doc, 0, "show", VariableKind::Visibility);
                import_sets(
                    &mut doc,
                    &path,
                    ImportOptions {
                        encoding,
                        ..ImportOptions::default()
                    },
                )
                .unwrap();
                let value = doc.variables.value(1, "photo").unwrap();
                let Value::File(file) = value else {
                    panic!("expected file")
                };
                assert!(file.ends_with(&format!(" café{separator}two\" words{newline} here ")));
                assert_eq!(
                    doc.variables.value(1, "show"),
                    Some(&Value::Visibility(false))
                );
            }
        }
    }
}

#[test]
fn variable_import_first_column_is_a_binding() {
    let dir = Scratch::new();
    let mut doc = with_file();
    bind(&mut doc, 0, "show", VariableKind::Visibility);
    let opts = ImportOptions {
        first_names: true,
        ..ImportOptions::default()
    };
    let input = dir.file("data.csv", b"show,photo\nfalse,portrait.png\n");
    import_sets(&mut doc, &input, opts).unwrap();
    assert_eq!(doc.variables.sets()[1].name, "false");
    assert_eq!(
        doc.variables.value(1, "show"),
        Some(&Value::Visibility(false))
    );
    let wrong = dir.file("wrong.csv", b"name,photo,show\nfirst,portrait.png,false\n");
    assert!(import_sets(&mut doc, &wrong, opts).is_err());
}

#[test]
fn variable_import_failure_preserves_sets() {
    let dir = Scratch::new();
    let mut doc = with_file();
    bind(&mut doc, 0, "show", VariableKind::Visibility);
    let before = doc.variables.clone();
    for case in ["unterminated,\"", "short", "value,nope", "first,true"] {
        let input = dir.file(
            "data.csv",
            format!("photo,show\nfirst,true\n{case}\n").as_bytes(),
        );
        assert!(import_sets(
            &mut doc,
            &input,
            ImportOptions {
                replace: true,
                first_names: true,
                ..ImportOptions::default()
            }
        )
        .is_err());
        assert_eq!(doc.variables, before);
    }
}

#[test]
fn variable_import_commits_once_and_keeps_paths() {
    let dir = Scratch::new();
    let mut doc = with_file();
    doc.new_data_set(None, "Data Set 1").unwrap();
    let input = dir.file("data.csv", b"photo\nrelative.png\n");
    let mut view = super::tests::install(doc.clone());
    let mut draft = doc.clone();
    import_sets(&mut draft, &input, ImportOptions::default()).unwrap();
    assert_eq!(draft.variables.sets()[2].name, "Data Set 2");
    let edit = install_template(&mut view, draft).unwrap();
    view.apply_edit("Import Variable Data Sets", edit);
    let bytes = pictura_codec::write_psd(view.session.doc.as_ref().unwrap()).unwrap();
    let reopened = pictura_codec::read_psd(&bytes).unwrap();
    let expected = Value::File(dir.0.join("relative.png").to_string_lossy().into_owned());
    assert_eq!(reopened.variables.value(2, "photo"), Some(&expected));
    assert_eq!(
        view.session.history.undo().unwrap().doc.variables,
        doc.variables
    );
    assert!(view.session.history.undo().is_none());
    let mut next = doc.clone();
    import_sets(
        &mut next,
        &input,
        ImportOptions {
            replace: true,
            ..ImportOptions::default()
        },
    )
    .unwrap();
    assert_eq!(next.variables.sets().len(), 1);
    assert_eq!(next.variables.value(0, "photo"), Some(&expected));
}

#[test]
fn variable_import_single_column_delimiters() {
    let dir = Scratch::new();
    let mut doc = with_file();
    for name in ["one.tsv", "one.txt", "one.CSV"] {
        let body: &[u8] = if name.ends_with("CSV") {
            b"photo\n\"one,two.png\"\n"
        } else {
            b"photo\none,two.png\n"
        };
        let path = dir.file(name, body);
        import_sets(&mut doc, &path, ImportOptions::default()).unwrap();
        let index = doc.variables.sets().len() - 1;
        assert!(
            matches!(doc.variables.value(index, "photo"), Some(Value::File(file)) if file.ends_with("one,two.png"))
        );
    }
    for (file, body) in [
        ("one.unknown", &b"photo\none.png\n"[..]),
        ("mixed.tsv", &b"photo,show\ttext\none.png\n"[..]),
    ] {
        assert!(import_sets(&mut doc, &dir.file(file, body), ImportOptions::default()).is_err());
    }
}

fn export(
    doc: &Document,
    folder: &Path,
    selected: &[usize],
    pattern: &str,
    load: impl Fn(&str) -> Result<ReplacementImage, String>,
    progress: impl FnMut(usize) -> bool,
) -> BatchResult {
    export_sets(doc, folder, selected, "Cover", pattern, load, progress)
}

#[test]
fn variable_export_uses_a_fresh_baseline() {
    let dir = Scratch::new();
    let mut doc = with_file();
    doc.new_data_set(None, "Keep").unwrap();
    doc.new_data_set(None, "Omit").unwrap();
    let result = export(&doc, &dir.0, &[1, 0], "", test_pixels, |_| true);
    assert_eq!(result.written.len(), 2);
    let opened = |name: &Path| pictura_codec::read_psd(&std::fs::read(name).unwrap()).unwrap();
    assert_ne!(
        opened(&result.written[0]).layers[0].channels(),
        doc.layers[0].channels()
    );
    assert_eq!(
        opened(&result.written[1]).layers[0].channels(),
        doc.layers[0].channels()
    );
    assert_eq!(opened(&result.written[0]).variables.sets().len(), 3);
}

#[test]
fn variable_export_avoids_path_collisions() {
    let dir = Scratch::new();
    let mut doc = with_file();
    doc.new_data_set(None, "Keep").unwrap();
    let existing = dir.file("bad_name.psd", b"keep this");
    let names = export(&doc, &dir.0, &[0, 1], "bad/name", test_pixels, |_| true);
    assert_eq!(names.written.len(), 2);
    assert_ne!(names.written[0], names.written[1]);
    assert!(names
        .written
        .iter()
        .all(|path| path.parent() == Some(dir.0.as_path())));
    assert_eq!(std::fs::read(existing).unwrap(), b"keep this");
    let long = export(&doc, &dir.0, &[0], &"ä".repeat(200), test_pixels, |_| true);
    assert!(long.written[0].file_name().unwrap().len() <= 255);
}

#[test]
fn variable_export_failure_keeps_completed_files() {
    let dir = Scratch::new();
    let mut doc = with_file();
    doc.new_data_set(None, "Keep").unwrap();
    doc.new_data_set(None, "Stop").unwrap();
    let rejected = export(&doc, &dir.0, &[0, 1, 2], "", super::tests::no_pixels, |i| {
        i < 2
    });
    assert_eq!(rejected.skipped.len(), 1);
    assert_eq!(rejected.written.len(), 1);
    assert!(rejected.cancelled);
    assert!(pictura_codec::read_psd(&std::fs::read(&rejected.written[0]).unwrap()).is_ok());
    let absent = export(&doc, &dir.0.join("absent"), &[0], "", test_pixels, |_| true);
    assert!(absent.failure.is_some());
    doc.layers[0].background = true;
    let rejected = export(&doc, &dir.0, &[0], "", test_pixels, |_| true);
    assert_eq!(rejected.skipped.len(), 1);
    assert!(rejected.failure.is_none());
    doc.layers[0].background = false;
    doc.mode = pictura_core::ColorMode::Duotone;
    let unsupported = export(&doc, &dir.0, &[0], "", test_pixels, |_| true);
    assert_eq!(unsupported.skipped.len(), 1);
    assert!(unsupported.skipped[0].contains("Duotone"));

    let mut batch = ExportBatch::new(&with_file(), &dir.0, &[0], "Race", "").unwrap();
    let target = batch.targets[0].1.clone();
    std::fs::write(&target, b"created by another process").unwrap();
    batch.step(test_pixels);
    assert!(batch.result.failure.is_some());
    assert_eq!(
        std::fs::read(&target).unwrap(),
        b"created by another process"
    );
}

#[test]
fn variable_export_skips_save_memory_refusal() {
    use super::export::ExportError;
    let dir = Scratch::new();
    let mut doc = with_file();
    doc.new_data_set(None, "Keep").unwrap();
    let mut batch = ExportBatch::new(&doc, &dir.0, &[0, 1], "Cover", "").unwrap();
    batch.step_with_writer(test_pixels, |_, _| {
        Err(ExportError::Resource("memory budget".into()))
    });
    assert_eq!(batch.result.skipped.len(), 1);
    assert!(!batch.done());
    batch.step(test_pixels);
    assert_eq!(batch.result.written.len(), 1);
    assert!(batch.result.failure.is_none());
    assert!(pictura_codec::read_psd(&std::fs::read(&batch.result.written[0]).unwrap()).is_ok());
}
