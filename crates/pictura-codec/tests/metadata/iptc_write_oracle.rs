//! IPTC write oracle: editing a document's IPTC fields must survive a save and
//! be readable by the independent `exiftool` decoder. Self-skips when exiftool
//! is unavailable.

use std::path::{Path, PathBuf};
use std::process::Command;

use pictura_codec::{
    decode_image_resources, read_metadata, read_psd, set_iptc_fields, write_psd, IPTC_NAA,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn exiftool_available() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("exiftool")
            .arg("-ver")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

fn exiftool_iptc(path: &Path) -> Option<Vec<String>> {
    let out = Command::new("exiftool")
        .arg("-s3")
        .arg("-IPTC:ObjectName")
        .arg("-IPTC:By-line")
        .arg("-IPTC:CopyrightNotice")
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    (lines.len() == 3).then_some(lines)
}

fn other_resources(doc: &pictura_core::Document) -> Vec<Vec<u8>> {
    decode_image_resources(doc)
        .into_iter()
        .filter(|r| r.id != IPTC_NAA)
        .map(|r| r.raw)
        .collect()
}

#[test]
fn edited_iptc_survives_save_and_exiftool() {
    let bytes = std::fs::read(fixture("metadata.psd")).expect("read fixture");
    let mut doc = read_psd(&bytes).expect("fixture parses");
    let before = other_resources(&doc);

    let fields: Vec<(u8, u8, String)> = vec![
        (2, 5, "Edited Title".into()),
        (2, 80, "Grace Hopper".into()),
        (2, 116, "(c) 2026 Edited".into()),
    ];
    assert!(
        set_iptc_fields(&mut doc, &fields),
        "the edit changed the doc"
    );

    let out = write_psd(&doc).expect("write");
    let back = read_psd(&out).expect("re-read");
    let iptc = read_metadata(&back).iptc;
    assert_eq!(iptc.text(2, 5).as_deref(), Some("Edited Title"));
    assert_eq!(iptc.text(2, 80).as_deref(), Some("Grace Hopper"));
    assert_eq!(iptc.text(2, 116).as_deref(), Some("(c) 2026 Edited"));
    assert_eq!(
        other_resources(&back),
        before,
        "untouched resources survive the save"
    );

    if !exiftool_available() {
        eprintln!("skipping exiftool check: exiftool not available");
        return;
    }
    let dir = std::env::temp_dir().join(format!("pictura-iptc-write-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("edited.psd");
    std::fs::write(&path, &out).unwrap();
    let values = exiftool_iptc(&path).expect("exiftool is available but produced no values");
    assert_eq!(
        values,
        vec![
            "Edited Title".to_string(),
            "Grace Hopper".to_string(),
            "(c) 2026 Edited".to_string()
        ]
    );
}
