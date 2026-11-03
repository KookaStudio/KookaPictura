//! One type layer edit on a 16507×16196 document: the whole-document path
//! against the text's reach, and undoing it across a history segment (#1135).

use super::super::document_session::DocumentSession;
use super::super::edit::Edit;
use super::{ms, pixel_layer, LayerTarget, PictureViewRust};
use crate::history::Restored;
use pictura_core::{BitDepth, ColorMode, Document, LayerStack, PsdRect, TypeSpec};

const WIDTH: u32 = 16507;
const HEIGHT: u32 = 16196;

fn view_with_text(spec: &TypeSpec) -> PictureViewRust {
    let mut doc = Document::new(WIDTH, HEIGHT, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer(
        "background",
        WIDTH,
        HEIGHT,
        (120, 170, 220),
    )]);
    assert!(!pictura_render::add_type_layer(&mut doc, "0", spec).is_empty());
    doc.composite = pictura_render::composite_rgba(&doc);
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    view.reset_pyramid();
    view
}

fn reach(view: &PictureViewRust) -> Option<PsdRect> {
    pictura_render::damage::ink_influence(view.session.doc.as_ref()?, "1")
}

fn edit_once(label: &str, before: &TypeSpec, after: &TypeSpec, bounded: bool) {
    let mut view = view_with_text(before);
    view.apply_edit("Open", Edit::Record);
    let old = reach(&view);
    let doc = view.session.doc.as_mut().expect("document");
    let t = std::time::Instant::now();
    assert!(pictura_render::replace_type_layer(doc, "1", after));
    ms(&format!("{label} replace"), t.elapsed());
    let edit = if bounded {
        let rect = pictura_render::damage::union(old, reach(&view)).expect("bounded");
        Edit::regions_hinted(vec![rect], Vec::new(), true)
    } else {
        Edit::Structural { damage: None }
    };
    let t = std::time::Instant::now();
    view.apply_edit("Edit Type Layer", edit);
    ms(
        &format!("{label} {}", if bounded { "bounded" } else { "whole" }),
        t.elapsed(),
    );
    let t = std::time::Instant::now();
    let session = &mut view.session;
    let doc = session.doc.as_mut().expect("document");
    let restored = session
        .history
        .undo_live(doc, &mut session.selection, &mut LayerTarget::default())
        .expect("a state to undo");
    match restored {
        Restored::Region(rect) | Restored::Composite(rect) => {
            view.refresh_from_composite(rect);
        }
        Restored::Everywhere => view.reset_pyramid(),
    }
    ms(
        &format!(
            "{label} undo, {}",
            if bounded { "bounded" } else { "whole" }
        ),
        t.elapsed(),
    );
}

#[test]
#[ignore = "16507x16196 type edit profile; run explicitly with --ignored --nocapture"]
fn type_edit_profile() {
    let base = TypeSpec {
        origin: (6000.0, 8000.0),
        ..TypeSpec::new("Lorem Ipsum Pictura", "Liberation Sans", 288.0)
    };
    let mut recoloured = base.clone();
    recoloured.character.fill_color = [1.0, 0.8, 0.1, 0.1];
    let resized = TypeSpec {
        character: pictura_core::CharacterAttrs {
            size: 320.0,
            ..base.character.clone()
        },
        ..base.clone()
    };
    for bounded in [false, true] {
        edit_once("colour", &base, &recoloured, bounded);
        edit_once("size", &base, &resized, bounded);
    }
    let boxed = TypeSpec {
        origin: (100.0, 100.0),
        box_bounds: Some([0.0, 0.0, 16000.0, 15800.0]),
        ..base.clone()
    };
    let mut boxed_recoloured = boxed.clone();
    boxed_recoloured.character.fill_color = recoloured.character.fill_color;
    for bounded in [false, true] {
        edit_once("huge box", &boxed, &boxed_recoloured, bounded);
    }
}
