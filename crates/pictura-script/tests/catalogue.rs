//! Catalogue golden and frozen-id drift checks.

use pictura_script::command_catalogue::{command_catalogue, command_id_for_path, const_names};

#[test]
fn golden_catalogue_matches_the_captured_table() {
    let golden = include_str!("golden/catalogue.tsv");
    let rows: Vec<&str> = golden.lines().filter(|line| !line.is_empty()).collect();
    let catalogue = command_catalogue();
    assert_eq!(rows.len(), catalogue.len(), "catalogue row count drifted");
    for (index, (line, spec)) in rows.iter().zip(catalogue).enumerate() {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 6, "malformed golden row {index}");
        assert_eq!(fields[0], spec.id, "id drifted at row {index}");
        assert_eq!(
            fields[1],
            spec.path.join("/"),
            "path drifted at row {index}"
        );
        assert_eq!(fields[2], spec.label, "label drifted at row {index}");
        assert_eq!(fields[3], spec.shortcut, "shortcut drifted at row {index}");
        assert_eq!(
            fields[4] == "1",
            spec.implemented,
            "implemented drifted at row {index}"
        );
        assert_eq!(
            fields[5] == "1",
            spec.checkable,
            "checkable drifted at row {index}"
        );
    }
}

#[test]
fn golden_const_names_match_the_captured_ids() {
    let golden = include_str!("golden/const_names.tsv");
    let expected: Vec<(&str, &str)> = golden
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (name, value) = line.split_once('\t').expect("name<TAB>value");
            (name, value)
        })
        .collect();
    assert_eq!(const_names(), expected.as_slice());
}

#[test]
fn cr_id_drift() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../pictura-app/cpp/command_ids.h");
    let header = std::fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "cannot read {} ({err}); generate command_ids.h from const_names()",
            path.display()
        )
    });
    let mut parsed = Vec::new();
    for line in header.lines() {
        let Some(rest) = line.trim().strip_prefix("inline constexpr char ") else {
            continue;
        };
        let Some((name, rest)) = rest.split_once("[] = \"") else {
            continue;
        };
        let Some((value, _)) = rest.split_once('"') else {
            continue;
        };
        parsed.push((name, value));
    }
    assert_eq!(
        parsed.as_slice(),
        const_names(),
        "the checked-in command_ids.h differs from the Rust catalogue"
    );
}

#[test]
fn every_implemented_command_is_classified() {
    assert!(
        pictura_script::command::coverage_gaps().is_empty(),
        "unclassified implemented commands: {:?}",
        pictura_script::command::coverage_gaps()
    );
}

#[test]
fn every_wired_filter_has_a_definition() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../pictura-app/src/cxxqt_object/filter_map/descriptor");
    let mut wired = Vec::new();
    for entry in std::fs::read_dir(&root).expect("filter descriptor directory") {
        let path = entry.expect("descriptor entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("descriptor file");
        for row in text.split("FilterDescriptor {").skip(1) {
            let Some(path_line) = row.lines().find(|line| line.trim().starts_with("path: &["))
            else {
                continue;
            };
            let menu = row
                .lines()
                .find(|line| line.trim().starts_with("menu:"))
                .is_some_and(|line| line.contains("true"));
            if !menu {
                continue;
            }
            let segments: Vec<&str> = path_line.split('"').skip(1).step_by(2).collect();
            wired.push(command_id_for_path(&segments));
        }
    }
    wired.push("filter.filter.gallery".to_string());
    assert!(
        wired.len() > 100,
        "parsed only {} wired filters",
        wired.len()
    );
    for id in &wired {
        assert!(
            pictura_script::command::definition(id).is_some(),
            "wired filter '{id}' has no command definition"
        );
    }
    for unwired in [
        "filter.lens.correction",
        "filter.vanishing.point",
        "filter.extract",
        "filter.video.de.interlace",
    ] {
        assert!(
            pictura_script::command::definition(unwired).is_none(),
            "unwired filter '{unwired}' must stay unimplemented"
        );
    }
}

#[test]
fn default_catalogue_shape_is_stable() {
    let catalogue = command_catalogue();
    assert_eq!(catalogue.len(), 589);
    assert_eq!(
        catalogue.iter().filter(|spec| spec.implemented).count(),
        340
    );
    assert_eq!(const_names().len(), 229);
}
