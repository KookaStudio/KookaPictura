use super::*;
use crate::{BitDepth, ColorMode, LayerContent, LayerParts, LayerStack};

fn template(layers: Vec<Layer>) -> Document {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(layers);
    doc
}

#[test]
fn variable_bindings_validate_names_and_targets() {
    let mut doc = template(vec![
        Layer::from_parts(
            LayerParts {
                background: true,
                ..Default::default()
            },
            LayerContent::Pixel,
        ),
        Layer::default(),
    ]);
    let placement = Placement::default();
    assert!(doc
        .bind_variable(&[0], "Title", VariableKind::Visibility, placement)
        .is_err());
    assert!(doc
        .bind_variable(&[1], "1Title", VariableKind::Visibility, placement)
        .is_err());
    assert!(doc
        .bind_variable(&[1], "Title", VariableKind::Text, placement)
        .is_err());
    doc.bind_variable(&[1], "Title", VariableKind::Visibility, placement)
        .unwrap();
    let previous = doc.variables.clone();
    assert!(doc
        .bind_variable(&[1], "Title", VariableKind::Pixels, placement)
        .is_err());
    assert_eq!(doc.variables, previous);
}

#[test]
fn variable_data_sets_keep_typed_values() {
    let mut doc = template(vec![Layer::default()]);
    doc.bind_variable(&[0], "show", VariableKind::Visibility, Placement::default())
        .unwrap();
    let a = doc.new_data_set(None, "").unwrap();
    let b = doc.new_data_set(Some(a), "").unwrap();
    doc.set_data_value(b, "show", Value::Visibility(false))
        .unwrap();
    doc.rename_variable("show", "display").unwrap();
    assert_eq!(
        doc.variables.value(a, "display"),
        Some(&Value::Visibility(true))
    );
    assert_eq!(
        doc.variables.value(b, "display"),
        Some(&Value::Visibility(false))
    );
    assert_eq!(doc.variables.sets()[b].name, "Data Set 2");
}

#[test]
fn variable_linked_defaults_follow_stack_order() {
    let type_layer = |text: &str| crate::TypeTool {
        transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        text: text.into(),
        bounds: [0; 4],
        text_desc: Default::default(),
        warp_desc: Default::default(),
        fonts: Vec::new(),
        style: Some(crate::TextStyle::default()),
        vertical: false,
        box_bounds: None,
        path: None,
    };
    let hidden_text = |text| {
        Layer::from_parts(
            LayerParts {
                visible: false,
                ..Default::default()
            },
            LayerContent::Text(Some(Arc::new(type_layer(text)))),
        )
    };
    let group = Layer::new(LayerContent::Group(LayerStack::from_layers(vec![
        hidden_text("Top"),
    ])));
    let mut doc = template(vec![hidden_text("Bottom"), group]);
    for path in [&[0][..], &[1, 0], &[1]] {
        doc.bind_variable(path, "show", VariableKind::Visibility, Placement::default())
            .unwrap();
    }
    doc.bind_variable(&[0], "title", VariableKind::Text, Placement::default())
        .unwrap();
    doc.bind_variable(&[1, 0], "title", VariableKind::Text, Placement::default())
        .unwrap();
    let index = doc.new_data_set(None, "").unwrap();
    assert_eq!(
        doc.variables.value(index, "show"),
        Some(&Value::Visibility(true))
    );
    assert_eq!(
        doc.variables.value(index, "title"),
        Some(&Value::Text("Top".into()))
    );
}
