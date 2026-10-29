use super::*;
use crate::type_style::shape_styled;
use pictura_core::{
    BitDepth, CharacterAttrs, ColorMode, Layer, LayerBlock, LayoutLine, ParagraphAttrs,
    PlacedGlyph, PsdRect, TextStyle, TypeTool,
};

fn bundled() -> BundledText {
    BundledText::new().expect("bundled font parses")
}

fn type_layer(style: Option<TextStyle>) -> Layer {
    Layer {
        name: "text".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 80,
            right: 200,
        },
        extra_blocks: vec![LayerBlock {
            key: *b"TySh",
            data: vec![1, 2, 3].into(),
        }],
        type_tool: Some(TypeTool {
            transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            text: "Hi".into(),
            bounds: [0, 0, 80, 200],
            text_desc: Vec::new().into(),
            warp_desc: Vec::new().into(),
            fonts: vec!["Arial".into()],
            style,
            vertical: false,
            box_bounds: None,
            path: None,
        }),
        ..Default::default()
    }
}

fn style() -> TextStyle {
    TextStyle {
        font: Some("Arial".into()),
        character: CharacterAttrs {
            size: 48.0,
            fill_color: [1.0, 0.0, 0.0, 0.0],
            ..CharacterAttrs::default()
        },
        paragraph: ParagraphAttrs::default(),
        ..TextStyle::default()
    }
}

fn doc_with(layer: Layer) -> Document {
    let mut doc = Document::new(200, 80, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    doc
}

#[test]
fn rasterizer_returns_coverage_of_width_times_height() {
    let bundled = bundled();
    let index = bundled.glyph_for('A');
    assert_ne!(index, 0, "'A' is in Liberation Sans");
    let mask = bundled
        .rasterizer()
        .rasterize(&RasterRequest {
            glyph: index,
            px_size: 48.0,
            subpixel_x: 0.0,
            subpixel_y: 0.0,
        })
        .expect("'A' rasterizes");
    assert_seeded(&mask);
}

fn assert_seeded(mask: &GlyphMask) {
    assert_eq!(mask.coverage.len(), (mask.width * mask.height) as usize);
    assert!(
        mask.coverage.iter().any(|&c| c > 0),
        "coverage is non-empty"
    );
}

#[test]
fn rasterizer_refuses_an_absent_index_without_panicking() {
    let bundled = bundled();
    let mask = bundled.rasterizer().rasterize(&RasterRequest {
        glyph: u16::MAX,
        px_size: 48.0,
        subpixel_x: 0.0,
        subpixel_y: 0.0,
    });
    assert!(mask.is_none());
}

#[test]
fn rasterizer_subpixel_offset_changes_the_mask() {
    let bundled = bundled();
    let index = bundled.glyph_for('A');
    let render = |subpixel_x: f32| {
        bundled
            .rasterizer()
            .rasterize(&RasterRequest {
                glyph: index,
                px_size: 48.0,
                subpixel_x,
                subpixel_y: 0.0,
            })
            .expect("'A' rasterizes")
    };
    assert_ne!(
        render(0.0),
        render(0.5),
        "the half-phase mask differs from the integer phase"
    );
}

fn paint_glyph_at(x: f32) -> PixelBuffer {
    let bundled = bundled();
    let id = bundled.glyph_for('A');
    let layout = TextLayout {
        lines: vec![LayoutLine {
            glyphs: vec![PlacedGlyph {
                id,
                x,
                y: 0.0,
                scale: 1.0,
                x_scale: 1.0,
                face: 0,
                cluster: 0,
            }],
            advance: 0.0,
            baseline: 0.0,
        }],
        width: 0.0,
        height: 0.0,
        face_count: 1,
    };
    paint_layout(&bundled, &layout, 200, 120, 48.0, [0.0, 0.0, 0.0, 1.0]).expect("paints")
}

#[test]
fn fractional_widths_off_rounds_advances() {
    let face = bundled();
    let c = CharacterAttrs {
        size: 17.0,
        fractional_widths: false,
        ..CharacterAttrs::default()
    };
    let glyphs = crate::type_style::shape_styled(&face, "AV", &c);
    assert!(!glyphs.is_empty());
    assert!(glyphs.iter().all(|g| g.advance.fract() == 0.0));
}

#[test]
fn fractional_pen_is_not_rounded_away() {
    let half = paint_glyph_at(12.5);
    assert_ne!(half, paint_glyph_at(12.0), "12.5 differs from 12.0");
    assert_ne!(half, paint_glyph_at(13.0), "12.5 differs from 13.0");
}

#[test]
fn shape_line_maps_two_glyphs_with_nonzero_advances() {
    let glyphs = bundled().shape_line("AB", 48.0, 0.0);
    assert_eq!(glyphs.len(), 2);
    assert!(glyphs.iter().all(|g| g.advance > 0.0));
}

#[test]
fn shape_line_plain_latin_has_zero_offsets() {
    let glyphs = bundled().shape_line("AB", 48.0, 0.0);
    assert!(glyphs
        .iter()
        .all(|g| g.x_offset == 0.0 && g.y_offset == 0.0));
}

#[test]
fn y_offset_paints_a_shifted_row() {
    let bundled = bundled();
    let font_size = 48.0;
    let id = bundled.glyph_for('A');
    assert_ne!(id, 0);
    let params = LayoutParams {
        font_size,
        units_per_em: font_size,
        tracking: 0.0,
        leading: font_size * 1.2,
        align: TextAlign::Left,
        wrap_width: None,
    };
    let plain = layout_lines(&[vec![ShapedGlyph::new(id, 0.0)]], &params);
    let mut raised = ShapedGlyph::new(id, 0.0);
    raised.y_offset = 5.0;
    let marked = layout_lines(&[vec![raised]], &params);

    let width = 200;
    let height = 120;
    let color = [0.0, 0.0, 0.0, 1.0];
    let plain_buf =
        paint_layout(&bundled, &plain, width, height, font_size, color).expect("plain paints");
    let marked_buf =
        paint_layout(&bundled, &marked, width, height, font_size, color).expect("mark paints");

    let top_row = |buf: &PixelBuffer| {
        let plane = buf.pixel_count();
        let w = width as usize;
        (0..height as usize)
            .find(|&row| (0..w).any(|col| buf.data[3 * plane + row * w + col] > 0))
            .expect("row painted")
    };
    assert_eq!(top_row(&plain_buf), top_row(&marked_buf) + 5);
}

#[test]
fn kerning_reduces_a_kerned_pair_but_not_an_unkerned_control() {
    let bundled = bundled();
    let size = 48.0;
    let total = |glyphs: &[ShapedGlyph]| glyphs.iter().map(|g| g.advance).sum::<f32>();
    let pair_total = |pair: &str| total(&bundled.shape_line(pair, size, 0.0));
    let alone = |pair: &str| {
        pair.chars()
            .map(|c| total(&bundled.shape_line(&c.to_string(), size, 0.0)))
            .sum::<f32>()
    };

    let candidates = ["AV", "AW", "To", "Va", "Ya", "We", "LT", "Ty"];
    let pair = candidates
        .into_iter()
        .find(|pair| pair_total(pair) < alone(pair))
        .expect("Liberation Sans kerns at least one known pair");
    assert!(
        pair_total(pair) < alone(pair),
        "{pair} kerns below the glyphs shaped alone"
    );

    let control = "AA";
    assert!(
        pair_total(control) >= alone(control) - 1e-4,
        "unkerned {control} is not reduced"
    );
}

#[test]
fn render_text_layer_paints_and_clears_the_type_tool() {
    let mut doc = doc_with(type_layer(Some(style())));
    assert!(render_text_layer(&mut doc, "0"));
    let layer = crate::resolve_path(&doc, "0").unwrap();
    assert!(layer.type_tool.is_none(), "type tool is cleared");
    assert!(
        layer.extra_blocks.iter().all(|b| &b.key != b"TySh"),
        "TySh block is gone"
    );
    let alpha = crate::channel(layer, -1).expect("alpha channel");
    assert!(alpha.iter().any(|&a| a > 0), "non-empty alpha coverage");
    assert_eq!(alpha.len(), 200 * 80);
}

#[test]
fn render_text_layer_refuses_a_missing_style_without_mutating() {
    let mut doc = doc_with(type_layer(None));
    let before = doc.clone();
    assert!(!render_text_layer(&mut doc, "0"));
    assert_eq!(doc, before, "refusal leaves the document unchanged");
}

#[test]
fn materialize_text_rgba_refuses_a_wrong_size_buffer_without_mutating() {
    let mut doc = doc_with(type_layer(Some(style())));
    let before = doc.clone();
    assert!(!materialize_text_rgba(&mut doc, "0", &[0u8; 10]));
    assert_eq!(doc, before, "refusal leaves the document unchanged");
}

#[test]
fn materialize_text_rgba_refuses_a_missing_layer() {
    let mut doc = doc_with(type_layer(Some(style())));
    assert!(!materialize_text_rgba(&mut doc, "9", &[]));
}

#[test]
fn materialize_text_rgba_sets_channels_and_clears_the_type_tool() {
    let _gamma = crate::test_support::gamma_guard();
    let mut doc = doc_with(type_layer(Some(style())));
    let n = 200 * 80;
    let mut rgba = vec![0u8; n * 4];
    rgba[0] = 10;
    rgba[1] = 20;
    rgba[2] = 30;
    rgba[3] = 40;
    assert!(materialize_text_rgba(&mut doc, "0", &rgba));
    let layer = crate::resolve_path(&doc, "0").unwrap();
    assert!(layer.type_tool.is_none(), "type tool is cleared");
    assert!(
        layer.extra_blocks.iter().all(|b| &b.key != b"TySh"),
        "TySh block is gone"
    );
    assert_eq!(crate::channel(layer, 0).unwrap()[0], 10);
    assert_eq!(crate::channel(layer, 1).unwrap()[0], 20);
    assert_eq!(crate::channel(layer, 2).unwrap()[0], 30);
    assert_eq!(
        crate::channel(layer, -1).unwrap()[0],
        27,
        "alpha 40 through the curve"
    );
}

#[test]
fn materialize_bakes_the_curve() {
    let _gamma = crate::test_support::gamma_guard();
    let mut doc = doc_with(type_layer(Some(style())));
    let n = 200 * 80;
    let mut rgba = vec![0u8; n * 4];
    for (i, px) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        px[3] = (i % 256) as u8;
    }
    assert!(materialize_text_rgba(&mut doc, "0", &rgba));
    let alpha = crate::channel(crate::resolve_path(&doc, "0").unwrap(), -1).unwrap();
    for (i, &stored) in alpha.iter().enumerate().take(256) {
        let curved = crate::text_gamma::text_gamma_alpha(
            i as f32 / 255.0,
            crate::text_gamma::DEFAULT_TEXT_GAMMA,
        );
        let expected = i32::from(crate::composite::to_u8(curved));
        assert!(
            (i32::from(stored) - expected).abs() <= 1,
            "alpha {i}: {stored}"
        );
    }
}

#[test]
fn materialize_bakes_the_curve_when_opacity_is_zero() {
    let _gamma = crate::test_support::gamma_guard();
    let mut doc = doc_with(type_layer(Some(style())));
    crate::resolve_path_mut(&mut doc, "0").unwrap().opacity = 0;
    let n = 200 * 80;
    let mut rgba = vec![0u8; n * 4];
    for (i, px) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        px[3] = (i % 256) as u8;
    }
    assert!(materialize_text_rgba(&mut doc, "0", &rgba));
    let alpha = crate::channel(crate::resolve_path(&doc, "0").unwrap(), -1).unwrap();
    for (i, &stored) in alpha.iter().enumerate().take(256) {
        let curved = crate::text_gamma::text_gamma_alpha(
            i as f32 / 255.0,
            crate::text_gamma::DEFAULT_TEXT_GAMMA,
        );
        let expected = i32::from(crate::composite::to_u8(curved));
        assert!(
            (i32::from(stored) - expected).abs() <= 1,
            "alpha {i}: {stored}"
        );
    }
}

#[test]
fn rasterized_type_composites_as_live_type() {
    for opacity in [255u8, 128] {
        let mut layer = type_layer(Some(style()));
        layer.opacity = opacity;
        layer.fill = 255;
        let live = doc_with(layer);
        let mut raster = live.clone();
        assert!(render_text_layer(&mut raster, "0"));
        let (live, raster) = (crate::composite_rgba(&live), crate::composite_rgba(&raster));
        let plane = 200 * 80;
        let worst = live.data[3 * plane..]
            .iter()
            .zip(&raster.data[3 * plane..])
            .map(|(&l, &r)| (i32::from(l) - i32::from(r)).abs())
            .max()
            .unwrap();
        assert!(worst <= 2, "opacity {opacity}: alpha differs by {worst}");
    }
}

#[test]
fn provenance_records_the_bundled_substitution() {
    let provenance = bundled().provenance("Arial");
    assert_eq!(provenance.requested_family, "Arial");
    assert_eq!(provenance.resolved_family, "Liberation Sans");
    assert_eq!(provenance.backend, "rustybuzz/swash");
    assert_ne!(provenance.font_hash, [0u8; 32]);
}

#[test]
fn vertical_subpixel_tracks_the_fractional_baseline() {
    let bundled = bundled();
    let id = bundled.glyph_for('H');
    let paint = |dy: f32| {
        let layout = TextLayout {
            lines: vec![LayoutLine {
                glyphs: vec![PlacedGlyph {
                    id,
                    x: 20.0,
                    y: dy,
                    scale: 1.0,
                    x_scale: 1.0,
                    face: 0,
                    cluster: 0,
                }],
                advance: 0.0,
                baseline: 0.0,
            }],
            width: 0.0,
            height: 0.0,
            face_count: 1,
        };
        paint_layout(&bundled, &layout, 200, 120, 48.0, [0.0, 0.0, 0.0, 1.0]).expect("paints")
    };
    let centroid = |buf: &PixelBuffer| {
        let plane = buf.pixel_count();
        let (w, h) = (200usize, 120usize);
        let (mut sum, mut weight) = (0.0f64, 0.0f64);
        for row in 0..h {
            for col in 0..w {
                let a = buf.data[3 * plane + row * w + col] as f64;
                sum += a * row as f64;
                weight += a;
            }
        }
        sum / weight
    };
    let base = centroid(&paint(0.0));
    let half = centroid(&paint(0.5));
    let one = centroid(&paint(1.0));
    assert!(
        (one - base - 1.0).abs() < 0.02,
        "a full baseline shift moves the centroid one row: {base} -> {one}"
    );
    assert!(
        (half - base - 0.5).abs() < 0.05,
        "a half baseline shift moves the centroid half a row: {base} -> {half}"
    );
}

#[test]
fn rtl_line_reverses_visual_order() {
    let face = bundled();
    let c = CharacterAttrs {
        size: 32.0,
        ..CharacterAttrs::default()
    };
    let faces = crate::fonts::faces_in_order(&face);
    // A strongly right-to-left script: Latin stays left-to-right even inside an
    // RTL paragraph (UAX #9), so it cannot show the line reversing.
    let logical = shape_styled_faces(
        &faces,
        "\u{5D0}\u{5D1}\u{5D2}",
        &c,
        rustybuzz::Direction::LeftToRight,
    );
    let mut expected: Vec<u16> = logical.iter().map(|g| g.id).collect();
    expected.reverse();
    let rtl = shape_line_directed(&faces, "\u{5D0}\u{5D1}\u{5D2}", &c, TextDirection::Rtl);
    let actual: Vec<u16> = rtl.iter().map(|g| g.id).collect();
    assert_eq!(actual, expected);
}

#[test]
fn mixed_direction_keeps_runs() {
    let face = bundled();
    let c = CharacterAttrs {
        size: 32.0,
        ..CharacterAttrs::default()
    };
    let faces = crate::fonts::faces_in_order(&face);
    let mixed = shape_line_directed(&faces, "אבab", &c, TextDirection::Rtl);
    let latin = shape_styled(&face, "ab", &c);
    let latin_ids: Vec<u16> = latin.iter().map(|g| g.id).collect();
    assert!(
        mixed
            .windows(latin_ids.len())
            .any(|run| run.iter().map(|g| g.id).eq(latin_ids.iter().copied())),
        "the Latin run keeps its left-to-right order"
    );
}

#[test]
fn rtl_line_starts_at_right_edge() {
    let face = bundled();
    let c = CharacterAttrs {
        size: 32.0,
        ..CharacterAttrs::default()
    };
    let faces = crate::fonts::faces_in_order(&face);
    let glyphs = shape_line_directed(&faces, "\u{5D0}\u{5D1}\u{5D2}", &c, TextDirection::Rtl);
    let mut p = LayoutParams {
        font_size: 32.0,
        units_per_em: 32.0,
        tracking: 0.0,
        leading: 38.4,
        align: TextAlign::Left,
        wrap_width: Some(200.0),
    };
    p.font_size = 32.0;
    p.units_per_em = 32.0;
    let layout = layout_lines(&[glyphs], &p);
    let line = &layout.lines[0];
    let first = line
        .glyphs
        .iter()
        .find(|g| g.cluster == 0)
        .expect("first char");
    let max = line.glyphs.iter().map(|g| g.x).fold(f32::MIN, f32::max);
    assert!(first.x >= max, "the first logical glyph is rightmost");
}

#[test]
fn ltr_line_unchanged_by_direction() {
    let face = bundled();
    let c = CharacterAttrs {
        size: 32.0,
        ..CharacterAttrs::default()
    };
    assert_eq!(
        shape_line_directed(
            &crate::fonts::faces_in_order(&face),
            "abc",
            &c,
            TextDirection::Ltr
        ),
        shape_styled(&face, "abc", &c)
    );
}

/// A set type layer, scaled 3× by Free Transform and then moved, as the canvas
/// shows it: its rect and planes.
fn transformed_type_doc() -> Document {
    use crate::document_ops::{transform_layer, LayerTransform};
    let mut doc = Document::new(600, 400, ColorMode::Rgb, BitDepth::Eight);
    let mut spec = pictura_core::TypeSpec::new("Lorem", "Liberation Sans", 24.0);
    spec.origin = (40.0, 60.0);
    spec.character.fill_color = [1.0, 0.0, 0.0, 1.0];
    let path = crate::add_type_layer(&mut doc, "", &spec);
    let scale = LayerTransform {
        scale_x: 3.0,
        scale_y: 3.0,
        angle_radians: 0.0,
        dx: 0.0,
        dy: 0.0,
    };
    assert!(transform_layer(&mut doc, &path, scale));
    assert!(crate::document_ops::translate_layer_path(
        &mut doc, &path, 25, 30
    ));
    assert!(doc.layers[0].is_type());
    doc
}

#[test]
fn rasterize_keeps_a_transformed_type_layer_as_shown() {
    let mut doc = transformed_type_doc();
    let live = doc.clone();
    let shown = live.layers[0].rect;
    assert!(render_text_layer(&mut doc, "0"));
    let layer = &doc.layers[0];
    assert!(layer.type_tool.is_none(), "type tool is cleared");
    assert!(layer.extra_blocks.iter().all(|b| &b.key != b"TySh"));
    assert_eq!(layer.rect, shown, "the stretched, moved rect is kept");
    for id in [0, 1, 2] {
        assert_eq!(
            crate::channel(layer, id),
            crate::channel(&live.layers[0], id),
            "colour plane {id} is the on-canvas render"
        );
    }
    let alpha = crate::channel(layer, -1).unwrap();
    let curved = crate::channel(&live.layers[0], -1)
        .unwrap()
        .iter()
        .map(|&a| {
            crate::composite::to_u8(crate::text_gamma::text_gamma_alpha(
                f32::from(a) / 255.0,
                crate::text_gamma::DEFAULT_TEXT_GAMMA,
            ))
        });
    assert!(
        alpha.iter().zip(curved).all(|(&a, c)| a.abs_diff(c) <= 1),
        "alpha carries the baked text gamma"
    );
    let (live, raster) = (crate::composite_rgba(&live), crate::composite_rgba(&doc));
    let worst = live
        .data
        .iter()
        .zip(&raster.data)
        .map(|(&l, &r)| l.abs_diff(r))
        .max()
        .unwrap();
    assert!(worst <= 2, "composite differs by {worst}");
}

#[test]
fn rasterize_unlocks_type_and_drops_its_style_state() {
    let mut doc = transformed_type_doc();
    doc.layers[0].applied_paragraph_style = Some("Body".into());
    assert_ne!(doc.layers[0].lock.bits(), 0, "type carries its locks");
    assert!(render_text_layer(&mut doc, "0"));
    let layer = &doc.layers[0];
    assert_eq!(layer.lock.bits(), 0, "the raster paints like any layer");
    assert!(layer.applied_paragraph_style.is_none());
}
