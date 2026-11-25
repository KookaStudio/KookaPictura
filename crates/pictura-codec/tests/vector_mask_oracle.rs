//! Independent `psd-tools` oracle for the `vmsk`/`vsms` vector-mask fixtures
//! and for the shape layers the shape tools author.
//!
//! `oracle.rs` is at its 1400-line cap, so this check lives in its own binary.
//! It reads a PSD with `psd-tools` (not with `pictura-codec`) and reports each
//! vector mask's version, flags, closed subpath, and the even-odd pixel
//! coverage of its anchors' polygon.
//!
//! Needs `python3` with `psd-tools`; self-skips with a message otherwise.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use pictura_codec::{read_psd, write_psd};
use pictura_core::{Document, LayerBlock, LayerMask, PsdRect};

const SCRIPT: &str = r#"
import sys
from psd_tools import PSDImage
from psd_tools.constants import Tag

psd = PSDImage.open(sys.argv[1], lazy=False)
w, h = psd.width, psd.height
lines = []
for layer in psd:
    blocks = layer.tagged_blocks
    block = blocks.get(Tag.VECTOR_MASK_SETTING1) if blocks else None
    if block is None and blocks:
        block = blocks.get(Tag.VECTOR_MASK_SETTING2)
    if block is None:
        continue
    mask = block.data
    closed = [item for item in mask.path if type(item).__name__ == "ClosedPath"]
    knots = []
    for subpath in closed:
        for knot in subpath:
            knots.append((knot.anchor[1] * w, knot.anchor[0] * h))
    inside = 0
    for py in range(h):
        for px in range(w):
            cx, cy = px + 0.5, py + 0.5
            crossings = 0
            for i in range(len(knots)):
                x0, y0 = knots[i]
                x1, y1 = knots[(i + 1) % len(knots)]
                if (y0 > cy) != (y1 > cy):
                    xint = x0 + (cy - y0) * (x1 - x0) / (y1 - y0)
                    if cx < xint:
                        crossings += 1
            if crossings % 2 == 1:
                inside += 1
    operation = closed[0].operation if closed else -99
    fields = [layer.name, str(mask.version), str(mask.flags), str(len(closed)),
              str(operation), str(inside)]
    fields.append(" ".join(f"{x:g},{y:g}" for x, y in knots))
    lines.append("|".join(fields))
print("\n".join(lines))
"#;

fn psd_tools_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("python3")
            .args(["-c", "import psd_tools"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

/// One `|`-separated row per vector-mask layer, as `SCRIPT` prints them.
fn read_with_psd_tools(path: &std::path::Path) -> (String, Vec<Vec<String>>) {
    run_psd_tools(SCRIPT, path)
}

fn run_psd_tools(script: &str, path: &std::path::Path) -> (String, Vec<Vec<String>>) {
    let out = Command::new("python3")
        .args(["-c", script, path.to_str().expect("utf-8 path")])
        .output()
        .expect("run python3");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "psd-tools failed:\n{stdout}\n{stderr}"
    );
    let lines = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split('|').map(str::to_owned).collect())
        .collect();
    (stdout, lines)
}

#[test]
fn psd_tools_reads_vector_mask_fixture() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vector_mask.psd");
    let (stdout, lines) = read_with_psd_tools(&fixture);
    assert_eq!(lines.len(), 2, "two vector-mask layers:\n{stdout}");

    assert_eq!(
        lines[0],
        vec![
            "Shape Inverted",
            "3",
            "1",
            "1",
            "1",
            "36",
            "0,0 6,0 6,6 0,6"
        ],
        "inverted (0,0)-(6,6) rectangle: {stdout}"
    );
    assert_eq!(
        lines[1],
        vec!["Shape", "3", "0", "1", "1", "4", "1,1 3,1 3,3 1,3"],
        "normal (1,1)-(3,3) rectangle: {stdout}"
    );
}

fn vsms_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vector_mask_vsms.psd")
}

fn read_vsms_fixture() -> pictura_core::Document {
    let bytes = std::fs::read(vsms_fixture()).expect("read vector_mask_vsms.psd");
    pictura_codec::read_psd(&bytes).expect("vector_mask_vsms.psd parses")
}

#[test]
fn vsms_fixture_decodes() {
    let doc = read_vsms_fixture();
    let shape = doc
        .layers
        .iter()
        .find(|layer| layer.name == "Shape")
        .expect("Shape layer");
    let mask = shape.vector_mask.as_ref().expect("Shape vsms mask");
    assert!(!mask.invert && !mask.disabled);
    assert_eq!(mask.subpaths.len(), 1);
    assert!(mask.subpaths[0].points.contains(&[256, 256]));
    assert!(mask.subpaths[0].points.contains(&[5 * 256, 5 * 256]));
    assert!(
        shape.extra_block(b"vmsk").is_none(),
        "no vmsk in the fixture"
    );
}

#[test]
fn vsms_fixture_round_trips() {
    let doc = read_vsms_fixture();
    let before = doc
        .layers
        .iter()
        .find(|layer| layer.name == "Shape")
        .and_then(|layer| layer.extra_block(b"vsms"))
        .expect("vsms block")
        .data
        .clone();

    let bytes = pictura_codec::write_psd(&doc).expect("writes");
    let back = pictura_codec::read_psd(&bytes).expect("re-reads");
    let shape = back
        .layers
        .iter()
        .find(|layer| layer.name == "Shape")
        .expect("Shape layer");
    assert!(shape.vector_mask.is_some());
    assert_eq!(
        shape.extra_block(b"vsms").expect("vsms preserved").data,
        before
    );
    assert_eq!(back, doc, "the whole document round-trips");
}

#[test]
fn vsms_fixture_matches_psd_tools() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    let (stdout, lines) = read_with_psd_tools(&vsms_fixture());
    assert_eq!(lines.len(), 1, "one vector-mask layer:\n{stdout}");
    assert_eq!(
        lines[0],
        vec!["Shape", "3", "0", "1", "1", "16", "1,1 5,1 5,5 1,5"],
        "the (1,1)-(5,5) vsms rectangle: {stdout}"
    );
}

const DERIVED_SCRIPT: &str = r#"
import sys
from psd_tools import PSDImage
from psd_tools.constants import Tag

psd = PSDImage.open(sys.argv[1], lazy=False)
for layer in psd:
    blocks = layer.tagged_blocks
    mask = layer._record.mask_data
    params = mask.parameters if mask is not None else None
    value = lambda v: "-" if v is None else str(v)
    print("|".join([
        layer.name,
        "1" if blocks is not None and Tag.VECTOR_MASK_SETTING1 in blocks else "0",
        "1" if mask is not None and mask.flags.user_mask_from_render else "0",
        "1" if mask is not None else "0",
        value(params.user_mask_density if params else None),
        value(params.vector_mask_density if params else None),
        value(params.vector_mask_feather if params else None),
    ]))
"#;

fn derived_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/derived_raster_mask.psd")
}

#[test]
fn codec_suppresses_a_rasterized_source_path() {
    let fixture = std::fs::read(derived_fixture_path()).expect("read derived_raster_mask.psd");
    let doc = read_psd(&fixture).expect("derived_raster_mask.psd parses");
    let layer = doc
        .layers
        .iter()
        .find(|layer| layer.name == "Derived")
        .expect("Derived layer");
    let mask = layer.mask.as_ref().expect("the raster mask stays");
    assert!(mask.data.is_some(), "the raster mask pixels decoded");
    assert_eq!(mask.density, 128, "the user density is kept");
    assert_eq!(mask.feather(), 0.0);
    assert!(
        layer.vector_mask.is_none(),
        "the rendered source path is not a live vector mask"
    );
    let block = layer
        .extra_block(b"vmsk")
        .expect("the raw vmsk block stays")
        .data
        .clone();
    assert!(
        pictura_codec::decode_vector_mask(&block, 8, 8).is_some(),
        "the block is parseable; suppression is not a decode failure"
    );

    let back = read_psd(&write_psd(&doc).expect("writes")).expect("re-reads");
    assert_eq!(back, doc, "the round trip preserves the document");
    let layer = back
        .layers
        .iter()
        .find(|layer| layer.name == "Derived")
        .expect("Derived layer");
    assert!(layer.vector_mask.is_none());
    assert_eq!(layer.mask.as_ref().expect("mask").density, 128);
    assert_eq!(
        layer.extra_block(b"vmsk").expect("raw vmsk block").data,
        block
    );
}

#[test]
fn a_live_vector_mask_without_a_real_user_channel_survives_save() {
    let mut doc = Document::from_rgba("Base", 8, 8, &[255; 8 * 8 * 4]);
    let data = pictura_codec::encode_vector_mask(&[], 8, 8);
    let layer = &mut doc.layers[0];
    layer.mask = Some(LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 8,
            right: 8,
        },
        flags: 0x08,
        ..Default::default()
    });
    layer.vector_mask = pictura_codec::decode_vector_mask(&data, 8, 8);
    pictura_codec::edit_layer(layer, |parts| {
        parts.extra_blocks.push(LayerBlock {
            key: *b"vmsk",
            data: data.into(),
        })
    });

    let back = read_psd(&write_psd(&doc).expect("writes")).expect("re-reads");
    assert!(
        back.layers[0].vector_mask.is_some(),
        "a live mask with no -3 channel survives a save"
    );
    assert_eq!(
        back.layers[0].mask.as_ref().expect("mask").flags & 0x08,
        0,
        "the writer clears the stale render marker"
    );
}

#[test]
fn codec_preserves_vector_mask_blocks() {
    let fixture = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vector_mask.psd"),
    )
    .expect("read vector_mask.psd");
    let doc = read_psd(&fixture).expect("vector_mask.psd parses");
    let back = read_psd(&write_psd(&doc).expect("writes")).expect("re-reads");
    assert_eq!(back, doc, "the vmsk blocks stay byte-identical");
}

#[test]
fn psd_tools_reads_the_derived_raster_mask_fixture() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    let (stdout, lines) = run_psd_tools(DERIVED_SCRIPT, &derived_fixture_path());
    assert_eq!(lines.len(), 1, "one layer:\n{stdout}");
    assert_eq!(
        lines[0],
        ["Derived", "1", "1", "1", "128", "200", "4.0"].map(str::to_owned),
        "the source-path facts: {stdout}"
    );

    let doc = read_psd(&std::fs::read(derived_fixture_path()).expect("read"))
        .expect("derived_raster_mask.psd parses");
    let path =
        std::env::temp_dir().join(format!("pictura-derived-mask-{}.psd", std::process::id()));
    std::fs::write(&path, write_psd(&doc).expect("writes")).expect("write");
    let (saved_stdout, saved) = run_psd_tools(DERIVED_SCRIPT, &path);
    let _ = std::fs::remove_file(&path);
    assert_eq!(saved.len(), 1, "one layer:\n{saved_stdout}");
    assert_eq!(
        saved[0],
        ["Derived", "1", "1", "1", "128", "-", "-"].map(str::to_owned),
        "the saved record normalizes the vector parameters: {saved_stdout}"
    );
}
