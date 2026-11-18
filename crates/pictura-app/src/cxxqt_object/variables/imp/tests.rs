use super::*;
use crate::cxxqt_object::{tests::pixel_layer, PictureViewRust};
use pictura_core::variables::{ScaleMethod, VariableKind};
use pictura_core::{BitDepth, ColorMode, LayerStack};

pub(super) fn template() -> Document {
    let mut doc = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("Tile", 4, 4, (8, 16, 24))]);
    doc
}

pub(super) fn bind(doc: &mut Document, index: usize, name: &str, kind: VariableKind) {
    doc.bind_variable(&[index], name, kind, Placement::default())
        .unwrap();
}

pub(super) fn test_pixels(_: &str) -> Result<ReplacementImage, String> {
    Ok(ReplacementImage {
        rgba: vec![200; 4 * 2 * 4],
        width: 4,
        height: 2,
    })
}

pub(super) fn no_pixels(_: &str) -> Result<ReplacementImage, String> {
    Err("missing image".into())
}

pub(super) fn with_file() -> Document {
    let mut doc = template();
    bind(&mut doc, 0, "photo", VariableKind::Pixels);
    doc.new_data_set(None, "Use photo").unwrap();
    doc.set_data_value(0, "photo", Value::File("example.png".into()))
        .unwrap();
    doc
}

pub(super) fn install(doc: Document) -> PictureViewRust {
    let mut view = PictureViewRust::default();
    view.session.doc.set(Some(doc));
    view.session.record(&view.layer_target, "Open");
    view
}

#[test]
fn variable_bindings_follow_layer_ids() {
    let mut doc = template();
    doc.layers
        .push(pixel_layer("Other", 4, 4, (1, 2, 3)))
        .unwrap();
    bind(&mut doc, 0, "status", VariableKind::Visibility);
    bind(&mut doc, 1, "status", VariableKind::Visibility);
    doc.layers[0].name = "Renamed".into();
    assert!(doc.layers.reverse(&[], 0, 2));
    assert_eq!(pictura_render::duplicate_paths(&mut doc, &["0"]).len(), 1);
    doc.new_data_set(None, "Hidden").unwrap();
    doc.set_data_value(0, "status", Value::Visibility(false))
        .unwrap();
    let next = evaluate(&doc, 0, no_pixels).unwrap().unwrap();
    assert_eq!(next.layers.iter().filter(|layer| !layer.visible).count(), 2);
    assert!(
        !next
            .layers
            .iter()
            .find(|layer| layer.name == "Renamed")
            .unwrap()
            .visible
    );
}

#[test]
fn variable_bindings_restore_on_undo() {
    let mut doc = template();
    bind(&mut doc, 0, "status", VariableKind::Visibility);
    doc.layers
        .push(pixel_layer("Other", 4, 4, (1, 2, 3)))
        .unwrap();
    doc.new_data_set(None, "One").unwrap();
    doc.new_data_set(None, "Two").unwrap();
    let mut view = install(doc.clone());
    let removed = pictura_render::delete_paths(view.session.doc.as_mut().unwrap(), &["0"]);
    assert_eq!(removed, 1);
    assert!(view
        .session
        .doc
        .as_ref()
        .unwrap()
        .variables
        .definitions()
        .is_empty());
    view.session.record(&view.layer_target, "Delete layer");
    let before = view.session.history.undo().unwrap().doc;
    assert_eq!(before.variables, doc.variables);
    assert_eq!(before.layers.len(), 2);
}

#[test]
fn variable_apply_is_one_edit() {
    let mut doc = with_file();
    doc.layers[0].mask = Some(pictura_core::LayerMask {
        rect: doc.layers[0].rect,
        default_color: 255,
        disabled: false,
        flags: 0,
        data: None,
        extra: Default::default(),
        density: 255,
        feather_bits: 0,
    });
    let title = pictura_render::add_type_layer(
        &mut doc,
        "",
        &pictura_core::TypeSpec::new("Old title", "Liberation Sans", 10.0),
    );
    let index = title.parse::<usize>().unwrap();
    bind(&mut doc, index, "title", VariableKind::Text);
    bind(&mut doc, 0, "status", VariableKind::Visibility);
    doc.set_data_value(0, "title", Value::Text("Line one\nLine two".into()))
        .unwrap();
    doc.set_data_value(0, "status", Value::Visibility(false))
        .unwrap();
    let mut view = install(doc.clone());
    let next = evaluate(view.session.doc.as_ref().unwrap(), 0, test_pixels)
        .unwrap()
        .unwrap();
    let edit = install_evaluation(&mut view, Some(next));
    view.apply_edit("Apply Data Set: Use photo", edit);
    let live = view.session.doc.as_ref().unwrap();
    assert!(!live.layers[0].visible);
    assert_eq!(live.layers[0].mask, doc.layers[0].mask);
    assert_ne!(live.layers[0].channels(), doc.layers[0].channels());
    assert_eq!(
        live.layers[index].type_tool().unwrap().text,
        "Line one\nLine two"
    );
    assert_eq!(
        live.layers[index].type_tool().unwrap().style,
        doc.layers[index].type_tool().unwrap().style
    );
    let history = &mut view.session.history;
    assert_eq!(history.undo().unwrap().doc.variables, doc.variables);
    assert!(history.undo().is_none());
    assert!(!history.redo().unwrap().doc.layers[0].visible);
}

#[test]
fn variable_empty_text_stays_editable() {
    let mut doc = template();
    let index = pictura_render::add_type_layer(
        &mut doc,
        "",
        &pictura_core::TypeSpec::new("Title", "Liberation Sans", 10.0),
    )
    .parse::<usize>()
    .unwrap();
    bind(&mut doc, index, "title", VariableKind::Text);
    doc.new_data_set(None, "Empty").unwrap();
    doc.set_data_value(0, "title", Value::Text(String::new()))
        .unwrap();
    let next = evaluate(&doc, 0, no_pixels).unwrap().unwrap();
    let reopened = pictura_codec::read_psd(&pictura_codec::write_psd(&next).unwrap()).unwrap();
    assert_eq!(reopened.layers[index].type_tool().unwrap().text, "");
    assert!(reopened.layers[index].rect.is_empty());
}

#[test]
fn variable_skip_keeps_current_pixels() {
    let mut doc = with_file();
    doc.new_data_set(None, "Keep photo").unwrap();
    let changed = evaluate(&doc, 0, test_pixels).unwrap().unwrap();
    assert_ne!(changed.layers[0].channels(), doc.layers[0].channels());
    assert!(evaluate(&changed, 1, no_pixels).unwrap().is_none());
}

#[test]
fn variable_apply_failure_is_atomic() {
    let mut doc = with_file();
    bind(&mut doc, 0, "show", VariableKind::Visibility);
    doc.set_data_value(0, "show", Value::Visibility(false))
        .unwrap();
    for ineligible in [false, true] {
        doc.layers[0].background = ineligible;
        let before = doc.layers.clone();
        assert!(evaluate(&doc, 0, no_pixels).unwrap_err().contains("photo"));
        assert_eq!(doc.layers, before);
    }
}

#[test]
fn variable_pixels_save_in_native_encoding() {
    for (mode, depth) in [
        (ColorMode::Rgb, BitDepth::Sixteen),
        (ColorMode::Grayscale, BitDepth::Eight),
    ] {
        let mut doc = with_file();
        if mode != ColorMode::Rgb {
            pictura_render::convert_mode(&mut doc, mode).unwrap();
        }
        if depth != BitDepth::Eight {
            pictura_render::convert_bit_depth(&mut doc, depth).unwrap();
        }
        let mut next = evaluate(&doc, 0, test_pixels).unwrap().unwrap();
        refresh_export_composite(&mut next).unwrap();
        let reopened = pictura_codec::read_psd(&pictura_codec::write_psd(&next).unwrap()).unwrap();
        assert_eq!(
            (
                reopened.document_color_mode(),
                reopened.document_bit_depth()
            ),
            (mode, depth)
        );
        assert_ne!(reopened.layers[0].channels(), doc.layers[0].channels());
    }
}

#[test]
fn variable_empty_frame_is_refused() {
    let mut doc = with_file();
    doc.layers[0].rect = PsdRect::default();
    assert!(evaluate(&doc, 0, test_pixels)
        .unwrap_err()
        .contains("empty"));
}

#[test]
fn psd_variables_ratio_as_is() {
    let mut doc = with_file();
    let id = pictura_core::layer_comp::layer_id(&doc.layers[0]).unwrap();
    let definitions = format!(
        r#"<variableSets xmlns="http://ns.adobe.com/Variables/1.0/"><variableSet><variables><variable varName="photo" trait="fileref" placementMethod="ratio" align="center" valign="middle" clip="false" docRef="id('{id}')"/></variables></variableSet></variableSets>"#
    );
    let values = r#"<sampleDataSets psdPath=""><sampleDataSet dataSetName="Example"><photo>local.png</photo></sampleDataSet></sampleDataSets>"#;
    doc.variables = Default::default();
    let mut raw = Vec::new();
    for (id, xml) in [(7000, definitions.as_str()), (7001, values)] {
        let mut resource = pictura_codec::frame_image_resource(id, "", xml.as_bytes()).raw;
        resource[..4].copy_from_slice(b"MeSa");
        raw.extend(resource);
    }
    doc.image_resources = raw.into();
    let reopened = pictura_codec::read_psd(&pictura_codec::write_psd(&doc).unwrap()).unwrap();
    assert_eq!(
        reopened.variables.definitions()["photo"].bindings[0]
            .placement
            .method,
        ScaleMethod::AsIs
    );
    let next = evaluate(&reopened, 0, test_pixels).unwrap().unwrap();
    assert_eq!(
        (next.layers[0].rect.top, next.layers[0].rect.height()),
        (1, 2)
    );
}
