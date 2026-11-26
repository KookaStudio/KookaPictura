//! Layer record flag bit 4 (pixel data irrelevant) through the PSD codec.
//!
//! `vector_fill.psd` is the positive case (its shape layer is `0x18`), and
//! `vector_mask.psd` is the negative case (vector-masked fill layers with bit 4
//! clear). The psd-tools checks self-skip when `python3` + `psd-tools` is not
//! available, following the existing oracle convention.

use pictura_testkit::oracle;
use std::path::PathBuf;

use pictura_core::{Document, Layer};

const KIND_SCRIPT: &str = r#"
import sys
from psd_tools import PSDImage

psd = PSDImage.open(sys.argv[1], lazy=False)
for layer in psd:
    f = layer._record.flags
    raw = ((f.transparency_protected * 1)
           | ((not f.visible) * 2)
           | (f.obsolete * 4)
           | (f.photoshop_v5_later * 8)
           | (f.pixel_data_irrelevant * 16)
           | (f.undocumented_1 * 32)
           | (f.undocumented_2 * 64)
           | (f.undocumented_3 * 128))
    print("|".join([layer.name, layer.kind, "%02x" % raw]))
"#;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn read_fixture(name: &str) -> Document {
    let bytes = std::fs::read(fixture_path(name)).expect("read fixture");
    pictura_codec::read_psd(&bytes).expect("fixture parses")
}

fn layer_named<'a>(doc: &'a Document, name: &str) -> &'a Layer {
    doc.layers
        .iter()
        .find(|layer| layer.name.as_str() == name)
        .unwrap_or_else(|| panic!("layer {name}"))
}

/// One `|`-separated `name|kind|flags` row per layer.
fn read_with_psd_tools(path: &std::path::Path) -> (String, Vec<Vec<String>>) {
    let out = oracle::Command::new("python3")
        .args(["-c", KIND_SCRIPT, path.to_str().expect("utf-8 path")])
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

fn row(lines: &[Vec<String>], name: &str, stdout: &str) -> Vec<String> {
    lines
        .iter()
        .find(|line| line.first().is_some_and(|field| field.as_str() == name))
        .unwrap_or_else(|| panic!("no {name} row in:\n{stdout}"))
        .clone()
}

fn expected(fields: &[&str]) -> Vec<String> {
    fields.iter().map(|field| (*field).to_owned()).collect()
}

fn write_temp(tag: &str, bytes: &[u8]) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "pictura-{tag}-{}-{}.psd",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|time| time.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::write(&path, bytes).expect("write temp PSD");
    path
}

#[test]
fn codec_reads_and_round_trips_the_pixel_data_irrelevant_flag() {
    let doc = read_fixture("vector_fill.psd");
    assert!(layer_named(&doc, "Shape").pixel_data_irrelevant);
    assert!(!layer_named(&doc, "Base").pixel_data_irrelevant);

    let psd = pictura_codec::write_psd(&doc).expect("writes PSD");
    let psd_back = pictura_codec::read_psd(&psd).expect("re-reads PSD");
    assert_eq!(psd_back, doc, "the PSD round trip equals the input");
    assert!(layer_named(&psd_back, "Shape").pixel_data_irrelevant);

    let mut expected_psb = doc.clone();
    expected_psb.is_psb = true;
    let psb = pictura_codec::write_psb(&doc).expect("writes PSB");
    let psb_back = pictura_codec::read_psd(&psb).expect("re-reads PSB");
    assert_eq!(
        psb_back, expected_psb,
        "the PSB round trip equals the input"
    );
    assert!(layer_named(&psb_back, "Shape").pixel_data_irrelevant);
}

#[test]
fn codec_preserves_a_clear_flag_on_vector_masked_fill_layers() {
    let doc = read_fixture("vector_mask.psd");
    assert!(doc.layers.iter().all(|layer| !layer.pixel_data_irrelevant));

    for bytes in [
        pictura_codec::write_psd(&doc).expect("writes PSD"),
        pictura_codec::write_psb(&doc).expect("writes PSB"),
    ] {
        let back = pictura_codec::read_psd(&bytes).expect("re-reads");
        assert!(back.layers.iter().all(|layer| !layer.pixel_data_irrelevant));
    }
}

#[test]
fn psd_tools_keeps_a_round_tripped_shape_layer_a_shape() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }

    let doc = read_fixture("vector_fill.psd");
    let path = write_temp("shape-flag-shape", &pictura_codec::write_psd(&doc).unwrap());
    let (stdout, lines) = read_with_psd_tools(&path);
    let _ = std::fs::remove_file(&path);

    assert_eq!(
        row(&lines, "Shape", &stdout),
        expected(&["Shape", "shape", "18"])
    );
    assert_eq!(
        row(&lines, "Base", &stdout),
        expected(&["Base", "pixel", "00"])
    );
}

#[test]
fn psd_tools_keeps_a_round_tripped_fill_layer_a_fill_layer() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }

    let doc = read_fixture("vector_mask.psd");
    let path = write_temp("shape-flag-fill", &pictura_codec::write_psd(&doc).unwrap());
    let (stdout, lines) = read_with_psd_tools(&path);
    let _ = std::fs::remove_file(&path);

    assert_eq!(
        row(&lines, "Base", &stdout),
        expected(&["Base", "pixel", "00"])
    );
    assert_eq!(
        row(&lines, "Shape Inverted", &stdout),
        expected(&["Shape Inverted", "solidcolorfill", "00"])
    );
    assert_eq!(
        row(&lines, "Shape", &stdout),
        expected(&["Shape", "solidcolorfill", "00"])
    );
}
