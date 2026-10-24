use std::sync::OnceLock;

use pictura_core::{
    BitDepth, CharacterAttrs, ColorMode, Document, KerningMode, PixelBuffer, TypeSpec,
};

use super::*;
use crate::type_paragraph::line_advance;

const SIZE: f64 = 48.0;
const NOTO_FALLBACK: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/fonts/NotoSans-Regular.ttf"
));

fn liberation() -> &'static BundledText {
    crate::text_render::bundled().expect("bundled font parses")
}

/// The repository's Noto Sans, which carries `liga`, `smcp`, `frac`, `ordn`,
/// and `salt` (the bundled Liberation Sans has none of them).
fn noto() -> &'static BundledText {
    static FACE: OnceLock<BundledText> = OnceLock::new();
    FACE.get_or_init(|| {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/fonts/NotoSans-Regular.ttf"
        );
        let bytes = std::fs::read(path).expect("Noto Sans is in assets/fonts");
        BundledText::from_bytes(Box::leak(bytes.into_boxed_slice())).expect("Noto Sans parses")
    })
}

fn attrs() -> CharacterAttrs {
    CharacterAttrs {
        size: SIZE,
        ..CharacterAttrs::default()
    }
}

fn ids(glyphs: &[ShapedGlyph]) -> Vec<u16> {
    glyphs.iter().map(|g| g.id).collect()
}

fn width(face: &BundledText, text: &str, c: &CharacterAttrs) -> f32 {
    line_advance(face, text, c)
}

/// The ink box `(left, top, right, bottom)` of a render in document pixels
/// (bottom and right exclusive), and its summed alpha.
fn ink(spec: &TypeSpec) -> ([i32; 4], u64) {
    let (rect, buffer) = crate::render_type(spec).expect("type renders");
    let (w, h) = (buffer.width as i32, buffer.height as i32);
    let plane = buffer.pixel_count();
    let mut bounds = [i32::MAX, i32::MAX, i32::MIN, i32::MIN];
    let mut total = 0u64;
    for y in 0..h {
        for x in 0..w {
            let alpha = buffer.data[3 * plane + (y * w + x) as usize];
            if alpha > 0 {
                total += u64::from(alpha);
                bounds = [
                    bounds[0].min(rect.left + x),
                    bounds[1].min(rect.top + y),
                    bounds[2].max(rect.left + x + 1),
                    bounds[3].max(rect.top + y + 1),
                ];
            }
        }
    }
    (bounds, total)
}

/// The alpha of document row `y` between document columns `x0..x1`.
fn row(spec: &TypeSpec, y: i32, x0: i32, x1: i32) -> Vec<u8> {
    let (rect, buffer): (_, PixelBuffer) = crate::render_type(spec).expect("type renders");
    let plane = buffer.pixel_count();
    let w = buffer.width as i32;
    (x0..x1)
        .map(|x| {
            let (lx, ly) = (x - rect.left, y - rect.top);
            if lx < 0 || ly < 0 || lx >= w || ly >= buffer.height as i32 {
                return 0;
            }
            buffer.data[3 * plane + (ly * w + lx) as usize]
        })
        .collect()
}

fn spec(text: &str, style: impl FnOnce(&mut CharacterAttrs)) -> TypeSpec {
    let mut spec = TypeSpec::new(text, "", SIZE);
    spec.origin = (40.0, 80.0);
    style(&mut spec.character);
    spec
}

#[test]
fn standard_ligatures_off_breaks_the_ligature() {
    let on = shape_styled(noto(), "fi", &attrs());
    let off = shape_styled(
        noto(),
        "fi",
        &CharacterAttrs {
            standard_ligatures: false,
            ..attrs()
        },
    );
    assert_eq!(on.len(), 1, "Noto Sans ligates fi");
    assert_eq!(off.len(), 2);
}

#[test]
fn fractions_change_the_glyphs_only_when_on() {
    let off = shape_styled(noto(), "1/2", &attrs());
    let on = shape_styled(
        noto(),
        "1/2",
        &CharacterAttrs {
            fractions: true,
            ..attrs()
        },
    );
    assert_ne!(ids(&off), ids(&on));
}

#[test]
fn a_feature_the_face_lacks_changes_nothing() {
    assert!(!has_feature(liberation(), b"swsh"));
    let off = shape_styled(liberation(), "Swash", &attrs());
    let on = shape_styled(
        liberation(),
        "Swash",
        &CharacterAttrs {
            swash: true,
            ..attrs()
        },
    );
    assert_eq!(off, on);
}

#[test]
fn support_mask_follows_the_feature_list() {
    assert_eq!(opentype_support(liberation()), 1 << 2, "dlig only");
    assert_eq!(
        opentype_support(noto()),
        1 << 0 | 1 << 4 | 1 << 6 | 1 << 7,
        "liga, salt, ordn, frac"
    );
}

#[test]
fn tsume_compresses_a_full_width_advance() {
    let c = CharacterAttrs {
        size: 40.0,
        tsume: 50.0,
        ..attrs()
    };
    assert!((tsume_advance(40.0, '漢', &c) - 30.0).abs() < 1e-4);
    assert_eq!(tsume_advance(40.0, 'A', &c), 40.0);
}

#[test]
fn all_caps_shapes_upper_case() {
    let caps = CharacterAttrs {
        all_caps: true,
        ..attrs()
    };
    assert_eq!(
        ids(&shape_styled(liberation(), "type", &caps)),
        ids(&shape_styled(liberation(), "TYPE", &attrs()))
    );
}

#[test]
fn synthesized_small_caps_are_scaled_capitals() {
    assert!(!has_feature(liberation(), b"smcp"));
    let small = CharacterAttrs {
        small_caps: true,
        ..attrs()
    };
    let glyphs = shape_styled(liberation(), "ab", &small);
    let capitals = shape_styled(liberation(), "AB", &attrs());
    assert_eq!(ids(&glyphs), ids(&capitals));
    for (g, capital) in glyphs.iter().zip(&capitals) {
        assert_eq!(g.scale, SMALL_CAPS);
        assert!((g.advance - capital.advance * SMALL_CAPS).abs() < 1e-3);
    }
}

#[test]
fn superscript_is_smaller_and_raised() {
    let (plain, _) = ink(&spec("x", |_| {}));
    let (sup, _) = ink(&spec("x", |c| c.superscript = true));
    assert!(
        sup[3] - sup[1] < plain[3] - plain[1],
        "{sup:?} vs {plain:?}"
    );
    assert!(
        sup[3] < 80 - 5,
        "the superscript sits above the baseline: {sup:?}"
    );
}

#[test]
fn subscript_is_smaller_and_lowered() {
    let (plain, _) = ink(&spec("x", |_| {}));
    let (sub, _) = ink(&spec("x", |c| c.subscript = true));
    assert!(sub[3] - sub[1] < plain[3] - plain[1]);
    assert!(sub[3] > plain[3] + 5, "{sub:?} vs {plain:?}");
}

#[test]
fn faux_bold_adds_ink_and_advance() {
    let (_, plain) = ink(&spec("l", |_| {}));
    let (_, bold) = ink(&spec("l", |c| c.faux_bold = true));
    assert!(bold > plain, "{bold} vs {plain}");
    let heavy = CharacterAttrs {
        faux_bold: true,
        ..attrs()
    };
    assert!(width(liberation(), "l", &heavy) > width(liberation(), "l", &attrs()));
}

#[test]
fn faux_italic_slants_right() {
    // The centre of the ink in `l`'s top row minus its bottom row's.
    let lean = |spec: &TypeSpec| {
        let (bounds, _) = ink(spec);
        let centre = |y: i32| {
            let alpha = row(spec, y, bounds[0], bounds[2]);
            let weight: f32 = alpha.iter().map(|&a| f32::from(a)).sum();
            alpha
                .iter()
                .enumerate()
                .map(|(i, &a)| i as f32 * f32::from(a))
                .sum::<f32>()
                / weight
        };
        centre(bounds[1] + 1) - centre(bounds[3] - 2)
    };
    let upright = lean(&spec("l", |_| {}));
    let italic = lean(&spec("l", |c| c.faux_italic = true));
    assert!(italic > upright + 5.0, "{italic} vs {upright}");
}

#[test]
fn underline_paints_below_the_baseline() {
    let c = CharacterAttrs {
        underline: true,
        ..attrs()
    };
    let (offset, _) = RunStyle::of(liberation(), &c).underline.expect("underline");
    let y = (80.0 + offset).floor() as i32;
    let end = 40 + width(liberation(), "xx", &c) as i32;
    let lined = row(&spec("xx", |c| c.underline = true), y, 42, end - 2);
    let plain = row(&spec("xx", |_| {}), y, 42, end - 2);
    assert!(lined.iter().all(|&a| a > 128), "{lined:?}");
    assert!(plain.iter().all(|&a| a == 0), "{plain:?}");
}

#[test]
fn strikethrough_crosses_the_letters() {
    let c = CharacterAttrs {
        strikethrough: true,
        ..attrs()
    };
    let (offset, _) = RunStyle::of(liberation(), &c)
        .strike
        .expect("strikethrough");
    let y = (80.0 - offset).floor() as i32;
    let end = 40 + width(liberation(), "xx", &c) as i32;
    let struck = row(&spec("xx", |c| c.strikethrough = true), y, 42, end - 2);
    let plain = row(&spec("xx", |_| {}), y, 42, end - 2);
    assert!(struck.iter().all(|&a| a > 128), "{struck:?}");
    assert!(plain.contains(&0), "the plain row has a gap: {plain:?}");
}

#[test]
fn manual_zero_turns_metrics_kerning_off() {
    let manual = CharacterAttrs {
        kerning: KerningMode::Manual(0),
        ..attrs()
    };
    let metrics = width(liberation(), "AV", &attrs());
    let off = width(liberation(), "AV", &manual);
    let apart = width(liberation(), "A", &manual) + width(liberation(), "V", &manual);
    assert!(off > metrics, "{off} vs {metrics}");
    assert!((off - apart).abs() < 1e-3, "{off} vs {apart}");
}

#[test]
fn manual_kerning_widens_the_line() {
    let at = |value| CharacterAttrs {
        kerning: KerningMode::Manual(value),
        ..attrs()
    };
    let gain = width(liberation(), "type", &at(100)) - width(liberation(), "type", &at(0));
    assert!((gain - 0.1 * SIZE as f32 * 4.0).abs() < 1e-3, "{gain}");
}

#[test]
fn optical_tightens_an_open_pair() {
    let mode = |kerning| CharacterAttrs { kerning, ..attrs() };
    let optical = width(liberation(), "AV", &mode(KerningMode::Optical));
    let unkerned = width(liberation(), "AV", &mode(KerningMode::Manual(0)));
    assert!(optical < unkerned, "{optical} vs {unkerned}");
}

#[test]
fn all_caps_widens_the_measured_line() {
    let caps = CharacterAttrs {
        all_caps: true,
        ..attrs()
    };
    let shouted = width(liberation(), "type", &caps);
    assert!((shouted - width(liberation(), "TYPE", &attrs())).abs() < 1e-3);
    assert!(shouted > width(liberation(), "type", &attrs()));
}

#[test]
fn caret_last_stop_meets_the_styled_advance() {
    let spec = spec("Type", |c| {
        c.all_caps = true;
        c.faux_bold = true;
    });
    let stops = crate::type_caret_stops(&spec);
    let (first, last) = (stops.first().unwrap(), stops.last().unwrap());
    let advance = width(liberation(), "Type", &spec.character);
    assert!(((last[0] - first[0]) as f32 - advance).abs() < 1e-3);
}

fn register_fallback() {
    assert!(crate::fonts::register_font(
        "Fallback Noto",
        "",
        NOTO_FALLBACK.to_vec()
    ));
}

fn shaped_fallback(text: &str, protection: bool) -> Vec<ShapedGlyph> {
    let _guard = crate::fonts::FALLBACK_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    crate::fonts::set_missing_glyph_protection(protection);
    register_fallback();
    let active = crate::fonts::face_for("Liberation Sans", "").expect("active");
    let faces = crate::fonts::faces_in_order(active);
    let out = shape_styled_faces(&faces, text, &attrs(), rustybuzz::Direction::LeftToRight);
    crate::fonts::set_missing_glyph_protection(true);
    out
}

#[test]
fn fallback_single_character_uses_covering_face() {
    let glyphs = shaped_fallback("A\u{370}", true);
    assert!(
        glyphs.iter().any(|g| g.face != 0),
        "the uncovered glyph falls back"
    );
    assert!(
        glyphs.iter().any(|g| g.face == 0),
        "the covered glyph stays active"
    );
}

#[test]
fn fallback_off_uses_active_face() {
    let glyphs = shaped_fallback("A\u{370}", false);
    assert!(glyphs.iter().all(|g| g.face == 0));
}

#[test]
fn fallback_prefers_active_face() {
    let glyphs = shaped_fallback("A", true);
    assert!(glyphs.iter().all(|g| g.face == 0));
}

#[test]
fn fallback_without_cover_keeps_active_face() {
    let glyphs = shaped_fallback("\u{10FFFF}", true);
    assert!(glyphs.iter().all(|g| g.face == 0));
}

#[test]
fn fallback_rtl_run_keeps_visual_order() {
    let _guard = crate::fonts::FALLBACK_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    register_fallback();
    let active = crate::fonts::face_for("Liberation Sans", "").expect("active");
    let faces = crate::fonts::faces_in_order(active);
    let logical = "\u{5D0}\u{5D1}\u{370}";
    let glyphs = shape_styled_faces(&faces, logical, &attrs(), rustybuzz::Direction::RightToLeft);
    let fallback = glyphs.first().expect("glyphs");
    assert_ne!(
        fallback.face, 0,
        "the uncovered last character paints first"
    );
    assert!(glyphs.iter().skip(1).all(|g| g.face == 0));
}

#[test]
fn small_caps_pieces_keep_source_clusters() {
    assert!(!has_feature(liberation(), b"smcp"));
    let c = CharacterAttrs {
        small_caps: true,
        ..attrs()
    };
    let glyphs = shape_one_dir(liberation(), "aB", &c, rustybuzz::Direction::LeftToRight);
    assert_eq!(glyphs.len(), 2);
    assert_eq!(glyphs[0].cluster, 0);
    assert_eq!(
        glyphs[1].cluster, 1,
        "the second piece keeps its source offset"
    );
}

#[test]
fn fallback_run_uses_second_face_advance() {
    let _guard = crate::fonts::FALLBACK_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    register_fallback();
    let active = crate::fonts::face_for("Liberation Sans", "").expect("active");
    let faces = crate::fonts::faces_in_order(active);
    let mixed = shape_styled_faces(
        &faces,
        "A\u{370}",
        &attrs(),
        rustybuzz::Direction::LeftToRight,
    );
    let fallback = mixed.iter().find(|g| g.face != 0).expect("fallback glyph");
    let face = faces[fallback.face as usize];
    let alone = shape_styled_faces(
        &[face],
        "\u{370}",
        &attrs(),
        rustybuzz::Direction::LeftToRight,
    );
    assert!((fallback.advance - alone[0].advance).abs() < 0.01);
}

#[test]
fn re_rendering_keeps_every_type_layer_live() {
    let mut doc = Document::new(300, 200, ColorMode::Rgb, BitDepth::Eight);
    let one = TypeSpec::new("One", "Liberation Sans", 32.0);
    let two = TypeSpec::new("Two", "Liberation Sans", 32.0);
    crate::add_type_layer(&mut doc, "", &one);
    crate::add_type_layer(&mut doc, "", &two);
    let texts: Vec<String> = doc
        .layers
        .iter()
        .filter_map(crate::type_layer_spec)
        .map(|s| s.text)
        .collect();
    assert_eq!(crate::re_render_type_layers(&mut doc), 2);
    assert!(doc.layers.iter().all(|layer| layer.type_tool.is_some()));
    let after: Vec<String> = doc
        .layers
        .iter()
        .filter_map(crate::type_layer_spec)
        .map(|s| s.text)
        .collect();
    assert_eq!(after, texts);
}
