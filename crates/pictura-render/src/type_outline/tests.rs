use super::*;
use crate::render_type;
use crate::BundledText;
use pictura_core::path::{PathPoint, Subpath};
use pictura_core::warp::{TextWarp, WarpStyle};
use pictura_core::{TextPath, TextPathKind};

const LIBERATION: &[u8] = include_bytes!("../../assets/LiberationSans-Regular.ttf");

fn spec(text: &str) -> TypeSpec {
    let mut spec = TypeSpec::new(text, "Liberation Sans", 48.0);
    spec.origin = (100.0, 120.0);
    spec
}

fn line() -> Subpath {
    Subpath {
        points: vec![
            PathPoint {
                anchor: (50.0, 100.0),
                in_handle: None,
                out_handle: None,
                smooth: false,
            },
            PathPoint {
                anchor: (350.0, 100.0),
                in_handle: None,
                out_handle: None,
                smooth: false,
            },
        ],
        closed: false,
    }
}

fn square() -> Subpath {
    Subpath {
        points: [(50.0, 50.0), (350.0, 50.0), (350.0, 250.0), (50.0, 250.0)]
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

/// The further a curve bows, the more a flattened anchor misses it; 0.25 px
/// keeps the bounds oracle within the one-pixel contract.
fn ink_bounds(paths: &[Subpath]) -> (f64, f64, f64, f64) {
    let mut lo = (f64::MAX, f64::MAX);
    let mut hi = (f64::MIN, f64::MIN);
    for path in paths {
        for (x, y) in path.flatten(0.25) {
            lo = (lo.0.min(x), lo.1.min(y));
            hi = (hi.0.max(x), hi.1.max(y));
        }
    }
    (lo.0, lo.1, hi.0, hi.1)
}

fn rendered_bounds(spec: &TypeSpec) -> (f64, f64, f64, f64) {
    let (rect, buffer) = render_type(spec).expect("renders");
    let (width, height) = (rect.width() as usize, rect.height() as usize);
    let plane = buffer.pixel_count();
    let mut lo = (f64::MAX, f64::MAX);
    let mut hi = (f64::MIN, f64::MIN);
    for y in 0..height {
        for x in 0..width {
            if buffer.data[3 * plane + y * width + x] > 0 {
                let px = f64::from(rect.left) + x as f64;
                let py = f64::from(rect.top) + y as f64;
                lo = (lo.0.min(px), lo.1.min(py));
                hi = (hi.0.max(px + 1.0), hi.1.max(py + 1.0));
            }
        }
    }
    (lo.0, lo.1, hi.0, hi.1)
}

fn assert_bounds_match(spec: &TypeSpec) {
    let paths = outline_paths(spec).expect("outlines");
    assert!(!paths.is_empty(), "the text inks at least one contour");
    let (l, t, r, b) = ink_bounds(&paths);
    let (rl, rt, rr, rb) = rendered_bounds(spec);
    assert!(
        (l - rl).abs() <= 1.5
            && (t - rt).abs() <= 1.5
            && (r - rr).abs() <= 1.5
            && (b - rb).abs() <= 1.5,
        "outline ({l}, {t}, {r}, {b}) vs ink ({rl}, {rt}, {rr}, {rb})"
    );
}

#[test]
fn type_outline_paths_match_rendered_ink() {
    assert_bounds_match(&spec("Hamburgefonts"));
    let mut transformed = spec("Hamburgefonts");
    transformed.matrix = [1.0, 0.0, 0.2, 1.0];
    assert_bounds_match(&transformed);
    let mut paragraph = spec("A wrapped paragraph");
    paragraph.box_bounds = Some([0.0, 0.0, 160.0, 120.0]);
    assert_bounds_match(&paragraph);
}

#[test]
fn type_outline_paths_keep_curves() {
    let paths = outline_paths(&spec("O")).expect("outlines");
    assert!(
        paths
            .iter()
            .flat_map(|path| &path.points)
            .any(|point| point.in_handle.is_some() || point.out_handle.is_some()),
        "a round glyph keeps cubic handles"
    );
}

#[test]
fn type_outline_paths_warped() {
    let mut warped = spec("Warped");
    warped.warp = TextWarp {
        style: WarpStyle::Arc,
        bend: 50.0,
        ..TextWarp::default()
    };
    assert_bounds_match(&warped);
}

#[test]
fn type_outline_paths_on_path() {
    let mut on_path = spec("On a path");
    on_path.path = Some(TextPath {
        kind: TextPathKind::OnPath,
        subpath: line(),
        start: 0.0,
    });
    assert_bounds_match(&on_path);
}

#[test]
fn type_outline_paths_area() {
    let mut area = spec("Area type in a box");
    area.path = Some(TextPath {
        kind: TextPathKind::Area,
        subpath: square(),
        start: 0.0,
    });
    area.box_bounds = Some([0.0, 0.0, 300.0, 200.0]);
    let paths = outline_paths(&area).expect("outlines");
    assert!(!paths.is_empty(), "the area type inks");
    let (l, t, r, b) = ink_bounds(&paths);
    let (rl, rt, _rr, rb) = rendered_bounds(&area);
    assert!(
        l <= rl + 1.5 && t <= rt + 1.5 && b + 1.5 >= rb,
        "the outline covers the rendered ink ({l}, {t}, {r}, {b}) vs ({rl}, {rt}, {rb})"
    );
    assert!(
        r > _rr + 1.0,
        "area outlines trace laid-out glyphs without the render clip"
    );
    let original_ink = rendered_bounds(&area);
    area.matrix = [2.0, 0.0, 0.0, 1.0];
    assert_eq!(rendered_bounds(&area), original_ink);
    assert_eq!(outline_paths(&area).expect("area outlines"), paths);
}

#[test]
fn type_outline_refuses_outline_less() {
    let stripped =
        BundledText::from_bytes(Box::leak(without_outlines(LIBERATION).into_boxed_slice()))
            .expect("parses without outlines");
    assert_eq!(
        outline_refusal_with(&spec("Bitmap"), Some(&stripped)),
        Some(OutlineRefusal::NoOutlines)
    );
    assert_eq!(
        outline_refusal_with(&spec("None"), None),
        Some(OutlineRefusal::NoFace)
    );
}

fn without_outlines(font: &[u8]) -> Vec<u8> {
    let u16_at = |at: usize| u16::from_be_bytes([font[at], font[at + 1]]);
    let u32_at = |at: usize| u32::from_be_bytes(font[at..at + 4].try_into().unwrap());
    let tables: Vec<([u8; 4], &[u8])> = (0..usize::from(u16_at(4)))
        .map(|i| {
            let record = 12 + 16 * i;
            let (offset, length) = (u32_at(record + 8) as usize, u32_at(record + 12) as usize);
            (
                font[record..record + 4].try_into().unwrap(),
                &font[offset..offset + length],
            )
        })
        .filter(|(tag, _)| !matches!(tag, b"glyf" | b"loca" | b"CFF " | b"CFF2"))
        .collect();
    let mut out = font[..4].to_vec();
    out.extend_from_slice(&(tables.len() as u16).to_be_bytes());
    out.extend_from_slice(&[0; 6]);
    let mut offset = 12 + 16 * tables.len();
    for (tag, data) in &tables {
        out.extend_from_slice(tag);
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        offset += data.len().next_multiple_of(4);
    }
    for (_, data) in &tables {
        out.extend_from_slice(data);
        out.resize(out.len().next_multiple_of(4), 0);
    }
    out
}
