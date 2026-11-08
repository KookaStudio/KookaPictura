//! Edit clipboard tests: Copy / Copy Merged / Clear / Paste (Into, Outside).

use super::clipboard::{
    clear_layer, copy_layer, copy_merged, coverage_bounds, paste_clip, plane_coverage_bounds,
    plane_has_coverage, Clip, PasteMode, COVERAGE_SCANS,
};
use super::create::empty_group;
use super::tests::{doc_with, pixel_layer};
use pictura_core::{LockFlags, PixelBuffer, PsdRect};

/// A 4×4 selection covering `(x0..x1, y0..y1)` at full strength.
fn select(x0: usize, y0: usize, x1: usize, y1: usize) -> Vec<u8> {
    let mut data = vec![0u8; 16];
    for y in y0..y1 {
        for x in x0..x1 {
            data[y * 4 + x] = 255;
        }
    }
    data
}

fn alpha(doc: &pictura_core::Document, index: usize) -> &[u8] {
    &doc.layers[index]
        .channels
        .iter()
        .find(|c| c.id == -1)
        .unwrap()
        .data
}

fn prect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
    }
}

#[test]
fn copy_keeps_only_the_selection_bounding_box() {
    let doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
    let mut sel = select(1, 1, 3, 3);
    sel[4 + 2] = 128;
    let clip = copy_layer(&doc, "0", Some(&sel)).unwrap();

    assert_eq!(clip.rect, prect(1, 1, 3, 3));
    assert_eq!(clip.rgba.len(), 2 * 2 * 4, "a small copy stays small");
    assert_eq!(&clip.rgba[..4], &[40, 40, 40, 255]);
    assert_eq!(clip.mask, vec![255, 128, 255, 255]);
}

#[test]
fn copy_without_a_selection_takes_the_layer_clipped_to_the_canvas() {
    let mut layer = pixel_layer("base", 4, 4, 40);
    layer.rect = prect(-1, -1, 3, 3);
    let doc = doc_with(vec![layer]);
    let clip = copy_layer(&doc, "0", None).unwrap();

    assert_eq!(clip.rect, prect(0, 0, 3, 3));
    assert!(clip.mask.iter().all(|&v| v == 255));
}

#[test]
fn copy_refuses_groups_empty_selections_and_transparent_pixels() {
    let mut clear = pixel_layer("clear", 4, 4, 40);
    clear.channels[3].data.fill(0);
    let doc = doc_with(vec![pixel_layer("base", 4, 4, 40), clear, empty_group("g")]);

    assert!(copy_layer(&doc, "2", None).is_none(), "group");
    assert!(
        copy_layer(&doc, "0", Some(&[0u8; 16])).is_none(),
        "empty selection"
    );
    assert!(
        copy_layer(&doc, "1", None).is_none(),
        "only transparent pixels"
    );
    assert!(copy_layer(&doc, "9", None).is_none(), "missing path");
}

#[test]
fn copy_replicates_a_grayscale_layer_into_rgb() {
    let mut layer = pixel_layer("gray", 4, 4, 90);
    layer.channels.retain(|c| c.id == 0 || c.id == -1);
    let doc = doc_with(vec![layer]);
    let clip = copy_layer(&doc, "0", None).unwrap();

    assert_eq!(&clip.rgba[..4], &[90, 90, 90, 255]);
}

#[test]
fn copy_merged_reads_the_composite_planes() {
    let doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
    let mut data = vec![0u8; 16 * 4];
    for (plane, value) in [10u8, 20, 30, 255].into_iter().enumerate() {
        data[plane * 16..(plane + 1) * 16].fill(value);
    }
    let composite = PixelBuffer {
        width: 4,
        height: 4,
        channels: 4,
        data: data.into(),
    };
    let clip = copy_merged(&doc, &composite, Some(&select(0, 0, 2, 1))).unwrap();

    assert_eq!(clip.rect, prect(0, 0, 2, 1));
    assert_eq!(clip.rgba, vec![10, 20, 30, 255, 10, 20, 30, 255]);
    let wrong = PixelBuffer {
        channels: 3,
        ..composite
    };
    assert!(copy_merged(&doc, &wrong, None).is_none());
}

#[test]
fn clear_scales_alpha_by_coverage_and_reports_no_change() {
    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
    let mut sel = select(0, 0, 2, 1);
    sel[1] = 128;

    assert!(clear_layer(&mut doc, "0", Some(&sel)));
    assert_eq!(&alpha(&doc, 0)[..3], &[0, 127, 255]);
    assert!(
        !clear_layer(&mut doc, "0", Some(&select(0, 0, 1, 1))),
        "already clear"
    );
    assert!(clear_layer(&mut doc, "0", None));
    assert!(alpha(&doc, 0).iter().all(|&a| a == 0));
}

#[test]
fn clear_respects_pixel_and_transparency_locks() {
    let mut pixels = pixel_layer("pixels", 4, 4, 40);
    pixels.lock = LockFlags::default().with(LockFlags::PIXELS, true);
    let mut transparency = pixel_layer("transparency", 4, 4, 40);
    transparency.lock = LockFlags::default().with(LockFlags::TRANSPARENCY, true);
    let mut doc = doc_with(vec![pixels, transparency]);

    assert!(!clear_layer(&mut doc, "0", None));
    assert!(!clear_layer(&mut doc, "1", None));
    assert!(alpha(&doc, 0)
        .iter()
        .chain(alpha(&doc, 1))
        .all(|&a| a == 255));
}

#[test]
fn clear_fills_a_background_toward_white() {
    let mut background = pixel_layer("Background", 4, 4, 40);
    background.channels.retain(|c| c.id != -1);
    background.lock = LockFlags::default().with(LockFlags::TRANSPARENCY, true);
    let mut doc = doc_with(vec![background]);

    assert!(clear_layer(&mut doc, "0", Some(&select(0, 0, 1, 1))));
    let red = &doc.layers[0].channels[0].data;
    assert_eq!((red[0], red[1]), (255, 40));
}

#[test]
fn paste_inserts_a_layer_above_the_target_with_the_mask_as_alpha() {
    let mut doc = doc_with(vec![
        pixel_layer("base", 4, 4, 40),
        pixel_layer("top", 4, 4, 9),
    ]);
    let clip = Clip {
        rect: prect(0, 0, 2, 1),
        rgba: vec![1, 2, 3, 255, 4, 5, 6, 200],
        mask: vec![128, 255],
    };
    let path = paste_clip(&mut doc, "0", &clip, (2, 3), PasteMode::Plain, None);

    assert_eq!(path, "1");
    let layer = &doc.layers[1];
    assert_eq!(layer.name, "Layer 1");
    assert_eq!(layer.rect, prect(2, 3, 4, 4));
    assert!(layer.mask.is_none());
    let plane = |id: i16| {
        layer
            .channels
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .data
            .clone()
    };
    assert_eq!(plane(0), vec![1, 4]);
    assert_eq!(plane(-1), vec![128, 200]);
    assert_eq!(doc.layers[2].name, "top");
}

#[test]
fn paste_into_and_outside_mask_by_the_selection() {
    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
    let clip = copy_layer(&doc, "0", None).unwrap();
    let sel = select(0, 0, 2, 2);

    assert!(paste_clip(&mut doc, "0", &clip, (0, 0), PasteMode::Into, None).is_empty());
    let into = paste_clip(&mut doc, "0", &clip, (0, 0), PasteMode::Into, Some(&sel));
    let mask = doc.layers[1].mask.as_ref().unwrap();
    assert_eq!(into, "1");
    assert_eq!(
        (mask.default_color, mask.data.as_deref()),
        (0, Some(&sel[..]))
    );

    let outside = paste_clip(&mut doc, "1", &clip, (0, 0), PasteMode::Outside, Some(&sel));
    let mask = doc.layers[2].mask.as_ref().unwrap();
    assert_eq!(outside, "2");
    assert_eq!(mask.default_color, 255);
    assert_eq!(mask.data.as_ref().unwrap()[0], 0);
    assert_eq!(mask.data.as_ref().unwrap()[15], 255);
}

#[test]
fn cut_then_paste_in_place_restores_the_pixels() {
    let mut layer = pixel_layer("base", 4, 4, 40);
    layer.channels[0].data[5] = 200;
    let mut doc = doc_with(vec![layer]);
    let sel = select(1, 1, 3, 3);

    let clip = copy_layer(&doc, "0", Some(&sel)).unwrap();
    assert!(clear_layer(&mut doc, "0", Some(&sel)));
    assert_eq!(alpha(&doc, 0)[5], 0);
    let origin = (clip.rect.left, clip.rect.top);
    let path = paste_clip(&mut doc, "0", &clip, origin, PasteMode::Plain, None);

    let pasted = &doc.layers[1];
    assert_eq!(path, "1");
    assert_eq!(pasted.rect, prect(1, 1, 3, 3));
    assert_eq!(pasted.channels[0].data[0], 200);
    assert!(pasted.channels[3].data.iter().all(|&a| a == 255));
}

#[test]
fn paste_refuses_a_malformed_clip() {
    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
    let clip = Clip {
        rect: prect(0, 0, 2, 2),
        rgba: vec![0; 4],
        mask: vec![255; 4],
    };
    assert!(paste_clip(&mut doc, "0", &clip, (0, 0), PasteMode::Plain, None).is_empty());
    assert_eq!(doc.layers.len(), 1);
}

#[test]
fn coverage_bounds_spans_every_selected_pixel() {
    let mut sel = vec![0u8; 16];
    sel[4 + 2] = 1;
    sel[3 * 4 + 1] = 255;

    assert_eq!(coverage_bounds(&sel, 4, 4), Some(prect(1, 1, 3, 4)));
    assert_eq!(coverage_bounds(&[0u8; 16], 4, 4), None);
    assert_eq!(coverage_bounds(&[255u8; 3], 4, 4), None, "short plane");
}

#[test]
fn a_foreign_image_becomes_a_fully_covered_clip_at_the_origin() {
    let clip = Clip::from_rgba(2, 1, vec![1, 2, 3, 4, 5, 6, 7, 8]).unwrap();

    assert_eq!(clip.rect, prect(0, 0, 2, 1));
    assert_eq!(clip.mask, vec![255, 255]);
    assert_eq!(clip.masked_rgba(), vec![1, 2, 3, 4, 5, 6, 7, 8]);
    assert!(Clip::from_rgba(2, 1, vec![0; 4]).is_none(), "short buffer");
    assert!(Clip::from_rgba(0, 1, Vec::new()).is_none(), "zero width");
}

#[test]
fn masked_rgba_folds_the_selection_into_alpha() {
    let clip = Clip {
        rect: prect(0, 0, 2, 1),
        rgba: vec![9, 9, 9, 255, 9, 9, 9, 200],
        mask: vec![128, 0],
    };
    assert_eq!(clip.masked_rgba(), vec![9, 9, 9, 128, 9, 9, 9, 0]);
}

fn coverage_scans() -> usize {
    COVERAGE_SCANS.with(std::cell::Cell::get)
}

#[test]
fn plane_coverage_test_matches_its_bounds() {
    let (w, h) = (40u32, 130u32);
    let n = (w * h) as usize;
    let mut one = vec![0u8; n];
    one[n - 3] = 1;
    let mut banded = pictura_core::Plane::from(vec![0u8; n]);
    banded.tile(w as usize, h as usize);
    banded.write_rect_from(7..8, 129..130, w as usize, &[9]);
    let cases = [
        pictura_core::Plane::from(vec![0u8; n]),
        pictura_core::Plane::from(one),
        pictura_core::Plane::from(vec![255u8; n]),
        pictura_core::Plane::from(vec![255u8; n - 1]),
        banded,
    ];
    for (i, plane) in cases.iter().enumerate() {
        assert_eq!(
            plane_has_coverage(plane, w, h),
            plane_coverage_bounds(plane, w, h).is_some(),
            "case {i}"
        );
    }
    assert!(!plane_has_coverage(&cases[2], 0, h));
}

#[test]
fn an_unchanged_plane_is_scanned_once() {
    let (w, h) = (32u32, 32u32);
    let mut plane = pictura_core::Plane::from(vec![0u8; (w * h) as usize]);
    let before = coverage_scans();
    assert!(!plane_has_coverage(&plane, w, h));
    assert!(!plane_has_coverage(&plane, w, h));
    assert!(!plane_has_coverage(&plane.clone(), w, h));
    assert_eq!(
        coverage_scans() - before,
        1,
        "a remembered plane is not read again"
    );

    plane.as_mut_slice()[5] = 1;
    assert!(plane_has_coverage(&plane, w, h), "a write is seen");
    assert_eq!(coverage_scans() - before, 2);
}

#[test]
fn a_diverged_plane_is_checked_without_materializing() {
    let (w, h) = (16u32, 200u32);
    let mut plane = pictura_core::Plane::from(vec![0u8; (w * h) as usize]);
    plane.tile(w as usize, h as usize);
    plane.write_rect_from(3..4, 150..151, w as usize, &[4]);
    let before = pictura_core::materializations();
    assert!(plane_has_coverage(&plane, w, h));
    assert_eq!(pictura_core::materializations(), before);
}
