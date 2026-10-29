use super::*;

/// A 4x4 RGB document with one layer holding an embedded smart object, which
/// the writer authors as a `SoLd` block plus a document-level `lnk2` record.
fn embedded_smart_doc() -> Document {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i * 7 + 1) as u8;
    }
    doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
        name: "Smart".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 4,
        },
        channels: vec![
            Channel {
                id: 0,
                data: vec![10; 16].into(),
            },
            Channel {
                id: 1,
                data: vec![20; 16].into(),
            },
            Channel {
                id: 2,
                data: vec![30; 16].into(),
            },
            Channel {
                id: -1,
                data: vec![255; 16].into(),
            },
        ],
        smart_object: Some(
            {
                let mut so = SmartObject::default();
                so.filename = "source.psb".into();
                so.kind = SmartObjectKind::Embedded;
                so
            }
            .with_payload((0..4096).map(|i| (i % 251) as u8).collect()),
        ),
        ..Default::default()
    }]);
    doc
}

/// Run psd-tools over `bytes`, asserting `PSD.read` and `PSDImage.open` both
/// succeed, and return its stdout.
fn psd_tools_reads(tag: &str, bytes: &[u8]) -> String {
    let dir = scratch_dir(tag);
    let path = dir.join("doc.psb");
    std::fs::write(&path, bytes).unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage
from psd_tools.psd import PSD
with open(sys.argv[1], "rb") as f:
    PSD.read(f)
image = PSDImage.open(sys.argv[1], lazy=False)
so = image[0].smart_object
print(so.kind)
print(so.filename)
print(len(so.data))
"#;
    let out = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&path)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "psd-tools failed to parse:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Assert `psd_tools_reads` stdout reports the authored embedded smart object.
fn assert_embedded_smart_object(stdout: &str) {
    let mut lines = stdout.lines();
    assert_eq!(
        lines.next().map(str::trim),
        Some("data"),
        "psd-tools sees the embedded smart object; stdout={stdout}"
    );
    assert_eq!(
        lines.next().map(str::trim),
        Some("source.psb"),
        "psd-tools reports the authored filename; stdout={stdout}"
    );
    let len: usize = lines
        .next()
        .expect("embedded data length line")
        .trim()
        .parse()
        .unwrap();
    assert_eq!(len, 4096, "embedded payload length matches");
}

/// CRITICAL repro: a PSB carrying an authored smart object frames its `lnk2`
/// big-key block with a `u64` length, so psd-tools parses the whole document
/// without a framing error (previously a MemoryError).
#[test]
fn psd_tools_parses_authored_smart_object_in_psb() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }
    let bytes = write_psb(&embedded_smart_doc()).unwrap();
    let stdout = psd_tools_reads("psb-authored-smart-object", &bytes);
    assert_embedded_smart_object(&stdout);
}

/// CRITICAL-1 regression: an odd-length tagged block is declared even with its
/// pad inside, so psd-tools still finds the block that follows it.
#[test]
fn psd_tools_parses_block_after_odd_length_block_in_psb() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }
    let mut doc = embedded_smart_doc();
    // A 5-byte (odd) and a 6-byte (non-4-multiple) per-layer block precede the
    // authored `SoLd`. With the old external pad the next `8BIM` is misread and
    // the smart object is dropped.
    doc.layers[0].extra_blocks = vec![
        LayerBlock {
            key: *b"zzzz",
            data: vec![1, 2, 3, 4, 5].into(),
        },
        LayerBlock {
            key: *b"yyyy",
            data: vec![1, 2, 3, 4, 5, 6].into(),
        },
    ];
    let bytes = write_psb(&doc).unwrap();
    let stdout = psd_tools_reads("psb-odd-block-then-smart", &bytes);
    assert_embedded_smart_object(&stdout);
}

/// CRITICAL-2 regression: a PSD-sourced document whose preserved `lnk2` is
/// u32-framed is re-framed to u64 when written as a PSB, so psd-tools reads it.
#[test]
fn psd_tools_parses_reframed_psd_big_key_in_psb() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }
    let psd_bytes = write_psd(&embedded_smart_doc()).unwrap();
    let src = read_psd(&psd_bytes).unwrap();
    assert!(!src.is_psb, "the intermediate is a PSD");

    let bytes = write_psb(&src).unwrap();
    assert_eq!(
        u16::from_be_bytes([bytes[4], bytes[5]]),
        2,
        "the re-save is a PSB"
    );
    let stdout = psd_tools_reads("psb-reframed-psd-big-key", &bytes);
    assert_embedded_smart_object(&stdout);
}

/// Fix A regression: a preserved document-level block whose length is not a
/// multiple of 4 declares its exact length and is padded externally to 4, so
/// psd-tools still reads the authored `lnk2` that follows it.
#[test]
fn psd_tools_parses_doc_level_block_not_multiple_of_four_in_psb() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }
    let mut doc = embedded_smart_doc();
    // A 6-byte unknown global block: exact length 6 + 2 external pad bytes, which
    // the old per-layer even-inside rule would have mis-framed.
    let mut block = b"8BIMzzzz".to_vec();
    block.extend_from_slice(&6u32.to_be_bytes());
    block.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
    block.extend_from_slice(&[0, 0]);
    doc.layer_section_extra = block.into();

    let bytes = write_psb(&doc).unwrap();
    let stdout = psd_tools_reads("psb-doc-block-not-mult-of-four", &bytes);
    assert_embedded_smart_object(&stdout);
}
