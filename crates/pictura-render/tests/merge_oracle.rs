//! External-oracle check for `pictura_render::merge_scope` (Merge Down).
//!
//! The committed `tests/fixtures/alpha_*.rgba` files are raw interleaved RGBA8
//! 8x8 layers produced by ImageMagick (`scripts/im_compose.py`), an independent
//! implementation of source-over compositing. A Merge Down of the same two
//! layers must reproduce the ImageMagick composite. The live variant regenerates
//! the reference with `magick` and self-skips when the tool is absent.

use std::path::PathBuf;
use std::process::Command;

use pictura_core::{
    BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LockFlags, PsdRect,
};
use pictura_render::{merge_scope, MergeScope};

const SIZE: u32 = 8;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/im_compose.py")
}

/// Split interleaved RGBA8 into planar per-channel vectors.
fn planar(interleaved: &[u8]) -> [Vec<u8>; 4] {
    let pixels = (SIZE * SIZE) as usize;
    let mut planes = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for i in 0..pixels {
        for (c, plane) in planes.iter_mut().enumerate() {
            plane.push(interleaved[i * 4 + c]);
        }
    }
    planes
}

fn layer(name: &str, interleaved: &[u8], with_alpha: bool) -> Layer {
    let [r, g, b, a] = planar(interleaved);
    let mut channels = vec![
        Channel {
            id: 0,
            data: r.into(),
        },
        Channel {
            id: 1,
            data: g.into(),
        },
        Channel {
            id: 2,
            data: b.into(),
        },
    ];
    if with_alpha {
        channels.push(Channel {
            id: -1,
            data: a.into(),
        });
    }
    Layer {
        name: name.into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: SIZE as i32,
            right: SIZE as i32,
        },
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
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

/// Merge Down the two alpha-scene layers and return the node as interleaved RGBA8.
fn merged_alpha_pair() -> Vec<u8> {
    let base = std::fs::read(fixtures_dir().join("alpha_base.rgba")).expect("alpha_base fixture");
    let src = std::fs::read(fixtures_dir().join("alpha_src.rgba")).expect("alpha_src fixture");

    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        layer("base", &base, false),
        layer("src", &src, true),
    ]);

    merge_scope(&mut doc, MergeScope::Down("1")).expect("merge down");
    let node = &doc.layers[0];
    let n = (node.rect.width() * node.rect.height()) as usize;
    let plane = |id: i16| node.channels.iter().find(|c| c.id == id).unwrap();
    let (r, g, b, a) = (plane(0), plane(1), plane(2), plane(-1));
    let mut out = Vec::with_capacity(n * 4);
    for i in 0..n {
        out.extend_from_slice(&[r.data[i], g.data[i], b.data[i], a.data[i]]);
    }
    out
}

#[test]
fn merge_down_matches_committed_imagemagick_fixture() {
    let reference =
        std::fs::read(fixtures_dir().join("alpha_Normal.rgba")).expect("alpha_Normal fixture");
    let diff = pictura_testkit::compare(&merged_alpha_pair(), &reference, 0)
        .expect("equal-length RGBA buffers");
    assert!(
        diff.is_empty(),
        "merged pair diverged from ImageMagick: {} samples, max delta {}",
        diff.differing,
        diff.max_delta
    );
}

#[test]
fn merge_down_matches_live_imagemagick() {
    if Command::new("magick").arg("-version").output().is_err() {
        eprintln!("skipping live ImageMagick merge oracle: `magick` not on PATH");
        return;
    }

    let dir = std::env::temp_dir().join(format!("pictura-merge-oracle-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("alpha_Normal.rgba");

    let status = Command::new("python3")
        .arg(script())
        .arg("compose")
        .arg("Over")
        .arg(fixtures_dir().join("alpha_base.rgba"))
        .arg(fixtures_dir().join("alpha_src.rgba"))
        .arg(&out)
        .status()
        .expect("run scripts/im_compose.py compose");
    assert!(status.success(), "im_compose.py compose failed");

    let reference = std::fs::read(&out).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    let diff = pictura_testkit::compare(&merged_alpha_pair(), &reference, 0)
        .expect("equal-length RGBA buffers");
    assert!(
        diff.is_empty(),
        "merged pair diverged from live ImageMagick: {} samples, max delta {}",
        diff.differing,
        diff.max_delta
    );
}
