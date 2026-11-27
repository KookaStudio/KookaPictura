//! Independent `psd-tools` oracle for the extra-channel sense modes.
//!
//! Writes one `Masked Areas` and one `Selected Areas` channel and asks
//! `psd-tools` for the DisplayInfo kinds. Needs `python3` with `psd-tools`;
//! self-skips with a message otherwise.

use std::path::Path;

use pictura_core::{BitDepth, Channel, ColorMode, Document};
use pictura_testkit::oracle;

const SCRIPT: &str = r#"
from psd_tools.constants import Resource
from psd_tools.psd import PSD
import sys
with open(sys.argv[1], 'rb') as f:
    psd = PSD.read(f)
info = psd.image_resources[Resource.DISPLAY_INFO].data
print(','.join(str(int(c.mode)) for c in info.alpha_channels))
"#;

fn read_modes(path: &Path) -> String {
    let out = oracle::Command::new("python3")
        .args(["-c", SCRIPT])
        .arg(path)
        .output()
        .expect("python3 runs");
    assert!(
        out.status.success(),
        "psd-tools failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("utf-8 output")
        .trim()
        .to_string()
}

#[test]
fn psd_tools_reports_channel_sense_modes() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    let mut doc = Document::new(2, 2, ColorMode::Grayscale, BitDepth::Eight);
    doc.channels = vec![
        Channel {
            id: 0,
            data: vec![0; 4].into(),
        },
        Channel {
            id: 1,
            data: vec![0; 4].into(),
        },
    ];
    assert!(pictura_codec::set_channel_selected_areas(&mut doc, 1, true));
    assert!(pictura_codec::set_channel_selected_areas(
        &mut doc, 0, false
    ));
    let bytes = pictura_codec::write_psd(&doc).expect("write PSD");
    let path =
        std::env::temp_dir().join(format!("pictura-channel-sense-{}.psd", std::process::id()));
    std::fs::write(&path, &bytes).expect("temp PSD");
    let modes = read_modes(&path);
    let _ = std::fs::remove_file(&path);
    assert_eq!(modes, "1,0");
}
