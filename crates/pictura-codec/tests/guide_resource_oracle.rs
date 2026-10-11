//! Independent `psd-tools` oracle for the grid-and-guides image resource
//! (1032) `pictura-codec` writes.
//!
//! The document is built and written with `pictura-codec`, then read back with
//! `psd-tools` (its own `GridGuidesInfo` parser), which reports the version and
//! each guide's raw location and direction.
//!
//! Needs `python3` with `psd-tools`; self-skips with a message otherwise.

use std::process::Command;

use pictura_core::{BitDepth, ColorMode, Document, Guide, GuideOrientation};

const SCRIPT: &str = r#"
import sys
from psd_tools import PSDImage

psd = PSDImage.open(sys.argv[1])
info = psd.image_resources.get_data(1032)
print(info.version)
for location, direction in info.data:
    print(f"{location},{direction}")
"#;

fn psd_tools_available() -> bool {
    Command::new("python3")
        .args(["-c", "import psd_tools"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn psd_tools_reads_the_written_guides() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 with psd-tools is not available");
        return;
    }
    let mut doc = Document::new(200, 100, ColorMode::Rgb, BitDepth::Eight);
    doc.guides = vec![
        Guide {
            orientation: GuideOrientation::Vertical,
            position: 40.0,
        },
        Guide {
            orientation: GuideOrientation::Horizontal,
            position: 12.5,
        },
    ];
    let bytes = pictura_codec::write_psd(&doc).expect("write");
    let dir = std::env::temp_dir().join(format!("pictura-guide-oracle-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("guides.psd");
    std::fs::write(&file, &bytes).expect("write file");

    let out = Command::new("python3")
        .args(["-c", SCRIPT, file.to_str().expect("utf-8 path")])
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "psd-tools failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rows: Vec<&str> = stdout.lines().collect();
    assert_eq!(rows, ["1", "1280,0", "400,1"]);
}
