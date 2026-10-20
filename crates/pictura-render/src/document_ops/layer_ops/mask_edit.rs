//! Editing a raster layer mask's coverage.
//!
//! The paint, fill, and filter engines edit a [`Layer`](pictura_core::Layer)'s
//! colour channels, so a mask is exposed as a standalone mask-sized grayscale
//! document they can edit unchanged: [`mask_document`] builds it and
//! [`write_mask_back`] copies the result into the real mask. The document's
//! single layer is transparency-locked so opacity stays 255 while its colour
//! plane carries the coverage.

use pictura_core::{BitDepth, Channel, ColorMode, Document, Layer, LockFlags, Plane, PsdRect};

use super::clipboard::coverage_bounds;
use super::paths::{resolve_path, resolve_path_mut};

/// A layer's raster mask as a standalone document: `document` is a mask-sized
/// grayscale image whose one layer's colour plane is the coverage, and `rect`
/// is the mask's document-space rectangle (the layer itself sits at the
/// origin, so mask-local edits map 1:1).
pub struct MaskDocument {
    pub document: Document,
    pub rect: PsdRect,
}

/// The mask of the layer at `path` as a grayscale document, or `None` without
/// a layer mask, for a zero-sized mask rectangle, or when the path does not
/// resolve. A data-less mask is materialised from its default colour.
pub fn mask_document(doc: &Document, path: &str) -> Option<MaskDocument> {
    build_mask_document(doc, path)
}

/// [`mask_document`] as a **single-channel** grayscale document: its one layer
/// carries the coverage as channel `0` only, sharing the mask's plane, and the
/// document has no composite. The paint stroke edits the mask through this
/// form, so starting a stroke copies nothing, a dab writes one byte per pixel,
/// and only the coverage plane forks, on the first write. Filters and fills
/// keep the RGBA [`mask_document`] a filter may expect.
pub fn mask_document_coverage(doc: &Document, path: &str) -> Option<MaskDocument> {
    let mask = resolve_path(doc, path)?.mask.as_ref()?;
    let rect = mask.rect;
    let (w, h) = (rect.width().max(0) as u32, rect.height().max(0) as u32);
    let count = (w as usize) * (h as usize);
    if count == 0 {
        return None;
    }
    let coverage: Plane<u8> = match &mask.data {
        Some(data) if data.len() == count => data.clone(),
        _ => vec![mask.default_color; count].into(),
    };
    // Sized after construction, so `new` allocates no composite.
    let mut document = Document::new(0, 0, ColorMode::Grayscale, BitDepth::Eight);
    document.width = w;
    document.height = h;
    document.layers = pictura_core::LayerStack::from_layers(vec![Layer {
        name: "Mask".to_string(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        },
        channels: vec![Channel {
            id: 0,
            data: coverage,
        }],
        lock: LockFlags::default().with(LockFlags::TRANSPARENCY, true),
        ..Default::default()
    }]);
    Some(MaskDocument { document, rect })
}

fn build_mask_document(doc: &Document, path: &str) -> Option<MaskDocument> {
    let layer = resolve_path(doc, path)?;
    let mask = layer.mask.as_ref()?;
    let rect = mask.rect;
    let (w, h) = (rect.width().max(0) as u32, rect.height().max(0) as u32);
    if w == 0 || h == 0 {
        return None;
    }
    let count = (w as usize) * (h as usize);
    let coverage = match &mask.data {
        Some(data) if data.len() == count => data.as_slice(),
        _ => {
            return Some(MaskDocument {
                document: gray_document(w, h, None, mask.default_color),
                rect,
            })
        }
    };
    Some(MaskDocument {
        document: gray_document(w, h, Some(coverage), 0),
        rect,
    })
}

/// A `w × h` opaque RGBA document whose colour plane is `coverage` (or `fill`
/// when absent), with transparency locked so an edit cannot rewrite the opacity.
fn gray_document(w: u32, h: u32, coverage: Option<&[u8]>, fill: u8) -> Document {
    let count = (w as usize) * (h as usize);
    let mut rgba = Vec::with_capacity(count * 4);
    for i in 0..count {
        let v = coverage.map_or(fill, |data| data.get(i).copied().unwrap_or(fill));
        rgba.extend_from_slice(&[v, v, v, 255]);
    }
    let mut document = Document::from_rgba("Mask", w, h, &rgba);
    if let Some(layer) = document.layers.first_mut() {
        layer.lock = LockFlags::default().with(LockFlags::TRANSPARENCY, true);
    }
    document
}

/// Copy the edited document's colour plane back into the mask of the layer at
/// `path`, limited to `region` in mask-local coordinates (`None` copies the
/// whole plane). A data-less mask is materialised from its default colour
/// first. Returns whether the mask exists and the copy ran.
pub fn write_mask_back(
    doc: &mut Document,
    path: &str,
    edited: &Document,
    region: Option<PsdRect>,
) -> bool {
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    let Some(mask) = layer.mask.as_mut() else {
        return false;
    };
    let w = mask.rect.width().max(0) as usize;
    let h = mask.rect.height().max(0) as usize;
    if w == 0 || h == 0 {
        return false;
    }
    let Some(source) = edited
        .layers
        .first()
        .and_then(|layer| layer.channels.iter().find(|c| c.id == 0))
    else {
        return false;
    };
    if source.data.len() < w * h {
        return false;
    }
    let src = source.data.as_slice();
    let dst: &mut [u8] = match &mut mask.data {
        Some(data) if data.len() == w * h => &mut *data,
        _ => {
            mask.data = Some(vec![mask.default_color; w * h].into());
            &mut *mask.data.as_mut().expect("just materialised")
        }
    };
    match region {
        None => dst.copy_from_slice(&src[..w * h]),
        Some(region) => {
            let (x0, y0) = (region.left.max(0) as usize, region.top.max(0) as usize);
            let (x1, y1) = (
                (region.right.max(0) as usize).min(w),
                (region.bottom.max(0) as usize).min(h),
            );
            if x1 <= x0 || y1 <= y0 {
                return true;
            }
            for y in y0..y1 {
                let (a, b) = (y * w + x0, y * w + x1);
                dst[a..b].copy_from_slice(&src[a..b]);
            }
        }
    }
    true
}

/// Move the coverage selected by `coverage` (mask-local, `0..=255`) inside the
/// mask of the layer at `path` by `(dx, dy)`. The vacated source coverage is
/// blended toward `background` by the selection coverage, and the moved
/// coverage is composited at the destination using the source coverage as its
/// alpha. With `duplicate`, the source coverage stays and the moved copy is
/// composited over the destination. The mask rectangle and the layer's pixels
/// are untouched. Returns false without a mask, for a zero-sized mask, a
/// zero offset, or a selection plane that does not match the mask size.
pub fn move_mask_selection_content(
    doc: &mut Document,
    path: &str,
    coverage: &[u8],
    dx: i32,
    dy: i32,
    duplicate: bool,
    background: u8,
) -> bool {
    if dx == 0 && dy == 0 {
        return false;
    }
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    let Some(mask) = layer.mask.as_mut() else {
        return false;
    };
    let w = mask.rect.width().max(0) as usize;
    let h = mask.rect.height().max(0) as usize;
    if w == 0 || h == 0 || coverage.len() != w * h {
        return false;
    }
    // An empty selection has nothing to lift; refusing here keeps it from
    // materialising a data-less mask and recording an empty move.
    let Some(bounds) = coverage_bounds(coverage, w as u32, h as u32) else {
        return false;
    };
    let snapshot: Vec<u8> = match &mask.data {
        Some(data) if data.len() == w * h => data.as_slice().to_vec(),
        _ => vec![mask.default_color; w * h],
    };
    let mut out = snapshot.clone();
    if !duplicate {
        for y in bounds.top..bounds.bottom {
            let row = y as usize * w;
            let start = row + bounds.left as usize;
            let end = row + bounds.right as usize;
            for (i, &selected) in coverage[start..end].iter().enumerate() {
                if selected != 0 {
                    let at = start + i;
                    out[at] = blend(background, selected, snapshot[at]);
                }
            }
        }
    }
    let dest_top = (bounds.top + dy).max(0);
    let dest_bottom = (bounds.bottom + dy).min(h as i32);
    let dest_left = (bounds.left + dx).max(0);
    let dest_right = (bounds.right + dx).min(w as i32);
    let span = (dest_right - dest_left).max(0) as usize;
    for dest_y in dest_top..dest_bottom {
        let src_start = (dest_y - dy) as usize * w + (dest_left - dx) as usize;
        let dest_start = dest_y as usize * w + dest_left as usize;
        let covered = &coverage[src_start..src_start + span];
        let source = &snapshot[src_start..src_start + span];
        let target = &mut out[dest_start..dest_start + span];
        for i in 0..span {
            let selected = covered[i];
            if selected != 0 {
                target[i] = blend(source[i], selected, target[i]);
            }
        }
    }
    mask.data = Some(out.into());
    true
}

/// `top` over `bottom` with `alpha` (`0..=255`), in straight 8-bit coverage.
fn blend(top: u8, alpha: u8, bottom: u8) -> u8 {
    let top = u32::from(top);
    let alpha = u32::from(alpha);
    let bottom = u32::from(bottom);
    ((top * alpha + bottom * (255 - alpha) + 127) / 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_ops::layer_ops::tests::{doc_with, pixel_layer};
    use pictura_core::LayerMask;

    fn masked_doc() -> Document {
        let mut layer = pixel_layer("base", 4, 4, 40);
        layer.mask = Some(LayerMask {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 4,
                right: 4,
            },
            default_color: 255,
            disabled: false,
            flags: 0,
            data: Some(
                (0..16)
                    .map(|i| if i % 4 < 2 { 0u8 } else { 255 })
                    .collect::<Vec<_>>()
                    .into(),
            ),
            extra: Vec::new(),
            ..Default::default()
        });
        doc_with(vec![layer])
    }

    fn coverage(doc: &Document) -> Vec<u8> {
        doc.layers[0]
            .mask
            .as_ref()
            .unwrap()
            .data
            .as_ref()
            .unwrap()
            .to_vec()
    }

    #[test]
    fn mask_document_mirrors_the_coverage() {
        let doc = masked_doc();
        let mask = mask_document(&doc, "0").unwrap();
        assert_eq!(mask.rect, doc.layers[0].mask.as_ref().unwrap().rect);
        let red = mask.document.layers[0]
            .channels
            .iter()
            .find(|c| c.id == 0)
            .unwrap();
        assert_eq!(red.data.as_slice(), coverage(&doc).as_slice());
        assert!(mask.document.layers[0]
            .lock
            .contains(LockFlags::TRANSPARENCY));
    }

    #[test]
    fn write_back_round_trips_and_limits_to_the_region() {
        let mut doc = masked_doc();
        let mut edited = mask_document(&doc, "0").unwrap().document;
        edited.layers[0]
            .channels
            .iter_mut()
            .find(|c| c.id == 0)
            .unwrap()
            .data = vec![7u8; 16].into();
        let region = PsdRect {
            top: 1,
            left: 1,
            bottom: 2,
            right: 2,
        };
        assert!(write_mask_back(&mut doc, "0", &edited, Some(region)));
        let cov = coverage(&doc);
        assert_eq!(cov[4 + 1], 7, "inside the region");
        assert_eq!(cov[0], 0, "outside keeps its value");
        assert_eq!(cov[3], 255, "outside keeps its value");

        assert!(write_mask_back(&mut doc, "0", &edited, None));
        assert!(coverage(&doc).iter().all(|&v| v == 7));
    }

    #[test]
    fn a_data_less_mask_materialises_from_its_default() {
        let mut doc = masked_doc();
        doc.layers[0].mask.as_mut().unwrap().data = None;
        let mut edited = mask_document(&doc, "0").unwrap().document;
        edited.layers[0]
            .channels
            .iter_mut()
            .find(|c| c.id == 0)
            .unwrap()
            .data = vec![9u8; 16].into();
        assert!(write_mask_back(&mut doc, "0", &edited, None));
        assert!(coverage(&doc).iter().all(|&v| v == 9));
    }

    #[test]
    fn a_coverage_document_is_single_channel() {
        let doc = masked_doc();
        let mask = mask_document_coverage(&doc, "0").unwrap();
        let layer = &mask.document.layers[0];
        assert_eq!(layer.channels.len(), 1, "one coverage channel");
        assert_eq!(layer.channels[0].id, 0);
        assert_eq!(layer.channels[0].data.len(), 16);
        assert_eq!(layer.channels[0].data[0], 0, "the coverage is mirrored");
        assert_eq!(layer.channels[0].data[2], 255);
        let source = resolve_path(&doc, "0").unwrap().mask.as_ref().unwrap();
        assert!(
            layer.channels[0].data.shares(source.data.as_ref().unwrap()),
            "the stroke start shares the mask's plane instead of copying it"
        );
        assert!(mask.document.composite.data.is_empty(), "no composite");
    }

    #[test]
    fn coverage_and_rgba_documents_paint_the_same_coverage() {
        use pictura_paint::spacing::SpacingMode;
        use pictura_paint::{Rgba, Stroke, StrokeConfig, StrokeSample};

        let doc = masked_doc();
        let cfg = StrokeConfig {
            color: Rgba {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            diameter: 3,
            hardness: 100,
            opacity: 100,
            flow: 100,
            spacing: SpacingMode::Fixed(10),
            ..Default::default()
        };
        let samples = [
            StrokeSample {
                x: 1.0,
                y: 1.0,
                pressure: 1.0,
            },
            StrokeSample {
                x: 2.0,
                y: 3.0,
                pressure: 1.0,
            },
        ];
        let paint = |mut document: Document| {
            let mut stroke = Stroke::begin_at(&document, "0", cfg).expect("stroke begins");
            for sample in samples {
                stroke.sample(&mut document, sample);
            }
            stroke.finish();
            document
        };

        let rgba = paint(mask_document(&doc, "0").unwrap().document);
        let coverage = paint(mask_document_coverage(&doc, "0").unwrap().document);

        let channel0 = |d: &Document| {
            d.layers[0]
                .channels
                .iter()
                .find(|c| c.id == 0)
                .expect("coverage channel")
                .data
                .to_vec()
        };
        let (a, b) = (channel0(&rgba), channel0(&coverage));
        assert_eq!(a.len(), b.len(), "same plane size");
        for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
            assert!(
                (*x as i32 - *y as i32).abs() <= 1,
                "pixel {i}: RGBA {x} vs 1-channel {y}"
            );
        }
    }

    #[test]
    fn refusals_leave_the_document_alone() {
        let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
        let copy = doc.clone();
        assert!(mask_document(&doc, "0").is_none(), "no mask");
        assert!(!write_mask_back(&mut doc, "0", &copy, None));
        assert!(mask_document(&doc, "9").is_none(), "no path");
        let masked = masked_doc();
        let empty = doc_with(vec![]);
        assert!(mask_document(&masked, "9").is_none());
        assert!(mask_document(&empty, "0").is_none());
    }

    #[test]
    fn a_zero_sized_mask_refuses() {
        let mut doc = masked_doc();
        doc.layers[0].mask.as_mut().unwrap().rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        };
        let copy = doc.clone();
        assert!(mask_document(&doc, "0").is_none());
        assert!(!write_mask_back(&mut doc, "0", &copy, None));
    }

    fn mask_rect(left: i32, top: i32, w: i32, h: i32) -> PsdRect {
        PsdRect {
            top,
            left,
            bottom: top + h,
            right: left + w,
        }
    }

    fn masked_with(rect: PsdRect, data: Option<Vec<u8>>, default_color: u8) -> Document {
        let mut layer = pixel_layer("base", 8, 8, 40);
        layer.mask = Some(LayerMask {
            rect,
            default_color,
            data: data.map(Into::into),
            ..Default::default()
        });
        doc_with(vec![layer])
    }

    #[test]
    fn move_mask_selection_translates_and_fills_background() {
        let mut doc = masked_with(mask_rect(0, 0, 4, 1), Some(vec![0, 64, 128, 255]), 255);
        let channels = doc.layers[0].channels.clone();
        assert!(move_mask_selection_content(
            &mut doc, "0", &[255; 4], 1, 0, false, 255
        ));
        assert_eq!(coverage(&doc), vec![255, 0, 64, 128]);
        assert_eq!(doc.layers[0].channels, channels, "layer pixels untouched");
        assert!(doc.layers[0].mask.is_some());
    }

    #[test]
    fn move_mask_selection_blends_partial_coverage() {
        let mut doc = masked_with(mask_rect(0, 0, 4, 1), Some(vec![0, 255, 0, 0]), 0);
        assert!(move_mask_selection_content(
            &mut doc,
            "0",
            &[0, 128, 128, 0],
            1,
            0,
            false,
            255
        ));
        assert_eq!(coverage(&doc), vec![0, 255, 192, 0]);
    }

    #[test]
    fn move_mask_selection_clips_at_the_mask_rect() {
        let mut doc = masked_with(mask_rect(5, 7, 3, 1), Some(vec![10, 20, 30]), 0);
        let rect = doc.layers[0].mask.as_ref().unwrap().rect;
        assert!(move_mask_selection_content(
            &mut doc, "0", &[255; 3], 1, 0, false, 99
        ));
        assert_eq!(coverage(&doc), vec![99, 10, 20]);
        assert_eq!(doc.layers[0].mask.as_ref().unwrap().rect, rect);
    }

    #[test]
    fn move_mask_selection_duplicate_keeps_the_source() {
        let mut doc = masked_with(mask_rect(0, 0, 6, 1), Some(vec![10, 20, 30, 0, 0, 0]), 0);
        assert!(move_mask_selection_content(
            &mut doc, "0", &[255; 6], 3, 0, true, 99
        ));
        assert_eq!(coverage(&doc), vec![10, 20, 30, 10, 20, 30]);
    }

    #[test]
    fn move_mask_selection_leaves_pixels_and_mask_metadata_unchanged() {
        let mut doc = masked_with(mask_rect(2, 3, 4, 2), Some(vec![0; 8]), 0);
        let channels = doc.layers[0].channels.clone();
        let rect = doc.layers[0].mask.as_ref().unwrap().rect;
        assert!(move_mask_selection_content(
            &mut doc, "0", &[255; 8], 1, 0, false, 0
        ));
        assert_eq!(doc.layers[0].channels, channels);
        let mask = doc.layers[0].mask.as_ref().unwrap();
        assert_eq!(mask.rect, rect);
        assert_eq!(mask.default_color, 0);
    }

    #[test]
    fn move_mask_selection_materializes_missing_data() {
        let mut doc = masked_with(mask_rect(0, 0, 2, 1), None, 200);
        assert!(move_mask_selection_content(
            &mut doc, "0", &[255; 2], 1, 0, false, 0
        ));
        assert_eq!(coverage(&doc), vec![0, 200]);

        let mut doc = masked_with(mask_rect(0, 0, 2, 1), Some(vec![7, 7, 7]), 200);
        assert!(move_mask_selection_content(
            &mut doc, "0", &[255; 2], 1, 0, false, 0
        ));
        assert_eq!(coverage(&doc), vec![0, 200]);
    }

    #[test]
    fn move_mask_selection_refuses_a_mismatched_selection_plane() {
        let mut doc = masked_with(mask_rect(0, 0, 2, 1), Some(vec![0, 0]), 0);
        let before = doc.clone();
        assert!(!move_mask_selection_content(
            &mut doc, "0", &[255; 3], 1, 0, false, 0
        ));
        assert_eq!(doc, before);

        let mut no_mask = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
        assert!(!move_mask_selection_content(
            &mut no_mask,
            "0",
            &[255; 4],
            1,
            0,
            false,
            0
        ));
    }

    #[test]
    fn move_mask_selection_refuses_an_empty_coverage() {
        let mut doc = masked_with(mask_rect(0, 0, 2, 1), Some(vec![0, 0]), 0);
        let before = doc.clone();
        assert!(!move_mask_selection_content(
            &mut doc,
            "0",
            &[0, 0],
            1,
            0,
            false,
            255
        ));
        assert_eq!(doc, before);
    }

    #[test]
    fn move_mask_selection_ignores_layer_locks() {
        let mut doc = masked_with(mask_rect(0, 0, 2, 1), Some(vec![0, 255]), 0);
        doc.layers[0].lock = LockFlags::default()
            .with(LockFlags::POSITION, true)
            .with(LockFlags::PIXELS, true);
        assert!(move_mask_selection_content(
            &mut doc, "0", &[255; 2], 1, 0, false, 99
        ));
        assert_eq!(coverage(&doc), vec![99, 0]);
    }

    #[test]
    fn move_mask_selection_works_on_a_group_mask() {
        let mut group = pixel_layer("group", 4, 1, 0);
        group.is_group = true;
        group.channels.clear();
        group.mask = Some(LayerMask {
            rect: mask_rect(0, 0, 4, 1),
            default_color: 0,
            data: Some(vec![0, 255, 0, 255].into()),
            ..Default::default()
        });
        let mut doc = doc_with(vec![group]);
        assert!(move_mask_selection_content(
            &mut doc, "0", &[255; 4], 1, 0, false, 0
        ));
        assert_eq!(
            doc.layers[0]
                .mask
                .as_ref()
                .unwrap()
                .data
                .as_ref()
                .unwrap()
                .to_vec(),
            vec![0, 0, 255, 0]
        );
    }

    /// The pre-bounding full-plane loop, kept as the oracle for the bounded move.
    fn reference_move(
        mask: &[u8],
        coverage: &[u8],
        w: usize,
        h: usize,
        dx: i32,
        dy: i32,
        duplicate: bool,
    ) -> Vec<u8> {
        let background = 7;
        let snapshot = mask.to_vec();
        let mut out = snapshot.clone();
        if !duplicate {
            for (i, &selected) in coverage.iter().enumerate() {
                if selected != 0 {
                    out[i] = blend(background, selected, snapshot[i]);
                }
            }
        }
        for dest_y in 0..h {
            for dest_x in 0..w {
                let src_x = dest_x as i32 - dx;
                let src_y = dest_y as i32 - dy;
                if src_x < 0 || src_y < 0 || src_x >= w as i32 || src_y >= h as i32 {
                    continue;
                }
                let src = src_y as usize * w + src_x as usize;
                let selected = coverage[src];
                if selected == 0 {
                    continue;
                }
                let dest = dest_y * w + dest_x;
                out[dest] = blend(snapshot[src], selected, out[dest]);
            }
        }
        out
    }

    fn plane_doc(w: u32, h: u32, mask: &[u8]) -> Document {
        let mut layer = pixel_layer("base", w, h, 40);
        layer.mask = Some(LayerMask {
            rect: mask_rect(0, 0, w as i32, h as i32),
            default_color: 255,
            data: Some(mask.to_vec().into()),
            ..Default::default()
        });
        let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        doc
    }

    /// Fixed-seed LCG so the equivalence case is deterministic.
    fn next_byte(state: &mut u64) -> u8 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (*state >> 33) as u8
    }

    fn moved_mask(
        mask: &[u8],
        coverage: &[u8],
        w: u32,
        h: u32,
        dx: i32,
        dy: i32,
        duplicate: bool,
    ) -> Vec<u8> {
        let mut doc = plane_doc(w, h, mask);
        assert!(move_mask_selection_content(
            &mut doc, "0", coverage, dx, dy, duplicate, 7
        ));
        doc.layers[0]
            .mask
            .as_ref()
            .unwrap()
            .data
            .as_ref()
            .unwrap()
            .to_vec()
    }

    #[test]
    fn bounded_mask_move_equals_the_full_plane_move() {
        let (w, h) = (64usize, 64usize);
        let mut state = 0x5eed_1234_u64;
        let mask: Vec<u8> = (0..w * h).map(|_| next_byte(&mut state)).collect();
        // A quarter-covered plane with soft edges, so partial coverage and the
        // empty border are both exercised.
        let coverage: Vec<u8> = (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                if (12..52).contains(&x) && (10..40).contains(&y) {
                    next_byte(&mut state)
                } else {
                    0
                }
            })
            .collect();
        for (dx, dy) in [(5, -3), (-9, 11), (0, 7), (-70, 3), (70, -70)] {
            for duplicate in [false, true] {
                let expected = reference_move(&mask, &coverage, w, h, dx, dy, duplicate);
                let actual = moved_mask(&mask, &coverage, w as u32, h as u32, dx, dy, duplicate);
                assert_eq!(
                    actual, expected,
                    "bounded move differs from the full-plane loop at ({dx}, {dy}), duplicate={duplicate}"
                );
            }
        }
    }

    #[test]
    fn bounded_mask_move_leaves_outside_pixels_untouched() {
        let (w, h) = (32usize, 32usize);
        let mask: Vec<u8> = (0..w * h).map(|i| (i * 7 % 251) as u8).collect();
        let mut coverage = vec![0u8; w * h];
        for y in 8..24 {
            for x in 8..24 {
                coverage[y * w + x] = 255;
            }
        }
        let (dx, dy) = (4, 4);
        let actual = moved_mask(&mask, &coverage, w as u32, h as u32, dx, dy, false);
        let source = (8, 8, 24, 24);
        let destination = (12, 12, 28, 28);
        for y in 0..h {
            for x in 0..w {
                let inside = ((source.0..source.2).contains(&x)
                    && (source.1..source.3).contains(&y))
                    || ((destination.0..destination.2).contains(&x)
                        && (destination.1..destination.3).contains(&y));
                if !inside {
                    assert_eq!(
                        actual[y * w + x],
                        mask[y * w + x],
                        "pixel ({x}, {y}) outside the move changed"
                    );
                }
            }
        }
    }
}
