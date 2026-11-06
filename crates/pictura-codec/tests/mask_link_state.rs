use std::process::Command;

use pictura_codec::{read_psd, write_psb, write_psd};
use pictura_core::{Document, PsdRect};
use pictura_render::{layer_mask_linked, set_layer_mask_linked};

const COVERAGE: [u8; 6] = [0, 64, 255, 192, 128, 32];

fn fixture(linked: bool) -> Document {
    let bytes: &[u8] = if linked {
        include_bytes!("fixtures/mask_linked.psd")
    } else {
        include_bytes!("fixtures/mask_unlinked.psd")
    };
    read_psd(bytes).expect("independent fixture reads")
}

fn check_external(linked: bool) {
    let doc = fixture(linked);
    assert_eq!(doc.layers.len(), 1);
    assert_eq!(
        doc.layers[0].rect,
        PsdRect {
            top: 3,
            left: 2,
            bottom: 9,
            right: 10
        }
    );
    let mask = doc.layers[0].mask.as_ref().unwrap();
    assert_eq!(mask.flags, u8::from(!linked));
    assert_eq!(
        mask.rect,
        PsdRect {
            top: 5,
            left: 4,
            bottom: 7,
            right: 7
        }
    );
    assert_eq!(mask.data.as_ref().unwrap().as_slice(), COVERAGE);
    assert_eq!(layer_mask_linked(&doc, "0"), linked);
    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back.layers[0].rect, doc.layers[0].rect);
    assert_eq!(back.layers[0].mask, doc.layers[0].mask);
    assert_eq!(layer_mask_linked(&back, "0"), linked);
}

#[test]
fn linked_external_mask_preserves_flags_and_bounds() {
    check_external(true);
}

#[test]
fn unlinked_external_mask_preserves_flags_and_bounds() {
    check_external(false);
}

#[test]
fn mask_link_toggles_survive_psd_and_psb() {
    for psb in [false, true] {
        let mut doc = fixture(false);
        doc.layers[0].mask.as_mut().unwrap().flags = 0xA1;
        let mut expected = doc.layers[0].mask.as_ref().unwrap().clone();
        let layer_rect = doc.layers[0].rect;
        for linked in [true, false] {
            assert!(set_layer_mask_linked(&mut doc, "0", linked));
            expected.flags = 0xA0 | u8::from(!linked);
            assert_eq!(doc.layers[0].mask.as_ref().unwrap(), &expected);
            let bytes = if psb {
                write_psb(&doc)
            } else {
                write_psd(&doc)
            }
            .unwrap();
            assert_eq!(
                u16::from_be_bytes([bytes[4], bytes[5]]),
                if psb { 2 } else { 1 }
            );
            doc = read_psd(&bytes).unwrap();
            assert_eq!(layer_mask_linked(&doc, "0"), linked);
            assert_eq!(doc.layers[0].rect, layer_rect);
            assert_eq!(doc.layers[0].mask.as_ref().unwrap(), &expected);
        }
    }
}

#[test]
fn psd_tools_reads_both_mask_link_states() {
    if !Command::new("python3")
        .args(["-c", "import psd_tools"])
        .output()
        .is_ok_and(|out| out.status.success())
    {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }
    let script = r#"
import sys
from psd_tools import PSDImage
p = PSDImage.open(sys.argv[1])
layer = p[0]
mask = layer._record.mask_data
print(mask.flags.tobytes()[0])
print(mask.left, mask.top, mask.right, mask.bottom)
print(' '.join(str(x) for x in layer.mask.topil().tobytes()))
"#;
    for psb in [false, true] {
        for linked in [true, false] {
            let mut doc = fixture(!linked);
            assert!(set_layer_mask_linked(&mut doc, "0", linked));
            let bytes = if psb {
                write_psb(&doc)
            } else {
                write_psd(&doc)
            }
            .unwrap();
            let path = std::env::temp_dir().join(format!(
                "pictura-mask-link-{}-{psb}-{linked}.psd",
                std::process::id()
            ));
            std::fs::write(&path, bytes).unwrap();
            let out = Command::new("python3")
                .args(["-c", script])
                .arg(&path)
                .output()
                .unwrap();
            std::fs::remove_file(path).unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert_eq!(
                String::from_utf8(out.stdout).unwrap(),
                format!("{}\n4 5 7 7\n0 64 255 192 128 32\n", u8::from(!linked))
            );
        }
    }
}
