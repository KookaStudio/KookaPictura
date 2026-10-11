//! Layer > Vector Mask authoring: create (Reveal All / Hide All / Current Path),
//! delete, enable/disable, link/unlink, and rasterize to a layer mask. The
//! compositor already honours [`pictura_core::VectorMask`]; these edit the
//! layer's preserved `vmsk` block, which stays the source of truth.

use pictura_core::path::{PathPoint, Subpath};
use pictura_core::{Document, Layer, LayerBlock, LayerMask, PsdRect};

use super::layer_masks::MASK_FLAG_LINKED;
use super::paths::{resolve_path, resolve_path_mut};

/// The `vmsk` additional-layer-information key.
const VMSK: &[u8; 4] = b"vmsk";

/// The vector mask kind [`add_vector_mask`] creates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorMaskKind {
    RevealAll,
    HideAll,
    CurrentPath,
}

/// PSD `vmsk` flag word bits (Adobe PSD spec, one-indexed): bit 1 invert, bit 2
/// not-link, bit 3 disable. Decoded by `pictura_codec::decode_vector_mask`.
pub const VECTOR_MASK_FLAG_INVERT: u32 = 0x01;
pub const VECTOR_MASK_FLAG_NOT_LINKED: u32 = 0x02;
pub const VECTOR_MASK_FLAG_DISABLED: u32 = 0x04;

/// The flags word of an encoded `vmsk` block (bytes `4..8`), or 0 when absent.
fn flags_of(data: &[u8]) -> u32 {
    if data.len() < 8 {
        return 0;
    }
    u32::from_be_bytes([data[4], data[5], data[6], data[7]])
}

/// Overwrite an encoded `vmsk` block's flags word.
fn write_flags(data: &mut [u8], flags: u32) {
    if data.len() >= 8 {
        data[4..8].copy_from_slice(&flags.to_be_bytes());
    }
}

/// Replace (`Some`) or remove (`None`) the layer's `vmsk` block.
fn set_vmsk_block(layer: &mut Layer, data: Option<Vec<u8>>) -> bool {
    let slot = layer.extra_blocks.iter().position(|b| &b.key == VMSK);
    match (slot, data) {
        (Some(i), Some(data)) => {
            layer.extra_blocks[i].data = data;
            true
        }
        (None, Some(data)) => {
            layer.extra_blocks.push(LayerBlock { key: *VMSK, data });
            true
        }
        (Some(i), None) => {
            layer.extra_blocks.remove(i);
            true
        }
        (None, None) => false,
    }
}

/// Set or clear one flags bit in the existing `vmsk` block and re-decode the
/// view from it, preserving the encoded path bytes exactly. Returns whether the
/// bit changed.
fn patch_flag(layer: &mut Layer, bit: u32, on: bool, width: u32, height: u32) -> bool {
    let Some(index) = layer.extra_blocks.iter().position(|b| &b.key == VMSK) else {
        return false;
    };
    let flags = flags_of(&layer.extra_blocks[index].data);
    let updated = if on { flags | bit } else { flags & !bit };
    if updated == flags {
        return false;
    }
    let view = {
        let data = &mut layer.extra_blocks[index].data;
        write_flags(data, updated);
        pictura_codec::decode_vector_mask(data, width, height)
    };
    layer.vector_mask = view;
    true
}

/// Re-encode a layer's in-memory `VectorMask` view into its `vmsk` block,
/// preserving the block's flags (invert / disabled / link). An external geometry
/// operation such as rotation mutates the view directly, so this keeps the
/// authoritative raw block from going stale on save. Returns false without a
/// view. `ponytail:` re-encodes the flattened polyline, so a rotated curved
/// mask serializes as its flattened vertices — the compositor already renders
/// that polyline, so the result stays consistent.
pub fn sync_vector_mask_block(layer: &mut Layer, width: u32, height: u32) -> bool {
    let Some(view) = layer.vector_mask.as_ref() else {
        return false;
    };
    let subpaths = view.subpaths.clone();
    let mut view_flags = 0u32;
    if view.invert {
        view_flags |= VECTOR_MASK_FLAG_INVERT;
    }
    if view.disabled {
        view_flags |= VECTOR_MASK_FLAG_DISABLED;
    }
    let flags = layer
        .extra_blocks
        .iter()
        .find(|b| &b.key == VMSK)
        .map(|b| flags_of(&b.data))
        .unwrap_or(view_flags);
    let mut data = pictura_codec::encode_vector_mask_view(&subpaths, width, height);
    write_flags(&mut data, flags);
    set_vmsk_block(layer, Some(data))
}

/// A closed rectangle subpath over `rect`, in document pixels.
fn rect_subpath(rect: PsdRect) -> Subpath {
    let corner = |x: i32, y: i32| PathPoint {
        anchor: (x as f64, y as f64),
        in_handle: None,
        out_handle: None,
        smooth: false,
    };
    Subpath {
        points: vec![
            corner(rect.left, rect.top),
            corner(rect.right, rect.top),
            corner(rect.right, rect.bottom),
            corner(rect.left, rect.bottom),
        ],
        closed: true,
    }
}

/// Add a vector mask to the layer at `path`. Reveal All authors a full-content
/// rectangle; Hide All authors the same rectangle with the invert flag; Current
/// Path authors the document's work path. Does nothing and returns false when
/// the layer already has a vector mask, the path does not resolve, or Current
/// Path is requested with an empty work path.
pub fn add_vector_mask(doc: &mut Document, path: &str, kind: VectorMaskKind) -> bool {
    let (width, height) = (doc.width, doc.height);
    let (subpaths, flags) = match kind {
        VectorMaskKind::CurrentPath => {
            if doc.work_path.is_empty() {
                return false;
            }
            (doc.work_path.subpaths.clone(), 0)
        }
        VectorMaskKind::RevealAll | VectorMaskKind::HideAll => {
            let Some(layer) = resolve_path(doc, path) else {
                return false;
            };
            if layer.vector_mask.is_some() {
                return false;
            }
            let invert = matches!(kind, VectorMaskKind::HideAll);
            let flags = if invert { VECTOR_MASK_FLAG_INVERT } else { 0 };
            (vec![rect_subpath(layer.rect)], flags)
        }
    };
    if subpaths.is_empty() {
        return false;
    }
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    if layer.vector_mask.is_some() {
        return false;
    }
    let mut data = pictura_codec::encode_vector_mask(&subpaths, width, height);
    write_flags(&mut data, flags);
    layer.vector_mask = pictura_codec::decode_vector_mask(&data, width, height);
    set_vmsk_block(layer, Some(data));
    true
}

/// Remove the vector mask at `path`, reporting whether one existed.
pub fn delete_vector_mask(doc: &mut Document, path: &str) -> bool {
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    if layer.vector_mask.take().is_none() {
        return false;
    }
    set_vmsk_block(layer, None);
    true
}

/// Set the vector mask's enabled state (`enabled == false` sets its disabled
/// bit). Returns false when there is no mask or the state did not change.
pub fn set_vector_mask_enabled(doc: &mut Document, path: &str, enabled: bool) -> bool {
    let (width, height) = (doc.width, doc.height);
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    let Some(mask) = layer.vector_mask.as_ref() else {
        return false;
    };
    let disabled = !enabled;
    if mask.disabled == disabled {
        return false;
    }
    patch_flag(layer, VECTOR_MASK_FLAG_DISABLED, disabled, width, height)
}

/// Set the vector mask's linked state. The PSD "not linked" bit is only set or
/// cleared; every other flag bit and the path bytes are preserved. Returns
/// false without a mask or with no change.
pub fn set_vector_mask_linked(doc: &mut Document, path: &str, linked: bool) -> bool {
    let (width, height) = (doc.width, doc.height);
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    if layer.vector_mask.is_none() {
        return false;
    }
    patch_flag(layer, VECTOR_MASK_FLAG_NOT_LINKED, !linked, width, height)
}

/// Convert the layer's vector mask into a raster layer mask and drop the vector
/// mask. The new mask is the product of the vector coverage and any existing
/// layer mask's coverage over the layer's content rectangle, so the composite
/// is unchanged. Returns false without a vector mask.
pub fn rasterize_vector_mask(doc: &mut Document, path: &str) -> bool {
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    let Some(vector) = layer.vector_mask.take() else {
        return false;
    };
    set_vmsk_block(layer, None);
    let rect = layer.rect;
    let width = rect.width().max(0) as usize;
    let height = rect.height().max(0) as usize;
    if width == 0 || height == 0 {
        return true;
    }
    let existing = layer.mask.take();
    let mut data = vec![255u8; width * height];
    for row in 0..height {
        for col in 0..width {
            let x = rect.left + col as i32;
            let y = rect.top + row as i32;
            let vector = u32::from(crate::vector_mask::coverage(Some(&vector), x, y));
            let raster = existing.as_ref().map_or(255, |mask| {
                u32::from(crate::composite::mask_value(mask, x, y))
            });
            data[row * width + col] = ((vector * raster + 127) / 255) as u8;
        }
    }
    layer.mask = Some(LayerMask {
        rect,
        default_color: 255,
        disabled: false,
        flags: MASK_FLAG_LINKED,
        data: Some(data.into()),
        extra: Vec::new(),
    });
    true
}

/// Whether the layer at `path` has a vector mask.
pub fn has_vector_mask(doc: &Document, path: &str) -> bool {
    resolve_path(doc, path).is_some_and(|layer| layer.vector_mask.is_some())
}

/// Whether the layer at `path`'s vector mask is linked. A new mask is linked.
pub fn vector_mask_linked(doc: &Document, path: &str) -> bool {
    resolve_path(doc, path)
        .and_then(|layer| layer.extra_block(VMSK))
        .is_some_and(|block| flags_of(&block.data) & VECTOR_MASK_FLAG_NOT_LINKED == 0)
}

/// Whether the layer at `path`'s vector mask is disabled (its pixels ignored).
pub fn vector_mask_disabled(doc: &Document, path: &str) -> bool {
    resolve_path(doc, path)
        .and_then(|layer| layer.vector_mask.as_ref())
        .is_some_and(|mask| mask.disabled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composite_rgba;
    use pictura_core::PixelBuffer;

    fn doc(width: u32, height: u32) -> Document {
        let rgba = vec![255u8; (width * height * 4) as usize];
        Document::from_rgba("L", width, height, &rgba)
    }

    fn alpha(buf: &PixelBuffer, x: u32, y: u32) -> u8 {
        let plane = (buf.width * buf.height) as usize;
        buf.data[3 * plane + (y * buf.width + x) as usize]
    }

    #[test]
    fn add_reveal_all_authors_a_rect_and_refuses_a_second() {
        let mut doc = doc(4, 4);
        let plain = composite_rgba(&doc).data;
        assert!(add_vector_mask(&mut doc, "0", VectorMaskKind::RevealAll));
        assert!(has_vector_mask(&doc, "0"));
        assert!(vector_mask_linked(&doc, "0"), "a new mask is linked");
        assert!(!vector_mask_disabled(&doc, "0"));
        assert_eq!(composite_rgba(&doc).data, plain, "reveal all is a no-op");
        assert!(!add_vector_mask(&mut doc, "0", VectorMaskKind::RevealAll));
        assert!(!add_vector_mask(&mut doc, "9", VectorMaskKind::RevealAll));
    }

    #[test]
    fn hide_all_hides_the_composite_and_delete_restores_it() {
        let mut doc = doc(4, 4);
        assert!(add_vector_mask(&mut doc, "0", VectorMaskKind::HideAll));
        assert_eq!(alpha(&composite_rgba(&doc), 0, 0), 0);
        assert!(delete_vector_mask(&mut doc, "0"));
        assert!(!has_vector_mask(&doc, "0"));
        assert_eq!(alpha(&composite_rgba(&doc), 0, 0), 255);
        assert!(!delete_vector_mask(&mut doc, "0"), "no mask left");
    }

    #[test]
    fn current_path_authors_the_work_path() {
        let mut doc = doc(4, 4);
        assert!(!add_vector_mask(&mut doc, "0", VectorMaskKind::CurrentPath));
        let corner = |x: f64, y: f64| PathPoint {
            anchor: (x, y),
            in_handle: None,
            out_handle: None,
            smooth: false,
        };
        doc.work_path.subpaths.push(Subpath {
            points: vec![
                corner(1.0, 1.0),
                corner(3.0, 1.0),
                corner(3.0, 3.0),
                corner(1.0, 3.0),
            ],
            closed: true,
        });
        assert!(add_vector_mask(&mut doc, "0", VectorMaskKind::CurrentPath));
        assert!(has_vector_mask(&doc, "0"));
        assert_eq!(alpha(&composite_rgba(&doc), 0, 0), 0, "outside the path");
        assert_eq!(alpha(&composite_rgba(&doc), 2, 2), 255, "inside the path");
    }

    #[test]
    fn set_enabled_toggles_the_bit_and_preserves_the_path() {
        let mut doc = doc(4, 4);
        assert!(!set_vector_mask_enabled(&mut doc, "0", false));
        assert!(add_vector_mask(&mut doc, "0", VectorMaskKind::HideAll));
        let path_before = doc.layers[0].extra_block(VMSK).unwrap().data[8..].to_vec();
        assert_eq!(alpha(&composite_rgba(&doc), 0, 0), 0);
        assert!(set_vector_mask_enabled(&mut doc, "0", false));
        assert!(vector_mask_disabled(&doc, "0"));
        assert_eq!(alpha(&composite_rgba(&doc), 0, 0), 255);
        let path_after = doc.layers[0].extra_block(VMSK).unwrap().data[8..].to_vec();
        assert_eq!(path_before, path_after, "the path bytes are untouched");
        assert!(!set_vector_mask_enabled(&mut doc, "0", false), "no change");
        assert!(set_vector_mask_enabled(&mut doc, "0", true));
        assert!(!vector_mask_disabled(&doc, "0"));
        assert_eq!(alpha(&composite_rgba(&doc), 0, 0), 0);
    }

    #[test]
    fn set_linked_round_trips_and_preserves_other_bits() {
        let mut doc = doc(4, 4);
        assert!(!set_vector_mask_linked(&mut doc, "0", false));
        assert!(add_vector_mask(&mut doc, "0", VectorMaskKind::RevealAll));
        assert!(vector_mask_linked(&doc, "0"));
        // Set an unrelated bit and the invert bit, then unlink.
        {
            let data = &mut doc.layers[0].extra_blocks[0].data;
            let flags = flags_of(data) | VECTOR_MASK_FLAG_INVERT | 0x08;
            write_flags(data, flags);
        }
        assert!(set_vector_mask_linked(&mut doc, "0", false));
        let flags = flags_of(&doc.layers[0].extra_block(VMSK).unwrap().data);
        assert_eq!(
            flags,
            VECTOR_MASK_FLAG_INVERT | 0x08 | VECTOR_MASK_FLAG_NOT_LINKED
        );
        assert!(!vector_mask_linked(&doc, "0"));
        assert!(!set_vector_mask_linked(&mut doc, "0", false), "no change");
        assert!(set_vector_mask_linked(&mut doc, "0", true));
        assert!(vector_mask_linked(&doc, "0"));
    }

    #[test]
    fn reads_report_absence() {
        let doc = doc(4, 4);
        assert!(!has_vector_mask(&doc, "0"));
        assert!(!vector_mask_linked(&doc, "0"));
        assert!(!vector_mask_disabled(&doc, "0"));
        assert!(!has_vector_mask(&doc, "9"));
        assert!(!vector_mask_linked(&doc, "9"));
        assert!(!vector_mask_disabled(&doc, "9"));
    }

    #[test]
    fn rasterize_replaces_the_vector_mask_with_a_layer_mask() {
        let mut doc = doc(4, 4);
        assert!(!rasterize_vector_mask(&mut doc, "0"));
        assert!(add_vector_mask(&mut doc, "0", VectorMaskKind::HideAll));
        let hidden = composite_rgba(&doc).data;
        assert!(rasterize_vector_mask(&mut doc, "0"));
        assert!(!has_vector_mask(&doc, "0"));
        assert!(doc.layers[0].mask.is_some());
        assert_eq!(
            composite_rgba(&doc).data,
            hidden,
            "the composite is unchanged"
        );
        assert!(!rasterize_vector_mask(&mut doc, "0"), "no vector mask left");
    }

    #[test]
    fn rasterize_multiplies_an_existing_layer_mask() {
        let mut doc = doc(4, 4);
        assert!(super::super::layer_masks::add_layer_mask(
            &mut doc,
            "0",
            super::super::layer_masks::LayerMaskKind::HideAll,
            None,
        ));
        assert!(add_vector_mask(&mut doc, "0", VectorMaskKind::RevealAll));
        let before = composite_rgba(&doc).data;
        assert!(rasterize_vector_mask(&mut doc, "0"));
        assert!(doc.layers[0].mask.is_some());
        assert_eq!(composite_rgba(&doc).data, before);
    }

    #[test]
    fn a_disabled_vector_mask_rasterizes_permissively() {
        let mut doc = doc(4, 4);
        assert!(add_vector_mask(&mut doc, "0", VectorMaskKind::HideAll));
        assert!(set_vector_mask_enabled(&mut doc, "0", false));
        let shown = composite_rgba(&doc).data;
        assert!(rasterize_vector_mask(&mut doc, "0"));
        let mask = doc.layers[0].mask.as_ref().unwrap();
        assert_eq!(mask.data.as_deref(), Some(&[255u8; 16][..]));
        assert_eq!(composite_rgba(&doc).data, shown);
    }
}
