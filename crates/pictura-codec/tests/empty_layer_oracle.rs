//! psd-tools oracle for an empty and a grown layer's stored rect (#1208).
//!
//! Self-skips when `python3`/`psd_tools` is absent.

use std::process::Command;

use pictura_codec::{read_psd, write_psd};
use pictura_core::{
    BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LayerStack, LockFlags,
    PsdRect,
};

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

fn pixel_layer(name: &str, rect: PsdRect, size: usize) -> Layer {
    let w = rect.width().max(0) as usize;
    let channels = (0..3)
        .map(|id| Channel {
            id,
            data: (0..size)
                .map(|i| ((i % w.max(1)) * 40 + id as usize * 7) as u8)
                .collect::<Vec<_>>()
                .into(),
        })
        .chain(std::iter::once(Channel {
            id: -1,
            data: vec![255u8; size].into(),
        }))
        .collect();
    Layer {
        name: name.to_string(),
        rect,
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels,
        children: LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

fn temp_file(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("pictura-{tag}-{}.psd", std::process::id()))
}

#[test]
fn psd_tools_reads_an_empty_layer() {
    if !psd_tools_available() {
        eprintln!("skipping psd-tools check: python3 + psd_tools not available");
        return;
    }
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        pixel_layer(
            "Base",
            PsdRect {
                top: 0,
                left: 0,
                bottom: 4,
                right: 4,
            },
            16,
        ),
        pixel_layer(
            "Empty",
            PsdRect {
                top: 0,
                left: 0,
                bottom: 0,
                right: 0,
            },
            0,
        ),
    ]);
    let file = temp_file("empty");
    std::fs::write(&file, write_psd(&doc).unwrap()).unwrap();
    let out = Command::new("python3")
        .args([
            "-c",
            "import sys\nfrom psd_tools import PSDImage\np = PSDImage.open(sys.argv[1])\nprint(len(list(p)))",
            file.to_str().unwrap(),
        ])
        .output()
        .expect("run python3");
    let _ = std::fs::remove_file(&file);
    assert!(
        out.status.success(),
        "psd-tools failed on an empty layer:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "2");
    // Pictura reads both layers back with the empty rect intact.
    let bytes = write_psd(&doc).unwrap();
    let back = read_psd(&bytes).unwrap();
    assert_eq!(
        back.layers[1].rect,
        PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0
        }
    );
}

#[test]
fn psd_tools_reads_a_grown_layers_rect_and_pixels() {
    if !psd_tools_available() {
        eprintln!("skipping psd-tools check: python3 + psd_tools not available");
        return;
    }
    let rect = PsdRect {
        top: 3,
        left: 2,
        bottom: 6,
        right: 6,
    };
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("Grown", rect, 12)]);
    let file = temp_file("grown");
    std::fs::write(&file, write_psd(&doc).unwrap()).unwrap();
    let out = Command::new("python3")
        .args([
            "-c",
            "import sys\nfrom psd_tools import PSDImage\np = PSDImage.open(sys.argv[1])\nl = list(p)[0]\nim = l.topil()\nprint(*l.bbox)\nprint(*im.getpixel((0, 0)))\nprint(*im.getpixel((im.width - 1, im.height - 1)))",
            file.to_str().unwrap(),
        ])
        .output()
        .expect("run python3");
    let _ = std::fs::remove_file(&file);
    assert!(
        out.status.success(),
        "psd-tools failed on a grown layer:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    let bbox: Vec<i32> = lines
        .next()
        .unwrap()
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect();
    assert_eq!(bbox, vec![2, 3, 6, 6]);
    let first: Vec<u8> = lines
        .next()
        .unwrap()
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect();
    let last: Vec<u8> = lines
        .next()
        .unwrap()
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect();
    assert_eq!(first[3], 255, "alpha survives the round trip");
    assert_eq!(last[3], 255);
    assert_ne!(first, last, "the layer's pixels are its own");
}

fn rect(top: i32, left: i32, bottom: i32, right: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
    }
}

#[test]
fn empty_layer_round_trips() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        pixel_layer("Base", rect(0, 0, 4, 4), 16),
        pixel_layer("Empty", rect(0, 0, 0, 0), 0),
    ]);
    let bytes = write_psd(&doc).unwrap();
    let back = read_psd(&bytes).unwrap();
    assert_eq!(back, doc);
    assert_eq!(back.layers[1].rect, rect(0, 0, 0, 0));
    assert_eq!(back.layers[1].channels.len(), 4);
    assert!(back.layers[1].channels.iter().all(|c| c.data.is_empty()));
}

#[test]
fn grown_layer_round_trips() {
    let mut doc = Document::new(6, 5, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("Grown", rect(1, 2, 4, 6), 12)]);
    let bytes = write_psd(&doc).unwrap();
    let back = read_psd(&bytes).unwrap();
    assert_eq!(back.layers[0].rect, rect(1, 2, 4, 6));
    assert_eq!(back.layers[0].channels, doc.layers[0].channels);
}

#[test]
fn existing_files_stay_byte_identical() {
    let golden = include_bytes!("fixtures/default_before.psd");
    let doc = read_psd(golden).unwrap();
    assert_eq!(write_psd(&doc).unwrap().as_slice(), golden.as_slice());
}
