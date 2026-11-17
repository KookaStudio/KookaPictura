use super::*;

/// Extra/alpha channels: a PSD written by `pictura-codec` with one extra plane
/// is read by `psd-tools`, which must report the bumped header channel count and
/// expose our plane as the composite alpha. Independent of our own reader, this
/// proves the alternate-channel layout matches a second PSD implementation.
#[test]
fn psd_tools_sees_written_extra_channel() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    let mut doc = Document::new(4, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.composite
        .edit_data(|data| {
            for (i, b) in data.iter_mut().enumerate() {
                *b = (i * 9 + 1) as u8;
            }
        })
        .expect("pixel edit preserves geometry");
    let alpha: Vec<u8> = (0..8).map(|i| 200 + i as u8).collect();
    doc.channels = vec![Channel {
        id: 0,
        data: alpha.clone().into(),
    }];

    let dir = scratch_dir("psd-alpha");
    let path = dir.join("extra_channel.psd");
    std::fs::write(&path, write_psd(&doc).unwrap()).unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1])
print(psd.channels)
print(psd.composite().getchannel("A").tobytes().hex())
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
    let mut lines = stdout.lines();
    let channels: u32 = lines
        .next()
        .expect("channel count line")
        .trim()
        .parse()
        .unwrap();
    let alpha_hex = lines.next().expect("alpha hex line").trim();
    let expected: String = alpha.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(channels, 4, "psd-tools sees color + extra channels");
    assert_eq!(
        alpha_hex, expected,
        "psd-tools alpha equals the extra plane"
    );
}

/// The independent oracle decodes both the RLE composite section and
/// engine-encoded layer channels: psd-tools must recover the exact pixels.
#[test]
fn psd_tools_reads_rle_composite_and_layer() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    let mut doc = Document::new(4, 2, ColorMode::Rgb, BitDepth::Eight);
    let composite: Vec<u8> = (0..24).map(|i| (i * 7 + 1) as u8).collect();
    doc.composite
        .replace_data(composite.clone().into())
        .expect("valid composite plane");
    let planes: Vec<Vec<u8>> = (0..4u8)
        .map(|c| (0..8).map(|i| c * 20 + i).collect())
        .collect();
    doc.layers = pictura_core::LayerStack::from_layers(vec![pictura_codec::layer_from_parts(pictura_core::LayerParts {
        name: "Rle".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 4,
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
                data: planes[0].clone().into(),
            },
            Channel {
                id: 1,
                data: planes[1].clone().into(),
            },
            Channel {
                id: 2,
                data: planes[2].clone().into(),
            },
            Channel {
                id: -1,
                data: planes[3].clone().into(),
            },
        ],
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    })]);

    let dir = scratch_dir("psd-rle-read");
    let path = dir.join("rle.psd");
    std::fs::write(&path, write_psd(&doc).unwrap()).unwrap();

    let script = r#"
import sys
import numpy as np
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1], lazy=False)
arr = np.array(psd.composite().convert("RGB"))
print(arr[:, :, 0].tobytes().hex())
print(arr[:, :, 1].tobytes().hex())
print(arr[:, :, 2].tobytes().hex())
print(np.round(psd[0].numpy()[:, :, 0] * 255).astype(np.uint8).tobytes().hex())
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
    let got: Vec<&str> = stdout.lines().map(str::trim).collect();
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    assert_eq!(got[0], hex(&composite[0..8]), "composite R plane");
    assert_eq!(got[1], hex(&composite[8..16]), "composite G plane");
    assert_eq!(got[2], hex(&composite[16..24]), "composite B plane");
    assert_eq!(got[3], hex(&planes[0]), "layer R channel");
}

/// The independent oracle opens a codec-written version-2 PSB and decodes the
/// same composite pixels.
#[test]
fn psd_tools_reads_written_psb_composite() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    let mut doc = Document::new(4, 2, ColorMode::Rgb, BitDepth::Eight);
    let composite: Vec<u8> = (0..24).map(|i| (i * 7 + 1) as u8).collect();
    doc.composite
        .replace_data(composite.clone().into())
        .expect("valid composite plane");

    let dir = scratch_dir("psb-write-read");
    let path = dir.join("written.psb");
    let bytes = write_psb(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes([bytes[4], bytes[5]]),
        2,
        "PSB version word"
    );
    std::fs::write(&path, &bytes).unwrap();

    let script = r#"
import sys
import numpy as np
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1], lazy=False)
arr = np.array(psd.composite().convert("RGB"))
print(arr[:, :, 0].tobytes().hex())
print(arr[:, :, 1].tobytes().hex())
print(arr[:, :, 2].tobytes().hex())
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
    let got: Vec<&str> = stdout.lines().map(str::trim).collect();
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    assert_eq!(got[0], hex(&composite[0..8]), "PSB composite R plane");
    assert_eq!(got[1], hex(&composite[8..16]), "PSB composite G plane");
    assert_eq!(got[2], hex(&composite[16..24]), "PSB composite B plane");
}

/// Engine-written RAW, ZIP, and ZIP-with-prediction composites, layer channels,
/// and a raster mask must round-trip through our own reader and decode in the
/// independent oracle to the same pixels.
#[test]
fn psd_tools_reads_written_compression_kinds() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    for (tag, compression) in [
        ("raw", Compression::Raw),
        ("zip", Compression::Zip),
        ("zip-prediction", Compression::ZipPrediction),
    ] {
        let mut doc = Document::new(4, 2, ColorMode::Rgb, BitDepth::Eight);
        let composite: Vec<u8> = (0..24).map(|i| (i * 7 + 1) as u8).collect();
        doc.composite
            .replace_data(composite.clone().into())
            .expect("valid composite plane");
        doc.composite_compression = compression;
        doc.layer_compression = compression;
        let planes: Vec<Vec<u8>> = (0..4u8)
            .map(|c| (0..8).map(|i| c * 20 + i).collect())
            .collect();
        let mask: Vec<u8> = (0..8).map(|i| 30 + i as u8).collect();
        doc.layers = pictura_core::LayerStack::from_layers(vec![pictura_codec::layer_from_parts(pictura_core::LayerParts {
            name: "Kind".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 2,
                right: 4,
            },
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            lock: LockFlags::default(),
            color: ColorLabel::None,
            clipping: false,
            visible: true,
            mask: Some(LayerMask {
                rect: PsdRect {
                    top: 0,
                    left: 0,
                    bottom: 2,
                    right: 4,
                },
                default_color: 0,
                disabled: false,
                flags: 0,
                data: Some(mask.clone().into()),
                ..Default::default()
            }),
            adjustment: None,
            channels: vec![
                Channel {
                    id: 0,
                    data: planes[0].clone().into(),
                },
                Channel {
                    id: 1,
                    data: planes[1].clone().into(),
                },
                Channel {
                    id: 2,
                    data: planes[2].clone().into(),
                },
                Channel {
                    id: -1,
                    data: planes[3].clone().into(),
                },
            ],
            children: pictura_core::LayerStack::default(),
            is_group: false,
            background: false,
            ..Default::default()
        })]);

        let bytes = write_psd(&doc).unwrap();
        assert_eq!(bytes, write_psd(&doc).unwrap(), "deterministic ({tag})");
        assert_eq!(read_psd(&bytes).unwrap(), doc, "self round-trip ({tag})");

        let dir = scratch_dir(&format!("psd-{tag}-write-read"));
        let path = dir.join("kind.psd");
        std::fs::write(&path, &bytes).unwrap();

        let script = r#"
import sys
import numpy as np
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1], lazy=False)
arr = np.array(psd.composite().convert("RGB"))
for c in range(3):
    print(arr[:, :, c].tobytes().hex())
layer = psd[0].numpy()
for c in range(2):
    print(np.round(layer[:, :, c] * 255).astype(np.uint8).tobytes().hex())
print(np.array(psd[0].mask.topil()).tobytes().hex())
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
            "psd-tools failed ({tag}):\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        let got: Vec<&str> = stdout.lines().map(str::trim).collect();
        let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(got[0], hex(&composite[0..8]), "composite R plane ({tag})");
        assert_eq!(got[1], hex(&composite[8..16]), "composite G plane ({tag})");
        assert_eq!(got[2], hex(&composite[16..24]), "composite B plane ({tag})");
        assert_eq!(got[3], hex(&planes[0]), "layer R channel ({tag})");
        assert_eq!(got[4], hex(&planes[1]), "layer G channel ({tag})");
        assert_eq!(got[5], hex(&mask), "raster mask ({tag})");
    }
}

/// Alpha-channel names written by the codec are visible to psd-tools through
/// the Unicode alpha-names resource (1045).
#[test]
fn psd_tools_reports_alpha_names() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    let mut doc = Document::new(4, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.channels = vec![
        Channel {
            id: 0,
            data: vec![0u8; 8].into(),
        },
        Channel {
            id: 1,
            data: vec![0u8; 8].into(),
        },
    ];
    let names = vec!["Hair".to_string(), "Ciel étoilé".to_string()];
    pictura_codec::set_alpha_names(&mut doc, &names);

    let dir = scratch_dir("psd-alpha-names");
    let path = dir.join("alpha_names.psd");
    std::fs::write(&path, write_psd(&doc).unwrap()).unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1])
print("|".join(str(name) for name in psd.image_resources.get_data(1045)))
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
        "Hair|Ciel étoilé",
        "psd-tools reports the Unicode alpha names"
    );
}

/// A PSD whose names exist only in the legacy 1006 resource is readable by
/// psd-tools: the Pascal strings are unpadded and MacRoman-encoded.
#[test]
fn psd_tools_reports_pascal_alpha_names() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    let mut doc = Document::new(4, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.channels = vec![
        Channel {
            id: 0,
            data: vec![0u8; 8].into(),
        },
        Channel {
            id: 1,
            data: vec![0u8; 8].into(),
        },
    ];
    let names = [&b"Hair"[..], &b"Caf\x8E"[..]];
    let mut data = Vec::new();
    for name in names {
        data.push(name.len() as u8);
        data.extend_from_slice(name);
    }
    doc.image_resources =
        pictura_codec::encode_image_resources(&[pictura_codec::frame_image_resource(
            pictura_codec::ALPHA_NAMES_PASCAL,
            "",
            &data,
        )])
        .into();

    let dir = scratch_dir("psd-alpha-pascal-names");
    let path = dir.join("alpha_pascal_names.psd");
    std::fs::write(&path, write_psd(&doc).unwrap()).unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1])
print("|".join(str(name) for name in psd.image_resources.get_data(1006)))
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
        "Hair|Café",
        "psd-tools reports the legacy Pascal alpha names"
    );
}
