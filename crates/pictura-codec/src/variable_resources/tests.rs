use super::*;
use crate::{read_psd, write_psb, write_psd};
use pictura_core::layer_comp::{set_layer_id, CompRecord, LayerComp};
use pictura_core::variables::{Placement, ScaleMethod, Value, VariableKind};
use pictura_core::LayerStack;

fn document() -> Document {
    let mut doc = Document::from_rgba("Tile", 2, 2, &[255; 16]);
    let mut other = doc.layers[0].clone();
    other.name = "Other".into();
    set_layer_id(&mut doc.layers[0], 10);
    set_layer_id(&mut other, 11);
    doc.layers.push(other).unwrap();
    doc
}

fn blocks(doc: &mut Document, definitions: &str, rows: &str) {
    doc.image_resources = [
        frame_variable_resource(7000, definitions.as_bytes()).raw,
        frame_variable_resource(7001, rows.as_bytes()).raw,
    ]
    .concat()
    .into();
}

fn definitions(binding: &str) -> String {
    format!(
        r#"<variableSets xmlns="http://ns.adobe.com/Variables/1.0/"><variableSet locked="none" varSetName="binding1"><variables>{binding}</variables></variableSet></variableSets>"#
    )
}

fn rows(value: &str) -> String {
    format!(
        r#"<sampleDataSets psdPath=""><sampleDataSet dataSetName="A &amp; B">{value}</sampleDataSet></sampleDataSets>"#
    )
}

#[test]
fn psd_variables_read_mesa_templates() {
    let mut doc = document();
    let defs = r#"<variable varName="image" trait="fileref" placementMethod="ratio" align="center" valign="middle" clip="false" docRef="id('10')"/><variable varName="image" trait="fileref" placementMethod="fit" align="center" valign="middle" clip="false" docRef="id('11')"/>"#;
    blocks(
        &mut doc,
        &definitions(defs),
        &rows("<image>café.png</image>"),
    );
    let opened = read_psd(&write_psd(&doc).unwrap()).unwrap();
    let bindings = &opened.variables.definitions()["image"].bindings;
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].layer_id, 10);
    assert_eq!(bindings[0].placement.method, ScaleMethod::AsIs);
    assert_eq!(bindings[1].placement.method, ScaleMethod::Fit);
    assert_eq!(opened.variables.sets()[0].name, "A & B");
    assert_eq!(
        opened.variables.value(0, "image"),
        Some(&Value::File("café.png".into()))
    );
}

#[test]
fn psd_variables_preserve_unsupported_blocks() {
    for defs in [
        "<variableSets",
        "<unsupported/>",
        "<variable varName=\"title\" trait=\"textcontent\" docRef=\"id('123')\"/>",
    ] {
        let mut doc = document();
        blocks(&mut doc, &definitions(defs), &rows("<title>Text</title>"));
        let mut opened = read_psd(&write_psd(&doc).unwrap()).unwrap();
        assert!(!opened.variables.issues().is_empty());
        assert!(opened
            .bind_variable(
                &[0],
                "present",
                VariableKind::Visibility,
                Placement::default()
            )
            .is_err());
        opened.layers[0].name = "Renamed".into();
        let saved = read_psd(&write_psd(&opened).unwrap()).unwrap();
        assert_eq!(saved.image_resources, doc.image_resources);
    }
}

#[test]
fn psd_variables_preserve_unknown_placement() {
    for (method, align) in [("unknown", "center"), ("ratio", "left")] {
        let mut doc = document();
        let defs = format!(
            r#"<variable varName="image" trait="fileref" placementMethod="{method}" align="{align}" valign="middle" clip="false" docRef="id('10')"/>"#
        );
        blocks(
            &mut doc,
            &definitions(&defs),
            &rows("<image>replacement.png</image>"),
        );
        let mut opened = read_psd(&write_psd(&doc).unwrap()).unwrap();
        assert!(!opened.variables.issues().is_empty());
        assert!(opened.variables.definitions().is_empty());
        opened.layers[0].visible = false;
        let saved = read_psd(&write_psd(&opened).unwrap()).unwrap();
        assert_eq!(saved.image_resources, doc.image_resources);
        assert!(saved.variables.definitions().is_empty());
    }
}

#[test]
fn psd_variables_preserve_duplicate_ids_with_comps() {
    let mut doc = document();
    set_layer_id(&mut doc.layers[1], 10);
    let mut unique = doc.layers[0].clone();
    set_layer_id(&mut unique, 12);
    doc.layers.push(unique).unwrap();
    let defs = r#"<variable varName="image" trait="fileref" placementMethod="ratio" align="center" valign="middle" clip="false" docRef="id('10')"/>"#;
    blocks(
        &mut doc,
        &definitions(defs),
        &rows("<image>replacement.png</image>"),
    );
    doc.layer_comps.push_comp(LayerComp::new(
        1,
        "View",
        "",
        CompRecord::all(),
        vec![pictura_core::layer_comp::CompEntry {
            layer_id: 12,
            state: pictura_core::layer_comp::CapturedState {
                visible: Some(false),
                ..Default::default()
            },
        }],
    ));
    doc.layer_comps.mark_dirty();
    let mut opened = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert!(opened
        .variables
        .issues()
        .iter()
        .any(|issue| issue.contains("ambiguous")));
    assert!(opened.variables.definitions().is_empty());
    assert_eq!(
        opened.layer_comps.comps()[0]
            .entry(12)
            .and_then(|s| s.visible),
        Some(false)
    );
    assert!(!opened.layer_comps.parsed);
    let shmd = opened.layers[2].extra_block(b"shmd").unwrap().data.clone();
    opened.layer_comps.mark_base_dirty();
    opened.layers[1].name = "Renamed".into();
    let saved = read_psd(&write_psd(&opened).unwrap()).unwrap();
    assert!(saved
        .variables
        .issues()
        .iter()
        .any(|issue| issue.contains("ambiguous")));
    assert_eq!(saved.image_resources, opened.image_resources);
    assert_eq!(saved.layers[2].extra_block(b"shmd").unwrap().data, shmd);
    let mut unsafe_edit = opened.clone();
    unsafe_edit.layer_comps.mark_dirty();
    assert!(write_psd(&unsafe_edit).is_err());
    assert_eq!(
        pictura_core::layer_comp::layer_id(&saved.layers[1]),
        Some(10)
    );
}

#[test]
fn psd_variables_roundtrip_without_layer_comps() {
    let mut doc = document();
    let placement = Placement {
        method: ScaleMethod::Conform,
        ..Placement::default()
    };
    doc.bind_variable(&[0], "pic", VariableKind::Pixels, placement)
        .unwrap();
    doc.bind_variable(&[1], "pic", VariableKind::Pixels, placement)
        .unwrap();
    doc.bind_variable(&[0], "show", VariableKind::Visibility, placement)
        .unwrap();
    doc.new_data_set(None, "Example").unwrap();
    doc.set_data_value(0, "pic", Value::File("x & y.png".into()))
        .unwrap();
    for bytes in [write_psd(&doc).unwrap(), write_psb(&doc).unwrap()] {
        let mut opened = read_psd(&bytes).unwrap();
        assert_eq!(
            opened.variables.value(0, "pic"),
            doc.variables.value(0, "pic")
        );
        assert_eq!(opened.variables.definitions()["pic"].bindings.len(), 2);
        assert_eq!(
            opened.variables.definitions()["pic"].bindings[0].placement,
            placement
        );
        let resources = crate::decode_image_resources(&opened);
        let seed = resources
            .iter()
            .find(|r| r.id == 1044)
            .expect("layer ID seed");
        assert_eq!(
            u32::from_be_bytes(seed.data.as_slice().try_into().unwrap()),
            12
        );
        assert!(resources
            .iter()
            .filter(|r| matches!(r.id, 7000 | 7001))
            .all(|r| &r.raw[..4] == b"MeSa"));
        opened
            .layers
            .push(crate::layer_from_parts(pictura_core::LayerParts {
                name: "Third".into(),
                ..Default::default()
            }))
            .unwrap();
        opened
            .bind_variable(&[2], "show", VariableKind::Visibility, Placement::default())
            .unwrap();
        assert!(pictura_core::layer_comp::layer_id(&opened.layers[2]).unwrap() > 11);
    }
}

#[test]
fn psd_variables_rewrite_resources_selectively() {
    let mut doc = document();
    let passthrough = crate::frame_image_resource(3010, "unrelated", b"opaque").raw;
    doc.image_resources = passthrough.clone().into();
    doc.bind_variable(&[0], "show", VariableKind::Visibility, Placement::default())
        .unwrap();
    doc.new_data_set(None, "Example").unwrap();
    let mut opened = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(opened.variables.sets()[0].name, "Example");
    assert!(crate::decode_image_resources(&opened)
        .iter()
        .any(|r| r.raw == passthrough));
    opened
        .unbind_variable(&[0], VariableKind::Visibility)
        .unwrap();
    let saved = read_psd(&write_psd(&opened).unwrap()).unwrap();
    let resources = crate::decode_image_resources(&saved);
    assert!(resources.iter().all(|r| !matches!(r.id, 7000 | 7001)));
    assert!(resources.iter().any(|r| r.raw == passthrough));
}

#[test]
fn psd_variables_preserve_truncated_resource_header() {
    for id in [7000u16, 7001] {
        let mut doc = document();
        let mut raw = b"MeSa".to_vec();
        raw.extend_from_slice(&id.to_be_bytes());
        raw.extend_from_slice(&[0, 0, 0, 0, 0, 10, b'<']);
        doc.image_resources = raw.clone().into();
        let mut opened = read_psd(&write_psd(&doc).unwrap()).unwrap();
        assert!(opened
            .variables
            .issues()
            .iter()
            .any(|issue| issue.contains("truncated")));
        assert!(opened
            .bind_variable(&[0], "show", VariableKind::Visibility, Placement::default())
            .is_err());
        opened.layers[0].name = "Edited".into();
        let reopened = read_psd(&write_psd(&opened).unwrap()).unwrap();
        assert_eq!(&*reopened.image_resources, raw.as_slice());
        assert!(!reopened.variables.issues().is_empty());
    }
}
