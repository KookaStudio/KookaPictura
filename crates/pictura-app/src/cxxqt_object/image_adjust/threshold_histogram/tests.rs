use super::*;
use crate::cxxqt_object::{document_session::DocumentSession, tests::pixel_layer};
use crate::layer_target::LayerTarget;
use pictura_core::{BitDepth, ColorMode, LayerMask};
use pictura_select::Selection;

#[test]
fn threshold_histogram_weights_editable_masks() {
    let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel_layer("base", 2, 1, (50, 60, 70));
    layer.mask = Some(LayerMask {
        rect: layer.rect,
        data: Some(vec![20, 200].into()),
        ..LayerMask::default()
    });
    doc.layers.push(layer).unwrap();
    let mut rust = PictureViewRust {
        session: DocumentSession::with_document(doc),
        layer_target: LayerTarget::only("0"),
        mask_edit_target: Some("0".into()),
        ..Default::default()
    };
    rust.session.set_selection(Some(Selection {
        width: 2,
        height: 1,
        data: vec![64, 255].into(),
    }));

    let mut expected = [0; 256];
    expected[20] = 64;
    expected[200] = 255;
    assert_eq!(threshold_bins(&rust, ""), expected);
    rust.session.quick_mask.active = true;
    rust.session.quick_mask.coverage = Some(Selection {
        width: 2,
        height: 1,
        data: vec![20, 200].into(),
    });
    assert_eq!(threshold_bins(&rust, ""), expected);
    rust.session.quick_mask.selected = true;
    expected = [0; 256];
    expected[235] = 64;
    expected[55] = 255;
    assert_eq!(threshold_bins(&rust, ""), expected);
}

#[test]
fn threshold_histogram_samples_small_selection_on_large_source() {
    let (w, h) = (1024, 768);
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel_layer("base", w, h, (21, 21, 21));
    for channel in layer.channels_mut() {
        if channel.id >= 0 {
            channel
                .data
                .write_rect_from(907..909, 738..740, w as usize, &[155; 4]);
        }
    }
    doc.layers.push(layer).unwrap();
    let mut rust = PictureViewRust {
        session: DocumentSession::with_document(doc),
        layer_target: LayerTarget::only("0"),
        ..Default::default()
    };
    rust.session
        .set_selection(Some(Selection::rect(w, h, 907, 738, 2, 2)));
    let mut expected = [0; 256];
    expected[155] = 4 * 255;
    assert_eq!(threshold_bins(&rust, ""), expected);
    assert_eq!(threshold_bins(&rust, ""), expected);
    assert_eq!(rust.selection_bounds_computes.get(), 1);

    let full = PsdRect {
        left: 0,
        top: 0,
        right: w as i32,
        bottom: h as i32,
    };
    let sampled: i32 = sample_tiles(full)
        .map(|tile| tile.width() * tile.height())
        .sum();
    assert_eq!(sampled, 512 * 512);
}
