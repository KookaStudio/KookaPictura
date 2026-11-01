//! Differential tests for 16- and 32-bit composites narrowed to the 8-bit
//! working model. The committed fixtures (authored by
//! `scripts/generate-fixtures.py`) are re-read with the independent `psd-tools`
//! decoder and compared exactly (0 tolerance). The oracle self-skips when
//! `psd-tools` is absent.

use std::path::PathBuf;
use std::process::Command;

use pictura_codec::{read_psd, write_psd};
use pictura_core::{BitDepth, ColorMode, Compression, Document};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load(name: &str) -> Document {
    let path = fixture_dir().join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}); run `python3 scripts/generate-fixtures.py`",
            path.display()
        )
    });
    read_psd(&bytes).unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()))
}

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

/// Decode `path` with `psd-tools` and return its composite as planar RGB hex.
fn psd_tools_composite_planes(path: &std::path::Path) -> Vec<String> {
    let script = r#"
import sys
import numpy as np
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1], lazy=False)
arr = np.array(psd.composite().convert("RGB"))
for c in range(3):
    print(arr[:, :, c].tobytes().hex())
"#;
    let out = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(path)
        .output()
        .expect("run python3");
    assert!(
        out.status.success(),
        "psd-tools failed on {}:\n{}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| line.trim().to_string())
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The native-depth view `psd-tools` decodes from a written file.
struct NativeInfo {
    depth: u16,
    mode: u16,
    /// One hex string per image-data plane (color channels then extras).
    planes: Vec<String>,
    /// `(channel id, compression, native hex)` per layer channel.
    layer_channels: Vec<(i16, u16, String)>,
}

/// Decode `path` with `psd-tools`, returning its header depth and the native
/// (pre-narrowing) bytes of the composite planes and every layer channel.
fn psd_tools_native(path: &std::path::Path) -> NativeInfo {
    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1], lazy=False)
print("H", int(psd.depth), int(psd.color_mode))
for plane in psd._record.image_data.get_data(psd._record.header):
    print("P", bytes(plane).hex())
for layer in psd:
    for info, ch in zip(layer._record.channel_info, layer._channels):
        data = ch.get_data(layer.width, layer.height, psd.depth, psd.version)
        print("L", int(info.id), int(ch.compression), bytes(data).hex())
"#;
    let out = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(path)
        .output()
        .expect("run python3");
    assert!(
        out.status.success(),
        "psd-tools failed on {}:\n{}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut depth = 0;
    let mut mode = 0;
    let mut planes = Vec::new();
    let mut layer_channels = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("H") => {
                depth = parts.next().unwrap().parse().unwrap();
                mode = parts.next().unwrap().parse().unwrap();
            }
            Some("P") => planes.push(parts.next().unwrap().to_string()),
            Some("L") => {
                layer_channels.push((
                    parts.next().unwrap().parse().unwrap(),
                    parts.next().unwrap().parse().unwrap(),
                    parts.next().unwrap().to_string(),
                ));
            }
            _ => {}
        }
    }
    NativeInfo {
        depth,
        mode,
        planes,
        layer_channels,
    }
}

/// Every narrowed 8-bit pixel is byte-identical to psd-tools' composite.
fn assert_matches_psd_tools(name: &str) {
    let path = fixture_dir().join(name);
    let doc = load(name);
    let plane = doc.composite.pixel_count();
    let theirs = psd_tools_composite_planes(&path);
    assert_eq!(theirs.len(), 3, "psd-tools printed three RGB planes");
    for (c, expected) in theirs.iter().enumerate() {
        assert_eq!(
            hex(&doc.composite.data[c * plane..(c + 1) * plane]),
            *expected,
            "{name}: plane {c}"
        );
    }
}

#[test]
fn depth16_fixture_matches_psd_tools_exactly() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }
    assert_matches_psd_tools("rgb16.psd");
}

#[test]
fn depth32_fixture_matches_psd_tools_exactly() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }
    assert_matches_psd_tools("rgb32.psd");
}

/// A high-depth read preserves its source depth on save: the output header
/// declares the source depth, re-reading yields the same 8-bit pixels and the
/// source depth, and `psd-tools` reads the native source samples exactly.
#[test]
fn depth_documents_preserve_source_depth_on_save() {
    for (name, bits, depth) in [
        ("rgb16.psd", BitDepth::Sixteen, 16u16),
        ("rgb32.psd", BitDepth::ThirtyTwo, 32u16),
    ] {
        let doc = load(name);
        assert_eq!(doc.mode, ColorMode::Rgb, "{name}: working mode");
        assert_eq!(doc.depth, BitDepth::Eight, "{name}: normalized depth");
        assert_eq!(
            doc.source_depth,
            Some(bits),
            "{name}: recorded source depth"
        );
        let source = doc
            .source_planes
            .clone()
            .unwrap_or_else(|| panic!("{name}: retained source planes"));
        assert_eq!(source.depth, bits);

        let out = write_psd(&doc).unwrap();
        assert_eq!(
            u16::from_be_bytes(out[22..24].try_into().unwrap()),
            depth,
            "{name}: output header declares the source depth"
        );
        let back = read_psd(&out).unwrap();
        assert_eq!(back.source_depth, Some(bits), "{name}: depth round-trips");
        assert_eq!(
            back.composite, doc.composite,
            "{name}: normalized pixels are stable across a save"
        );

        if !psd_tools_available() {
            eprintln!("skipping: python3 + psd-tools not available");
            continue;
        }
        let dir = scratch_dir("depth-roundtrip");
        let path = dir.join(name);
        std::fs::write(&path, &out).unwrap();
        let native = psd_tools_native(&path);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(native.depth, depth, "{name}: psd-tools header depth");
        assert_eq!(native.mode, 3, "{name}: psd-tools RGB mode");
        assert_eq!(native.planes.len(), 3, "{name}: three color planes");
        let plane = source.samples.to_bytes().len() / 3;
        for (c, theirs) in native.planes.iter().enumerate() {
            assert_eq!(
                hex(&source.samples.to_bytes()[c * plane..(c + 1) * plane]),
                *theirs,
                "{name}: psd-tools native samples, plane {c}"
            );
        }
    }
}

/// Every recorded composite compression re-encodes the retained source samples
/// at the source depth: raw, PackBits RLE, ZIP, and ZIP-with-prediction all
/// round-trip, and `psd-tools` decodes the same native planes.
#[test]
fn high_depth_composite_re_encodes_every_compression() {
    for (name, bits, depth) in [
        ("rgb16.psd", BitDepth::Sixteen, 16u16),
        ("rgb32.psd", BitDepth::ThirtyTwo, 32u16),
    ] {
        let mut doc = load(name);
        let source = doc.source_planes.clone().unwrap();
        assert_eq!(source.depth, bits);
        let plane = source.samples.to_bytes().len() / 3;
        for kind in [
            Compression::Raw,
            Compression::Rle,
            Compression::Zip,
            Compression::ZipPrediction,
        ] {
            doc.composite_compression = kind;
            let out = write_psd(&doc).unwrap();
            assert_eq!(
                u16::from_be_bytes(out[22..24].try_into().unwrap()),
                depth,
                "{name} {kind:?}: header depth"
            );
            let back = read_psd(&out).unwrap();
            assert_eq!(back.composite, doc.composite, "{name} {kind:?}: pixels");

            if !psd_tools_available() {
                eprintln!("skipping: python3 + psd-tools not available");
                continue;
            }
            let dir = scratch_dir("depth-compression");
            let path = dir.join(format!("{name}-{kind:?}.psd"));
            std::fs::write(&path, &out).unwrap();
            let native = psd_tools_native(&path);
            let _ = std::fs::remove_dir_all(&dir);
            assert_eq!(native.depth, depth, "{name} {kind:?}: psd-tools depth");
            assert_eq!(native.planes.len(), 3, "{name} {kind:?}: three planes");
            for (c, theirs) in native.planes.iter().enumerate() {
                assert_eq!(
                    hex(&source.samples.to_bytes()[c * plane..(c + 1) * plane]),
                    *theirs,
                    "{name} {kind:?}: psd-tools native plane {c}"
                );
            }
        }
    }
}

/// An edited plane keeps the source depth and is the 8-bit byte widened
/// (`v * 257` at 16, `v / 255` at 32): reading it back narrows to the edited
/// byte, and `psd-tools` sees the widened native sample.
#[test]
fn edited_plane_saves_widened_at_source_depth() {
    for (name, depth) in [("rgb16.psd", 16u16), ("rgb32.psd", 32u16)] {
        let mut doc = load(name);
        let edited = doc.composite.data[0].wrapping_add(1);
        doc.composite.data[0] = edited;
        let out = write_psd(&doc).unwrap();
        assert_eq!(
            u16::from_be_bytes(out[22..24].try_into().unwrap()),
            depth,
            "{name}: edited save keeps the source depth"
        );
        let back = read_psd(&out).unwrap();
        assert_eq!(
            back.composite.data[0], edited,
            "{name}: the widened plane narrows back to the edit"
        );

        if !psd_tools_available() {
            eprintln!("skipping: python3 + psd-tools not available");
            continue;
        }
        let dir = scratch_dir("depth-edited");
        let path = dir.join(name);
        std::fs::write(&path, &out).unwrap();
        let native = psd_tools_native(&path);
        let _ = std::fs::remove_dir_all(&dir);
        let sample = match depth {
            16 => (edited as u16 * 257).to_be_bytes().to_vec(),
            _ => (edited as f32 / 255.0).to_be_bytes().to_vec(),
        };
        let bytes = sample.len();
        assert_eq!(
            &native.planes[0][..bytes * 2],
            hex(&sample),
            "{name}: the edited sample is the widened 8-bit byte"
        );
    }
}

/// A 1x1 RGB document at `depth` with one layer carrying a single unmodeled
/// `id` channel (a spot/selection channel) whose raw samples are `plane`. The
/// layer has no color channels, so `psd-tools` sees exactly this channel.
fn layered_spot_psd(depth: u16, id: i16, plane: &[u8]) -> Vec<u8> {
    let mut rec = Vec::new();
    for v in [0i32, 0, 1, 1] {
        rec.extend_from_slice(&v.to_be_bytes());
    }
    rec.extend_from_slice(&1u16.to_be_bytes()); // one layer channel
    rec.extend_from_slice(&id.to_be_bytes());
    rec.extend_from_slice(&((2 + plane.len()) as u32).to_be_bytes());
    rec.extend_from_slice(b"8BIMnorm");
    rec.extend_from_slice(&[255, 0, 0, 0]); // opacity, clipping, flags, filler
    let extra = {
        let mut e = Vec::new();
        e.extend_from_slice(&0u32.to_be_bytes()); // no mask
        e.extend_from_slice(&0u32.to_be_bytes()); // blending ranges
        e.extend_from_slice(&[1, b'L', 0, 0]); // pascal name "L"
        e
    };
    rec.extend_from_slice(&(extra.len() as u32).to_be_bytes());
    rec.extend_from_slice(&extra);

    let mut info = Vec::new();
    info.extend_from_slice(&1i16.to_be_bytes());
    info.extend_from_slice(&rec);
    info.extend_from_slice(&0u16.to_be_bytes()); // raw channel compression
    info.extend_from_slice(plane);
    while info.len() % 4 != 0 {
        info.push(0);
    }

    let mut out = Vec::new();
    out.extend_from_slice(b"8BPS");
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&[0u8; 6]);
    out.extend_from_slice(&3u16.to_be_bytes()); // RGB
    out.extend_from_slice(&1u32.to_be_bytes()); // height
    out.extend_from_slice(&1u32.to_be_bytes()); // width
    out.extend_from_slice(&depth.to_be_bytes());
    out.extend_from_slice(&3u16.to_be_bytes()); // color mode code
    out.extend_from_slice(&0u32.to_be_bytes()); // color mode data
    out.extend_from_slice(&0u32.to_be_bytes()); // image resources
    let section_len = 4 + info.len() + 4;
    out.extend_from_slice(&(section_len as u32).to_be_bytes());
    out.extend_from_slice(&(info.len() as u32).to_be_bytes());
    out.extend_from_slice(&info);
    out.extend_from_slice(&0u32.to_be_bytes()); // global layer mask
    out.extend_from_slice(&0u16.to_be_bytes()); // raw composite compression
                                                // Three RGB planes at the document's sample width (the pixel values are
                                                // irrelevant to this test; only the layer channel is checked).
    out.extend(std::iter::repeat_n(0u8, 3 * (depth as usize / 8)));
    out
}

/// An open→save of a high-depth document re-emits an unmodeled layer channel at
/// the source depth; the engine still narrows it to the same 8-bit stream, and
/// `psd-tools` decodes the native samples.
#[test]
fn unmodeled_layer_channel_preserves_source_depth() {
    for (depth, plane, bits) in [
        (16u16, 0xab00u16.to_be_bytes().to_vec(), BitDepth::Sixteen),
        (32u16, 0.5f32.to_be_bytes().to_vec(), BitDepth::ThirtyTwo),
        (16u16, 0x0100u16.to_be_bytes().to_vec(), BitDepth::Sixteen),
    ] {
        let input = layered_spot_psd(depth, 3, &plane);
        let doc = read_psd(&input).unwrap();
        assert_eq!(doc.source_depth, Some(bits), "depth {depth}: source depth");
        let retained = doc.layers[0]
            .source_channels
            .as_ref()
            .expect("retained layer channels")
            .planes
            .clone();
        assert_eq!(
            retained
                .iter()
                .map(|(id, s)| (*id, s.to_bytes()))
                .collect::<Vec<_>>(),
            vec![(3, plane.clone())],
            "depth {depth}: native spot samples kept"
        );

        let out = write_psd(&doc).unwrap();
        assert_eq!(
            u16::from_be_bytes(out[22..24].try_into().unwrap()),
            depth,
            "depth {depth}: output header preserves the source depth"
        );
        let back = read_psd(&out).unwrap();
        assert_eq!(
            back.layers[0].raw_channels[0].data, doc.layers[0].raw_channels[0].data,
            "depth {depth}: the re-read 8-bit stream narrows back unchanged"
        );

        if !psd_tools_available() {
            eprintln!("skipping: python3 + psd-tools not available");
            continue;
        }
        let dir = scratch_dir("unmodeled-depth");
        let path = dir.join(format!("spot{depth}.psd"));
        std::fs::write(&path, &out).unwrap();
        let native = psd_tools_native(&path);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(native.depth, depth);
        assert_eq!(native.layer_channels.len(), 1);
        let (id, _compression, native_hex) = &native.layer_channels[0];
        assert_eq!(*id, 3, "depth {depth}: spot channel id");
        assert_eq!(
            *native_hex,
            hex(&plane),
            "depth {depth}: native spot samples"
        );
    }
}

/// The layered depth fixtures exercise a document extra channel and a layer
/// carrying color, transparency, a mask, and an unmodeled channel. A save
/// preserves the depth and every native plane, and `psd-tools` agrees.
#[test]
fn layered_depth_fixtures_preserve_depth_and_channels() {
    for (name, bits, depth) in [
        ("rgb16_layered.psd", BitDepth::Sixteen, 16u16),
        ("rgb16_tagged.psd", BitDepth::Sixteen, 16u16),
        ("rgb32_layered.psd", BitDepth::ThirtyTwo, 32u16),
    ] {
        let doc = load(name);
        assert_eq!(doc.source_depth, Some(bits));
        assert_eq!(doc.channels.len(), 1, "{name}: one document extra channel");
        let source = doc.source_planes.clone().unwrap();
        let layer_source = doc.layers[0].source_channels.clone().unwrap();
        assert_eq!(layer_source.depth, bits);
        assert_eq!(
            layer_source
                .planes
                .iter()
                .map(|(id, _)| *id)
                .collect::<Vec<_>>(),
            vec![-2, -1, 0, 1, 2, 3],
            "{name}: every layer channel retained"
        );

        let out = write_psd(&doc).unwrap();
        assert_eq!(
            u16::from_be_bytes(out[22..24].try_into().unwrap()),
            depth,
            "{name}: output header preserves the source depth"
        );
        let back = read_psd(&out).unwrap();
        assert_eq!(
            back, doc,
            "{name}: an unedited high-depth layered document round-trips whole"
        );

        if !psd_tools_available() {
            eprintln!("skipping: python3 + psd-tools not available");
            continue;
        }
        let dir = scratch_dir("layered-depth");
        let path = dir.join(name);
        std::fs::write(&path, &out).unwrap();
        let native = psd_tools_native(&path);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(native.depth, depth);
        assert_eq!(native.planes.len(), 4, "{name}: color + one extra channel");
        let plane = source.samples.to_bytes().len() / 4;
        for (c, theirs) in native.planes.iter().enumerate() {
            assert_eq!(
                hex(&source.samples.to_bytes()[c * plane..(c + 1) * plane]),
                *theirs,
                "{name}: psd-tools native plane {c}"
            );
        }
        assert_eq!(native.layer_channels.len(), 6, "{name}: six layer channels");
        for (id, compression, native_hex) in &native.layer_channels {
            let expected = layer_source
                .planes
                .iter()
                .find(|(cid, _)| cid == id)
                .unwrap_or_else(|| panic!("{name}: psd-tools channel {id}"));
            assert_eq!(*compression, 0, "{name}: channel {id} raw");
            assert_eq!(
                *native_hex,
                hex(&expected.1.to_bytes()),
                "{name}: psd-tools channel {id} samples"
            );
        }
    }
}

/// A grouped high-depth document round-trips whole-`Document`-equal: a folder's
/// placeholder channels are not retained as source samples.
#[test]
fn grouped_depth_fixtures_round_trip_whole() {
    for (name, bits, depth) in [
        ("rgb16_grouped.psd", BitDepth::Sixteen, 16u16),
        ("rgb32_grouped.psd", BitDepth::ThirtyTwo, 32u16),
    ] {
        let doc = load(name);
        assert_eq!(doc.source_depth, Some(bits));
        assert_eq!(doc.layers.len(), 2, "{name}: a plain layer and a group");
        let plain = &doc.layers[0];
        assert!(!plain.is_group(), "{name}: plain layer first");
        assert!(
            plain.source_channels.is_some(),
            "{name}: an lsct=0 layer keeps its retained samples"
        );
        let group = &doc.layers[1];
        assert!(group.is_group(), "{name}: group second");
        assert!(
            group.source_channels.is_none(),
            "{name}: a group retains no source channels"
        );
        assert!(
            group.children[0].source_channels.is_some(),
            "{name}: an inner pixel layer retains its channels"
        );

        let out = write_psd(&doc).unwrap();
        assert_eq!(
            u16::from_be_bytes(out[22..24].try_into().unwrap()),
            depth,
            "{name}: header depth"
        );
        assert_eq!(
            read_psd(&out).unwrap(),
            doc,
            "{name}: grouped document round-trips whole"
        );
    }
}

/// Every recorded layer compression re-encodes the retained layer channels at
/// the source depth; `psd-tools` decodes the same native samples.
#[test]
fn layered_depth_layer_channels_re_encode_every_compression() {
    for kind in [
        Compression::Rle,
        Compression::Zip,
        Compression::ZipPrediction,
    ] {
        for (name, depth) in [("rgb16_layered.psd", 16u16), ("rgb32_layered.psd", 32u16)] {
            let mut doc = load(name);
            let layer_source = doc.layers[0].source_channels.clone().unwrap();
            doc.layer_compression = kind;
            let out = write_psd(&doc).unwrap();
            assert_eq!(
                u16::from_be_bytes(out[22..24].try_into().unwrap()),
                depth,
                "{name} {kind:?}: header depth"
            );
            let back = read_psd(&out).unwrap();
            assert_eq!(
                back.layers[0].channels, doc.layers[0].channels,
                "{name} {kind:?}: color/alpha stable"
            );
            assert_eq!(
                back.layers[0].mask, doc.layers[0].mask,
                "{name} {kind:?}: mask stable"
            );
            assert_eq!(
                back.layers[0].raw_channels, doc.layers[0].raw_channels,
                "{name} {kind:?}: unmodeled stable"
            );

            if !psd_tools_available() {
                eprintln!("skipping: python3 + psd-tools not available");
                continue;
            }
            let dir = scratch_dir("layer-compression");
            let path = dir.join(format!("{name}-{kind:?}.psd"));
            std::fs::write(&path, &out).unwrap();
            let native = psd_tools_native(&path);
            let _ = std::fs::remove_dir_all(&dir);
            assert_eq!(native.depth, depth, "{name} {kind:?}: psd-tools depth");
            assert_eq!(
                native.layer_channels.len(),
                6,
                "{name} {kind:?}: six layer channels"
            );
            for (id, compression, native_hex) in &native.layer_channels {
                assert_eq!(
                    *compression,
                    kind.to_code(),
                    "{name} {kind:?}: psd-tools channel {id} kind"
                );
                let expected = layer_source
                    .planes
                    .iter()
                    .find(|(cid, _)| cid == id)
                    .unwrap_or_else(|| panic!("{name} {kind:?}: channel {id}"));
                assert_eq!(
                    *native_hex,
                    hex(&expected.1.to_bytes()),
                    "{name} {kind:?}: psd-tools channel {id} samples"
                );
            }
        }
    }
}

fn scratch_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "pictura-codec-depth-{}-{tag}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
