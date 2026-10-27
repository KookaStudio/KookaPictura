use super::*;
use pictura_codec::attach_pictura_raw_filter;
use pictura_core::{CrsSettings, PicturaRawSettings};

/// A 4x4 RGB document with one authored embedded smart-object layer: no
/// preserved `SoLd`, so the writer authors the descriptor from the typed object.
fn authoring_smart_doc() -> Document {
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
        smart_object: Some(
            {
                let mut so = SmartObject::default();
                so.filename = "source.psb".into();
                so.kind = SmartObjectKind::Embedded;
                so
            }
            .with_payload((0..512).map(|i| (i % 251) as u8).collect()),
        ),
        ..Default::default()
    }]);
    doc
}

/// Run psd-tools over `bytes` and print the camera-raw smart filter's id and
/// three of its `Fltr` values, failing if any descriptor key is missing.
fn psd_tools_filter_values(tag: &str, bytes: &[u8]) -> String {
    let dir = scratch_dir(tag);
    let path = dir.join("doc.psd");
    std::fs::write(&path, bytes).unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage
from psd_tools.constants import Tag
with open(sys.argv[1], "rb") as f:
    image = PSDImage.open(f, lazy=False)
data = image[0].tagged_blocks.get_data(Tag.SMART_OBJECT_LAYER_DATA1)
desc = data.data
item = desc[b"filterFX"][b"filterFXList"][0]
fltr = item[b"Fltr"]
print(item[b"filterID"])
print(fltr[b"Ex12"])
print(fltr[b"Temp"])
print(fltr[b"Cl12"])
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

#[test]
fn psd_tools_reads_the_authored_pictura_raw_filter() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }
    let mut doc = authoring_smart_doc();
    let settings = PicturaRawSettings {
        basic: CrsSettings {
            temperature: Some(20.0),
            exposure: Some(0.75),
            clarity: Some(-12.0),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(attach_pictura_raw_filter(&mut doc.layers[0], &settings).is_ok());

    let bytes = write_psd(&doc).unwrap();
    let stdout = psd_tools_filter_values("authored-pictura-raw-filter", &bytes);
    let lines: Vec<&str> = stdout.lines().map(str::trim).collect();
    assert_eq!(
        lines[0], "2683",
        "psd-tools reports the Camera Raw filter id"
    );
    assert_eq!(lines[1], "0.75", "Ex12 round-trips as a double");
    assert_eq!(lines[2], "20", "Temp round-trips as an integer");
    assert_eq!(lines[3], "-12", "Cl12 round-trips as an integer");
}
