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

/// A fill-content layer with a vector mask: its fill comes from a modeled
/// adjustment block (an app-authored shape) or a preserved `vscg` vector-fill
/// block (an imported CC shape).
pub fn is_shape_layer(layer: &Layer) -> bool {
    layer.extra_block(VMSK).is_some()
        && matches!(
            crate::fill::decode_layer_fill(layer),
            Some(
                pictura_adjust::Adjustment::SolidFill(_)
                    | pictura_adjust::Adjustment::GradientFill(_)
                    | pictura_adjust::Adjustment::PatternFill(_)
            )
        )
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
    match crate::fill::decode_layer_fill(layer)? {
        pictura_adjust::Adjustment::SolidFill(rgba) if is_shape_layer(layer) => Some(rgba),
        _ => None,
    }
}

/// The layer's vector-mask coverage (0–255) at a document pixel; `clip` is the
/// sampled region the coverage's row table is bounded to.
pub fn shape_coverage(layer: &Layer, clip: PsdRect, x: i32, y: i32) -> u8 {
    crate::vector_mask::coverage(layer.vector_mask.as_ref(), clip, x, y)
}

/// The box a Move or a transform of the shape layer hugs: its outline bounds
/// rounded out to whole pixels. The layer's own `rect` is the canvas (a fill
/// layer has no pixels), so it cannot stand in. `None` for another layer or an
/// empty outline.
pub fn shape_layer_rect(layer: &Layer, width: u32, height: u32) -> Option<PsdRect> {
    // The `vmsk` fixed-point coordinates and a rotation's `cos` leave a whole
    // pixel a hair off; that must not widen the box by one.
    const SLACK: f64 = 1e-3;
    let (l, t, r, b) = super::shape_style::shape_bounds(layer, width, height)?;
    Some(PsdRect {
        left: (l + SLACK).floor() as i32,
        top: (t + SLACK).floor() as i32,
        right: (r - SLACK).ceil() as i32,
        bottom: (b - SLACK).ceil() as i32,
    })
}

/// Send every anchor and handle of the shape layer at `path` through `map`
/// (document space), as CS6 transforms a shape layer's path rather than its
/// pixels. `translate` is the move a pure translation makes: a live shape
/// moves with it and stays live; any other map leaves a regular path. False
/// (and nothing changes) for another layer, a position lock, or a point the
/// map cannot send.
///
/// ponytail: a projective map or a warp moves the Bézier control points, not
/// the curve itself, so a curved segment is approximated under them.
pub(crate) fn map_shape_layer(
    doc: &mut Document,
    path: &str,
    translate: Option<(f64, f64)>,
    map: impl Fn(f64, f64) -> Option<(f64, f64)>,
) -> bool {
    let (width, height) = (doc.width, doc.height);
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    if pictura_core::layer_move_locked(layer) {
        return false;
    }
    let Some(mut paths) = layer_shape_paths(layer, width, height) else {
        return false;
    };
    let send = |p: (f64, f64)| map(p.0, p.1).filter(|q| q.0.is_finite() && q.1.is_finite());
    for point in paths.iter_mut().flat_map(|s| s.points.iter_mut()) {
        let Some(anchor) = send(point.anchor) else {
            return false;
        };
        point.anchor = anchor;
        for handle in [&mut point.in_handle, &mut point.out_handle] {
            if let Some(h) = *handle {
                let Some(moved) = send(h) else {
                    return false;
                };
                *handle = Some(moved);
            }
        }
    }
    let live = translate.and_then(|(dx, dy)| layer_live_shape(layer).map(|l| l.translated(dx, dy)));
    set_layer_shape_paths(layer, &paths, width, height);
    set_layer_live_shape(layer, live.as_ref());
    true
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

    #[test]
    fn an_imported_vscg_shape_layer_is_recognized_and_colored() {
        let square = pictura_core::shape::outline(
            ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3),
            (1.0, 1.0),
            (5.0, 5.0),
            false,
            false,
        )
        .unwrap();
        let vmsk = pictura_codec::encode_vector_mask(std::slice::from_ref(&square), 8, 8);
        // A preserved `vscg` block: the 4-byte fill key, then the version-16
        // descriptor that `encode_solid_color_fill` builds.
        let fill = crate::encode_solid_color_fill([10, 20, 30]);
        let mut vscg = fill.key.to_vec();
        vscg.extend_from_slice(&fill.data);
        let layer = Layer {
            extra_blocks: vec![
                LayerBlock {
                    key: *b"vmsk",
                    data: vmsk,
                },
                LayerBlock {
                    key: *b"vscg",
                    data: vscg,
                },
            ],
            ..Default::default()
        };
        assert!(is_shape_layer(&layer));
        assert_eq!(shape_fill_color(&layer), Some([10, 20, 30, 255]));
    }

    /// A 20×20 canvas with a live blue rectangle over (2, 4)–(10, 8).
    fn live_rectangle() -> (Document, String) {
        let mut doc = Document::from_rgba("Background", 20, 20, &[255; 20 * 20 * 4]);
        let rect = outline_in_box(
            ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3),
            (2.0, 4.0, 8.0, 4.0),
        )
        .unwrap();
        let live = LiveShape {
            origin_type: pictura_codec::ORIGIN_RECTANGLE,
            bounds: (2.0, 4.0, 10.0, 8.0),
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
        (doc, path)
    }

    fn outline_rect(doc: &Document, path: &str) -> Option<PsdRect> {
        let layer = super::super::paths::resolve_path(doc, path)?;
        shape_layer_rect(layer, doc.width, doc.height)
    }

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
        PsdRect {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn a_shape_layer_moves_and_transforms_its_outline() {
        let (mut doc, path) = live_rectangle();
        assert_eq!(outline_rect(&doc, &path), Some(rect(2, 4, 10, 8)));
        let canvas = super::super::paths::resolve_path(&doc, &path).unwrap().rect;

        // A move shifts the outline, keeps the shape live, and leaves the fill's
        // canvas-sized rect alone.
        assert!(crate::translate_layer_path(&mut doc, &path, 3, 5));
        assert_eq!(outline_rect(&doc, &path), Some(rect(5, 9, 13, 13)));
        let layer = super::super::paths::resolve_path(&doc, &path).unwrap();
        assert_eq!(layer.rect, canvas);
        assert_eq!(
            layer_live_shape(layer).map(|l| l.bounds),
            Some((5.0, 9.0, 13.0, 13.0))
        );

        // A quarter turn about the outline box's centre (9, 11) swaps its sides
        // and leaves a regular path.
        let turned = crate::LayerTransform {
            scale_x: 1.0,
            scale_y: 1.0,
            angle_radians: std::f64::consts::FRAC_PI_2,
            dx: 0.0,
            dy: 0.0,
        };
        assert!(crate::transform_layer(&mut doc, &path, turned));
        assert_eq!(outline_rect(&doc, &path), Some(rect(7, 7, 11, 15)));
        let layer = super::super::paths::resolve_path(&doc, &path).unwrap();
        assert_eq!(layer_live_shape(layer), None);
        assert!(is_shape_layer(layer));

        // A quad sends the box's corners where it says.
        let quad = [(1.0, 1.0), (9.0, 1.0), (9.0, 5.0), (1.0, 5.0)];
        assert!(crate::transform_layer_quad(&mut doc, &path, quad));
        assert_eq!(outline_rect(&doc, &path), Some(rect(1, 1, 9, 5)));

        // A warp's frame moves the outline too, without resampling.
        let warp = pictura_core::warp::Warp::none([1.0, 1.0, 9.0, 5.0]);
        let moved = [(3.0, 2.0), (11.0, 2.0), (11.0, 6.0), (3.0, 6.0)];
        assert!(crate::transform_layer_warp(
            &mut doc,
            &path,
            &warp,
            Some(moved),
            crate::Interpolation::Bilinear
        ));
        assert_eq!(outline_rect(&doc, &path), Some(rect(3, 2, 11, 6)));
        let layer = super::super::paths::resolve_path(&doc, &path).unwrap();
        assert!(layer.channels.is_empty(), "the shape stays vector");
        assert_eq!(crate::content_rect(&doc, &path), Some(rect(3, 2, 11, 6)));
    }

    #[test]
    fn a_position_locked_shape_layer_does_not_move() {
        let (mut doc, path) = live_rectangle();
        super::super::paths::resolve_path_mut(&mut doc, &path)
            .unwrap()
            .lock =
            pictura_core::LockFlags::default().with(pictura_core::LockFlags::POSITION, true);
        let before = doc.clone();
        assert!(!crate::translate_layer_path(&mut doc, &path, 1, 1));
        let quad = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
        assert!(!crate::transform_layer_quad(&mut doc, &path, quad));
        assert_eq!(doc, before);
    }
}
