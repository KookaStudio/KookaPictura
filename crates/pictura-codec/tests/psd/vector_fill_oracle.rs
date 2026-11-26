//! Independent `psd-tools` oracle for the `vscg` vector-fill fixture.
//!
//! `oracle.rs` is at its 1400-line cap, so this check lives in its own binary.
//! It reads `vector_fill.psd` with `psd-tools` (not with `pictura-codec`) and
//! reports the shape layer's `vscg` key, version, and colour, and the composite
//! colour inside and outside the authored `vmsk` rectangle.
//!
//! Needs `python3` with `psd-tools`; self-skips with a message otherwise.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use pictura_codec::DescValue;

const SCRIPT: &str = r#"
import sys
from psd_tools import PSDImage
from psd_tools.constants import Tag

psd = PSDImage.open(sys.argv[1], lazy=False)
lines = []
for layer in psd:
    block = layer.tagged_blocks.get(Tag.VECTOR_STROKE_CONTENT_DATA) if layer.tagged_blocks else None
    if block is None:
        continue
    content = block.data
    clr = content[b"Clr "]
    fields = [
        layer.name,
        layer.kind,
        content.key.decode("ascii"),
        str(content.version),
        f'{clr[b"Rd  "].value:g},{clr[b"Grn "].value:g},{clr[b"Bl  "].value:g}',
    ]
    lines.append("|".join(fields))
comp = psd.composite(force=True)
inside = comp.getpixel((3, 3))
outside = comp.getpixel((0, 0))
lines.append("inside|" + ",".join(str(c) for c in inside))
lines.append("outside|" + ",".join(str(c) for c in outside))
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

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vector_fill.psd")
}

#[test]
fn psd_tools_reads_vector_fill_fixture() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    let fixture = fixture_path();
    let out = Command::new("python3")
        .args(["-c", SCRIPT, fixture.to_str().expect("utf-8 fixture path")])
        .output()
        .expect("run python3");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "psd-tools failed:\n{stdout}\n{stderr}"
    );

    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        3,
        "one vscg layer plus two composite samples:\n{stdout}"
    );
    assert_eq!(
        lines[0], "Shape|shape|SoCo|16|255,0,0",
        "shape layer vscg key/version/colour: {stdout}"
    );
    assert_eq!(lines[1], "inside|255,0,0,255", "inside the vmsk: {stdout}");
    assert_eq!(
        lines[2], "outside|200,100,50,255",
        "outside the vmsk: {stdout}"
    );
}

#[test]
fn codec_preserves_vector_fill_block() {
    let bytes = std::fs::read(fixture_path()).expect("read vector_fill.psd");
    let doc = pictura_codec::read_psd(&bytes).expect("vector_fill.psd parses");

    let shape = doc
        .layers
        .iter()
        .find(|l| l.name == "Shape")
        .expect("Shape layer");
    let block = shape
        .extra_block(b"vscg")
        .expect("the raw vscg block stays in extra_blocks");
    assert_eq!(block.key, *b"vscg");
    assert_eq!(&block.data[..4], b"SoCo", "the fill key prefixes the block");
    let version = u32::from_be_bytes(block.data[4..8].try_into().expect("version field"));
    assert_eq!(version, 16, "a version-16 descriptor block");

    let back = pictura_codec::read_psd(&pictura_codec::write_psd(&doc).expect("writes"))
        .expect("re-reads");
    assert_eq!(back, doc, "whole-document round trip preserves the block");
}

#[test]
fn decode_vector_fill_matches_authored_solid_fill() {
    let bytes = std::fs::read(fixture_path()).expect("read vector_fill.psd");
    let doc = pictura_codec::read_psd(&bytes).expect("vector_fill.psd parses");
    let shape = doc
        .layers
        .iter()
        .find(|l| l.name == "Shape")
        .expect("Shape layer");
    let block = shape.extra_block(b"vscg").expect("vscg block");

    let obj = pictura_codec::read_descriptor(&block.data[4..]).expect("descriptor");
    let DescValue::Object { items, .. } = &obj else {
        panic!("descriptor object");
    };
    let clr = items
        .iter()
        .find(|(k, _)| k.as_slice() == b"Clr ")
        .map(|(_, v)| v)
        .expect("Clr item");
    let DescValue::Object {
        class_id, items, ..
    } = clr
    else {
        panic!("Clr object");
    };
    assert_eq!(class_id.as_slice(), b"RGBC");
    let component = |key: &[u8]| {
        items
            .iter()
            .find(|(k, _)| k.as_slice() == key)
            .map(|(_, v)| v.clone())
            .expect("component")
    };
    assert_eq!(component(b"Rd  "), DescValue::Double(255.0));
    assert_eq!(component(b"Grn "), DescValue::Double(0.0));
    assert_eq!(component(b"Bl  "), DescValue::Double(0.0));
}
