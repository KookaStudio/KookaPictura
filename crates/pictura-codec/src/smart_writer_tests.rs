//! Authoring tests for embedded smart objects (write-side).

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use pictura_core::*;

use crate::{read_psd, write_psd};

/// Deterministic pseudo-random payload with a fixed seed.
fn payload(seed: u64, len: usize) -> Vec<u8> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) as u8
        })
        .collect()
}

fn embedded(filename: &str, data: Vec<u8>) -> SmartObject {
    SmartObject {
        filename: filename.to_string(),
        kind: SmartObjectKind::Embedded,
        payload: Some(data),
        ..Default::default()
    }
}

fn smart_layer(name: &str, so: SmartObject) -> Layer {
    Layer {
        name: name.to_string(),
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
        smart_object: Some(so),
        ..Default::default()
    }
}

fn doc_with(layers: Vec<Layer>) -> Document {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i * 7 + 3) as u8;
    }
    doc.layers = layers;
    doc
}

/// Count the `u64`-length-prefixed records in the first `lnk2` block.
fn lnk2_record_count(extra: &[u8]) -> usize {
    let pos = extra
        .windows(4)
        .position(|w| w == b"lnk2")
        .expect("an authored lnk2 block");
    let len = u32::from_be_bytes(extra[pos + 4..pos + 8].try_into().unwrap()) as usize;
    let data = &extra[pos + 8..pos + 8 + len];
    let mut count = 0;
    let mut cursor = 0;
    while cursor + 8 <= data.len() {
        let record = u64::from_be_bytes(data[cursor..cursor + 8].try_into().unwrap()) as usize;
        cursor += 8 + record;
        cursor += (4 - record % 4) % 4;
        count += 1;
    }
    count
}

#[test]
fn authored_smart_object_round_trips() {
    let data = payload(0x1234_5678, 4096);
    let doc = doc_with(vec![smart_layer(
        "Smart",
        embedded("source.psb", data.clone()),
    )]);

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    let so = back.layers[0]
        .smart_object
        .as_ref()
        .expect("authored smart object resolves");

    assert_eq!(so.kind, SmartObjectKind::Embedded);
    assert_eq!(so.filename, "source.psb");
    assert_eq!(so.filetype, *b"8BPB");
    assert_eq!(so.creator, *b"8BIM");
    assert!(!so.uuid.is_empty(), "authored uuid is present");
    assert_eq!(
        so.payload.as_deref(),
        Some(&data[..]),
        "payload bytes survive write and read"
    );
}

#[test]
fn authored_smart_object_with_filter_round_trips() {
    let settings = PicturaRawSettings {
        basic: CrsSettings {
            temperature: Some(-20.0),
            exposure: Some(0.5),
            ..Default::default()
        },
        ..Default::default()
    };
    let options = crate::write_descriptor(&crate::encode_pictura_raw_fltr(&settings));
    let mut so = embedded("source.psb", payload(0x55, 256));
    so.smart_filters = vec![SmartFilter {
        filter_id: crate::CAMERA_RAW_FILTER_ID,
        name: crate::CAMERA_RAW_FILTER_NAME.to_string(),
        enabled: false,
        options,
    }];
    so.filter_mask_enabled = false;
    so.filter_mask_linked = true;
    so.filter_mask_extend_with_white = false;

    let doc = doc_with(vec![smart_layer("Smart", so)]);
    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    let so = back.layers[0]
        .smart_object
        .as_ref()
        .expect("authored smart object resolves");

    assert_eq!(so.smart_filters.len(), 1, "the filter survives write/read");
    let filter = &so.smart_filters[0];
    assert_eq!(filter.filter_id, crate::CAMERA_RAW_FILTER_ID);
    assert_eq!(filter.name, crate::CAMERA_RAW_FILTER_NAME);
    assert!(!filter.enabled, "the disabled flag survives");
    assert_eq!(
        crate::decode_pictura_raw_settings(&filter.options),
        settings
    );
    assert!(!so.filter_mask_enabled, "group mask enable survives");
    assert!(so.filter_mask_linked, "group mask linked survives");
    assert!(
        !so.filter_mask_extend_with_white,
        "group extend-with-white survives"
    );
}

#[test]
fn authored_disabled_filter_group_round_trips() {
    let settings = PicturaRawSettings {
        basic: CrsSettings {
            exposure: Some(0.5),
            ..Default::default()
        },
        ..Default::default()
    };
    let options = crate::write_descriptor(&crate::encode_pictura_raw_fltr(&settings));
    let mut so = embedded("source.psb", payload(0x66, 256));
    so.smart_filters = vec![SmartFilter {
        filter_id: crate::CAMERA_RAW_FILTER_ID,
        name: crate::CAMERA_RAW_FILTER_NAME.to_string(),
        enabled: true,
        options,
    }];
    so.smart_filters_enabled = false;

    let doc = doc_with(vec![smart_layer("Smart", so)]);
    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    let so = back.layers[0]
        .smart_object
        .as_ref()
        .expect("authored smart object resolves");

    assert!(
        !so.smart_filters_enabled,
        "the disabled group flag survives"
    );
    assert!(
        so.smart_filters[0].enabled,
        "the per-filter enab is independent of the group flag"
    );
}

#[test]
fn authored_shared_payload_emits_one_record() {
    let data = payload(0xabcd, 1024);
    let doc = doc_with(vec![
        smart_layer("A", embedded("shared.psb", data.clone())),
        smart_layer("B", embedded("shared.psb", data.clone())),
    ]);

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    let a = back.layers[0]
        .smart_object
        .as_ref()
        .expect("layer A resolves");
    let b = back.layers[1]
        .smart_object
        .as_ref()
        .expect("layer B resolves");

    assert_eq!(a.kind, SmartObjectKind::Embedded);
    assert_eq!(b.kind, SmartObjectKind::Embedded);
    assert_eq!(a.uuid, b.uuid, "the same source dedupes to one uuid");
    assert_eq!(
        lnk2_record_count(&back.layer_section_extra),
        1,
        "the shared source emits a single record"
    );
}

#[test]
fn authored_descriptor_is_sold_v4() {
    let doc = doc_with(vec![smart_layer(
        "Smart",
        embedded("s.psb", payload(7, 64)),
    )]);
    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();

    let block = back.layers[0]
        .extra_blocks
        .iter()
        .find(|b| &b.key == b"SoLd")
        .expect("the writer authored a SoLd block");
    assert_eq!(&block.data[..4], b"soLD");
    assert_eq!(
        u32::from_be_bytes(block.data[4..8].try_into().unwrap()),
        4,
        "outer descriptor version"
    );
}

fn psd_tools_available() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("python3")
            .args(["-c", "import psd_tools"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

fn scratch_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "pictura-codec-smart-writer-{}-{tag}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn psd_tools_parses_authored_smart_object() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/test_with_smart_object01.psd");
    let Ok(bytes) = std::fs::read(&fixture) else {
        eprintln!("skipping: {} not found", fixture.display());
        return;
    };
    let src = read_psd(&bytes).expect("fixture parses");
    let source = src
        .layers
        .iter()
        .find(|l| l.name == "Layer 1")
        .and_then(|l| l.smart_object.as_ref())
        .expect("fixture smart object resolves");
    let payload = source.payload.clone().expect("fixture payload");
    let filename = source.filename.clone();

    let doc = doc_with(vec![smart_layer(
        "Layer 1",
        embedded(&filename, payload.clone()),
    )]);
    let out = write_psd(&doc).unwrap();

    let dir = scratch_dir("authored");
    let path = dir.join("authored.psd");
    std::fs::write(&path, &out).unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1], lazy=False)
so = psd[0].smart_object
print(so.kind)
print(so.filename)
print(len(so.data))
"#;
    let result = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&path)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        result.status.success(),
        "psd-tools failed:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8_lossy(&result.stdout);
    let mut lines = stdout.lines();
    assert_eq!(
        lines.next().map(str::trim),
        Some("data"),
        "psd-tools reports an embedded data smart object; stdout={stdout}"
    );
    assert_eq!(
        lines.next().map(str::trim),
        Some(filename.as_str()),
        "psd-tools reports the authored filename; stdout={stdout}"
    );
    let len: usize = lines
        .next()
        .expect("embedded data length line")
        .trim()
        .parse()
        .unwrap();
    assert_eq!(
        len,
        payload.len(),
        "embedded data length matches the payload"
    );
}
