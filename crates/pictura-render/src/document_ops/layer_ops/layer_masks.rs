//! Layer > Layer Mask authoring: create (Reveal All / Hide All / Reveal
//! Selection / Hide Selection / From Transparency), delete, apply, enable, and
//! link. The compositor already honours [`LayerMask`]; these edit the model.

use pictura_core::{Channel, Document, LayerMask, PsdRect};

use super::create::layer_from_background;
use super::paths::{resolve_path, resolve_path_mut};

/// The mask kind [`add_layer_mask`] creates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerMaskKind {
    RevealAll,
    HideAll,
    RevealSelection,
    HideSelection,
    FromTransparency,
}

/// PSD layer-mask flags bit 0: the mask position is linked to (moves with) the
/// layer. Bit 1 is the disabled flag and lives on [`LayerMask::disabled`]; the
/// remaining bits are preserved verbatim.
pub const MASK_FLAG_LINKED: u8 = 0x01;

fn area(rect: PsdRect) -> usize {
    rect.width().max(0) as usize * rect.height().max(0) as usize
}

/// A coverage plane's samples, or `default_color` repeated for a data-less mask.
fn coverage_of(mask: &LayerMask) -> Vec<u8> {
    match &mask.data {
        Some(data) => data.as_slice().to_vec(),
        None => vec![mask.default_color; area(mask.rect)],
    }
}

/// Add a raster layer mask to the layer at `path`. Reveal All, Hide All, and
/// From Transparency size the mask to the layer's content rect; the selection
/// variants size it to `selection` and use its coverage (inverted for Hide
/// Selection). Does nothing and returns false when the layer already has a mask,
/// the path does not resolve, or a selection variant has no selection.
pub fn add_layer_mask(
    doc: &mut Document,
    path: &str,
    kind: LayerMaskKind,
    selection: Option<&LayerMask>,
) -> bool {
    let doc_rect = PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    if layer.mask.is_some() {
        return false;
    }
    // A group has no content rectangle of its own, and an adjustment or fill
    // layer may carry a zero-area rect; both take a document-sized mask.
    let content_rect = if layer.is_group || layer.rect.width() <= 0 || layer.rect.height() <= 0 {
        doc_rect
    } else {
        layer.rect
    };
    let (rect, data, default_color) = match kind {
        LayerMaskKind::RevealAll => (content_rect, vec![255; area(content_rect)], 255),
        LayerMaskKind::HideAll => (content_rect, vec![0; area(content_rect)], 0),
        LayerMaskKind::FromTransparency => {
            let pixels = area(content_rect);
            let data = layer
                .channels
                .iter()
                .find(|channel| channel.id == -1)
                .map(|channel| channel.data.as_slice().to_vec())
                .filter(|data| data.len() == pixels)
                .unwrap_or_else(|| vec![255; pixels]);
            (content_rect, data, 255)
        }
        LayerMaskKind::RevealSelection => {
            let Some(selection) = selection else {
                return false;
            };
            (selection.rect, coverage_of(selection), 0)
        }
        LayerMaskKind::HideSelection => {
            let Some(selection) = selection else {
                return false;
            };
            let mut data = coverage_of(selection);
            for value in &mut data {
                *value = 255 - *value;
            }
            (selection.rect, data, 255)
        }
    };
    layer.mask = Some(LayerMask {
        rect,
        default_color,
        disabled: false,
        flags: MASK_FLAG_LINKED,
        data: Some(data.into()),
        extra: Vec::new().into(),
        ..Default::default()
    });
    true
}

/// Add a layer mask and, only when the add succeeds, convert a Background layer
/// to an ordinary layer (as CS6 does, so the four forced locks do not keep the
/// mask unusable). A refused add — an existing mask, a selection variant with no
/// selection, or an unresolved path — leaves the Background untouched.
pub fn add_layer_mask_converting_background(
    doc: &mut Document,
    path: &str,
    kind: LayerMaskKind,
    selection: Option<&LayerMask>,
) -> bool {
    if !add_layer_mask(doc, path, kind, selection) {
        return false;
    }
    layer_from_background(doc, path);
    true
}

/// Install `mask` on the layer at `path`, replacing any mask it has, and report
/// whether the path resolved. A retained native `-2` plane is dropped so the
/// compositor cannot prefer the old high-depth mask over this 8-bit one.
pub fn replace_layer_mask(doc: &mut Document, path: &str, mask: LayerMask) -> bool {
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    layer.mask = Some(mask);
    if let Some(source) = layer.source_channels.as_mut() {
        source.planes.retain(|(id, _)| *id != -2);
    }
    true
}

/// Remove the layer mask at `path`, reporting whether one existed.
pub fn delete_layer_mask(doc: &mut Document, path: &str) -> bool {
    resolve_path_mut(doc, path).is_some_and(|layer| layer.mask.take().is_some())
}

/// Fold the mask coverage into the layer's alpha and clear the mask. Refused on
/// a smart-object layer, which cannot take a permanent mask, and on a group or
/// an adjustment/fill layer, which renders from its children or its content
/// rather than its own alpha, so dropping the mask would reveal what it hid. A
/// disabled mask is a no-op on the pixels but is still cleared.
pub fn apply_layer_mask(doc: &mut Document, path: &str) -> bool {
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    if layer.smart_object.is_some() || layer.is_group || layer.adjustment.is_some() {
        return false;
    }
    let Some(mask) = layer.mask.take() else {
        return false;
    };
    let rect = layer.rect;
    let (width, height) = (rect.width().max(0) as usize, rect.height().max(0) as usize);
    if width == 0 || height == 0 {
        return true;
    }
    let alpha = match layer.channels.iter().position(|channel| channel.id == -1) {
        Some(index) => index,
        None => {
            layer.channels.push(Channel {
                id: -1,
                data: vec![255; width * height].into(),
            });
            layer.channels.len() - 1
        }
    };
    for row in 0..height {
        for col in 0..width {
            let x = rect.left + col as i32;
            let y = rect.top + row as i32;
            let coverage = u32::from(crate::mask_refine::raster(&mask, rect, x, y));
            let index = row * width + col;
            if let Some(sample) = layer.channels[alpha].data.get_mut(index) {
                *sample = ((u32::from(*sample) * coverage + 127) / 255) as u8;
            }
        }
    }
    true
}

/// Set the mask's density (PSD 0-255; 255 is the mask as painted). Returns
/// false without a mask or with no change.
pub fn set_layer_mask_density(doc: &mut Document, path: &str, density: u8) -> bool {
    let Some(mask) = resolve_path_mut(doc, path).and_then(|layer| layer.mask.as_mut()) else {
        return false;
    };
    if mask.density == density {
        return false;
    }
    mask.density = density;
    true
}

/// Set the mask's feather radius in pixels. Returns false without a mask, for a
/// negative or non-finite radius, or with no change.
pub fn set_layer_mask_feather(doc: &mut Document, path: &str, radius: f64) -> bool {
    if !radius.is_finite() || radius < 0.0 {
        return false;
    }
    let Some(mask) = resolve_path_mut(doc, path).and_then(|layer| layer.mask.as_mut()) else {
        return false;
    };
    if mask.feather() == radius {
        return false;
    }
    mask.set_feather(radius);
    true
}

/// Invert the mask's pixels and its default colour in place, the Properties
/// panel's Invert. Returns false without a mask.
pub fn invert_layer_mask(doc: &mut Document, path: &str) -> bool {
    let Some(mask) = resolve_path_mut(doc, path).and_then(|layer| layer.mask.as_mut()) else {
        return false;
    };
    mask.default_color = 255 - mask.default_color;
    if let Some(data) = mask.data.as_mut() {
        for v in data.as_mut_slice() {
            *v = 255 - *v;
        }
    }
    true
}

/// The mask's density (0-255) and feather radius in pixels; `None` without a
/// mask.
pub fn layer_mask_refinement(doc: &Document, path: &str) -> Option<(u8, f64)> {
    let mask = resolve_path(doc, path)?.mask.as_ref()?;
    Some((mask.density, mask.feather()))
}

/// Set the mask's enabled state (`enabled == false` sets its disabled bit).
/// Returns false when there is no mask or the state did not change.
pub fn set_layer_mask_enabled(doc: &mut Document, path: &str, enabled: bool) -> bool {
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    let Some(mask) = layer.mask.as_mut() else {
        return false;
    };
    let disabled = !enabled;
    if mask.disabled == disabled {
        return false;
    }
    mask.disabled = disabled;
    true
}

/// Set the mask's linked state by writing only [`MASK_FLAG_LINKED`], preserving
/// every other flags bit. Returns false without a mask or with no change.
pub fn set_layer_mask_linked(doc: &mut Document, path: &str, linked: bool) -> bool {
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    let Some(mask) = layer.mask.as_mut() else {
        return false;
    };
    let flags = if linked {
        mask.flags | MASK_FLAG_LINKED
    } else {
        mask.flags & !MASK_FLAG_LINKED
    };
    if flags == mask.flags {
        return false;
    }
    mask.flags = flags;
    true
}

/// Whether the layer at `path` has a raster layer mask.
pub fn has_layer_mask(doc: &Document, path: &str) -> bool {
    resolve_path(doc, path).is_some_and(|layer| layer.mask.is_some())
}

/// Whether the layer at `path`'s mask is linked to the layer.
pub fn layer_mask_linked(doc: &Document, path: &str) -> bool {
    resolve_path(doc, path)
        .and_then(|layer| layer.mask.as_ref())
        .is_some_and(|mask| mask.flags & MASK_FLAG_LINKED != 0)
}

/// Whether the layer at `path`'s mask is disabled (its pixels are ignored).
pub fn layer_mask_disabled(doc: &Document, path: &str) -> bool {
    resolve_path(doc, path)
        .and_then(|layer| layer.mask.as_ref())
        .is_some_and(|mask| mask.disabled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composite_rgba;
    use pictura_core::{PixelBuffer, SmartObject};

    fn doc(width: u32, height: u32) -> Document {
        let rgba = vec![255u8; (width * height * 4) as usize];
        Document::from_rgba("L", width, height, &rgba)
    }

    fn full(width: i32, height: i32) -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: height,
            right: width,
        }
    }

    fn alpha(buf: &PixelBuffer, x: u32, y: u32) -> u8 {
        let plane = (buf.width * buf.height) as usize;
        buf.data[3 * plane + (y * buf.width + x) as usize]
    }

    fn set_layer_alpha(doc: &mut Document, values: &[u8]) {
        let alpha = doc.layers[0]
            .channels
            .iter_mut()
            .find(|channel| channel.id == -1)
            .expect("layer has alpha");
        alpha.data.as_mut_slice().copy_from_slice(values);
    }

    fn selection(data: &[u8]) -> LayerMask {
        LayerMask {
            rect: full(data.len() as i32, 1),
            default_color: 0,
            data: Some(data.to_vec().into()),
            ..Default::default()
        }
    }

    #[test]
    fn density_reveals_a_hidden_layer_and_reports_changes() {
        let mut doc = doc(2, 2);
        assert!(!set_layer_mask_density(&mut doc, "0", 128), "no mask");
        assert!(add_layer_mask(&mut doc, "0", LayerMaskKind::HideAll, None));
        assert_eq!(alpha(&composite_rgba(&doc), 0, 0), 0);
        assert!(set_layer_mask_density(&mut doc, "0", 128));
        assert!(!set_layer_mask_density(&mut doc, "0", 128), "unchanged");
        assert_eq!(alpha(&composite_rgba(&doc), 0, 0), 127);
        assert_eq!(layer_mask_refinement(&doc, "0"), Some((128, 0.0)));
    }

    #[test]
    fn feather_rejects_unusable_radii() {
        let mut doc = doc(2, 2);
        assert!(add_layer_mask(
            &mut doc,
            "0",
            LayerMaskKind::RevealAll,
            None
        ));
        for bad in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(!set_layer_mask_feather(&mut doc, "0", bad), "{bad}");
        }
        assert!(set_layer_mask_feather(&mut doc, "0", 2.5));
        assert!(!set_layer_mask_feather(&mut doc, "0", 2.5), "unchanged");
        assert_eq!(layer_mask_refinement(&doc, "0"), Some((255, 2.5)));
        assert_eq!(layer_mask_refinement(&doc, "9"), None);
    }

    #[test]
    fn invert_flips_pixels_and_default_colour() {
        let mut doc = doc(2, 1);
        assert!(!invert_layer_mask(&mut doc, "0"), "no mask");
        doc.layers[0].mask = Some(selection(&[40, 255]));
        assert!(invert_layer_mask(&mut doc, "0"));
        let mask = doc.layers[0].mask.as_ref().unwrap();
        assert_eq!(mask.data.as_deref(), Some(&[215u8, 0][..]));
        assert_eq!(mask.default_color, 255);
        assert_eq!(alpha(&composite_rgba(&doc), 1, 0), 0);
    }

    #[test]
    fn apply_bakes_the_refined_mask() {
        let mut doc = doc(2, 2);
        assert!(add_layer_mask(&mut doc, "0", LayerMaskKind::HideAll, None));
        assert!(set_layer_mask_density(&mut doc, "0", 128));
        let shown = composite_rgba(&doc).data;
        assert!(apply_layer_mask(&mut doc, "0"));
        assert_eq!(
            composite_rgba(&doc).data,
            shown,
            "the composite is unchanged"
        );
    }

    #[test]
    fn add_creates_one_sized_to_the_layer_and_refuses_a_second() {
        let mut doc = doc(2, 2);
        let plain = composite_rgba(&doc).data;
        assert!(add_layer_mask(
            &mut doc,
            "0",
            LayerMaskKind::RevealAll,
            None
        ));
        let mask = doc.layers[0].mask.as_ref().unwrap();
        assert_eq!(mask.rect, full(2, 2));
        assert_eq!(mask.default_color, 255);
        assert_eq!(mask.data.as_deref(), Some(&[255u8; 4][..]));
        assert!(has_layer_mask(&doc, "0"));
        assert_eq!(composite_rgba(&doc).data, plain, "reveal all is a no-op");
        assert!(!add_layer_mask(&mut doc, "0", LayerMaskKind::HideAll, None));
        assert_eq!(doc.layers[0].mask.as_ref().unwrap().default_color, 255);
        assert!(!add_layer_mask(
            &mut doc,
            "9",
            LayerMaskKind::RevealAll,
            None
        ));
    }

    #[test]
    fn add_layer_mask_group_is_document_sized() {
        let mut group = doc(4, 3);
        group.layers[0].is_group = true;
        group.layers[0].rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        };
        assert!(add_layer_mask(
            &mut group,
            "0",
            LayerMaskKind::RevealAll,
            None
        ));
        let mask = group.layers[0].mask.as_ref().unwrap();
        assert_eq!(mask.rect, full(4, 3));
        let data = mask.data.as_ref().unwrap();
        assert_eq!(data.len(), 12);
        assert!(data.iter().all(|&v| v == 255));

        let mut hidden = doc(4, 3);
        hidden.layers[0].is_group = true;
        hidden.layers[0].rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        };
        assert!(add_layer_mask(
            &mut hidden,
            "0",
            LayerMaskKind::HideAll,
            None
        ));
        let mask = hidden.layers[0].mask.as_ref().unwrap();
        assert_eq!(mask.rect, full(4, 3));
        assert!(mask.data.as_ref().unwrap().iter().all(|&v| v == 0));
    }

    #[test]
    fn a_zero_rect_layer_mask_is_document_sized() {
        let mut doc = doc(4, 3);
        doc.layers[0].rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        };
        assert!(add_layer_mask(
            &mut doc,
            "0",
            LayerMaskKind::RevealAll,
            None
        ));
        let mask = doc.layers[0].mask.as_ref().unwrap();
        assert_eq!(mask.rect, full(4, 3));
        assert_eq!(mask.data.as_ref().unwrap().len(), 12);
    }

    #[test]
    fn a_refused_mask_add_does_not_convert_the_background() {
        let mut doc = doc(2, 2);
        doc.layers[0].background = true;

        // Reveal Selection with no selection is refused: the Background stays.
        assert!(!add_layer_mask_converting_background(
            &mut doc,
            "0",
            LayerMaskKind::RevealSelection,
            None
        ));
        assert!(
            doc.layers[0].background,
            "refused add leaves the Background"
        );
        assert!(!has_layer_mask(&doc, "0"));

        // A successful add converts it to an ordinary layer.
        assert!(add_layer_mask_converting_background(
            &mut doc,
            "0",
            LayerMaskKind::RevealAll,
            None
        ));
        assert!(!doc.layers[0].background, "successful add converts");
        assert!(has_layer_mask(&doc, "0"));
    }

    #[test]
    fn hide_all_zeroes_the_composite_and_delete_restores_it() {
        let mut doc = doc(2, 2);
        assert!(add_layer_mask(&mut doc, "0", LayerMaskKind::HideAll, None));
        assert_eq!(doc.layers[0].mask.as_ref().unwrap().default_color, 0);
        assert_eq!(alpha(&composite_rgba(&doc), 0, 0), 0);
        assert!(delete_layer_mask(&mut doc, "0"));
        assert!(!has_layer_mask(&doc, "0"));
        assert_eq!(alpha(&composite_rgba(&doc), 0, 0), 255);
        assert!(!delete_layer_mask(&mut doc, "0"));
    }

    #[test]
    fn from_transparency_derives_coverage_from_alpha() {
        let mut doc = doc(2, 1);
        set_layer_alpha(&mut doc, &[0, 200]);
        assert!(add_layer_mask(
            &mut doc,
            "0",
            LayerMaskKind::FromTransparency,
            None
        ));
        let mask = doc.layers[0].mask.as_ref().unwrap();
        assert_eq!(mask.data.as_deref(), Some(&[0u8, 200][..]));
        assert_eq!(alpha(&composite_rgba(&doc), 0, 0), 0);
    }

    #[test]
    fn selection_variants_use_the_supplied_coverage() {
        let pick = selection(&[255, 0]);

        let mut reveal = doc(2, 1);
        assert!(add_layer_mask(
            &mut reveal,
            "0",
            LayerMaskKind::RevealSelection,
            Some(&pick)
        ));
        let mask = reveal.layers[0].mask.as_ref().unwrap();
        assert_eq!(mask.data.as_deref(), Some(&[255u8, 0][..]));
        assert_eq!(mask.default_color, 0);
        assert_eq!(alpha(&composite_rgba(&reveal), 0, 0), 255);
        assert_eq!(alpha(&composite_rgba(&reveal), 1, 0), 0);

        let mut hide = doc(2, 1);
        assert!(add_layer_mask(
            &mut hide,
            "0",
            LayerMaskKind::HideSelection,
            Some(&pick)
        ));
        let mask = hide.layers[0].mask.as_ref().unwrap();
        assert_eq!(mask.data.as_deref(), Some(&[0u8, 255][..]));
        assert_eq!(mask.default_color, 255);
        assert_eq!(alpha(&composite_rgba(&hide), 0, 0), 0);
        assert_eq!(alpha(&composite_rgba(&hide), 1, 0), 255);

        let mut none = doc(2, 1);
        assert!(!add_layer_mask(
            &mut none,
            "0",
            LayerMaskKind::RevealSelection,
            None
        ));
        assert!(!has_layer_mask(&none, "0"));
    }

    #[test]
    fn apply_folds_coverage_into_alpha_and_clears_the_mask() {
        let mut doc = doc(2, 1);
        assert!(add_layer_mask(
            &mut doc,
            "0",
            LayerMaskKind::RevealSelection,
            Some(&selection(&[255, 0]))
        ));
        let masked = composite_rgba(&doc);
        assert!(apply_layer_mask(&mut doc, "0"));
        assert!(!has_layer_mask(&doc, "0"));
        let applied = composite_rgba(&doc);
        for (before, after) in masked.data.iter().zip(applied.data.iter()) {
            assert!(
                (*before as i32 - *after as i32).abs() <= 1,
                "apply changed the composite: {before} vs {after}"
            );
        }
        assert!(!apply_layer_mask(&mut doc, "0"), "no mask left");
    }

    #[test]
    fn apply_refuses_a_smart_object() {
        let mut doc = doc(2, 1);
        assert!(add_layer_mask(&mut doc, "0", LayerMaskKind::HideAll, None));
        doc.layers[0].smart_object = Some(SmartObject::default());
        assert!(!apply_layer_mask(&mut doc, "0"));
        assert!(has_layer_mask(&doc, "0"));
    }

    #[test]
    fn apply_refuses_a_group_and_a_fill_layer() {
        let mut group = doc(2, 1);
        assert!(add_layer_mask(
            &mut group,
            "0",
            LayerMaskKind::HideAll,
            None
        ));
        group.layers[0].is_group = true;
        assert!(!apply_layer_mask(&mut group, "0"));
        assert!(has_layer_mask(&group, "0"), "the mask keeps hiding");

        let mut fill = doc(2, 1);
        assert!(add_layer_mask(&mut fill, "0", LayerMaskKind::HideAll, None));
        fill.layers[0].adjustment = Some(crate::encode_solid_color_fill([255, 0, 0]));
        assert!(!apply_layer_mask(&mut fill, "0"));
        assert!(has_layer_mask(&fill, "0"));
    }

    #[test]
    fn set_enabled_toggles_the_disabled_bit() {
        let mut bare = doc(2, 1);
        assert!(!set_layer_mask_enabled(&mut bare, "0", false));
        let mut doc = doc(2, 1);
        assert!(add_layer_mask(&mut doc, "0", LayerMaskKind::HideAll, None));
        assert_eq!(alpha(&composite_rgba(&doc), 0, 0), 0);
        assert!(set_layer_mask_enabled(&mut doc, "0", false));
        assert!(doc.layers[0].mask.as_ref().unwrap().disabled);
        assert_eq!(alpha(&composite_rgba(&doc), 0, 0), 255);
        assert!(!set_layer_mask_enabled(&mut doc, "0", false));
        assert!(set_layer_mask_enabled(&mut doc, "0", true));
        assert!(!doc.layers[0].mask.as_ref().unwrap().disabled);
        assert_eq!(alpha(&composite_rgba(&doc), 0, 0), 0);
    }

    #[test]
    fn set_linked_round_trips_the_flag_bit_and_preserves_the_rest() {
        let mut doc = doc(2, 1);
        assert!(add_layer_mask(
            &mut doc,
            "0",
            LayerMaskKind::RevealAll,
            None
        ));
        assert!(layer_mask_linked(&doc, "0"), "a new mask is linked");
        doc.layers[0].mask.as_mut().unwrap().flags = 0x0E;
        assert!(set_layer_mask_linked(&mut doc, "0", true));
        assert_eq!(doc.layers[0].mask.as_ref().unwrap().flags, 0x0F);
        assert!(layer_mask_linked(&doc, "0"));
        assert!(!set_layer_mask_linked(&mut doc, "0", true));
        assert!(set_layer_mask_linked(&mut doc, "0", false));
        assert_eq!(doc.layers[0].mask.as_ref().unwrap().flags, 0x0E);
        assert!(!layer_mask_linked(&doc, "0"));
    }

    #[test]
    fn reads_report_absence_without_a_mask_or_path() {
        let doc = doc(2, 1);
        assert!(!has_layer_mask(&doc, "0"));
        assert!(!layer_mask_linked(&doc, "0"));
        assert!(!layer_mask_disabled(&doc, "0"));
        assert!(!has_layer_mask(&doc, "9"));
        assert!(!layer_mask_linked(&doc, "9"));
        assert!(!layer_mask_disabled(&doc, "9"));
    }

    #[test]
    fn disabled_read_tracks_the_disabled_bit() {
        let mut doc = doc(2, 1);
        assert!(add_layer_mask(&mut doc, "0", LayerMaskKind::HideAll, None));
        assert!(!layer_mask_disabled(&doc, "0"), "a new mask is enabled");
        assert!(set_layer_mask_enabled(&mut doc, "0", false));
        assert!(layer_mask_disabled(&doc, "0"));
        assert!(set_layer_mask_enabled(&mut doc, "0", true));
        assert!(!layer_mask_disabled(&doc, "0"));
    }
}
