//! Editing a raster layer mask's coverage.
//!
//! The paint, fill, and filter engines edit a [`Layer`](pictura_core::Layer)'s
//! colour channels, so a mask is exposed as a standalone mask-sized grayscale
//! document they can edit unchanged: [`mask_document`] builds it and
//! [`write_mask_back`] copies the result into the real mask. The document's
//! single layer is transparency-locked so opacity stays 255 while its colour
//! plane carries the coverage.

use pictura_core::{Document, LockFlags, PsdRect};

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

/// A `w × h` opaque document whose colour plane is `coverage` (or `fill` when
/// absent), with transparency locked so an edit cannot rewrite the opacity.
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
}
