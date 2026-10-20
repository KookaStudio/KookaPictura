//! Independent `psd-tools` oracle for clipping groups: a document with a base
//! and two clipped layers is written with `pictura_codec::write_psd`, and
//! psd-tools' own compositor renders it for comparison with
//! `pictura_render::composite_rgba`.
//!
//! Needs `python3` with `psd-tools` (and numpy); self-skips otherwise.

use std::process::Command;

use pictura_core::{BitDepth, BlendMode, Channel, ColorMode, Document, Layer, PsdRect};

const SCRIPT: &str = r#"
import sys
from psd_tools import PSDImage
image = PSDImage.open(sys.argv[1]).composite(force=True).convert("RGB")
sys.stdout.write(" ".join(str(v) for px in image.getdata() for v in px))
"#;

fn layer(name: &str, rect: PsdRect, rgba: [u8; 4], opacity: u8, clipping: bool) -> Layer {
    let n = (rect.width() * rect.height()) as usize;
    Layer {
        name: name.into(),
        rect,
        opacity,
        fill: 255,
        visible: true,
        clipping,
        blend: BlendMode::Normal,
        channels: [0i16, 1, 2, -1]
            .into_iter()
            .zip(rgba)
            .map(|(id, v)| Channel {
                id,
                data: vec![v; n].into(),
            })
            .collect(),
        ..Default::default()
    }
}

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
    }
}

#[test]
fn psd_tools_composites_a_clipping_group_alike() {
    let available = Command::new("python3")
        .args(["-c", "import psd_tools, numpy"])
        .output()
        .is_ok_and(|o| o.status.success());
    if !available {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }
    let mut doc = Document::new(24, 16, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        layer(
            "Background",
            rect(0, 0, 24, 16),
            [255, 255, 255, 255],
            255,
            false,
        ),
        layer("Base", rect(4, 4, 14, 12), [200, 30, 30, 255], 255, false),
        layer("Green", rect(0, 0, 24, 16), [20, 200, 40, 255], 153, true),
        layer("Blue", rect(8, 0, 24, 8), [30, 60, 220, 255], 255, true),
    ]);
    let ours = pictura_render::composite_rgba(&doc);
    let path = std::env::temp_dir().join(format!("pictura-clip-{}.psd", std::process::id()));
    std::fs::write(&path, pictura_codec::write_psd(&doc).expect("writes")).expect("write");
    let out = Command::new("python3")
        .args(["-c", SCRIPT, path.to_str().expect("utf-8 path")])
        .output()
        .expect("run python3");
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "psd-tools failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let theirs: Vec<u8> = String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .map(|v| v.parse().expect("a byte"))
        .collect();
    assert_eq!(theirs.len(), 24 * 16 * 3);
    let plane = 24 * 16;
    let mut worst = 0u8;
    for i in 0..plane {
        for c in 0..3 {
            worst = worst.max(ours.data[c * plane + i].abs_diff(theirs[i * 3 + c]));
        }
    }
    assert!(
        worst <= 2,
        "max channel difference against psd-tools: {worst}"
    );
}
