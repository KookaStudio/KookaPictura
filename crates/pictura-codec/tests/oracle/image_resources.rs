use super::*;

/// The committed `image_resources.psd` carries an EXIF (1058) and an XMP (1060)
/// resource; `decode_image_resources` must return the same id and bytes the
/// independent psd-tools reader reports.
#[test]
fn image_resources_decode_matches_psd_tools() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }

    let doc = load("image_resources.psd");
    let parsed = pictura_codec::decode_image_resources(&doc);
    assert!(
        parsed
            .iter()
            .any(|r| r.id == pictura_codec::EXIF_DATA_1 && !r.data.is_empty()),
        "the EXIF resource decodes"
    );
    assert!(
        parsed
            .iter()
            .any(|r| r.id == pictura_codec::XMP_METADATA && r.data == XMP_BYTES),
        "the XMP resource decodes with its exact bytes"
    );

    let path = fixture_dir().join("image_resources.psd");
    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1], lazy=False)
for key, res in psd.image_resources.items():
    if isinstance(res.data, (bytes, bytearray)):
        print(int(key), bytes(res.data).hex())
"#;
    let out = oracle::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&path)
        .output()
        .expect("run python3");
    assert!(
        out.status.success(),
        "psd-tools failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut checked = 0;
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        let (id, hex) = line.split_once(' ').expect("id then hex");
        let id: u16 = id.parse().expect("resource id");
        let record = parsed
            .iter()
            .find(|r| r.id == id)
            .unwrap_or_else(|| panic!("psd-tools reports id {id} that we did not decode"));
        let ours: String = record.data.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(ours, hex, "resource {id} bytes differ from psd-tools");
        checked += 1;
    }
    assert_eq!(checked, 2, "EXIF and XMP bytes-resources; stdout={stdout}");
}

/// The raw image-resource section still survives an open→save byte-for-byte, and
/// the typed view is stable across the round trip.
#[test]
fn image_resources_survive_write() {
    let doc = load("image_resources.psd");
    let out = write_psd(&doc).expect("write");
    let back = read_psd(&out).expect("re-read");
    assert_eq!(
        back.image_resources, doc.image_resources,
        "raw section bytes"
    );
    assert_eq!(
        pictura_codec::decode_image_resources(&back),
        pictura_codec::decode_image_resources(&doc),
        "typed view is stable"
    );
}

/// The fixture's XMP payload; must match `scripts/generate-fixtures.py`.
const XMP_BYTES: &[u8] = b"<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"></x:xmpmeta>";
