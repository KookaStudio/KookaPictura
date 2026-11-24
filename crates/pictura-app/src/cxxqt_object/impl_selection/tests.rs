//! Save/Load Selection bridge tests.

use super::*;
use crate::cxxqt_object::document_session::DocumentSession;
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::state::PictureViewRust;
use pictura_core::{BitDepth, Channel, ColorMode, Document, LayerMask};

#[test]
fn save_selection_notifies() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.composite
        .replace(pictura_core::PixelBuffer {
            width: 4,
            height: 4,
            channels: 4,
            data: vec![0u8; 4 * 4 * 4].into(),
        })
        .expect("valid composite");
    let mut rust = PictureViewRust {
        session: DocumentSession::with_document(doc),
        ..Default::default()
    };
    rust.session.selection = Some(Selection::all(4, 4));
    let before = rust
        .session
        .doc
        .as_ref()
        .expect("document")
        .composite
        .buffer()
        .clone();
    let outcome = save_selection_shape(
        &mut rust,
        Selection::all(4, 4),
        SelectionTarget::NewChannel,
        "",
        CombineMode::New,
    );
    let applied = rust.apply_edit("Save Selection", outcome);
    assert_eq!(applied.declaration, Some(Edit::Meta));
    assert!(applied.notify);
    assert_eq!(rust.session.history.count(), 1);
    assert_eq!(
        rust.session.doc.as_ref().expect("document").channels.len(),
        1
    );
    assert_eq!(
        rust.session.doc.as_ref().expect("document").composite,
        before
    );
}

fn channel(id: i16, len: usize) -> Channel {
    Channel {
        id,
        data: vec![7; len].into(),
    }
}

#[test]
fn channel_name_maps_positionally() {
    assert_eq!(parse_channel_name("Alpha 1"), Some(0));
    assert_eq!(parse_channel_name("Alpha 3"), Some(2));
    assert_eq!(parse_channel_name("Alpha 0"), None);
    assert_eq!(parse_channel_name("alpha 1"), None);
    assert_eq!(parse_channel_name("Alpha x"), None);
    assert_eq!(parse_channel_name(""), None);
}

#[test]
fn wrong_length_source_refused() {
    let short = channel(1, 3);
    assert!(Selection::from_channel(&short, 2, 2).is_err());
    assert!(Selection::from_channel(&short, 1, 3).is_ok());
    assert!(coverage_selection(&[0; 3], 2, 2).is_none());
    assert!(coverage_selection(&[0; 6], 2, 3).is_some());
}

#[test]
fn gray_and_ink_match_the_panel_derivation() {
    assert_eq!(q_gray(100, 150, 200), 140);
    assert_eq!(q_gray(0, 150, 200), 106);
    assert_eq!(q_gray(255, 255, 255), 255);
    // Pure red: no cyan or black ink, full magenta and yellow.
    assert_eq!(cmyk_ink(255, 0, 0, 0), 0);
    assert_eq!(cmyk_ink(255, 0, 0, 1), 255);
    assert_eq!(cmyk_ink(0, 0, 0, 3), 255);
}

#[test]
fn channel_row_selection_reads_the_row_plane() {
    let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
    // Planes 0/1/2 are R/G/B.
    doc.composite
        .replace_data(vec![100, 100, 150, 150, 200, 200].into())
        .expect("valid composite plane");

    let red = channel_row_selection(&doc, doc.composite.buffer(), 1, 0x7).unwrap();
    assert_eq!(red.data, vec![100, 100]);
    let green = channel_row_selection(&doc, doc.composite.buffer(), 2, 0x7).unwrap();
    assert_eq!(green.data, vec![150, 150]);
    let blue = channel_row_selection(&doc, doc.composite.buffer(), 3, 0x7).unwrap();
    assert_eq!(blue.data, vec![200, 200]);

    let composite = channel_row_selection(&doc, doc.composite.buffer(), 0, 0x7).unwrap();
    assert_eq!(composite.data, vec![140, 140]);

    // A hidden red channel is zeroed in the composite row.
    let hidden = channel_row_selection(&doc, doc.composite.buffer(), 0, 0x6).unwrap();
    assert_eq!(hidden.data, vec![106, 106]);

    // Rows past the colour channels are refused.
    assert!(channel_row_selection(&doc, doc.composite.buffer(), 4, 0x7).is_none());
    assert!(channel_row_selection(&doc, doc.composite.buffer(), -1, 0x7).is_none());
}

#[test]
fn multichannel_rows_derive_naive_inks() {
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.source
        .set_encoding(Some(ColorMode::Multichannel), None)
        .expect("valid source encoding");
    doc.composite
        .replace_data(vec![255, 0, 0].into())
        .expect("valid composite plane");
    // Row 0 is Cyan (ink 0), rows 1/2 Magenta and Yellow (full ink) — and
    // there is no composite row in Multichannel.
    assert_eq!(
        channel_row_selection(&doc, doc.composite.buffer(), 0, 0x7)
            .unwrap()
            .data,
        vec![255]
    );
    assert_eq!(
        channel_row_selection(&doc, doc.composite.buffer(), 1, 0x7)
            .unwrap()
            .data,
        vec![0]
    );
    assert_eq!(
        channel_row_selection(&doc, doc.composite.buffer(), 2, 0x7)
            .unwrap()
            .data,
        vec![0]
    );
    assert!(channel_row_selection(&doc, doc.composite.buffer(), 3, 0x7).is_none());
}

#[test]
fn single_row_modes_have_no_second_channel() {
    let mut doc = Document::new(1, 1, ColorMode::Grayscale, BitDepth::Eight);
    doc.composite
        .replace_data(vec![200].into())
        .expect("valid composite plane");
    // The one Gray row is the composite; there is no colour channel after it.
    assert_eq!(
        channel_row_selection(&doc, doc.composite.buffer(), 0, 0x7)
            .unwrap()
            .data,
        vec![200]
    );
    assert!(channel_row_selection(&doc, doc.composite.buffer(), 1, 0x7).is_none());
}

fn test_doc(w: u32, h: u32) -> Document {
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.composite
        .replace(pictura_core::PixelBuffer {
            width: w,
            height: h,
            channels: 4,
            data: vec![0u8; w as usize * h as usize * 4].into(),
        })
        .expect("valid composite");
    doc
}

fn test_rust(doc: Document) -> PictureViewRust {
    PictureViewRust {
        session: DocumentSession::with_document(doc),
        ..Default::default()
    }
}

fn selection(value: u8, w: u32, h: u32) -> Selection {
    Selection {
        width: w,
        height: h,
        data: vec![value; w as usize * h as usize].into(),
    }
}

fn channel_value(value: u8, len: usize) -> Channel {
    Channel {
        id: 1,
        data: vec![value; len].into(),
    }
}

fn full(w: u32, h: u32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: w as i32,
    }
}

fn mask_with(value: u8, w: u32, h: u32) -> LayerMask {
    LayerMask {
        rect: full(w, h),
        default_color: 0,
        disabled: true,
        flags: 1,
        density: 128,
        feather_bits: 2.0f64.to_bits(),
        data: Some(vec![value; w as usize * h as usize].into()),
        ..Default::default()
    }
}

fn layer_with_mask(doc: &mut Document, mask: LayerMask) {
    let _ = doc
        .layers
        .push(pictura_codec::layer_from_parts(pictura_core::LayerParts {
            name: "M".to_string(),
            rect: full(doc.width, doc.height),
            mask: Some(mask),
            ..Default::default()
        }));
}

fn layer_with_mask_rect(doc: &mut Document, rect: PsdRect, mask: LayerMask) {
    let _ = doc
        .layers
        .push(pictura_codec::layer_from_parts(pictura_core::LayerParts {
            name: "M".to_string(),
            rect,
            mask: Some(mask),
            ..Default::default()
        }));
}

#[test]
fn save_selection_replaces_existing_channel() {
    let mut doc = test_doc(2, 2);
    doc.channels = vec![channel_value(7, 4)];
    let mut rust = test_rust(doc);
    let outcome = save_selection_shape(
        &mut rust,
        selection(200, 2, 2),
        SelectionTarget::Channel(0),
        "",
        CombineMode::New,
    );
    assert!(matches!(outcome, Edit::Meta));
    assert_eq!(
        rust.session.doc.as_ref().expect("doc").channels[0]
            .data
            .to_vec(),
        vec![200; 4]
    );
}

#[test]
fn save_selection_combines_existing_channel() {
    let expected = [
        (CombineMode::Add, 128u8),
        (CombineMode::Subtract, 64),
        (CombineMode::Intersect, 32),
    ];
    for (mode, value) in expected {
        let mut doc = test_doc(2, 2);
        doc.channels = vec![channel_value(128, 4)];
        let mut rust = test_rust(doc);
        let outcome = save_selection_shape(
            &mut rust,
            selection(64, 2, 2),
            SelectionTarget::Channel(0),
            "",
            mode,
        );
        assert!(matches!(outcome, Edit::Meta));
        assert_eq!(
            rust.session.doc.as_ref().expect("doc").channels[0]
                .data
                .to_vec(),
            vec![value; 4],
            "{mode:?}"
        );
    }
}

#[test]
fn save_selection_into_channel_one_state() {
    let mut doc = test_doc(2, 2);
    doc.channels = vec![channel_value(64, 4)];
    let mut rust = test_rust(doc);
    let outcome = save_selection_shape(
        &mut rust,
        selection(64, 2, 2),
        SelectionTarget::Channel(0),
        "",
        CombineMode::Add,
    );
    let applied = rust.apply_edit("Save Selection", outcome);
    assert_eq!(applied.declaration, Some(Edit::Meta));
    assert_eq!(rust.session.history.count(), 1);
}

#[test]
fn load_selection_inverts_before_combining() {
    let mut doc = test_doc(2, 2);
    doc.channels = vec![channel_value(64, 4)];
    let mut rust = test_rust(doc);
    rust.session.selection = Some(selection(128, 2, 2));
    let shape =
        selection_from_target(rust.session.doc.as_ref().expect("doc"), "alpha:0").expect("source");
    combine_selection(&mut rust, shape.invert(), CombineMode::Add, false);
    assert_eq!(
        rust.session.selection.expect("selection").data.to_vec(),
        vec![191; 4]
    );
}

#[test]
fn load_selection_without_existing_uses_op() {
    let cases = [
        (CombineMode::Add, 200u8),
        (CombineMode::Subtract, 0),
        (CombineMode::Intersect, 0),
    ];
    for (mode, expected) in cases {
        let mut rust = test_rust(test_doc(2, 2));
        combine_selection(&mut rust, selection(200, 2, 2), mode, false);
        assert_eq!(
            rust.session.selection.expect("selection").data.to_vec(),
            vec![expected; 4],
            "{mode:?}"
        );
    }
}

#[test]
fn load_selection_one_state() {
    let mut rust = test_rust(test_doc(2, 2));
    combine_selection(&mut rust, selection(200, 2, 2), CombineMode::Add, false);
    rust.apply_edit("Load Selection", Edit::Meta);
    assert_eq!(rust.session.history.count(), 1);
}

#[test]
fn save_selection_into_mask_replaces_and_preserves_metadata() {
    let mut doc = test_doc(2, 2);
    layer_with_mask(&mut doc, mask_with(0, 2, 2));
    let mut rust = test_rust(doc);
    let outcome = save_selection_shape(
        &mut rust,
        selection(255, 2, 2),
        SelectionTarget::Mask("0".to_string()),
        "",
        CombineMode::New,
    );
    assert!(matches!(outcome, Edit::Regions { .. } | Edit::Full));
    let doc = rust.session.doc.as_ref().expect("doc");
    let mask = doc.layers[0].mask.as_ref().expect("mask");
    assert_eq!(mask.data.as_ref().expect("data").as_slice(), &[255; 4]);
    assert_eq!(mask.density, 128);
    assert_eq!(mask.feather(), 2.0);
    assert!(mask.disabled);
    assert_eq!(mask.flags, 1);
    assert_eq!(mask.rect, full(2, 2));
}

#[test]
fn save_selection_into_mask_combines() {
    let cases = [
        (CombineMode::Add, 128u8),
        (CombineMode::Subtract, 64),
        (CombineMode::Intersect, 32),
    ];
    for (mode, expected) in cases {
        let mut doc = test_doc(2, 2);
        layer_with_mask(&mut doc, mask_with(128, 2, 2));
        let mut rust = test_rust(doc);
        let outcome = save_selection_shape(
            &mut rust,
            selection(64, 2, 2),
            SelectionTarget::Mask("0".to_string()),
            "",
            mode,
        );
        assert!(!matches!(outcome, Edit::None), "{mode:?}");
        let mask = rust.session.doc.as_ref().expect("doc").layers[0]
            .mask
            .as_ref()
            .expect("mask");
        assert_eq!(
            mask.data.as_ref().expect("data").as_slice(),
            vec![expected; 4].as_slice(),
            "{mode:?}"
        );
    }
}

#[test]
fn save_selection_into_mask_one_state() {
    let mut doc = test_doc(2, 2);
    layer_with_mask(&mut doc, mask_with(0, 2, 2));
    let mut rust = test_rust(doc);
    let outcome = save_selection_shape(
        &mut rust,
        selection(255, 2, 2),
        SelectionTarget::Mask("0".to_string()),
        "",
        CombineMode::New,
    );
    rust.apply_edit("Save Selection", outcome);
    assert_eq!(rust.session.history.count(), 1);
}

#[test]
fn mask_save_history_captures_tiles_outside_the_layer() {
    let mut doc = test_doc(256, 256);
    layer_with_mask_rect(
        &mut doc,
        PsdRect {
            top: 0,
            left: 0,
            bottom: 64,
            right: 64,
        },
        mask_with(0, 256, 256),
    );
    let mut rust = test_rust(doc);
    {
        let doc = rust.session.doc.as_mut().expect("doc");
        rust.session
            .history
            .capture_live(doc, &None, &Default::default(), "Base");
    }
    let outcome = save_selection_shape(
        &mut rust,
        selection(255, 256, 256),
        SelectionTarget::Mask("0".to_string()),
        "",
        CombineMode::New,
    );
    assert!(!matches!(outcome, Edit::None));
    rust.apply_edit("Save Selection", outcome);
    let saved = rust.session.history.jump(1).expect("saved state");
    let mask = saved.doc.layers[0].mask.as_ref().expect("mask");
    assert_eq!(
        mask.data.as_ref().expect("data").as_slice()[200 * 256 + 200],
        255,
        "the saved state keeps mask changes outside the layer influence"
    );
    let base = rust.session.history.jump(0).expect("base state");
    let mask = base.doc.layers[0].mask.as_ref().expect("mask");
    assert_eq!(
        mask.data.as_ref().expect("data").as_slice()[200 * 256 + 200],
        0
    );
}

#[test]
fn save_selection_into_missing_mask_refuses() {
    let mut rust = test_rust(test_doc(2, 2));
    let outcome = save_selection_shape(
        &mut rust,
        selection(255, 2, 2),
        SelectionTarget::Mask("9".to_string()),
        "",
        CombineMode::New,
    );
    assert!(matches!(outcome, Edit::None));
}

#[test]
fn save_selection_refuses_channel_57() {
    let mut doc = test_doc(1, 1);
    doc.channels = (0..53).map(|id| channel(id, 1)).collect();
    let mut rust = test_rust(doc);
    let outcome = save_selection_shape(
        &mut rust,
        selection(255, 1, 1),
        SelectionTarget::NewChannel,
        "",
        CombineMode::New,
    );
    assert!(matches!(outcome, Edit::None));
}

#[test]
fn save_then_load_round_trip() {
    let mut rust = test_rust(test_doc(2, 2));
    let outcome = save_selection_shape(
        &mut rust,
        selection(123, 2, 2),
        SelectionTarget::NewChannel,
        "Hair",
        CombineMode::New,
    );
    rust.apply_edit("Save Selection", outcome);
    let doc = rust.session.doc.as_ref().expect("doc");
    let loaded = selection_from_target(doc, "alpha:0").expect("loaded");
    assert_eq!(loaded.data.to_vec(), vec![123; 4]);
    assert_eq!(pictura_codec::alpha_names(doc), vec!["Hair".to_string()]);
}

#[test]
fn selected_channel_stores_protection() {
    let mut doc = test_doc(2, 2);
    doc.channels = vec![channel_value(0, 4)];
    assert!(pictura_codec::set_channel_selected_areas(&mut doc, 0, true));
    let mut rust = test_rust(doc);
    let outcome = save_selection_shape(
        &mut rust,
        selection(200, 2, 2),
        SelectionTarget::Channel(0),
        "",
        CombineMode::New,
    );
    assert!(matches!(outcome, Edit::Meta));
    let doc = rust.session.doc.as_ref().expect("doc");
    assert_eq!(doc.channels[0].data.to_vec(), vec![55; 4]);
    assert_eq!(
        selection_from_target(doc, "alpha:0")
            .expect("load")
            .data
            .to_vec(),
        vec![200; 4]
    );
}

#[test]
fn same_plane_reads_opposite_under_each_sense() {
    let mut doc = test_doc(2, 2);
    doc.channels = vec![channel_value(64, 4), channel_value(64, 4)];
    assert!(pictura_codec::set_channel_selected_areas(&mut doc, 1, true));
    assert!(!pictura_codec::channel_selected_areas(&doc, 0));
    assert_eq!(
        selection_from_target(&doc, "alpha:0")
            .expect("masked")
            .data
            .to_vec(),
        vec![64; 4]
    );
    assert_eq!(
        selection_from_target(&doc, "alpha:1")
            .expect("selected")
            .data
            .to_vec(),
        vec![191; 4]
    );
}

#[test]
fn save_into_selected_channel_writes_protection() {
    let mut doc = test_doc(2, 2);
    doc.channels = vec![channel_value(0, 4)];
    assert!(pictura_codec::set_channel_selected_areas(&mut doc, 0, true));
    let mut rust = test_rust(doc);
    let outcome = save_selection_shape(
        &mut rust,
        selection(64, 2, 2),
        SelectionTarget::Channel(0),
        "",
        CombineMode::New,
    );
    assert!(matches!(outcome, Edit::Meta));
    let doc = rust.session.doc.as_ref().expect("doc");
    assert_eq!(doc.channels[0].data.to_vec(), vec![191; 4]);
    assert_eq!(
        selection_from_target(doc, "alpha:0")
            .expect("load")
            .data
            .to_vec(),
        vec![64; 4]
    );
}

#[test]
fn combine_under_selected_channel_uses_canonical_coverage() {
    let mut doc = test_doc(2, 2);
    doc.channels = vec![channel_value(127, 4)];
    assert!(pictura_codec::set_channel_selected_areas(&mut doc, 0, true));
    let mut rust = test_rust(doc);
    let outcome = save_selection_shape(
        &mut rust,
        selection(64, 2, 2),
        SelectionTarget::Channel(0),
        "",
        CombineMode::Add,
    );
    let applied = rust.apply_edit("Save Selection", outcome);
    assert_eq!(applied.declaration, Some(Edit::Meta));
    assert_eq!(rust.session.history.count(), 1);
    let doc = rust.session.doc.as_ref().expect("doc");
    assert_eq!(doc.channels[0].data.to_vec(), vec![127; 4]);
    assert_eq!(
        selection_from_target(doc, "alpha:0")
            .expect("load")
            .data
            .to_vec(),
        vec![128; 4]
    );
}

#[test]
fn load_selected_channel_applies_sense_before_invert() {
    let mut doc = test_doc(2, 2);
    doc.channels = vec![channel_value(64, 4)];
    assert!(pictura_codec::set_channel_selected_areas(&mut doc, 0, true));
    let shape = selection_from_target(&doc, "alpha:0").expect("source");
    assert_eq!(shape.data.to_vec(), vec![191; 4]);
    assert_eq!(shape.invert().data.to_vec(), vec![64; 4]);
}

#[test]
fn load_selected_channel_is_sense_aware() {
    let mut doc = test_doc(2, 2);
    doc.channels = vec![channel_value(64, 4)];
    assert!(pictura_codec::set_channel_selected_areas(&mut doc, 0, true));
    let shape = selection_from_target(&doc, "alpha:0").expect("source");
    let mut rust = test_rust(doc);
    combine_selection(&mut rust, shape, CombineMode::New, false);
    rust.apply_edit("Load Selection", Edit::Meta);
    assert_eq!(rust.session.history.count(), 1);
    assert_eq!(
        rust.session.selection.expect("selection").data.to_vec(),
        vec![191; 4]
    );
}

#[test]
fn sense_toggle_preserves_the_selection() {
    let mut doc = test_doc(2, 2);
    doc.channels = vec![Channel {
        id: 0,
        data: vec![0, 64, 127, 255].into(),
    }];
    let before = selection_from_target(&doc, "alpha:0").expect("before");
    let outcome = crate::cxxqt_object::channels::toggle_channel_sense(&mut doc, 0, true);
    assert_eq!(outcome, Edit::Meta);
    assert!(pictura_codec::channel_selected_areas(&doc, 0));
    assert_eq!(doc.channels[0].data.to_vec(), vec![255, 191, 128, 0]);
    let after = selection_from_target(&doc, "alpha:0").expect("after");
    assert_eq!(after.data.to_vec(), before.data.to_vec());

    let mut rust = test_rust(doc);
    rust.apply_edit("Channel Options", outcome);
    assert_eq!(rust.session.history.count(), 1);
}

#[test]
fn selected_channel_soft_coverage_is_not_quantized() {
    let mut doc = test_doc(2, 2);
    doc.channels = vec![channel_value(127, 4)];
    assert!(pictura_codec::set_channel_selected_areas(&mut doc, 0, true));
    assert_eq!(
        selection_from_target(&doc, "alpha:0")
            .expect("load")
            .data
            .to_vec(),
        vec![128; 4]
    );
}
