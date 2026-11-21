use super::{add_type_layer, re_render_type_layers};
use crate::{resolve_path, resolve_path_mut};
use pictura_codec::DescValue;
use pictura_core::{BitDepth, ColorMode, Document, TypeSpec};

#[test]
fn refreshing_type_preserves_descriptors_and_imported_mixed_runs() {
    let mut doc = Document::new(200, 100, ColorMode::Rgb, BitDepth::Eight);
    let mut spec = TypeSpec::new("Mixed", "Liberation Sans", 24.0);
    spec.origin = (10.0, 45.0);
    let mixed = add_type_layer(&mut doc, "", &spec);
    let layer = resolve_path_mut(&mut doc, &mixed).expect("mixed type layer");
    let source = b"<< /EngineDict <<
        /StyleRun << /RunArray [
        << /StyleSheet << /StyleSheetData << /Font 0 /FontSize 24.0 >> >> >>
        << /StyleSheet << /StyleSheetData << /Font 0 /FontSize 40.0 >> >> >> ]
        /RunLengthArray [ 2 4 ] >>
        /ParagraphRun << /RunArray [ << /ParagraphSheet << /Properties <<
        /Justification 0 >> >> >> ] >> >> >>";
    pictura_codec::parse_engine_data(source).expect("valid imported runs");
    let mut data = b"tdta".to_vec();
    data.extend_from_slice(&(source.len() as u32).to_be_bytes());
    data.extend_from_slice(source);
    let mut tool = layer.type_tool().expect("type tool").clone();
    tool.text_desc = pictura_codec::write_descriptor(&DescValue::Object {
        name: String::new(),
        class_id: b"TxLr".to_vec(),
        items: vec![
            (b"Txt ".to_vec(), DescValue::Text("Mixed\0".into())),
            (b"EngineData".to_vec(), DescValue::Raw(data)),
        ],
    })
    .into();
    pictura_codec::edit_layer(layer, |parts| {
        let block = parts
            .extra_blocks
            .iter_mut()
            .find(|block| &block.key == b"TySh")
            .expect("type block");
        block.data = pictura_codec::encode_type_tool(&tool).into();
        parts.type_tool = Some(tool);
    });
    let saved_mixed = layer.clone();

    spec.text = "Uniform\rSecond line".into();
    let uniform = add_type_layer(&mut doc, &mixed, &spec);
    let layer = resolve_path_mut(&mut doc, &uniform).expect("uniform type layer");
    layer.rect.left += 7;
    layer.rect.right += 7;
    let tool_before = layer.type_tool().expect("type tool").clone();

    assert_eq!(re_render_type_layers(&mut doc), 1);
    assert_eq!(resolve_path(&doc, &mixed), Some(&saved_mixed));
    let layer = resolve_path(&doc, &uniform).expect("refreshed type layer");
    assert_eq!(layer.type_tool(), Some(&tool_before));
}
