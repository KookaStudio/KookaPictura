use super::*;
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::tests::pixel_layer;
use pictura_core::memory::eviction::{self, run_pass};
use pictura_core::scratch::{self, Fault};
use pictura_core::{BitDepth, ColorMode, Document, LayerStack, Residency};

/// A document whose one layer holds noisy, dense tiles.
pub(in crate::cxxqt_object) fn dense_doc() -> Document {
    let (w, h) = (600u32, 400u32);
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel_layer("dense", w, h, (0, 0, 0));
    let mut s = 0x2545_f491_4f6c_dd1du64;
    for channel in &mut layer.channels {
        assert!(channel.data.tile(w as usize, h as usize));
        let mut row = vec![0u8; w as usize];
        for y in 0..h as usize {
            for v in &mut row {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                *v = s as u8;
            }
            channel.data.write_row_from(y, w as usize, &row);
        }
        channel.data.settle();
    }
    doc.layers = LayerStack::from_layers(vec![layer]);
    doc
}

/// Spill every tile of the process to a fresh volume, then read `doc`'s
/// layer with reads failing, so its tiles become unreadable.
pub(in crate::cxxqt_object) fn lose_tiles(doc: &Document) {
    let dir = scratch::default_volume()
        .with_file_name("test-scratch")
        .join(format!("app-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    scratch::set_volumes(&[dir]);
    for _ in 0..4 {
        run_pass(u64::MAX);
    }
    let plane = &doc.layers[0].channels[0].data;
    assert!((0..plane.tile_count()).any(|i| plane.tile_residency(i) == Some(Residency::Spilled)));
    scratch::inject(Fault::ReadError);
    let mut row = vec![0u8; doc.width as usize];
    plane.read_row_into(0, doc.width as usize, &mut row);
    scratch::clear_faults();
    assert!(plane.unreadable_tiles() > 0);
}

#[test]
fn an_edit_on_unreadable_tiles_changes_nothing() {
    let mut view = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(dense_doc()),
        ..Default::default()
    };
    view.apply_edit("Setup", Edit::Full);
    let states = view.session.history.count();
    let before = view.session.doc.clone().expect("document");
    lose_tiles(&before);
    memory_gate::clear();
    // The "filter": invert the first rows of the layer, reading lost tiles.
    {
        let doc = view.session.doc.as_mut().expect("document");
        let w = doc.width as usize;
        let plane = &mut doc.layers[0].channels[0].data;
        let mut row = vec![0u8; w];
        for y in 0..8 {
            plane.read_row_into(y, w, &mut row);
            row.iter_mut().for_each(|v| *v = 255 - *v);
            plane.write_row_from(y, w, &row);
        }
    }
    view.apply_edit("Invert", Edit::Full);
    assert!(memory_gate::edit_refused_for_scratch());
    assert_eq!(view.session.history.count(), states, "no state was added");
    let after = view.session.doc.as_ref().expect("document");
    assert!(
        after.layers[0].channels[0]
            .data
            .shares(&before.layers[0].channels[0].data),
        "the document is the state before the edit"
    );
    assert!(eviction::unreadable_count() > 0);
}
