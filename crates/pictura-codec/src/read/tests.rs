use super::decode::{decode_channel_tasks, plan_channel_tasks, ChannelTask, PlannedChannel};
use super::*;
use crate::budget::DecodeBudget;
use crate::depth::{apply_prediction, undo_prediction};
use crate::probe::ImageBudget;
use crate::{write_psb, write_psd};
use flate2::write::ZlibEncoder;
use flate2::Compression as FlateCompression;
use pictura_core::{Channel, Compression, Document, LayerStack, RawChannel};
use std::io::Write;

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut enc = ZlibEncoder::new(Vec::new(), FlateCompression::fast());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

#[test]
fn zip_bomb_is_rejected_with_bounded_allocation() {
    // 256 MiB of zeros compresses to a few hundred KiB. The bounded decode
    // reads at most `expected + 1` bytes, so the expansion is never
    // materialised; before the guard this allocated the whole 256 MiB (and
    // a real bomb scales without limit).
    let mut enc = ZlibEncoder::new(Vec::new(), FlateCompression::fast());
    let chunk = vec![0u8; 1 << 20];
    for _ in 0..256 {
        enc.write_all(&chunk).unwrap();
    }
    let bomb = enc.finish().unwrap();
    assert!(
        bomb.len() < 4 << 20,
        "bomb input stays small: {}",
        bomb.len()
    );

    let err = inflate(&bomb, 4).unwrap_err();
    assert!(matches!(err, PsdError::Invalid(_)), "typed error: {err:?}");
}

#[test]
fn zip_exact_is_ok_and_both_mismatches_error() {
    assert_eq!(inflate(&zlib(&[1, 2, 3, 4]), 4).unwrap(), vec![1, 2, 3, 4]);
    assert!(matches!(
        inflate(&zlib(&[1, 2]), 4).unwrap_err(),
        PsdError::Invalid(_)
    ));
    // Over-long output is malformed, not silently truncated.
    assert!(matches!(
        inflate(&zlib(&[1, 2, 3, 4, 5]), 4).unwrap_err(),
        PsdError::Invalid(_)
    ));
    // A bare deflate stream (no zlib framing) still decodes via the fallback.
    use flate2::write::DeflateEncoder;
    let mut enc = DeflateEncoder::new(Vec::new(), FlateCompression::fast());
    enc.write_all(&[9, 9, 9]).unwrap();
    assert_eq!(inflate(&enc.finish().unwrap(), 3).unwrap(), vec![9, 9, 9]);
}

// ---------------------------------------------------------------------------
// Parallel decode: byte-identical output and ordered errors
// ---------------------------------------------------------------------------

/// The same decode on a one-thread pool, the serial oracle for these tests.
fn serial<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("one-thread pool")
        .install(f)
}

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {} ({e})", path.display()))
}

fn assert_parallel_matches_serial(name: &str, bytes: &[u8]) {
    let parallel = read_psd(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
    let serial_doc = serial(|| read_psd(bytes)).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(parallel, serial_doc, "{name}");
}

/// The 26-byte PSD file header: signature, version, reserved, channels,
/// dimensions, depth, and mode.
fn header_bytes(
    version: u16,
    channels: u16,
    width: u32,
    height: u32,
    depth: u16,
    mode: u16,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"8BPS");
    out.extend_from_slice(&version.to_be_bytes());
    out.extend_from_slice(&[0u8; 6]);
    out.extend_from_slice(&channels.to_be_bytes());
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&depth.to_be_bytes());
    out.extend_from_slice(&mode.to_be_bytes());
    out
}

/// A layered PSD with one record whose rect and optional mask are given. Each
/// channel is `(id, declared length, on-disk stream including the 2-byte
/// compression word)`. `composite: None` ends the file at the layer section,
/// which lets a channel declare more bytes than the file holds without the
/// section check firing.
fn layered_psd(
    depth: u16,
    mode: u16,
    header_channels: u16,
    rect: [i32; 4],
    mask: Option<[i32; 4]>,
    channels: &[(i16, u32, &[u8])],
    composite: Option<&[u8]>,
) -> Vec<u8> {
    let mut info = Vec::new();
    info.extend_from_slice(&1i16.to_be_bytes());
    for value in rect {
        info.extend_from_slice(&value.to_be_bytes());
    }
    info.extend_from_slice(&(channels.len() as u16).to_be_bytes());
    for (id, declared, _) in channels {
        info.extend_from_slice(&id.to_be_bytes());
        info.extend_from_slice(&declared.to_be_bytes());
    }
    info.extend_from_slice(b"8BIM");
    info.extend_from_slice(b"norm");
    info.extend_from_slice(&[255, 0, 0, 0]);
    let mut extra = Vec::new();
    match mask {
        Some(rect) => {
            extra.extend_from_slice(&20u32.to_be_bytes());
            for value in rect {
                extra.extend_from_slice(&value.to_be_bytes());
            }
            extra.extend_from_slice(&[0, 0, 0, 0]);
        }
        None => extra.extend_from_slice(&0u32.to_be_bytes()),
    }
    extra.extend_from_slice(&0u32.to_be_bytes());
    extra.extend_from_slice(&[1, b'L', 0, 0]);
    info.extend_from_slice(&(extra.len() as u32).to_be_bytes());
    info.extend_from_slice(&extra);
    for (_, _, data) in channels {
        info.extend_from_slice(data);
    }
    while info.len() % 4 != 0 {
        info.push(0);
    }

    let mut out = header_bytes(1, header_channels, 1, 1, depth, mode);
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&((4 + info.len() + 4) as u32).to_be_bytes());
    out.extend_from_slice(&(info.len() as u32).to_be_bytes());
    out.extend_from_slice(&info);
    out.extend_from_slice(&0u32.to_be_bytes());
    if let Some(composite) = composite {
        out.extend_from_slice(&COMPRESSION_RAW.to_be_bytes());
        out.extend_from_slice(composite);
    }
    out
}

/// A flat PSD whose image-data section is `image` (compression word included).
fn flat_composite_psd(mode: u16, channels: u16, width: u32, height: u32, image: &[u8]) -> Vec<u8> {
    let mut out = header_bytes(1, channels, width, height, 8, mode);
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(image);
    out
}

/// A predicted (ZIP-with-prediction) channel stream for a single `width == 1`
/// row: the forward prediction of `samples`, zlib-wrapped under the prediction
/// compression word.
fn predicted_channel(depth: u16, samples: &[u8]) -> Vec<u8> {
    let mut data = samples.to_vec();
    apply_prediction(&mut data, 1, 1, depth);
    let mut stream = COMPRESSION_ZIP_PREDICTION.to_be_bytes().to_vec();
    stream.extend_from_slice(&zlib(&data));
    stream
}

fn sample_document() -> Document {
    let width = 8u32;
    let height = 8u32;
    let rgba = vec![128u8; (width * height * 4) as usize];
    let mut doc = Document::from_rgba("Base", width, height, &rgba);
    let base = doc.layers.get(0).expect("base layer").clone();
    let mut top = base.clone();
    top.name = "Top".to_string();
    for channel in &mut top.channels {
        channel.data.as_mut_slice().fill(200);
    }
    doc.layers = LayerStack::from_layers(vec![base, top]);
    doc
}

fn extra_channel_document() -> Document {
    let mut doc = sample_document();
    let plane = vec![7u8; 8 * 8];
    doc.channels = vec![Channel {
        id: 3,
        data: plane.clone().into(),
    }];
    let mut stream = COMPRESSION_RAW.to_be_bytes().to_vec();
    stream.extend_from_slice(&plane);
    doc.layers.get_mut(0).expect("layer").raw_channels = vec![RawChannel {
        id: -3,
        data: stream.into(),
    }];
    doc
}

fn encoded(doc: &Document, compression: Compression) -> Vec<u8> {
    let mut doc = doc.clone();
    doc.composite_compression = compression;
    doc.layer_compression = compression;
    write_psd(&doc).expect("encode")
}

#[test]
fn parallel_layer_decode_matches_serial() {
    let mut cases: Vec<(String, Vec<u8>)> = vec![
        (
            "rle".to_string(),
            encoded(&sample_document(), Compression::Rle),
        ),
        (
            "zip".to_string(),
            encoded(&sample_document(), Compression::Zip),
        ),
        (
            "zip-prediction".to_string(),
            encoded(&sample_document(), Compression::ZipPrediction),
        ),
        (
            "psb".to_string(),
            write_psb(&sample_document()).expect("psb encode"),
        ),
        (
            "extra-channels".to_string(),
            write_psd(&extra_channel_document()).expect("encode"),
        ),
        (
            "bitmap-layer".to_string(),
            layered_psd(
                1,
                MODE_BITMAP,
                1,
                [0, 0, 1, 1],
                None,
                &[(0, 3, &[0, 0, 0xAA])],
                Some(&[0xAA]),
            ),
        ),
    ];
    for name in [
        "two_layers.psd",
        "masked.psd",
        "group.psd",
        "rgb16_layered.psd",
        "rgb32_layered.psd",
        "bitmap.psd",
        "cmyk.psd",
        "lab.psd",
        "indexed.psd",
    ] {
        cases.push((name.to_string(), fixture(name)));
    }
    for (name, bytes) in cases {
        assert_parallel_matches_serial(&name, &bytes);
    }
}

#[test]
fn parallel_decode_reports_the_same_error() {
    let mut short_zip = vec![0, 2];
    short_zip.extend_from_slice(&zlib(&[]));
    let mut long_zip = vec![0, 2];
    long_zip.extend_from_slice(&zlib(&[1, 2]));
    let cases = [
        (
            "over-long channel length",
            layered_psd(
                8,
                MODE_RGB,
                3,
                [0, 0, 1, 1],
                None,
                &[(0, 100, &[0, 0, 7])],
                None,
            ),
        ),
        (
            "truncated RLE count table",
            layered_psd(8, MODE_RGB, 3, [0, 0, 1, 1], None, &[(0, 2, &[0, 1])], None),
        ),
        (
            "invalid RLE scanline",
            layered_psd(
                8,
                MODE_RGB,
                3,
                [0, 0, 1, 1],
                None,
                &[(0, 6, &[0, 1, 0, 2, 0x02, 0x41])],
                None,
            ),
        ),
        (
            "short ZIP payload",
            layered_psd(
                8,
                MODE_RGB,
                3,
                [0, 0, 1, 1],
                None,
                &[(0, short_zip.len() as u32, &short_zip)],
                None,
            ),
        ),
        (
            "long ZIP payload",
            layered_psd(
                8,
                MODE_RGB,
                3,
                [0, 0, 1, 1],
                None,
                &[(0, long_zip.len() as u32, &long_zip)],
                None,
            ),
        ),
    ];
    for (name, bytes) in cases {
        let parallel = read_psd(&bytes).expect_err(name);
        let serial_error = serial(|| read_psd(&bytes)).expect_err(name);
        assert_eq!(
            format!("{parallel:?}"),
            format!("{serial_error:?}"),
            "{name}"
        );
    }
}

#[test]
fn parallel_decode_reports_the_first_error_in_order() {
    // Channel 0 fails with Invalid (an RLE literal overrun); channel 1 declares
    // more bytes than remain (Truncated). The earlier channel's error wins.
    let bytes = layered_psd(
        8,
        MODE_RGB,
        3,
        [0, 0, 1, 1],
        None,
        &[(0, 6, &[0, 1, 0, 2, 0x02, 0x41]), (1, 100, &[0, 0, 9])],
        None,
    );
    let error = read_psd(&bytes).expect_err("both channels fail");
    assert!(
        matches!(error, PsdError::Invalid(_)),
        "earlier channel wins: {error:?}"
    );
    let serial_error = serial(|| read_psd(&bytes)).expect_err("both channels fail");
    assert_eq!(format!("{error:?}"), format!("{serial_error:?}"));
}

#[test]
fn merged_rle_reports_the_first_row_error() {
    // Row 0 is an RLE literal overrun (Invalid); row 1 declares 100 bytes but
    // only one is present (Truncated). The earlier row's error wins.
    let mut image = vec![0, 1];
    image.extend_from_slice(&2u16.to_be_bytes());
    image.extend_from_slice(&100u16.to_be_bytes());
    image.extend_from_slice(&[0x02, 0x41]);
    image.push(0x81);
    let bytes = flat_composite_psd(MODE_GRAYSCALE, 1, 1, 2, &image);
    let parallel = read_psd(&bytes).expect_err("both rows fail");
    let serial_error = serial(|| read_psd(&bytes)).expect_err("both rows fail");
    assert!(matches!(parallel, PsdError::Invalid(_)), "{parallel:?}");
    assert_eq!(format!("{parallel:?}"), format!("{serial_error:?}"));
}

#[test]
fn layer_channel_tasks_record_their_own_offsets() {
    let channels = [
        PlannedChannel {
            id: 0,
            len: 5,
            width: 1,
            height: 1,
        },
        PlannedChannel {
            id: 1,
            len: 9,
            width: 1,
            height: 1,
        },
        PlannedChannel {
            id: -3,
            len: 4,
            width: 1,
            height: 1,
        },
        PlannedChannel {
            id: 2,
            len: 7,
            width: 1,
            height: 1,
        },
    ];
    let mut budget = DecodeBudget::new(ImageBudget {
        max_dimension: 100,
        max_alloc_bytes: 1 << 20,
        max_document_bytes: 1 << 20,
    });
    let (tasks, total) = plan_channel_tasks(&channels, 3, 8, &mut budget).expect("plan");
    assert_eq!(
        tasks.iter().map(ChannelTask::offset).collect::<Vec<_>>(),
        vec![0, 5, 14, 18]
    );
    assert_eq!(total, 25);
    assert!(
        matches!(tasks[2], ChannelTask::Raw { .. }),
        "a depth-8 unmodeled channel is copied verbatim"
    );
}

#[test]
fn decode_channel_tasks_reads_each_channel_from_its_own_slice() {
    // Raw compression plus three bytes, then raw compression plus two bytes.
    let data = vec![0, 0, 2, 3, 4, 0, 0, 9, 9];
    let tasks = [
        ChannelTask::Decode {
            offset: 0,
            len: 5,
            width: 3,
            height: 1,
        },
        ChannelTask::Decode {
            offset: 5,
            len: 4,
            width: 2,
            height: 1,
        },
    ];
    let planes: Vec<Vec<u8>> = decode_channel_tasks(&data, 0, &tasks, false, 8)
        .into_iter()
        .map(|result| result.expect("decode").0)
        .collect();
    assert_eq!(planes, vec![vec![2, 3, 4], vec![9, 9]]);
}

#[test]
fn a_mask_channel_is_sized_by_the_mask_rect() {
    // The -2 channel is sized by the 2x2 mask rect, not the 16x4 layer rect: a
    // 4-byte raw plane must decode, where a layer-sized read would demand 64.
    let mask_stream = {
        let mut stream = COMPRESSION_RAW.to_be_bytes().to_vec();
        stream.extend_from_slice(&[7u8; 4]);
        stream
    };
    let color_stream = {
        let mut stream = COMPRESSION_RAW.to_be_bytes().to_vec();
        stream.extend_from_slice(&[9u8; 64]);
        stream
    };
    let bytes = layered_psd(
        8,
        MODE_RGB,
        3,
        [0, 0, 16, 4],
        Some([0, 0, 2, 2]),
        &[
            (-2, mask_stream.len() as u32, &mask_stream),
            (0, color_stream.len() as u32, &color_stream),
        ],
        None,
    );
    let doc = read_psd(&bytes).expect("mask-sized channel reads");
    let layer = &doc.layers[0];
    let mask = layer.mask.as_ref().expect("mask");
    assert_eq!(mask.rect.width(), 2);
    assert_eq!(mask.rect.height(), 2);
    assert_eq!(mask.data.as_deref().map(<[u8]>::len), Some(4));
    let color = layer.channels.iter().find(|c| c.id == 0).expect("color");
    assert_eq!(color.data.len(), 64);
}

#[test]
fn parallel_rle_composite_matches_serial() {
    assert_parallel_matches_serial("rle8", &encoded(&sample_document(), Compression::Rle));
    for name in ["bitmap.psd", "rgb16.psd", "rgb32.psd"] {
        let mut doc = read_psd(&fixture(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        doc.composite_compression = Compression::Rle;
        let bytes = write_psd(&doc).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_parallel_matches_serial(name, &bytes);
    }
}

#[test]
fn merged_zip_composite_matches_serial() {
    for compression in [Compression::Zip, Compression::ZipPrediction] {
        assert_parallel_matches_serial("zip", &encoded(&sample_document(), compression));
    }
    let mut doc = read_psd(&fixture("rgb32.psd")).expect("rgb32");
    doc.composite_compression = Compression::ZipPrediction;
    let bytes = write_psd(&doc).expect("encode");
    assert_parallel_matches_serial("zip-prediction-32", &bytes);
}

#[test]
fn parallel_prediction_matches_serial() {
    for depth in [8u16, 16, 32] {
        let width = 7usize;
        let rows = 5usize;
        let row = width * depth as usize / 8;
        for extra in [0usize, 3] {
            let data: Vec<u8> = (0..row * rows + extra)
                .map(|i| (i * 37 + 11) as u8)
                .collect();
            let mut parallel = data.clone();
            undo_prediction(&mut parallel, width, rows, depth);
            let mut serial_data = data.clone();
            serial(|| undo_prediction(&mut serial_data, width, rows, depth));
            assert_eq!(parallel, serial_data, "depth {depth} extra {extra}");
        }
    }
    for (depth, samples) in [
        (16u16, 0x1234u16.to_be_bytes().to_vec()),
        (32u16, 0.5f32.to_be_bytes().to_vec()),
    ] {
        let stream = predicted_channel(depth, &samples);
        let bytes = layered_psd(
            depth,
            MODE_RGB,
            3,
            [0, 0, 1, 1],
            None,
            &[(0, stream.len() as u32, &stream)],
            None,
        );
        assert_parallel_matches_serial(&format!("predicted-layer-{depth}"), &bytes);
    }
}

fn flat_document(width: u32, height: u32) -> Document {
    // Varied bytes so the RLE encoder emits literals, not one long run: a
    // uniform fill collapses to a few PackBits repeat runs and measures the
    // allocator rather than the decoder.
    let rgba: Vec<u8> = (0..width as usize * height as usize * 4)
        .map(|i| (i * 37 + 11) as u8)
        .collect();
    let mut doc = Document::from_rgba("Base", width, height, &rgba);
    doc.layers = LayerStack::default();
    doc
}

fn layered_document(width: u32, height: u32, layers: usize, side: u32) -> Document {
    let mut doc = flat_document(width, height);
    let mut stack = Vec::with_capacity(layers);
    for i in 0..layers {
        let rgba: Vec<u8> = (0..side as usize * side as usize * 4)
            .map(|p| (p * 37 + i * 11) as u8)
            .collect();
        stack.push(
            Document::from_rgba(&format!("Layer {i}"), side, side, &rgba)
                .layers
                .get(0)
                .expect("layer")
                .clone(),
        );
    }
    doc.layers = LayerStack::from_layers(stack);
    doc
}

/// Times the decode stages the change touches. Run manually:
/// `cargo test -p pictura-codec --profile release -- profile_psd_decode_stages --ignored --nocapture`.
#[test]
#[ignore = "profiling: run with --ignored --nocapture to time the decode stages"]
fn profile_psd_decode_stages() {
    // Each case is encoded, decoded, and dropped before the next, so the peak
    // stays at one fixture instead of the sum of them.
    profile_case("small 8x8 RLE", sample_document(), Compression::Rle);
    let flat_rle = encoded(&flat_document(8000, 6000), Compression::Rle);
    profile_case_bytes("flat 8000x6000 RLE", &flat_rle);
    profile_isolated_rle("flat 8000x6000 RLE", &flat_rle, 4, 8000, 6000);
    drop(flat_rle);
    profile_case(
        "flat 8000x6000 RAW",
        flat_document(8000, 6000),
        Compression::Raw,
    );
    profile_case(
        "flat 8000x6000 ZIP",
        flat_document(8000, 6000),
        Compression::Zip,
    );
    profile_case(
        "flat 8000x6000 ZIP-prediction",
        flat_document(8000, 6000),
        Compression::ZipPrediction,
    );
    profile_isolated_prediction(8000, 4 * 6000, 8);
    profile_case(
        "30 layers 1024^2 + 8000x6000 RAW",
        layered_document(8000, 6000, 30, 1024),
        Compression::Raw,
    );
    profile_case(
        "30 layers 1024^2 + 8000x6000 RLE",
        layered_document(8000, 6000, 30, 1024),
        Compression::Rle,
    );
    profile_case(
        "30 layers 1024^2 + 8000x6000 ZIP",
        layered_document(8000, 6000, 30, 1024),
        Compression::Zip,
    );
}

fn profile_case(name: &str, doc: Document, compression: Compression) {
    let bytes = encoded(&doc, compression);
    drop(doc);
    profile_case_bytes(name, &bytes);
}

fn profile_case_bytes(name: &str, bytes: &[u8]) {
    use std::time::Instant;

    let start = Instant::now();
    let parallel_doc = read_psd(bytes).expect("parallel decode");
    let parallel = start.elapsed();
    let start = Instant::now();
    let serial_doc = serial(|| read_psd(bytes)).expect("serial decode");
    let serial = start.elapsed();
    assert_eq!(parallel_doc, serial_doc, "{name}");
    println!(
        "{name}: parallel {parallel:?}, one-thread {serial:?}, {:.1}x",
        serial.as_secs_f64() / parallel.as_secs_f64()
    );
}

/// The offset of an image-data section's scanline count table (the compression
/// word is already skipped).
fn image_data_offset(bytes: &[u8]) -> usize {
    let field = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let mut at = 26;
    at += 4 + field(at);
    at += 4 + field(at);
    at += 4 + field(at);
    at + 2
}

/// The merged RLE row decode alone, so the parallel win is not diluted by the
/// serial post-decode copies in `read_psd`.
fn profile_isolated_rle(
    name: &str,
    bytes: &[u8],
    channels: usize,
    row_bytes: usize,
    height: usize,
) {
    use std::time::Instant;

    let offset = image_data_offset(bytes);
    let start = Instant::now();
    let parallel_planes = {
        let mut r = Reader::new(&bytes[offset..]);
        super::decode::read_rle(&mut r, channels, row_bytes, height, false).expect("parallel rle")
    };
    let parallel = start.elapsed();
    let start = Instant::now();
    let serial_planes = serial(|| {
        let mut r = Reader::new(&bytes[offset..]);
        super::decode::read_rle(&mut r, channels, row_bytes, height, false).expect("serial rle")
    });
    let serial = start.elapsed();
    assert_eq!(parallel_planes, serial_planes, "{name}");
    println!(
        "{name} (RLE rows only): parallel {parallel:?}, one-thread {serial:?}, {:.1}x",
        serial.as_secs_f64() / parallel.as_secs_f64()
    );
}

/// The ZIP-with-prediction inversion alone over a 24 MP, four-plane buffer.
fn profile_isolated_prediction(width: usize, rows: usize, depth: u16) {
    use std::time::Instant;

    let data: Vec<u8> = (0..width * rows * depth as usize / 8)
        .map(|i| (i * 37 + 11) as u8)
        .collect();
    let start = Instant::now();
    let mut parallel_planes = data.clone();
    undo_prediction(&mut parallel_planes, width, rows, depth);
    let parallel = start.elapsed();
    let start = Instant::now();
    let mut serial_planes = data.clone();
    serial(|| undo_prediction(&mut serial_planes, width, rows, depth));
    let serial = start.elapsed();
    assert_eq!(parallel_planes, serial_planes);
    println!(
        "prediction rows only: parallel {parallel:?}, one-thread {serial:?}, {:.1}x",
        serial.as_secs_f64() / parallel.as_secs_f64()
    );
}
