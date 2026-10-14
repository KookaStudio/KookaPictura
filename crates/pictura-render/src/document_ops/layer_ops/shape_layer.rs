//! Shape layers: a solid-color fill layer cut to an outline by an authored
//! `vmsk` vector mask (the legacy shape-layer form CS6 reads), optionally live,
//! carrying the parametric shape it was drawn as in a `vogk` block (a CC
//! feature; see `pictura_codec::LiveShape`). The `vmsk` block is the source of
//! truth; the decoded [`pictura_core::VectorMask`] view is rebuilt from it on
//! every write.

use pictura_codec::LiveShape;
use pictura_core::path::Subpath;
use pictura_core::{Document, Layer, LayerBlock, PsdRect};

use super::create::{add_solid_fill, next_layer_name};
use super::paths::resolve_path_mut;
use super::rasterize::is_fill_content_layer;

const VMSK: &[u8; 4] = b"vmsk";
const VOGK: &[u8; 4] = b"vogk";

/// Insert a shape layer above `selection_path`: a `rgba` fill named
/// `"<name> N"`, cut to `outline`, live when `live` is given. Returns the new
/// path, or empty for a zero-dimension document.
pub fn add_shape_layer(
    doc: &mut Document,
    selection_path: &str,
    rgba: [u8; 4],
    name: &str,
    outline: &Subpath,
    live: Option<&LiveShape>,
) -> String {
    let name = next_layer_name(doc, name);
    let (width, height) = (doc.width, doc.height);
    let created = add_solid_fill(doc, selection_path, rgba);
    if let Some(layer) = resolve_path_mut(doc, &created) {
        layer.name = name;
        set_layer_shape_paths(layer, std::slice::from_ref(outline), width, height);
        set_layer_live_shape(layer, live);
    }
    created
}

/// A fill-content layer with a vector mask.
pub fn is_shape_layer(layer: &Layer) -> bool {
    layer.extra_block(VMSK).is_some() && is_fill_content_layer(layer)
}

/// Whether the layer forces Lock Transparency and Lock Image on in the Layers
/// panel (CS6: type and shape layers).
pub fn has_forced_locks(layer: &Layer) -> bool {
    is_shape_layer(layer) || layer.is_type()
}

/// The shape layer's outline as editable subpaths (document pixels); `None`
/// when the layer is not a shape layer or its `vmsk` is malformed.
pub fn layer_shape_paths(layer: &Layer, width: u32, height: u32) -> Option<Vec<Subpath>> {
    if !is_shape_layer(layer) {
        return None;
    }
    pictura_codec::decode_vector_mask_paths(&layer.extra_block(VMSK)?.data, width, height)
}

/// Replace the layer's `vmsk` outline with `subpaths` and rebuild its view.
pub fn set_layer_shape_paths(layer: &mut Layer, subpaths: &[Subpath], width: u32, height: u32) {
    let data = pictura_codec::encode_vector_mask(subpaths, width, height);
    layer.vector_mask = super::vector_masks::keep_params(
        layer.vector_mask.as_ref(),
        pictura_codec::decode_vector_mask(&data, width, height),
    );
    set_block(layer, VMSK, Some(data));
}

/// The live shape the layer was drawn as, or `None` once it is a regular path.
pub fn layer_live_shape(layer: &Layer) -> Option<LiveShape> {
    pictura_codec::decode_live_shape(&layer.extra_block(VOGK)?.data)
}

/// Make the layer live as `live`, or (`None`) a regular path.
pub fn set_layer_live_shape(layer: &mut Layer, live: Option<&LiveShape>) {
    set_block(layer, VOGK, live.map(pictura_codec::encode_live_shape));
}

/// The shape layer's solid fill color; `None` for another fill or layer.
pub fn shape_fill_color(layer: &Layer) -> Option<[u8; 4]> {
    match crate::decode_adjustment(layer.adjustment.as_ref()?)? {
        pictura_adjust::Adjustment::SolidFill(rgba) if is_shape_layer(layer) => Some(rgba),
        _ => None,
    }
}

/// The layer's vector-mask coverage (0–255) at a document pixel; `clip` is the
/// sampled region the coverage's row table is bounded to.
pub fn shape_coverage(layer: &Layer, clip: PsdRect, x: i32, y: i32) -> u8 {
    crate::vector_mask::coverage(layer.vector_mask.as_ref(), clip, x, y)
}

fn set_block(layer: &mut Layer, key: &[u8; 4], data: Option<Vec<u8>>) {
    let slot = layer.extra_blocks.iter().position(|b| &b.key == key);
    match (slot, data) {
        (Some(i), Some(data)) => layer.extra_blocks[i].data = data,
        (None, Some(data)) => layer.extra_blocks.push(LayerBlock { key: *key, data }),
        (Some(i), None) => {
            layer.extra_blocks.remove(i);
        }
        (None, None) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::shape::{outline_in_box, ShapeKind, ShapeOptions};

    #[test]
    fn a_live_shape_layer_edits_its_path_and_goes_regular() {
        let mut doc = Document::from_rgba("Background", 16, 16, &[255; 16 * 16 * 4]);
        let rect = outline_in_box(
            ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3),
            (2.0, 2.0, 8.0, 8.0),
        )
        .unwrap();
        let live = LiveShape {
            origin_type: pictura_codec::ORIGIN_RECTANGLE,
            bounds: (2.0, 2.0, 10.0, 10.0),
            radii: [0.0; 4],
        };
        let path = add_shape_layer(
            &mut doc,
            "",
            [0, 0, 255, 255],
            "Rectangle",
            &rect,
            Some(&live),
        );
        let layer = super::super::paths::resolve_path(&doc, &path)
            .unwrap()
            .clone();
        let clip = PsdRect {
            top: 0,
            left: 0,
            bottom: 16,
            right: 16,
        };
        assert!(is_shape_layer(&layer));
        assert!(!is_shape_layer(&doc.layers[0]));
        assert_eq!(layer_live_shape(&layer), Some(live));
        assert_eq!(shape_coverage(&layer, clip, 5, 5), 255);
        assert_eq!(shape_fill_color(&layer).map(|c| c[2]), Some(255));
        assert_eq!(shape_coverage(&layer, clip, 12, 12), 0);

        let mut paths = layer_shape_paths(&layer, 16, 16).unwrap();
        assert_eq!(paths[0].points[2].anchor, (10.0, 10.0));
        paths[0].points[2].anchor = (14.0, 14.0);
        let mut edited = layer;
        set_layer_shape_paths(&mut edited, &paths, 16, 16);
        set_layer_live_shape(&mut edited, None);
        assert_eq!(
            shape_coverage(&edited, clip, 11, 11),
            255,
            "the outline grew"
        );
        assert_eq!(layer_live_shape(&edited), None);
        assert!(edited.extra_block(b"vogk").is_none());
        assert_eq!(
            edited
                .extra_blocks
                .iter()
                .filter(|b| &b.key == b"vmsk")
                .count(),
            1
        );
    }
}
