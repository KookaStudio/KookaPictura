use super::*;

// -- Hand-built ZIP/ZIP-prediction files, psd-tools as the decoder oracle --
//
// psd-tools' writer only emits raw/RLE, so these files are assembled by hand
// here and psd-tools is used as an independent decoder of the same bytes.

fn psd_header(channels: u16, width: u32, height: u32) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"8BPS");
    v.extend_from_slice(&1u16.to_be_bytes());
    v.extend_from_slice(&[0u8; 6]);
    v.extend_from_slice(&channels.to_be_bytes());
    v.extend_from_slice(&height.to_be_bytes());
    v.extend_from_slice(&width.to_be_bytes());
    v.extend_from_slice(&8u16.to_be_bytes());
    v.extend_from_slice(&3u16.to_be_bytes()); // RGB
    v
}

fn zlib_compress(data: &[u8]) -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

/// Forward byte-wise delta per `row_len` scanline: inverse of the reader's
/// `undo_prediction`.
fn predict_forward(data: &[u8], row_len: usize) -> Vec<u8> {
    let mut out = data.to_vec();
    for row_start in (0..out.len()).step_by(row_len) {
        let row_end = (row_start + row_len).min(out.len());
        for i in (row_start + 1)..row_end {
            out[i] = data[i].wrapping_sub(data[i - 1]);
        }
    }
    out
}

/// A 2x2 RGB PSD with one pixel layer carrying only channel 0, whose data is
/// `compression` followed by `encoded`; the merged composite is raw zeros.
fn one_layer_zip_psd(compression: u16, encoded: &[u8]) -> Vec<u8> {
    let channel_len = 2 + encoded.len();

    let mut extra = Vec::new();
    extra.extend_from_slice(&0u32.to_be_bytes()); // no mask
    extra.extend_from_slice(&0u32.to_be_bytes()); // blending ranges
    extra.extend_from_slice(&[1, b'L', 0, 0]); // pascal name "L"

    let mut rec = Vec::new();
    for v in [0i32, 0, 2, 2] {
        rec.extend_from_slice(&v.to_be_bytes());
    }
    rec.extend_from_slice(&1u16.to_be_bytes()); // one channel
    rec.extend_from_slice(&0i16.to_be_bytes()); // id 0
    rec.extend_from_slice(&(channel_len as u32).to_be_bytes());
    rec.extend_from_slice(b"8BIM");
    rec.extend_from_slice(b"norm");
    rec.push(255);
    rec.push(0);
    rec.push(0);
    rec.push(0);
    rec.extend_from_slice(&(extra.len() as u32).to_be_bytes());
    rec.extend_from_slice(&extra);

    let mut info = Vec::new();
    info.extend_from_slice(&1i16.to_be_bytes());
    info.extend_from_slice(&rec);
    info.extend_from_slice(&compression.to_be_bytes());
    info.extend_from_slice(encoded);
    while info.len() % 4 != 0 {
        info.push(0);
    }

    let mut out = psd_header(3, 2, 2);
    out.extend_from_slice(&0u32.to_be_bytes()); // color mode data
    out.extend_from_slice(&0u32.to_be_bytes()); // image resources
    let section_len = 4 + info.len() + 4;
    out.extend_from_slice(&(section_len as u32).to_be_bytes());
    out.extend_from_slice(&(info.len() as u32).to_be_bytes());
    out.extend_from_slice(&info);
    out.extend_from_slice(&0u32.to_be_bytes()); // global layer mask
    out.extend_from_slice(&0u16.to_be_bytes()); // raw composite
    out.extend_from_slice(&[0u8; 12]); // 2x2 RGB planes
    out
}

/// A ZIP-with-prediction layer channel built by hand must decode, in
/// psd-tools, to the same bytes our reader recovers. psd-tools 1.19 returns
/// `numpy()` scaled to 0..1 floats, so a byte is `round(v * 255)`.
#[test]
fn zip_prediction_layer_channel_matches_psd_tools() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }

    let expected = [10u8, 20, 30, 40];
    let encoded = zlib_compress(&predict_forward(&expected, 2));
    let psd = one_layer_zip_psd(3, &encoded);

    let dir = scratch_dir("psd-zip-layer");
    let path = dir.join("zip_layer.psd");
    std::fs::write(&path, &psd).unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1])
layer = psd[0]
arr = layer.numpy()
print(",".join(str(int(round(v * 255))) for v in arr[..., 0].flatten()))
"#;
    let out = oracle::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&path)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        out.status.success(),
        "psd-tools failed on the hand-built file:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let got: Vec<u8> = String::from_utf8_lossy(&out.stdout)
        .trim()
        .split(',')
        .map(|s| s.parse().unwrap())
        .collect();
    let ours = read_psd(&psd).unwrap();
    assert_eq!(ours.layers[0].channels()[0].data, expected);
    assert_eq!(
        got, expected,
        "psd-tools decodes the ZIP-prediction channel"
    );
}

/// A bare ZIP-with-prediction composite built by hand must decode, in
/// psd-tools, to the same RGB planes our reader recovers.
#[test]
fn zip_prediction_composite_matches_psd_tools() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }

    let planes: Vec<u8> = (1..=12).collect();
    let encoded = zlib_compress(&predict_forward(&planes, 2));

    let mut psd = psd_header(3, 2, 2);
    psd.extend_from_slice(&[0u8; 12]); // color mode, resources, empty layer section
    psd.extend_from_slice(&3u16.to_be_bytes());
    psd.extend_from_slice(&encoded);

    let dir = scratch_dir("psd-zip-composite");
    let path = dir.join("zip_composite.psd");
    std::fs::write(&path, &psd).unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1])
arr = psd.numpy()
vals = []
for c in range(arr.shape[2]):
    vals.extend(int(round(v * 255)) for v in arr[..., c].flatten())
print(",".join(str(v) for v in vals))
"#;
    let out = oracle::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&path)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        out.status.success(),
        "psd-tools failed on the hand-built file:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let got: Vec<u8> = String::from_utf8_lossy(&out.stdout)
        .trim()
        .split(',')
        .map(|s| s.parse().unwrap())
        .collect();
    let ours = read_psd(&psd).unwrap();
    assert_eq!(ours.composite.data, planes);
    assert_eq!(
        got, planes,
        "psd-tools decodes the ZIP-prediction composite"
    );
}
