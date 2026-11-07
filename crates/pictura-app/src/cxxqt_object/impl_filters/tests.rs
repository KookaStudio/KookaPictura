//! Tests for the destructive filter/adjustment application in `impl_filters`.

use super::*;
use crate::cxxqt_object::tests::pixel_layer;
use pictura_core::{Document, LockFlags};

fn state() -> PictureViewRust {
    let mut doc = Document::new(
        16,
        16,
        pictura_core::ColorMode::Rgb,
        pictura_core::BitDepth::Eight,
    );
    let mut layer = pixel_layer("px", 16, 16, (40, 120, 200));
    // A left/right step so a blur has a gradient to smooth.
    for ch in layer.channels.iter_mut().filter(|c| c.id >= 0) {
        ch.data = (0..16 * 16)
            .map(|i| if i % 16 < 8 { 40u8 } else { 200 })
            .collect();
    }
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        layer_target: LayerTarget::only("0"),
        gpu_compute: false,
        ..Default::default()
    }
}

fn red(rust: &PictureViewRust) -> Vec<u8> {
    rust.session.doc.as_ref().unwrap().layers[0]
        .channels
        .iter()
        .find(|c| c.id == 0)
        .unwrap()
        .data
        .to_vec()
}

/// A 256×256 layer large enough that a section preview's apron does not
/// span the whole layer, so a crop changes a positional filter's result.
fn big_state() -> PictureViewRust {
    let mut doc = Document::new(
        256,
        256,
        pictura_core::ColorMode::Rgb,
        pictura_core::BitDepth::Eight,
    );
    let mut layer = pixel_layer("px", 256, 256, (40, 120, 200));
    for (c, ch) in layer.channels.iter_mut().filter(|c| c.id >= 0).enumerate() {
        let plane: Vec<u8> = (0..256 * 256)
            .map(|i| {
                let x = i % 256;
                let y = i / 256;
                ((x * 7 + y * 13 + c * 53) % 256) as u8
            })
            .collect();
        ch.data = plane.into();
    }
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        layer_target: LayerTarget::only("0"),
        gpu_compute: false,
        ..Default::default()
    }
}

#[test]
fn blur_grows_the_layer() {
    let mut doc = Document::new(
        32,
        32,
        pictura_core::ColorMode::Rgb,
        pictura_core::BitDepth::Eight,
    );
    let mut layer = pixel_layer("px", 12, 12, (40, 120, 200));
    layer.rect = pictura_core::PsdRect {
        top: 10,
        left: 10,
        bottom: 22,
        right: 22,
    };
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    let mut rust = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        layer_target: LayerTarget::only("0"),
        gpu_compute: false,
        ..Default::default()
    };
    assert!(apply_filter_active(&mut rust, "gaussian-blur", &[2.0], true).is_some());
    let layer = &rust.session.doc.as_ref().unwrap().layers[0];
    assert!(
        layer.rect.width() > 12 && layer.rect.height() > 12,
        "the blur grew the layer by its apron: {:?}",
        layer.rect
    );
    let alpha = layer.channels.iter().find(|c| c.id == -1).unwrap();
    let w = layer.rect.width() as usize;
    let at = |x: i32, y: i32| {
        alpha.data[(y - layer.rect.top) as usize * w + (x - layer.rect.left) as usize]
    };
    assert!(at(12, 9) > 0, "the blurred edge reaches into the padding");
}

#[test]
fn wrong_arity_is_refused_without_mutation() {
    let mut rust = state();
    let before = red(&rust);
    assert!(apply_filter_active(&mut rust, "gaussian-blur", &[1.0, 2.0], false).is_none());
    assert!(apply_filter_active(&mut rust, "bogus", &[], false).is_none());
    assert_eq!(red(&rust), before, "refusal must not mutate the layer");
    assert!(rust.session.live.filter_preview().is_none());
    assert!(rust.last_filter.is_none());
    assert!(
        rust.filter_error.as_deref().is_some_and(|e| !e.is_empty()),
        "a refusal leaves a reason"
    );
}

#[test]
fn success_clears_the_refusal_reason() {
    let mut rust = state();
    assert!(apply_filter_active(&mut rust, "bogus", &[], false).is_none());
    assert!(rust.filter_error.is_some());
    assert!(apply_filter_active(&mut rust, "gaussian-blur", &[2.0], false).is_some());
    assert!(rust.filter_error.is_none(), "success clears the reason");
}

#[test]
fn locked_layer_is_refused() {
    let mut rust = state();
    rust.session.doc.as_mut().unwrap().layers[0].lock =
        LockFlags::default().with(LockFlags::PIXELS, true);
    let before = red(&rust);
    assert!(apply_filter_active(&mut rust, "gaussian-blur", &[2.0], false).is_none());
    assert_eq!(red(&rust), before);
}

#[test]
fn non_pixel_active_layer_is_refused() {
    let mut rust = state();
    let pixel_before = red(&rust);
    // An adjustment layer (no pixels) as the sole active target.
    let adj = adjustment_layer("invert", None).expect("adjustment");
    {
        let _ = rust.session.doc.as_mut().unwrap().layers.push(adj);
    };
    rust.select_only_layer("1");
    assert!(apply_filter_active(&mut rust, "gaussian-blur", &[2.0], true).is_none());
    assert_eq!(red(&rust), pixel_before, "pixel layer untouched");
    assert!(rust.last_filter.is_none());
}

#[test]
fn preview_on_another_layer_is_refused_and_keeps_baseline() {
    let mut rust = state();
    let _ =
        rust.session
            .doc
            .as_mut()
            .unwrap()
            .layers
            .push(pixel_layer("px2", 16, 16, (10, 20, 30)));
    let original = red(&rust);
    assert!(apply_filter_active(&mut rust, "gaussian-blur", &[4.0], false).is_some());
    assert!(rust
        .session
        .live
        .filter_preview()
        .is_some_and(|p| p.layer_path == "0"));
    // Switching the active layer must not steal layer 0's baseline.
    rust.select_only_layer("1");
    assert!(apply_filter_active(&mut rust, "gaussian-blur", &[4.0], false).is_none());
    assert!(rust
        .session
        .live
        .filter_preview()
        .is_some_and(|p| p.layer_path == "0"));
    assert!(cancel_filter_preview(&mut rust).is_some());
    assert_eq!(red(&rust), original, "layer 0 baseline restored");
}

#[test]
fn preview_re_filters_the_original_not_the_last_preview() {
    let mut rust = state();
    let original = red(&rust);
    assert!(apply_filter_active(&mut rust, "gaussian-blur", &[2.0], false).is_some());
    assert!(
        rust.session.live.filter_preview().is_some(),
        "preview state established"
    );
    assert!(rust.last_filter.is_none(), "preview commits nothing");
    let after_two = red(&rust);
    // A second, different radius must equal filtering the original once.
    assert!(apply_filter_active(&mut rust, "gaussian-blur", &[6.0], false).is_some());
    let after_six = red(&rust);
    let mut expected = state();
    apply_filter_active(&mut expected, "gaussian-blur", &[6.0], false);
    assert_eq!(after_six, red(&expected));
    assert_ne!(after_six, after_two);
    assert_ne!(after_six, original);
}

#[test]
fn cancel_restores_bit_identical() {
    let mut rust = state();
    let original = red(&rust);
    apply_filter_active(&mut rust, "gaussian-blur", &[4.0], false).expect("preview");
    assert_ne!(red(&rust), original, "preview changed pixels");
    let region = cancel_filter_preview(&mut rust).expect("preview open");
    assert!(region.is_some());
    assert_eq!(red(&rust), original, "cancel is bit-identical");
    assert!(rust.session.live.filter_preview().is_none());
    assert!(rust.last_filter.is_none());
}

#[test]
fn commit_clears_preview_and_records_last_filter() {
    let mut rust = state();
    apply_filter_active(&mut rust, "gaussian-blur", &[3.0], false).expect("preview");
    assert!(apply_filter_active(&mut rust, "gaussian-blur", &[3.0], true).is_some());
    assert!(
        rust.session.live.filter_preview().is_none(),
        "commit closes the preview"
    );
    assert_eq!(
        rust.last_filter,
        Some(("gaussian-blur".to_string(), vec![3.0]))
    );
    // Committing after a preview must equal a single fresh apply.
    let mut fresh = state();
    apply_filter_active(&mut fresh, "gaussian-blur", &[3.0], true).expect("commit");
    assert_eq!(red(&rust), red(&fresh), "no double application");
}

/// A section preview writes and reports only its section; the next one
/// puts that section back first, so the layer never compounds and the
/// refresh covers both.
#[test]
fn section_previews_restore_only_what_they_wrote() {
    let rect = |top, left, bottom, right| pictura_core::PsdRect {
        top,
        left,
        bottom,
        right,
    };
    let original = red(&big_state());
    let mut rust = big_state();
    let a = rect(16, 16, 48, 48);
    let b = rect(200, 200, 232, 232);
    // Gaussian radius 3 keeps the minimum 8 px apron.
    let (a_out, b_out) = (rect(8, 8, 56, 56), rect(192, 192, 240, 240));
    let region = apply_filter_active_region(&mut rust, "gaussian-blur", &[3.0], false, Some(a))
        .expect("preview a");
    assert_eq!(region, Some(a_out));
    let region = apply_filter_active_region(&mut rust, "gaussian-blur", &[3.0], false, Some(b))
        .expect("preview b");
    assert_eq!(region, Some(rect(8, 8, 240, 240)), "a put back, b written");

    let mut fresh = big_state();
    apply_filter_active_region(&mut fresh, "gaussian-blur", &[3.0], false, Some(b))
        .expect("fresh preview b");
    assert_eq!(red(&rust), red(&fresh), "section a was restored");
    let inside = |i: usize| {
        let (x, y) = ((i % 256) as i32, (i / 256) as i32);
        (b_out.left..b_out.right).contains(&x) && (b_out.top..b_out.bottom).contains(&y)
    };
    let px = red(&rust);
    assert!((0..px.len()).all(|i| inside(i) || px[i] == original[i]));
    assert!((0..px.len()).any(|i| px[i] != original[i]));

    let region = cancel_filter_preview(&mut rust).expect("preview open");
    assert_eq!(region, Some(b_out), "cancel refreshes the last section");
    assert_eq!(red(&rust), original, "cancel is bit-identical");

    apply_filter_active_region(&mut rust, "gaussian-blur", &[3.0], false, Some(a))
        .expect("preview a again");
    let region =
        apply_filter_active_region(&mut rust, "gaussian-blur", &[3.0], true, None).expect("commit");
    assert_eq!(region, Some(rect(0, 0, 256, 256)));
    let mut fresh = big_state();
    apply_filter_active_region(&mut fresh, "gaussian-blur", &[3.0], true, None)
        .expect("fresh commit");
    assert_eq!(
        red(&rust),
        red(&fresh),
        "no section of the preview leaks into the commit"
    );
}

/// A section preview keeps a copy of its section alone, so the live
/// planes are written where they are, never copied whole.
#[test]
fn section_previews_write_the_live_planes_in_place() {
    let mut rust = big_state();
    let plane = |rust: &PictureViewRust| {
        rust.session.doc.as_ref().unwrap().layers[0].channels[0]
            .data
            .as_ptr()
    };
    let before = plane(&rust);
    let section = pictura_core::PsdRect {
        top: 100,
        left: 100,
        bottom: 140,
        right: 140,
    };
    for radius in [3.0, 5.0] {
        apply_filter_active_region(&mut rust, "gaussian-blur", &[radius], false, Some(section))
            .expect("section preview");
        assert_eq!(plane(&rust), before, "radius {radius} copied the plane");
    }
    cancel_filter_preview(&mut rust).expect("preview open");
    assert_eq!(plane(&rust), before);
    assert_eq!(red(&rust), red(&big_state()));
}

#[test]
fn section_preview_escalates_radial_clouds_fibers() {
    use super::super::filter_tools::filter_preview_needs_whole_layer;

    for kind in ["radial-blur", "clouds", "difference-clouds", "fibers"] {
        assert!(filter_preview_needs_whole_layer(kind), "{kind}");
    }
}

#[test]
fn positional_kinds_preview_the_whole_layer() {
    use super::super::filter_map::SHEAR_SLOTS;
    use super::super::filter_tools::filter_preview_needs_whole_layer;

    let viewport = pictura_core::PsdRect {
        top: 200,
        left: 200,
        bottom: 232,
        right: 232,
    };
    for (kind, params) in [
        ("diffuse", vec![0.0f64]),
        (
            "lighting-effects",
            vec![
                255.0, 255.0, 255.0, 0.0, 0.0, 0.0, 0.0, 0.0, 50.0, // rig
                0.0, 1.0, 255.0, 255.0, 255.0, 35.0, 69.0, 0.85, 0.85, -35.0, 0.3, 0.2, 50.0,
            ],
        ),
        ("lens-flare", vec![100.0, 0.5, 0.5, 0.0]),
        (
            "field-blur",
            vec![
                2.0, 0.25, 0.5, 0.0, 0.75, 0.5, 40.0, // two pins
                0.0, 0.0, 0.0, 0.0, 0.0, // bokeh off
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, // noise off
            ],
        ),
        (
            "iris-blur",
            vec![
                1.0, 0.85, 0.85, 0.05, 0.05, 0.0, 50.0, 0.3, 30.0, // one iris
                0.0, 0.0, 0.0, 0.0, 0.0, // bokeh off
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, // noise off
            ],
        ),
        (
            "tilt-shift",
            vec![
                1.0, 0.5, 0.85, 15.0, 0.03, 0.03, 0.05, 0.05, 50.0, 0.0, 30.0, // one band
                0.0, 0.0, 0.0, 0.0, 0.0, // bokeh off
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, // noise off
            ],
        ),
        (
            "blur-gallery",
            vec![
                0.0, // no Field pins
                1.0, 0.85, 0.85, 0.05, 0.05, 0.0, 50.0, 0.3, 30.0, // one iris
                1.0, 0.5, 0.85, 15.0, 0.03, 0.03, 0.05, 0.05, 50.0, 0.0, 30.0, // one band
                0.0,  // no paths
                0.0,  // no spins
                50.0, 0.0, 1.0, 0.0, 0.0, 4.0, // motion
                1.0, // flash duration
                0.0, 0.0, 0.0, 0.0, 0.0, // bokeh off
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, // noise off
            ],
        ),
        (
            "path-blur",
            vec![
                1.0, 2.0, 0.8, 0.85, 0.95, 0.7, 40.0, 10.0, // one path
                100.0, 0.0, 1.0, 0.0, 0.0, 4.0, // motion
                0.0, 0.0, 0.0, 0.0, 0.0, // bokeh off
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, // noise off
            ],
        ),
        ("shear", {
            let mut shear = vec![0.0f64; SHEAR_SLOTS];
            shear[0] = 2.0;
            shear[1] = -1.0;
            shear[2] = -0.5;
            shear[3] = 1.0;
            shear[4] = 0.5;
            shear
        }),
    ] {
        assert!(filter_preview_needs_whole_layer(kind), "{kind}");

        // A cropped section preview does not match the commit...
        let mut cropped = big_state();
        apply_filter_active_region(&mut cropped, kind, &params, false, Some(viewport))
            .expect("section preview");

        // ...so the bridge escalates the kind to a whole-layer preview.
        let section = (!filter_preview_needs_whole_layer(kind)).then_some(viewport);
        let mut preview = big_state();
        apply_filter_active_region(&mut preview, kind, &params, false, section)
            .expect("whole-layer preview");

        let mut commit = big_state();
        apply_filter_active_region(&mut commit, kind, &params, true, None).expect("commit");

        let (preview_px, commit_px, cropped_px) = (red(&preview), red(&commit), red(&cropped));
        let w = 256usize;
        let mut cropped_differs = false;
        for y in viewport.top..viewport.bottom {
            for x in viewport.left..viewport.right {
                let i = y as usize * w + x as usize;
                assert_eq!(
                    preview_px[i], commit_px[i],
                    "{kind} whole-layer preview != commit at ({x},{y})"
                );
                if cropped_px[i] != commit_px[i] {
                    cropped_differs = true;
                }
            }
        }
        assert!(
            cropped_differs,
            "{kind}: a cropped preview unexpectedly matched the commit"
        );
    }
    // A spin's apron is the widest (its reach is a fraction of the
    // buffer), so on this picture a crop is the whole layer and cannot
    // show the mismatch; the kind still escalates.
    assert!(filter_preview_needs_whole_layer("spin-blur"));
}

fn mask_coverage(rust: &PictureViewRust) -> Vec<u8> {
    rust.session.doc.as_ref().unwrap().layers[0]
        .mask
        .as_ref()
        .unwrap()
        .data
        .as_ref()
        .unwrap()
        .to_vec()
}

fn step_mask_state() -> PictureViewRust {
    let mut rust = state();
    let layer = &mut rust.session.doc.as_mut().unwrap().layers[0];
    layer.mask = Some(pictura_core::LayerMask {
        rect: pictura_core::PsdRect {
            top: 0,
            left: 0,
            bottom: 16,
            right: 16,
        },
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(
            (0..16 * 16)
                .map(|i| if i % 16 < 8 { 0u8 } else { 255 })
                .collect::<Vec<_>>()
                .into(),
        ),
        extra: Vec::new().into(),
        ..Default::default()
    });
    rust.mask_edit_target = Some("0".to_string());
    rust
}

#[test]
fn a_filter_previews_and_commits_on_the_mask() {
    let mut rust = step_mask_state();
    let original = mask_coverage(&rust);
    let layer_red = red(&rust);

    let preview =
        apply_filter_active(&mut rust, "gaussian-blur", &[2.0], false).expect("mask preview");
    assert!(
        preview.is_some(),
        "the mask rectangle is the refresh region"
    );
    assert!(
        rust.session.live.mask_filter_preview().is_some(),
        "a baseline is kept"
    );
    let softened = mask_coverage(&rust);
    assert_ne!(softened, original, "the blur changed the edge");
    assert!(
        softened[7] > 0 && softened[7] < 255,
        "the black/white edge is softened"
    );
    assert_eq!(red(&rust), layer_red, "the layer's pixels are untouched");

    cancel_filter_preview(&mut rust).expect("cancel");
    assert_eq!(
        mask_coverage(&rust),
        original,
        "cancel restores bit-identically"
    );
    assert!(rust.session.live.mask_filter_preview().is_none());

    apply_filter_active(&mut rust, "gaussian-blur", &[2.0], true).expect("mask commit");
    assert!(rust.session.live.mask_filter_preview().is_none());
    assert_ne!(mask_coverage(&rust), original, "the commit keeps the blur");
    assert_eq!(red(&rust), layer_red, "the layer's pixels are untouched");
}

#[test]
fn a_mask_filter_needs_the_target() {
    let mut rust = step_mask_state();
    rust.mask_edit_target = None;
    let original = mask_coverage(&rust);
    apply_filter_active(&mut rust, "gaussian-blur", &[2.0], true).expect("layer filter");
    assert_eq!(mask_coverage(&rust), original, "the mask is untouched");
}
