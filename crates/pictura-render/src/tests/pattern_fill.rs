use super::*;
use pictura_adjust::{Adjustment, PatternFillParams};

fn params(pattern_id: &str, scale: f32, link: bool, origin: (i32, i32)) -> PatternFillParams {
    PatternFillParams {
        pattern_id: pattern_id.into(),
        scale,
        link_with_layer: link,
        origin,
    }
}

fn object(class_id: &[u8], items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: class_id.to_vec(),
        items,
    }
}

fn ptrn(id: &str) -> DescValue {
    object(
        b"Ptrn",
        vec![(b"Idnt".to_vec(), DescValue::Text(format!("{id}\0")))],
    )
}

fn ptfl_from(items: Vec<(Vec<u8>, DescValue)>) -> AdjustmentData {
    adjdata(*b"PtFl", write_descriptor(&object(b"PtFl", items)))
}

#[test]
fn pattern_fill_decodes_params() {
    let d = ptfl(&params("pictura-pattern", 50.0, false, (3, 4)));
    assert_eq!(
        decode_adjustment(&d),
        Some(Adjustment::PatternFill(params(
            "pictura-pattern",
            50.0,
            false,
            (3, 4)
        )))
    );
}

#[test]
fn pattern_fill_defaults_and_unit_float() {
    // No Scl / Algn: scale 100 and link true.
    let minimal = ptfl_from(vec![(b"Ptrn".to_vec(), ptrn("p"))]);
    assert_eq!(
        decode_adjustment(&minimal),
        Some(Adjustment::PatternFill(params("p", 100.0, true, (0, 0))))
    );

    // A unit-float scale decodes like a double.
    let unit = ptfl_from(vec![
        (b"Ptrn".to_vec(), ptrn("p")),
        (
            b"Scl ".to_vec(),
            DescValue::UnitFloat {
                unit: *b"#Prc",
                value: 87.0,
            },
        ),
        (b"Algn".to_vec(), DescValue::Bool(false)),
    ]);
    assert_eq!(
        decode_adjustment(&unit),
        Some(Adjustment::PatternFill(params("p", 87.0, false, (0, 0))))
    );
}

#[test]
fn pattern_fill_decode_rejects_malformed() {
    // Missing Ptrn.
    assert_eq!(
        decode_adjustment(&ptfl_from(vec![(b"Algn".to_vec(), DescValue::Bool(true))])),
        None
    );
    // Wrong-typed Idnt.
    let bad_id = ptfl_from(vec![(
        b"Ptrn".to_vec(),
        object(b"Ptrn", vec![(b"Idnt".to_vec(), DescValue::Long(3))]),
    )]);
    assert_eq!(decode_adjustment(&bad_id), None);
    // Non-finite scale.
    let bad_scale = ptfl_from(vec![
        (b"Ptrn".to_vec(), ptrn("p")),
        (b"Scl ".to_vec(), DescValue::Double(f64::NAN)),
    ]);
    assert_eq!(decode_adjustment(&bad_scale), None);
    // Not a descriptor at all.
    assert_eq!(
        decode_adjustment(&adjdata(*b"PtFl", vec![0, 1, 2, 3])),
        None
    );
    // A truncated descriptor.
    let mut truncated = ptfl(&params("p", 100.0, true, (0, 0)));
    truncated.data.truncate(truncated.data.len() / 2);
    assert_eq!(decode_adjustment(&truncated), None);
    // A `Ptrn` item whose class id is not `Ptrn`.
    let bad_class = ptfl_from(vec![(
        b"Ptrn".to_vec(),
        object(
            b"null",
            vec![(b"Idnt".to_vec(), DescValue::Text("p\0".into()))],
        ),
    )]);
    assert_eq!(decode_adjustment(&bad_class), None);
}

#[test]
fn fixture_pattern_fill_decodes_params() {
    let doc = pattern_fixture_doc();
    let fill = doc
        .layers
        .iter()
        .find(|l| l.name == "Pattern Fill")
        .expect("Pattern Fill layer");
    assert_eq!(
        decode_adjustment(fill.adjustment.as_ref().expect("PtFl block")),
        Some(Adjustment::PatternFill(params(
            "pictura-pattern",
            100.0,
            true,
            (0, 0)
        )))
    );
}

#[test]
fn fixture_pattern_fill_tiles_exactly() {
    let out = composite_rgba(&pattern_fixture_doc());
    for y in 0..8u32 {
        for x in 0..8u32 {
            assert_eq!(
                rgb(&out, x, y),
                FIXTURE_TILE[(y % 2) as usize][(x % 2) as usize],
                "tile cell at ({x}, {y})"
            );
            assert_eq!(px(&out, x, y)[3], 255, "pattern alpha at ({x}, {y})");
        }
    }
}

#[test]
fn pattern_fill_differs_from_backdrop_and_repeats() {
    let mut hidden = pattern_fixture_doc();
    hidden.layers[1].visible = false;
    let plain = composite_rgba(&hidden);

    let out = composite_rgba(&pattern_fixture_doc());
    assert_ne!(out.data, plain.data, "the fill must change the composite");
    assert_eq!(rgb(&out, 0, 0), FIXTURE_TILE[0][0]);
    assert_eq!(rgb(&out, 0, 0), rgb(&out, 2, 0), "one tile width repeats");
    assert_ne!(rgb(&out, 0, 0), rgb(&out, 1, 0), "adjacent cells differ");
}

#[test]
fn pattern_fill_masked_out_is_noop() {
    let mut masked = pattern_fixture_doc();
    masked.layers[1].mask = Some(LayerMask {
        rect: full(8, 8),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(vec![0; 64].into()),
        ..Default::default()
    });
    let out = composite_rgba(&masked);

    let mut hidden = pattern_fixture_doc();
    hidden.layers[1].visible = false;
    let plain = composite_rgba(&hidden);
    assert_eq!(out.data, plain.data, "a masked-out fill is a no-op");
}

#[test]
fn missing_pattern_uses_placeholder() {
    let mut doc = pattern_fixture_doc();
    resolve_path_mut(&mut doc, "1").unwrap().adjustment =
        Some(ptfl(&params("not-installed", 100.0, true, (0, 0))));
    let out = composite_rgba(&doc);
    assert_eq!(rgb(&out, 3, 5), [128, 128, 128], "opaque 50% grey");
    assert_eq!(px(&out, 3, 5)[3], 255, "placeholder is opaque");
}

#[test]
fn link_with_layer_false_anchors_at_document_origin() {
    let mut doc = pattern_fixture_doc();
    let mut layer = doc.layers[1].clone();
    layer.rect = rect(1, 1, 8, 8); // top, left, bottom, right: a (1,1) offset
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    resolve_path_mut(&mut doc, "0").unwrap().adjustment =
        Some(ptfl(&params("pictura-pattern", 100.0, false, (0, 0))));

    let out = composite_rgba(&doc);
    // With the anchor at the document origin, canvas (1,1) samples tile (1,1):
    // white. With Link With Layer it would sample the layer-local (0,0): red.
    assert_eq!(rgb(&out, 1, 1), [255, 255, 255]);
    assert_eq!(rgb(&out, 2, 1), [0, 0, 255]);
}

#[test]
fn link_with_layer_true_anchors_at_layer_rect() {
    let mut doc = pattern_fixture_doc();
    let mut layer = doc.layers[1].clone();
    layer.rect = rect(1, 1, 8, 8); // top, left, bottom, right: a (1,1) offset
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    resolve_path_mut(&mut doc, "0").unwrap().adjustment =
        Some(ptfl(&params("pictura-pattern", 100.0, true, (0, 0))));

    let out = composite_rgba(&doc);
    // Link With Layer anchors the tile at the layer's top-left, so the layer
    // origin does not shift the pattern: canvas (1,1) is the tile's (0,0).
    assert_eq!(
        rgb(&out, 1, 1),
        FIXTURE_TILE[0][0],
        "layer top-left is cell 0,0"
    );
    assert_eq!(rgb(&out, 2, 1), FIXTURE_TILE[0][1]);
    assert_eq!(rgb(&out, 1, 2), FIXTURE_TILE[1][0]);
    assert_eq!(rgb(&out, 2, 2), FIXTURE_TILE[1][1]);
}

#[test]
fn non_zero_origin_shifts_the_tile() {
    fn shifted(origin: (i32, i32)) -> [u8; 3] {
        let mut doc = pattern_fixture_doc();
        resolve_path_mut(&mut doc, "1").unwrap().adjustment =
            Some(ptfl(&params("pictura-pattern", 100.0, true, origin)));
        rgb(&composite_rgba(&doc), 0, 0)
    }

    assert_eq!(shifted((0, 0)), FIXTURE_TILE[0][0], "no offset is cell 0,0");
    assert_eq!(
        shifted((1, 0)),
        FIXTURE_TILE[0][1],
        "a +1 x origin samples the next cell"
    );
    assert_eq!(
        shifted((0, 1)),
        FIXTURE_TILE[1][0],
        "a +1 y origin samples the next row"
    );
    assert_eq!(shifted((-1, 0)), FIXTURE_TILE[0][1], "modular in x");
}

#[test]
fn fixture_16bit_pattern_falls_back_to_placeholder() {
    let bytes = include_bytes!("../../../pictura-codec/tests/fixtures/pattern_fill_16bit.psd");
    let doc = pictura_codec::read_psd(bytes).expect("fixture parses");
    let out = composite_rgba(&doc);
    assert_eq!(
        rgb(&out, 0, 0),
        [128, 128, 128],
        "a skipped 16-bit pattern renders the placeholder"
    );
    assert_eq!(px(&out, 0, 0)[3], 255);
}

#[test]
fn psd_tools_draw_pattern_fill_matches_composite() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools + skimage not available");
        return;
    }
    let dir = scratch_dir("draw-pattern-fill");
    let path = dir.join("pattern_fill.psd");
    std::fs::write(&path, PATTERN_FIXTURE).unwrap();

    let script = r#"
import sys
import numpy as np
from psd_tools import PSDImage
from psd_tools.constants import Tag
from psd_tools.composite.paint import draw_pattern_fill
psd = PSDImage.open(sys.argv[1], lazy=False)
layer = psd[1]
desc = layer.tagged_blocks.get_data(Tag.PATTERN_FILL_SETTING)
rgb, _alpha = draw_pattern_fill((0, 0, 8, 8), psd, desc)
q = np.round(rgb * 255).astype(np.uint8)
for c in range(3):
    print(q[:, :, c].tobytes().hex())
"#;
    let out = std::process::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&path)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "psd-tools failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let got: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .map(str::to_string)
        .collect();
    let composite = composite_rgba(&pattern_fixture_doc());
    let plane = 64;
    let ours: Vec<String> = (0..3)
        .map(|c| {
            composite.data[c * plane..(c + 1) * plane]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        })
        .collect();
    assert_eq!(got, ours, "psd-tools tiling matches the composite");
}

fn psd_tools_available() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        std::process::Command::new("python3")
            .args(["-c", "import psd_tools, skimage, numpy"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

fn scratch_dir(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "pictura-render-pattern-{}-{tag}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn encode_pattern_fill_round_trips() {
    let source = params("p1", 50.0, false, (3, 4));
    let data = crate::encode_pattern_fill(&source);
    assert_eq!(
        decode_adjustment(&data),
        Some(Adjustment::PatternFill(source))
    );
}

#[test]
fn encode_pattern_fill_omits_default_origin() {
    let source = params("p1", 100.0, true, (0, 0));
    let data = crate::encode_pattern_fill(&source);
    let Ok(DescValue::Object { items, .. }) = pictura_codec::read_descriptor(&data.data) else {
        panic!("PtFl descriptor");
    };
    assert!(!items.iter().any(|(key, _)| key.as_slice() == b"phase"));
    assert_eq!(
        decode_adjustment(&data),
        Some(Adjustment::PatternFill(source))
    );
}
