use super::*;
use crate::depth::apply_prediction;
use crate::read::decode::{decode_channel_data, decode_depth_channel};
use flate2::write::{DeflateEncoder, ZlibEncoder};
use flate2::Compression as FlateCompression;
use std::io::Write;

/// Seeded native bytes for a `width × height` channel at `depth`, with runs
/// so PackBits writes both packet kinds.
fn native(width: usize, height: usize, depth: u16, seed: u32) -> Vec<u8> {
    let mut state = seed;
    let mut next = move || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        state >> 8
    };
    let mut out = Vec::with_capacity(row_bytes(width, depth) * height);
    for y in 0..height {
        for x in 0..width {
            let run = (x / 9 + y) % 3 == 0;
            let v = if run { 0x00ab_cdef } else { next() };
            match depth {
                16 => out.extend_from_slice(&(v as u16).to_be_bytes()),
                // Some values sit outside [0, 1] so the narrowing clamps.
                32 => out.extend_from_slice(&((v % 1300) as f32 / 1000.0 - 0.1).to_be_bytes()),
                _ => out.push(v as u8),
            }
        }
    }
    out
}

fn rle(raw: &[u8], stride: usize, is_psb: bool) -> Vec<u8> {
    let rows: Vec<Vec<u8>> = raw
        .chunks_exact(stride)
        .map(|row| {
            let mut out = Vec::new();
            crate::write::encode_packbits_row(row, &mut out);
            out
        })
        .collect();
    let mut payload = Vec::new();
    for row in &rows {
        if is_psb {
            payload.extend_from_slice(&(row.len() as u32).to_be_bytes());
        } else {
            payload.extend_from_slice(&(row.len() as u16).to_be_bytes());
        }
    }
    rows.iter().for_each(|row| payload.extend_from_slice(row));
    payload
}

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut enc = ZlibEncoder::new(Vec::new(), FlateCompression::fast());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut enc = DeflateEncoder::new(Vec::new(), FlateCompression::fast());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

/// `raw` encoded as `compression`.
fn encode(
    raw: &[u8],
    width: usize,
    height: usize,
    depth: u16,
    compression: u16,
    is_psb: bool,
) -> Vec<u8> {
    let stride = row_bytes(width, depth);
    match compression {
        COMPRESSION_RAW => raw.to_vec(),
        COMPRESSION_RLE => rle(raw, stride, is_psb),
        COMPRESSION_ZIP => zlib(raw),
        _ => {
            let mut predicted = raw.to_vec();
            apply_prediction(&mut predicted, width, height, depth);
            zlib(&predicted)
        }
    }
}

type Outcome = Result<(Vec<u8>, Option<Samples>), String>;

fn dense(payload: &[u8], compression: u16, w: usize, h: usize, psb: bool, depth: u16) -> Outcome {
    let result = if depth == 8 {
        decode_channel_data(compression, payload, w, h, psb).map(|plane| (plane, None))
    } else {
        decode_depth_channel(compression, payload, w, h, psb, depth).map(|(n, s)| (n, Some(s)))
    };
    result.map_err(|e| format!("{e:?}"))
}

fn streamed(
    payload: &[u8],
    compression: u16,
    w: usize,
    h: usize,
    psb: bool,
    depth: u16,
) -> Outcome {
    stream_channel(compression, payload, w, h, psb, depth, true)
        .map(|(plane, native)| {
            assert!(plane.is_tiled(), "the 8-bit plane is tiled");
            if let Some(native) = &native {
                let tiled = match native {
                    Samples::U16(p) => p.is_tiled(),
                    Samples::F32(p) => p.is_tiled(),
                    Samples::U8(p) => p.is_tiled(),
                };
                assert!(tiled, "the native plane is tiled");
            }
            (plane.to_vec(), native)
        })
        .map_err(|e| format!("{e:?}"))
}

#[test]
fn streamed_channels_match_the_dense_decode() {
    let compressions = [
        COMPRESSION_RAW,
        COMPRESSION_RLE,
        COMPRESSION_ZIP,
        COMPRESSION_ZIP_PREDICTION,
    ];
    for (w, h) in [(600, 300), (257, 513)] {
        for depth in [8u16, 16, 32] {
            let raw = native(w, h, depth, (w * h) as u32 + u32::from(depth));
            for compression in compressions {
                for psb in [false, true] {
                    let payload = encode(&raw, w, h, depth, compression, psb);
                    let want = dense(&payload, compression, w, h, psb, depth).expect("dense");
                    let got = streamed(&payload, compression, w, h, psb, depth).expect("streamed");
                    assert_eq!(
                        got, want,
                        "{w}x{h} depth {depth} compression {compression} psb {psb}"
                    );
                }
            }
        }
    }
}

#[test]
fn streamed_channels_report_the_dense_errors() {
    let (w, h) = (300, 270);
    let raw = native(w, h, 8, 7);
    let mut long = raw.clone();
    long.push(1);
    let mut short_rle = rle(&raw, w, false);
    short_rle.truncate(short_rle.len() - 5);
    let mut overrun = rle(&raw, w, false);
    // Row 0's count claims one more byte than it holds, so it overruns.
    let first = u16::from_be_bytes([overrun[0], overrun[1]]) + 1;
    overrun[..2].copy_from_slice(&first.to_be_bytes());
    let cases: Vec<(&str, u16, Vec<u8>)> = vec![
        ("short raw", COMPRESSION_RAW, raw[..raw.len() - 1].to_vec()),
        ("truncated RLE", COMPRESSION_RLE, short_rle),
        ("overrunning RLE row", COMPRESSION_RLE, overrun),
        ("short ZIP", COMPRESSION_ZIP, zlib(&raw[..raw.len() - 10])),
        ("long ZIP", COMPRESSION_ZIP, zlib(&long)),
        ("raw deflate", COMPRESSION_ZIP, deflate(&raw)),
        ("garbage ZIP", COMPRESSION_ZIP, vec![0xff; 64]),
        ("unknown compression", 9, raw.clone()),
    ];
    for (name, compression, payload) in cases {
        let want = dense(&payload, compression, w, h, false, 8);
        let got = streamed(&payload, compression, w, h, false, 8);
        assert_eq!(got, want, "{name}");
    }
    assert!(
        streamed(&deflate(&raw), COMPRESSION_ZIP, w, h, false, 8).is_ok(),
        "raw deflate decodes"
    );
}

#[test]
fn a_uniform_channel_streams_without_allocation() {
    let (w, h) = (1024, 1024);
    let raw = vec![77u8; w * h];
    let (plane, _) = stream_channel(COMPRESSION_RLE, &rle(&raw, w, false), w, h, false, 8, true)
        .expect("decode");
    assert!(plane.allocated_bytes() < w * h / 4);
    assert_eq!(plane.to_vec(), raw);
}
