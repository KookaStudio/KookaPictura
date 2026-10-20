use super::*;

/// M36: the independent oracle reads back the `lspf`/`lclr`/`iOpa` tags with
/// the values the model set.
#[test]
fn psd_tools_sees_layer_attributes() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    for (i, b) in doc.composite.data.iter_mut().enumerate() {
        *b = (i * 3 + 1) as u8;
    }
    let layer = Layer {
        name: "Attrs".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 128,
        lock: LockFlags::default()
            .with(LockFlags::TRANSPARENCY, true)
            .with(LockFlags::POSITION, true),
        color: ColorLabel::Violet,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![
            Channel {
                id: 0,
                data: vec![7; 4].into(),
            },
            Channel {
                id: 1,
                data: vec![7; 4].into(),
            },
            Channel {
                id: 2,
                data: vec![7; 4].into(),
            },
        ],
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    };
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

    let dir = scratch_dir("psd-attrs");
    let path = dir.join("attrs.psd");
    std::fs::write(&path, write_psd(&doc).unwrap()).unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1])
layer = psd[0]
print(layer.fill_opacity)
print(1 if layer.locks.transparency else 0)
print(1 if layer.locks.composite else 0)
print(1 if layer.locks.position else 0)
print(layer.sheet_color.value)
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
        "psd-tools failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let got: Vec<i32> = stdout.lines().map(|l| l.trim().parse().unwrap()).collect();
    assert_eq!(
        got,
        vec![128, 1, 0, 1, 6],
        "psd-tools must report fill=128, transparency+position locks, violet"
    );
}

/// The codec writes a flagged layer under the `"Background"` name even when the
/// model layer was named differently, and psd-tools reads that name back.
#[test]
fn psd_tools_sees_background_name_for_flagged_layer() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    let layer = Layer {
        name: "Base".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
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
        channels: vec![
            Channel {
                id: 0,
                data: vec![7; 4].into(),
            },
            Channel {
                id: 1,
                data: vec![7; 4].into(),
            },
            Channel {
                id: 2,
                data: vec![7; 4].into(),
            },
        ],
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: true,
        ..Default::default()
    };
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

    let dir = scratch_dir("psd-background");
    let path = dir.join("background.psd");
    std::fs::write(&path, write_psd(&doc).unwrap()).unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1])
print(psd[0].name)
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
        "psd-tools failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "Background",
        "a flagged layer is written under the Background name"
    );
}
