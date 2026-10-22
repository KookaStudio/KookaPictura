use super::*;
use crate::render_type;
use pictura_core::path::{PathPoint, Subpath};
use pictura_core::warp::WarpStyle;
use pictura_core::{BitDepth, ColorMode, Document, TextPath, TextPathKind};

const LIBERATION: &[u8] = include_bytes!("../../assets/LiberationSans-Regular.ttf");

fn spec(text: &str) -> TypeSpec {
    let mut spec = TypeSpec::new(text, "Liberation Sans", 48.0);
    spec.origin = (100.0, 120.0);
    spec.character.fill_color = [1.0, 0.0, 0.0, 0.0];
    spec
}

fn warped(style: WarpStyle, bend: f64, h: f64, v: f64, vertical: bool) -> TypeSpec {
    TypeSpec {
        warp: TextWarp {
            style,
            bend,
            h_distort: h,
            v_distort: v,
            vertical,
        },
        ..spec("Warped Text")
    }
}

/// The rendered alpha plane and its width and height.
fn alpha(spec: &TypeSpec) -> (Vec<u8>, usize, usize) {
    let (rect, buffer) = render_type(spec).expect("renders");
    let n = buffer.pixel_count();
    (
        buffer.data[3 * n..].to_vec(),
        rect.width() as usize,
        rect.height() as usize,
    )
}

/// The first row with ink among columns `x0..x1`.
fn top_ink(plane: &[u8], w: usize, x0: usize, x1: usize) -> usize {
    plane
        .chunks(w)
        .position(|row| row[x0..x1].iter().any(|&a| a > 0))
        .expect("the band has ink")
}

/// `plane` cropped to its ink.
fn trim((plane, w, h): (Vec<u8>, usize, usize)) -> (Vec<u8>, usize, usize) {
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for (i, _) in plane.iter().enumerate().filter(|(_, &a)| a > 0) {
        let (x, y) = (i % w, i / w);
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
    }
    let (tw, th) = (x1 - x0, y1 - y0);
    let mut out = Vec::with_capacity(tw * th);
    for y in y0..y1 {
        out.extend_from_slice(&plane[y * w + x0..y * w + x1]);
    }
    (out, tw, th)
}

#[test]
fn identity_warp_renders_unwarped() {
    let plain = render_type(&spec("Warped Text")).expect("renders");
    for warp in [
        TextWarp::default(),
        TextWarp {
            style: WarpStyle::Arc,
            ..TextWarp::default()
        },
    ] {
        let same = TypeSpec {
            warp,
            ..spec("Warped Text")
        };
        assert!(!is_warped(&same));
        let rendered = render_type(&same).expect("renders");
        assert_eq!(rendered.0, plain.0);
        assert_eq!(rendered.1.data, plain.1.data);
    }
}

#[test]
fn arc_bends_the_line() {
    let arc = warped(WarpStyle::Arc, 50.0, 0.0, 0.0, false);
    let (plane, w, _) = alpha(&arc);
    let fifth = w / 5;
    let left = top_ink(&plane, w, 0, fifth);
    let middle = top_ink(&plane, w, 2 * fifth, 3 * fifth);
    let right = top_ink(&plane, w, 4 * fifth, w);
    assert!(
        middle + 4 < left && middle + 4 < right,
        "left {left} middle {middle} right {right}"
    );
    // The same outlines in a rect grown by 8 px on every side carry no more
    // ink: the placement rect holds all of it.
    let rect = placement(&arc).expect("places").rect;
    let grown = PsdRect {
        top: rect.top - 8,
        left: rect.left - 8,
        bottom: rect.bottom + 8,
        right: rect.right + 8,
    };
    let sum = |b: PixelBuffer| -> u64 {
        let n = b.pixel_count();
        b.data[3 * n..].iter().map(|&a| u64::from(a)).sum()
    };
    assert_eq!(
        sum(render(&arc, rect).expect("renders")),
        sum(render(&arc, grown).expect("renders"))
    );
}

#[test]
fn orientation_and_distortion_differ() {
    let renders = [
        alpha(&spec("Warped Text")),
        alpha(&warped(WarpStyle::Arc, 50.0, 0.0, 0.0, false)),
        alpha(&warped(WarpStyle::Arc, 50.0, 0.0, 0.0, true)),
        alpha(&warped(WarpStyle::Arc, 0.0, 50.0, 0.0, false)),
    ];
    for (i, a) in renders.iter().enumerate() {
        for b in &renders[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

#[test]
fn warped_edges_stay_sharp() {
    let partial = |(plane, _, _): (Vec<u8>, usize, usize)| {
        let ink = plane.iter().filter(|&&a| a > 0).count() as f64;
        plane.iter().filter(|&&a| a > 0 && a < 255).count() as f64 / ink
    };
    let plain = partial(alpha(&spec("Warped Text")));
    let flag = partial(alpha(&warped(WarpStyle::Flag, 20.0, 0.0, 0.0, false)));
    assert!(flag <= plain * 1.5, "plain {plain:.3} flag {flag:.3}");
}

#[test]
fn warp_precedes_transform() {
    let arc = warped(WarpStyle::Arc, 50.0, 0.0, 0.0, false);
    let rotated = TypeSpec {
        matrix: [0.0, 1.0, -1.0, 0.0],
        ..arc.clone()
    };
    let (u, uw, uh) = trim(alpha(&arc));
    let (r, rw, rh) = trim(alpha(&rotated));
    assert!(
        rw.abs_diff(uh) <= 1 && rh.abs_diff(uw) <= 1,
        "{uw}x{uh} vs {rw}x{rh}"
    );
    // (u, v) maps to (-v, u): a clockwise quarter turn.
    let (w, h) = (rw.min(uh), rh.min(uw));
    let mut differ = 0;
    for y in 0..h {
        for x in 0..w {
            let expected = u[(uh - 1 - x) * uw + y];
            if expected.abs_diff(r[y * rw + x]) > 64 {
                differ += 1;
            }
        }
    }
    let ink = u.iter().filter(|&&a| a > 0).count();
    assert!(
        differ * 100 < ink * 3,
        "{differ} of {ink} ink pixels differ"
    );
}

#[test]
fn warp_survives_save_and_open() {
    let mut doc = Document::new(400, 300, ColorMode::Rgb, BitDepth::Eight);
    let fish = warped(WarpStyle::Fish, 40.0, -10.0, 25.0, true);
    let path = crate::add_type_layer(&mut doc, "", &fish);
    assert!(!path.is_empty());
    let bytes = pictura_codec::write_psd(&doc).expect("writes");
    let back = pictura_codec::read_psd(&bytes).expect("reads");
    let layer = crate::resolve_path(&back, &path).expect("layer");
    let reopened = crate::type_layer_spec(layer).expect("type spec");
    assert_eq!(reopened.warp, fish.warp);
}

/// `font` without its outline tables: a stand-in for a bitmap-only face.
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

#[test]
fn refusals() {
    assert_eq!(type_warp_refusal(&spec("Plain")), None);

    let mut bold = spec("Bold");
    bold.character.faux_bold = true;
    assert_eq!(type_warp_refusal(&bold), Some(WarpRefusal::FauxBold));

    let line = Subpath {
        points: [(0.0, 100.0), (300.0, 100.0), (300.0, 200.0)]
            .map(|anchor| PathPoint {
                anchor,
                in_handle: None,
                out_handle: None,
                smooth: false,
            })
            .to_vec(),
        closed: false,
    };
    for kind in [TextPathKind::OnPath, TextPathKind::Area] {
        let mut on_path = spec("Path");
        on_path.path = Some(TextPath {
            kind,
            subpath: line.clone(),
            start: 0.0,
        });
        assert_eq!(type_warp_refusal(&on_path), Some(WarpRefusal::OnPath));
    }

    // A face built here, never registered: the process-wide registry keys
    // faces by PostScript name too, so a registered copy would replace
    // Liberation Sans for every test sharing this process.
    assert!(crate::fonts::font_has_outlines("Liberation Sans"));
    let stripped =
        BundledText::from_bytes(Box::leak(without_outlines(LIBERATION).into_boxed_slice()))
            .expect("parses without outlines");
    assert!(!crate::fonts::face_has_outlines(&stripped));
    assert_eq!(
        refusal_with(&spec("Bitmap"), Some(&stripped)),
        Some(WarpRefusal::NoOutlines)
    );
}

#[test]
fn hostile_warp_values_stay_bounded() {
    let hundred = warped(WarpStyle::Arc, 100.0, 100.0, -100.0, false);
    for (bend, h, v) in [
        (1e308, f64::NAN, -1e308),
        (f64::INFINITY, 1e9, f64::NEG_INFINITY),
    ] {
        let wild = warped(WarpStyle::Arc, bend, h, v, false);
        let flat = type_placement(&unwarped(&wild)).expect("places").rect;
        if let Some(placed) = placement(&wild) {
            let span = flat.width().max(flat.height());
            assert!(
                placed.rect.width().max(placed.rect.height()) <= 4 * span + 66,
                "{:?} vs {:?}",
                placed.rect,
                flat
            );
        }
    }
    // Out-of-range values act as the ±100 % they clamp to.
    let clamped = warped(WarpStyle::Arc, 1e308, 1e308, -1e308, false);
    assert_eq!(
        placement(&clamped).map(|p| p.rect),
        placement(&hundred).map(|p| p.rect)
    );
}
