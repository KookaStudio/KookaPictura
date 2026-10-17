use super::*;
use pictura_core::{BitDepth, ColorMode, Document, Layer, PsdRect};

const OPEN: TextPolicy = TextPolicy {
    transparency: true,
    cff: true,
};

fn spec(text: &str) -> TypeSpec {
    let mut spec = TypeSpec::new(text, "Liberation Sans", 32.0);
    spec.origin = (20.0, 60.0);
    spec.character.fill_color = [1.0, 1.0, 0.0, 0.0];
    spec
}

/// A pixel layer with a type layer above it.
fn doc() -> Document {
    let mut doc = Document::from_rgba("photo", 300, 100, &vec![200u8; 300 * 100 * 4]);
    let path = crate::add_type_layer(&mut doc, "0", &spec("office"));
    assert!(!path.is_empty());
    doc
}

#[test]
fn a_top_type_layer_becomes_live_text_and_is_hidden() {
    let doc = doc();
    let live = pdf_live_type(&doc, OPEN).expect("live text");
    assert_eq!(live.texts.len(), 1);
    let text = &live.texts[0];
    assert_eq!(text.color, [255, 0, 0]);
    assert_eq!(text.opacity, 255);
    assert_eq!(text.size, 32.0);
    assert_eq!(text.origin, (20.0, 60.0));
    assert!(!text.glyphs.is_empty());
    assert!(
        text.glyphs.iter().all(|g| g.v == text.glyphs[0].v),
        "one baseline"
    );
    let type_index = doc.layers.iter().position(Layer::is_type).unwrap();
    assert!(!live.hidden.layers[type_index].visible);
    assert!(live.hidden.layers.iter().any(|l| l.visible && !l.is_type()));

    let font = &live.fonts[text.font];
    assert!(!font.cff);
    assert_eq!(font.postscript, "LiberationSans");
    assert!(text
        .glyphs
        .iter()
        .all(|g| font.widths.iter().any(|(id, w)| *id == g.id && *w > 0)));
    let read: String = text
        .glyphs
        .iter()
        .filter_map(|g| font.unicode.iter().find(|(id, _)| *id == g.id))
        .map(|(_, s)| s.as_str())
        .collect();
    assert_eq!(
        read, "office",
        "every glyph, a ligature included, reads back"
    );
}

#[test]
fn covered_type_stays_in_the_image() {
    let mut doc = doc();
    doc.layers.push(Layer {
        name: "cover".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 10,
            right: 10,
        },
        ..doc.layers[0].clone()
    });
    assert!(pdf_live_type(&doc, OPEN).is_none());
}

#[test]
fn faded_type_needs_transparency() {
    let mut doc = doc();
    let index = doc.layers.iter().position(Layer::is_type).unwrap();
    doc.layers[index].opacity = 128;
    let closed = TextPolicy {
        transparency: false,
        cff: true,
    };
    assert!(pdf_live_type(&doc, closed).is_none());
    assert_eq!(pdf_live_type(&doc, OPEN).unwrap().texts[0].opacity, 128);
}

#[test]
fn a_blended_or_masked_type_layer_stays_in_the_image() {
    let mut doc = doc();
    let index = doc.layers.iter().position(Layer::is_type).unwrap();
    doc.layers[index].blend = BlendMode::Multiply;
    assert!(pdf_live_type(&doc, OPEN).is_none());
}

#[test]
fn type_fonts_lists_each_family_once() {
    let mut doc = doc();
    crate::add_type_layer(&mut doc, "1", &spec("again"));
    assert_eq!(pdf_type_fonts(&doc), vec!["Liberation Sans".to_string()]);
    let empty = Document::new(10, 10, ColorMode::Rgb, BitDepth::Eight);
    assert!(pdf_type_fonts(&empty).is_empty());
}
