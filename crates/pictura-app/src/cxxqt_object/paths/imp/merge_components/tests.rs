use super::*;
use crate::cxxqt_object::document_session::DocumentSession;
use pictura_core::{
    BitDepth, ColorMode, Document, PsdRect,
    path::{PathPoint, Subpath},
};

fn contour(anchors: &[(f64, f64)], closed: bool) -> Subpath {
    Subpath {
        closed,
        points: anchors
            .iter()
            .map(|&anchor| PathPoint {
                anchor,
                in_handle: None,
                out_handle: None,
                smooth: false,
            })
            .collect(),
    }
}

fn square(left: f64, top: f64) -> Subpath {
    contour(
        &[
            (left, top),
            (left + 20.0, top),
            (left + 20.0, top + 20.0),
            (left, top + 20.0),
        ],
        true,
    )
}

fn fixture(paths: &[Subpath]) -> (PictureViewRust, String) {
    let mut doc = Document::new(64, 64, ColorMode::Rgb, BitDepth::Eight);
    let key = pictura_render::add_shape_layer(
        &mut doc,
        "",
        [255, 0, 0, 255],
        "Shape",
        &paths[0],
        None,
    );
    let layer = pictura_render::resolve_path_mut(&mut doc, &key).unwrap();
    pictura_render::set_layer_shape_paths(layer, paths, 64, 64);
    let target = crate::layer_target::LayerTarget::for_opened(&doc, Some(key.clone()));
    let mut rust = PictureViewRust {
        session: DocumentSession::with_document(doc),
        path_on_layer: true,
        ..Default::default()
    };
    rust.set_layer_target(target);
    (rust, key)
}

fn mask(rust: &PictureViewRust, key: &str) -> VectorMask {
    pictura_render::resolve_path(rust.session.doc.as_ref().unwrap(), key)
        .unwrap()
        .vector_mask
        .clone()
        .unwrap()
}

#[test]
fn merge_curved_mask_preserves_rendered_geometry() {
    let mut curved = square(4.0, 8.0);
    curved.points[0].out_handle = Some((8.0, 4.0));
    curved.points[1].in_handle = Some((20.0, 4.0));
    let open = contour(&[(38.0, 11.0), (45.0, 15.0), (50.0, 11.0)], false);
    let (mut rust, key) = fixture(&[curved, square(14.0, 6.0), open.clone()]);
    let before = mask(&rust, &key);
    assert!(!matches!(apply_component_merge(&mut rust), Edit::None));
    let after = mask(&rust, &key);
    let layer = pictura_render::resolve_path(rust.session.doc.as_ref().unwrap(), &key).unwrap();
    assert_eq!(
        pictura_render::vector_mask_paths(layer, 64, 64).unwrap().last(),
        Some(&open),
    );

    let bounds = PsdRect { left: 0, top: 0, right: 64, bottom: 64 };
    for y in 0..64 {
        for x in 0..64 {
            let original = before.coverage(x, y, bounds);
            let merged = after.coverage(x, y, bounds);
            assert!(original.abs_diff(merged) <= 2, "({x}, {y}): {original} vs {merged}");
        }
    }
    for point in after.subpaths.iter().filter(|p| p.closed).flat_map(|p| &p.points) {
        assert!(before.subpaths.iter().filter(|p| p.closed).any(|p| {
            p.points.windows(2).any(|segment| distance(*point, segment[0], segment[1]) <= 1.0)
        }));
    }
}

fn distance(p: [i32; 2], a: [i32; 2], b: [i32; 2]) -> f64 {
    let d = [f64::from(b[0] - a[0]), f64::from(b[1] - a[1])];
    let v = [f64::from(p[0] - a[0]), f64::from(p[1] - a[1])];
    let squared_length = d[0] * d[0] + d[1] * d[1];
    if squared_length == 0.0 {
        return v[0].hypot(v[1]);
    }
    let t = ((v[0] * d[0] + v[1] * d[1]) / squared_length).clamp(0.0, 1.0);
    (v[0] - t * d[0]).hypot(v[1] - t * d[1])
}

fn assert_refused_on_redo_branch(mut rust: PictureViewRust, key: &str) {
    rust.apply_edit("First", Edit::Meta);
    rust.session.doc.as_mut().unwrap().work_path.append_corner(1.0, 1.0);
    rust.apply_edit("Second", Edit::Meta);
    let original = rust.session.history.undo().unwrap().doc;
    *rust.session.doc.as_mut().unwrap() = original.clone();
    let old_mask = mask(&rust, key);
    let result = apply_component_merge(&mut rust);
    assert_eq!(result, Edit::None);
    rust.apply_edit(LABEL, result);
    assert_eq!(rust.session.doc.as_ref(), Some(&original));
    assert_eq!(mask(&rust, key), old_mask);
    assert!(rust.session.history.redo().is_some());
}

#[test]
fn declines_unsafe_imported_mask_merge() {
    let (mut rust, key) = fixture(&[square(4.0, 4.0), square(12.0, 12.0)]);
    let layer = pictura_render::resolve_path_mut(rust.session.doc.as_mut().unwrap(), &key).unwrap();
    pictura_codec::edit_layer(layer, |parts| {
        let block = parts.extra_blocks.iter_mut().find(|block| &block.key == b"vmsk").unwrap();
        let mut bytes = block.data.to_vec();
        // Header (8 bytes), fill record (26), then selector and knot count.
        bytes[38..40].copy_from_slice(&2i16.to_be_bytes());
        block.data = bytes.into();
    });
    layer.vector_mask = pictura_codec::decode_vector_mask(&layer.extra_block(b"vmsk").unwrap().data, 64, 64);
    assert_refused_on_redo_branch(rust, &key);
}

#[test]
fn declines_permissive_empty_mask_merge() {
    let shape = square(4.0, 4.0);
    let (rust, key) = fixture(&[shape.clone(), shape]);
    assert_refused_on_redo_branch(rust, &key);
}

#[test]
fn declines_mask_round_trip_that_closes_a_gap() {
    let (rust, key) = fixture(&[square(0.0, 0.0), square(21.0, 0.0)]);
    let mut before = mask(&rust, &key);
    // Model pre- and post-encode geometry with a one-grid-unit gap that closes.
    // Redo preservation is covered by the separate refusal fixtures above.
    for point in &mut before.subpaths[1].points {
        point[0] -= 255;
    }
    let mut after = before.clone();
    for point in &mut after.subpaths[1].points {
        point[0] -= 1;
    }
    assert!(!pictura_select::mask_fills_match(&before.subpaths, &after.subpaths, VectorFillRule::EvenOdd));
}
