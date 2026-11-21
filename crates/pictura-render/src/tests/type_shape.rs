use crate::LayerMaskKind;
use crate::{
    add_layer_mask, add_type_layer, convert_type_to_shape, is_shape_layer, layer_shape_paths,
    render_type, resolve_path, resolve_path_mut, set_layer_shape_paths, set_vector_mask_enabled,
    shape_coverage, shape_fill_color, type_layer_spec,
};
use pictura_core::path::{PathPoint, Subpath};
use pictura_core::{BitDepth, BlendMode, ColorMode, Document, LockFlags, PsdRect, TypeSpec};

fn empty_doc() -> Document {
    Document::new(200, 220, ColorMode::Rgb, BitDepth::Eight)
}

fn type_spec() -> TypeSpec {
    let mut spec = TypeSpec::new("Shape", "Liberation Sans", 48.0);
    spec.origin = (30.0, 120.0);
    spec.character.fill_color = [1.0, 0.2, 0.4, 0.6];
    spec
}

fn type_layer() -> (Document, String) {
    let mut doc = empty_doc();
    let path = add_type_layer(&mut doc, "", &type_spec());
    (doc, path)
}

fn square() -> Subpath {
    Subpath {
        points: [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
            .into_iter()
            .map(|anchor| PathPoint {
                anchor,
                in_handle: None,
                out_handle: None,
                smooth: false,
            })
            .collect(),
        closed: true,
    }
}

#[test]
fn convert_type_to_shape_replaces_content() {
    let (mut doc, path) = type_layer();
    assert!(convert_type_to_shape(&mut doc, &path));
    let layer = resolve_path(&doc, &path).expect("layer");
    assert!(is_shape_layer(layer), "the row becomes a shape");
    assert!(!layer.is_type(), "the type tool data is gone");
    assert!(layer.type_tool().is_none());
    assert_eq!(
        shape_fill_color(layer),
        Some([51, 102, 153, 255]),
        "the type fill colour becomes the shape fill (alpha is stored opaque)"
    );
    let outlined = layer_shape_paths(layer, doc.width, doc.height).expect("vector mask");
    let mut bounds = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for point in outlined.iter().flat_map(|path| path.flatten(0.25)) {
        bounds = (
            bounds.0.min(point.0),
            bounds.1.min(point.1),
            bounds.2.max(point.0),
            bounds.3.max(point.1),
        );
    }
    assert!(bounds.2 > bounds.0 && bounds.3 > bounds.1, "the mask inks");
    assert!(
        !convert_type_to_shape(&mut doc, &path),
        "a shape layer is no longer type"
    );
}

#[test]
fn convert_type_to_shape_keeps_effects() {
    let (mut doc, path) = type_layer();
    {
        let layer = resolve_path_mut(&mut doc, &path).expect("layer");
        layer.name = "Styled Type".to_string();
        layer.blend = BlendMode::Multiply;
        layer.opacity = 128;
        layer.set_metadata_block(*b"abcd", Some(vec![1, 2, 3].into()));
    }
    assert!(add_layer_mask(
        &mut doc,
        &path,
        LayerMaskKind::RevealAll,
        None
    ));
    assert!(convert_type_to_shape(&mut doc, &path));
    let layer = resolve_path(&doc, &path).expect("layer");
    assert_eq!(layer.name, "Styled Type");
    assert_eq!(layer.blend, BlendMode::Multiply);
    assert_eq!(layer.opacity, 128);
    assert!(layer.mask.is_some(), "the raster layer mask is retained");
    assert!(layer.extra_block(b"abcd").is_some(), "extra blocks survive");
    assert!(layer.vector_mask.is_some());
}

#[test]
fn convert_type_to_shape_replaces_an_existing_vector_mask() {
    let (mut doc, path) = type_layer();
    {
        let (width, height) = (doc.width, doc.height);
        let layer = resolve_path_mut(&mut doc, &path).expect("layer");
        set_layer_shape_paths(layer, &[square()], width, height);
        layer.vector_mask_hides_effects = true;
        layer.lock = LockFlags::all();
    }
    assert!(set_vector_mask_enabled(&mut doc, &path, false));
    assert!(convert_type_to_shape(&mut doc, &path));
    let layer = resolve_path(&doc, &path).expect("layer");
    assert!(layer.lock.is_all());
    assert!(!layer.vector_mask_hides_effects);
    let mask = layer.vector_mask.as_ref().expect("glyph mask");
    assert!(!mask.disabled);
    assert_eq!(shape_coverage(layer, layer.rect, 0, 0), 0);
    let paths = layer_shape_paths(layer, doc.width, doc.height).expect("vector mask");
    let first = paths[0].flatten(1.0).first().copied().unwrap_or_default();
    assert!(
        first.0 > 10.0 || first.1 > 10.0,
        "the glyph outline replaced the 10 px square"
    );
}

#[test]
fn convert_type_to_shape_keeps_overlapping_ink() {
    let mut doc = empty_doc();
    let mut spec = TypeSpec::new("HH", "Liberation Sans", 60.0);
    spec.origin = (30.0, 100.0);
    spec.character.tracking = -550.0;
    let path = add_type_layer(&mut doc, "", &spec);
    let original = type_layer_spec(resolve_path(&doc, &path).expect("type layer")).expect("spec");
    let (rect, pixels) = render_type(&original).expect("renders");
    let width = rect.width();
    let alpha = &pixels.data[3 * pixels.pixel_count()..];
    assert!(convert_type_to_shape(&mut doc, &path));
    let shape = resolve_path(&doc, &path).expect("shape");
    let mut solid_pixels = 0;
    for y in 1..rect.height() - 1 {
        for x in 1..width - 1 {
            let interior = (-1..=1)
                .all(|dy| (-1..=1).all(|dx| alpha[((y + dy) * width + x + dx) as usize] == 255));
            if interior {
                solid_pixels += 1;
                assert!(shape_coverage(shape, shape.rect, rect.left + x, rect.top + y) > 0);
            }
        }
    }
    assert!(solid_pixels > 0);
}

#[test]
fn convert_type_to_shape_round_trip() {
    let (mut doc, path) = type_layer();
    assert!(convert_type_to_shape(&mut doc, &path));
    let bytes = pictura_codec::write_psd(&doc).expect("writes");
    let back = pictura_codec::read_psd(&bytes).expect("reads");
    let layer = resolve_path(&back, &path).expect("layer");
    assert!(is_shape_layer(layer), "reopens as a shape");
    assert_eq!(shape_fill_color(layer), Some([51, 102, 153, 255]));
    assert!(layer_shape_paths(layer, back.width, back.height).is_some());
    assert_eq!(
        layer.rect,
        PsdRect {
            top: 0,
            left: 0,
            bottom: back.height as i32,
            right: back.width as i32,
        }
    );
}
