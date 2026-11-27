//! Olympus-compressed ORF data (`codec/vendor-raw-containers`, #1535)
//! against streams written by `tests/camera_raw/raw/orf_encode.rs`, and,
//! opt-in, LibRaw's decode of real camera files.

use pictura_codec::{read_raw, DngImage, ImageBudget, PsdError};

#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[allow(dead_code)]
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;
#[allow(dead_code)]
#[path = "raw/orf_builder.rs"]
mod orf_builder;
#[path = "raw/orf_encode.rs"]
mod orf_encode;

use builder::noise_plane;
use orf_builder::{orf, Data, Note, Orf};
use orf_encode::{encode, CAMERA};

const WIDTH: usize = 40;
const HEIGHT: usize = 24;

fn read(bytes: &[u8]) -> Result<DngImage, PsdError> {
    read_raw(bytes, &ImageBudget::default())
}

/// Noise with full-scale spikes over the left half, a smooth ramp over the
/// right, so codes both large and small, escapes and runs, all occur, yet
/// the stream stays shorter than the packed size that names it compressed.
fn scene(bits: u16, seed: u64) -> Vec<u16> {
    let max = (1u16 << bits) - 1;
    let noise = noise_plane(WIDTH, HEIGHT, 255, seed).samples;
    (0..WIDTH * HEIGHT)
        .map(|i| {
            let (x, y) = (i % WIDTH, i / WIDTH);
            if x < WIDTH / 2 && i % 13 == 0 {
                max
            } else if x < WIDTH / 2 {
                max / 4 + noise[i]
            } else {
                (u32::from(max) * (x + 2 * y) as u32 / (WIDTH + 2 * HEIGHT) as u32) as u16
            }
        })
        .collect()
}

/// A compressed ORF of `samples` coded with `params`.
fn compressed(samples: Vec<u16>, bits: u16, params: [u32; 14]) -> Orf {
    let stream = encode(&samples, WIDTH, HEIGHT, &params);
    assert!(
        2 * stream.len() < 3 * WIDTH * HEIGHT,
        "{} bytes for {params:?} at {bits} bits",
        stream.len()
    );
    let mut o = Orf::new(WIDTH, HEIGHT, samples, Data::Strips(vec![stream]));
    o.bits = bits;
    o.params = Some(params);
    o
}

/// `o`'s samples normalized by its black, scaled by `scale`, and `white`.
fn normalized(o: &Orf, white: f64, scale: f64) -> Vec<u16> {
    o.samples
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let black = o.black[(i / o.width % 2) * 2 + i % o.width % 2] as f64 * scale;
            ((v as f64 - black) * 65535.0 / (white - black))
                .round()
                .clamp(0.0, 65535.0) as u16
        })
        .collect()
}

#[test]
fn compressed_orfs_decode_exactly() {
    let mut params = CAMERA;
    for fine_bits in [2, 1, 0] {
        params[3] = fine_bits;
        let o = compressed(scene(12, 31 + u64::from(fine_bits)), 12, params);
        let image = read(&orf(&o)).unwrap();
        assert_eq!(
            image.samples,
            normalized(&o, 4095.0, 1.0),
            "fine bits {fine_bits}"
        );
    }

    // 14-bit levels are recorded at 12 bits.
    let mut o = compressed(scene(14, 41), 14, CAMERA);
    o.white = Some(4000);
    let image = read(&orf(&o)).unwrap();
    assert_eq!(image.samples, normalized(&o, 16000.0, 4.0), "14 bits");

    // Two raw low bits under each prediction, and the fine context off.
    let mut params = CAMERA;
    params[0] = 2;
    params[4] = 15;
    let o = compressed(scene(14, 41), 14, params);
    let image = read(&orf(&o)).unwrap();
    assert_eq!(image.samples, normalized(&o, 16383.0, 4.0), "low bits");
}

#[test]
fn every_note_with_image_processing_carries_the_coding() {
    for note in [Note::Olympus, Note::OmSystem, Note::OlympValue] {
        let mut o = compressed(scene(12, 51), 12, CAMERA);
        o.note = note;
        let image = read(&orf(&o)).unwrap();
        assert_eq!(image.samples, normalized(&o, 4095.0, 1.0), "{note:?}");
    }
}

#[test]
fn compressed_orfs_need_their_coding() {
    let mut o = compressed(scene(12, 61), 12, CAMERA);
    o.params = None;
    match read(&orf(&o)) {
        Err(PsdError::Unsupported(why)) => assert!(why.contains("coding parameters"), "{why}"),
        other => panic!("{:?}", other.err()),
    }

    for (i, v) in [
        (0, 17),
        (1, 31),
        (3, 17),
        (4, 16),
        (5, 31),
        (11, 16),
        (13, 0),
    ] {
        let mut o = compressed(scene(12, 61), 12, CAMERA);
        o.params.as_mut().unwrap()[i] = v;
        assert!(
            matches!(read(&orf(&o)), Err(PsdError::Unsupported(_))),
            "parameter {i} = {v}"
        );
    }

    let mut o = compressed(scene(12, 61), 12, CAMERA);
    o.bits = 16;
    assert!(
        matches!(read(&orf(&o)), Err(PsdError::Unsupported(_))),
        "16 bits"
    );

    let mut header = compressed(scene(12, 61), 12, CAMERA);
    if let Data::Strips(strips) = &mut header.data {
        strips[0][4] = 2;
    }
    assert!(
        matches!(read(&orf(&header)), Err(PsdError::Invalid(_))),
        "header"
    );
}

#[test]
fn hostile_compressed_orfs_fail_cleanly() {
    let o = compressed(scene(12, 71), 12, CAMERA);
    let bytes = orf(&o);
    for len in 0..bytes.len() {
        assert!(read(&bytes[..len]).is_err(), "cut at {len}");
    }
    let Data::Strips(strips) = &o.data else {
        unreachable!()
    };
    // The stream follows the 8-byte header.
    for at in (8..8 + strips[0].len()).step_by(3) {
        for flip in [0x01, 0x80, 0xFF] {
            let mut broken = bytes.clone();
            broken[at] ^= flip;
            // Any answer but a panic.
            let _ = read(&broken);
        }
    }

    // A strip far too short for its plane.
    let mut short = o.clone();
    short.data = Data::Strips(vec![[&strips[0][..7], &[0; 8][..]].concat()]);
    assert!(read(&orf(&short)).is_err(), "short strip");
}

/// LibRaw's raw plane of a compressed ORF, with the maker note's black,
/// white and valid bits, through rawpy; `None` when the ORF has no coding
/// parameters (it is not compressed).
const LIBRAW_DUMP: &str = r#"
import rawpy, struct, sys
b = open(sys.argv[1], 'rb').read()
o = '>' if b[:2] == b'MM' else '<'
def ifd(buf, at, base):
    out = {}
    for i in range(struct.unpack(o + 'H', buf[at:at + 2])[0]):
        e = buf[at + 2 + 12 * i:at + 14 + 12 * i]
        tag, typ, n = struct.unpack(o + 'HHI', e[:8])
        size = {3: 2, 4: 4, 8: 2, 13: 4}.get(typ, 1)
        off = struct.unpack(o + 'I', e[8:])[0]
        if size * n > 4 and base + off + size * n > len(buf):
            continue
        raw = e[8:12] if size * n <= 4 else buf[base + off:][:size * n]
        fmt = {3: 'H', 4: 'I', 8: 'h', 13: 'I'}.get(typ, 'B')
        out[tag] = (list(struct.unpack(o + fmt * n, raw[:size * n])), base + off)
    return out
ifd0 = ifd(b, struct.unpack(o + 'I', b[4:8])[0], 0)
exif = ifd(b, ifd0[34665][0][0], 0)
note = exif[0x927C][1]
if b[note:note + 8] == b'OLYMPUS\0':
    base, at = note, note + 12
elif b[note:note + 10] == b'OM SYSTEM\0':
    base, at = note, note + 16
else:
    base, at = 0, note + 8
main = ifd(b, at, base)
ip = main.get(0x2040)
proc = ifd(b, base + ip[0][0], base) if ip else {}
out = sys.stdout.buffer
if 0x0653 not in proc:
    out.write(struct.pack('<I', 0))
    sys.exit()
black = proc[0x0600][0][:4]
bits = proc[0x0611][0][0]
white = proc.get(0x0805, ([0],))[0][0]
with rawpy.imread(sys.argv[1]) as r:
    a = r.raw_image
    out.write(struct.pack('<IIIIi', 1, a.shape[1], a.shape[0], bits, white))
    out.write(struct.pack('<4I', *black))
    out.write(a.astype('<u2').tobytes())
"#;

struct LibRaw {
    width: usize,
    height: usize,
    bits: u32,
    white: i32,
    black: [u32; 4],
    plane: Vec<u16>,
}

fn libraw(path: &std::path::Path) -> Option<LibRaw> {
    let out = std::process::Command::new("python3")
        .args(["-c", LIBRAW_DUMP])
        .arg(path)
        .output()
        .expect("python3");
    assert!(
        out.status.success(),
        "{}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let b = out.stdout;
    let word = |i: usize| u32::from_le_bytes(b[4 * i..4 * i + 4].try_into().unwrap());
    if word(0) == 0 {
        return None;
    }
    let plane = b[4 * 9..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|w| u16::from_le_bytes(*w))
        .collect();
    Some(LibRaw {
        width: word(1) as usize,
        height: word(2) as usize,
        bits: word(3),
        white: word(4) as i32,
        black: [5, 6, 7, 8].map(word),
        plane,
    })
}

/// What the reader owes for LibRaw's plane: the levels recorded at 12 bits,
/// scaled to the samples' precision.
fn expected(want: &LibRaw) -> Vec<u16> {
    let scale = f64::from(1u32 << (want.bits - 12));
    let black = want.black.map(|b| f64::from(b) * scale);
    let top = black.iter().fold(0.0f64, |a, &b| a.max(b));
    let white = Some(f64::from(want.white) * scale)
        .filter(|&w| w > top && w <= 4095.0 * scale)
        .unwrap_or(((1u32 << want.bits) - 1) as f64);
    want.plane
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let black = black[(i / want.width % 2) * 2 + i % want.width % 2];
            ((v as f64 - black) * 65535.0 / (white - black))
                .round()
                .clamp(0.0, 65535.0) as u16
        })
        .collect()
}

#[test]
fn compressed_orfs_match_libraw_when_present() {
    let Some(dir) = std::env::var_os("PICTURA_RAW_CORPUS") else {
        eprintln!("SKIP compressed orf corpus: PICTURA_RAW_CORPUS is not set");
        return;
    };
    let rawpy = std::process::Command::new("python3")
        .args(["-c", "import rawpy"])
        .status()
        .is_ok_and(|s| s.success());
    if !rawpy {
        eprintln!("SKIP compressed orf corpus: python3 with rawpy is not available");
        return;
    }
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .expect("corpus directory")
        .map(|e| e.expect("corpus entry").path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("orf")))
        .collect();
    paths.sort();
    let mut compared = 0;
    for path in paths {
        let Some(want) = libraw(&path) else {
            continue;
        };
        let name = path.display();
        let image = read(&std::fs::read(&path).unwrap()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            (image.width, image.height),
            (want.width, want.height),
            "{name}"
        );
        let differ = image
            .samples
            .iter()
            .zip(&expected(&want))
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(differ, 0, "{name}: samples differing from LibRaw");
        eprintln!(
            "{name}: {} x {} at {} bits equal to LibRaw",
            want.width, want.height, want.bits
        );
        compared += 1;
    }
    eprintln!("{compared} compressed ORFs compared");
}
