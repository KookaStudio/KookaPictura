//! Fujifilm-compressed RAF data (`codec/vendor-raw-containers`, #1620)
//! against data written by `tests/camera_raw/raw/raf_encode.rs`, and, opt-in,
//! LibRaw's decode of real camera files.

use pictura_codec::{read_raw, DngImage, ImageBudget, PsdError};

#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[allow(dead_code)]
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;
#[allow(dead_code)]
#[path = "raw/raf_builder.rs"]
mod raf_builder;
#[allow(dead_code)]
#[path = "raw/raf_encode.rs"]
mod raf_encode;

use builder::noise_plane;
use raf_builder::{raf, Coding, Raf};
use raf_encode::{encode, Plane, COMPRESSED_XTRANS, RGGB};

/// Two strips, the second 744 photosites wide.
const WIDTH: usize = 1512;

fn read(bytes: &[u8]) -> Result<DngImage, PsdError> {
    read_raw(bytes, &ImageBudget::default())
}

type QBases = dyn Fn(usize, usize) -> u8;

/// A compressed RAF of noise and the plane a decoder must produce from it.
fn compressed(cfa: &[u8], bits: u32, height: usize, q_bases: Option<&QBases>) -> (Raf, Vec<u16>) {
    let max = ((1u32 << bits) - 1) as u16;
    let samples = noise_plane(WIDTH, height, max, u64::from(bits) * 7 + cfa.len() as u64).samples;
    let plane = Plane {
        width: WIDTH,
        height,
        samples: &samples,
        cfa,
        bits,
    };
    let (data, decoded) = encode(&plane, q_bases);
    let mut r = Raf::new(WIDTH, height, samples, Coding::Bytes(data, None));
    if cfa.len() == 4 {
        r = r.bayer();
    } else {
        r.xtrans_record = Some(cfa.iter().rev().copied().collect());
    }
    r.bits = bits;
    (r, decoded)
}

/// `plane` normalized by `r`'s black at each photosite's position.
fn normalized(r: &Raf, plane: &[u16]) -> Vec<u16> {
    normalize(plane, r.width, &r.black, r.bits)
}

fn normalize(plane: &[u16], width: usize, black: &[u32], bits: u32) -> Vec<u16> {
    let side = match black.len() {
        1 => 1,
        4 => 2,
        _ => 6,
    };
    let white = ((1u32 << bits) - 1) as f64;
    plane
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let (x, y) = (i % width, i / width);
            let black = black[(y % side) * side + x % side] as f64;
            ((v as f64 - black) * 65535.0 / (white - black))
                .round()
                .clamp(0.0, 65535.0) as u16
        })
        .collect()
}

#[test]
fn lossless_compressed_rafs_decode_exactly() {
    for (cfa, bits) in [
        (&COMPRESSED_XTRANS[..], 14),
        (&COMPRESSED_XTRANS[..], 12),
        (&RGGB[..], 14),
        (&RGGB[..], 16),
    ] {
        let (r, decoded) = compressed(cfa, bits, 18, None);
        assert_eq!(decoded, r.samples, "lossless coding of {bits} bits");
        let image = read(&raf(&r)).expect("lossless compressed RAF");
        assert_eq!((image.width, image.height), (WIDTH, 18));
        assert_eq!(image.cfa.colors, cfa);
        assert_eq!(
            image.samples,
            normalized(&r, &decoded),
            "{} CFA, {bits} bits",
            cfa.len()
        );
    }
}

#[test]
fn lossy_compressed_rafs_decode_as_coded() {
    let bases: &QBases = &|block, line| [0, 1, 2, 4, 9, 1][(2 * block + line) % 6];
    for (cfa, bits) in [
        (&COMPRESSED_XTRANS[..], 14),
        (&RGGB[..], 14),
        (&RGGB[..], 16),
    ] {
        let (r, decoded) = compressed(cfa, bits, 36, Some(bases));
        let worst = decoded
            .iter()
            .zip(&r.samples)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        assert!(
            (1..=9).contains(&worst),
            "lossy error {worst} for {bits} bits"
        );
        let image = read(&raf(&r)).expect("lossy compressed RAF");
        assert_eq!(
            image.samples,
            normalized(&r, &decoded),
            "{} CFA, {bits} bits",
            cfa.len()
        );
    }
}

/// The RAF with its compressed header's byte `at` set to `value`.
fn with_header_byte(r: &Raf, at: usize, value: u8) -> Vec<u8> {
    let Coding::Bytes(mut data, words) = r.coding.clone() else {
        unreachable!()
    };
    data[at] = value;
    let mut r = r.clone();
    r.coding = Coding::Bytes(data, words);
    raf(&r)
}

#[test]
fn compressed_headers_are_checked() {
    let (r, _) = compressed(&COMPRESSED_XTRANS, 14, 18, None);
    read(&raf(&r)).expect("valid header");
    // Lossless flag 2, raw type 3, 10 bits, height 17, width 1511, block
    // width 0x200, three blocks, four lines.
    for (at, value) in [
        (2, 2),
        (3, 3),
        (4, 10),
        (6, 17),
        (10, 0xE7),
        (11, 0x02),
        (13, 3),
        (15, 4),
    ] {
        match read(&with_header_byte(&r, at, value)) {
            Err(PsdError::Invalid(why)) => assert!(why.contains("compressed"), "{why}"),
            other => panic!("byte {at} = {value}: {:?}", other.map(|i| i.width)),
        }
    }
    // An X-Trans header in a Bayer-layout file.
    let mut bayer = r.clone().bayer();
    bayer.coding = r.coding.clone();
    assert!(matches!(read(&raf(&bayer)), Err(PsdError::Invalid(_))));
    // Short data without the header.
    let short = Raf::new(
        WIDTH,
        18,
        r.samples.clone(),
        Coding::Bytes(vec![0x5A; WIDTH * 18], None),
    );
    match read(&raf(&short)) {
        Err(PsdError::Unsupported(why)) => assert!(why.contains("compressed-data header")),
        other => panic!("headerless: {:?}", other.map(|i| i.width)),
    }
}

#[test]
fn hostile_compressed_rafs_fail_cleanly() {
    let bases: &QBases = &|block, line| (block + line) as u8 % 4;
    for q_bases in [None, Some(bases)] {
        let (r, _) = compressed(&COMPRESSED_XTRANS, 14, 12, q_bases);
        let bytes = raf(&r);
        for len in (0..bytes.len())
            .step_by(97)
            .chain(bytes.len() - 64..bytes.len())
        {
            assert!(read(&bytes[..len]).is_err(), "cut at {len}");
        }
        let Coding::Bytes(data, _) = &r.coding else {
            unreachable!()
        };
        let data_at = bytes.len() - data.len();
        for at in (data_at..bytes.len()).step_by(41) {
            for flip in [0x01, 0x80, 0xFF] {
                let mut broken = bytes.clone();
                broken[at] ^= flip;
                // Any answer but a panic.
                let _ = read(&broken);
            }
        }
        // A strip size past the section.
        let mut long = bytes.clone();
        long[data_at + 16..data_at + 20].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(read(&long).is_err());
    }
}

/// LibRaw's raw plane of a compressed RAF, with the black levels and the
/// precision the RAF records, through rawpy; `None` when the file is not
/// compressed (a word section never is).
const LIBRAW_DUMP: &str = r#"
import rawpy, struct, sys
b = open(sys.argv[1], 'rb').read()
so, sl = struct.unpack('>II', b[0x64:0x6c])
s = b[so:so + sl]
out = sys.stdout.buffer
if s[:2] not in (b'II', b'MM'):
    out.write(struct.pack('<I', 0))
    sys.exit()
o = '<' if s[:2] == b'II' else '>'
def ifd(at):
    out = {}
    for i in range(struct.unpack(o + 'H', s[at:at + 2])[0]):
        e = s[at + 2 + 12 * i:at + 14 + 12 * i]
        tag, typ, n = struct.unpack(o + 'HHI', e[:8])
        size = {3: 2, 4: 4}.get(typ, 1)
        raw = e[8:12] if size * n <= 4 else s[struct.unpack(o + 'I', e[8:])[0]:][:size * n]
        out[tag] = list(struct.unpack(o + {2: 'H', 4: 'I'}.get(size, 'B') * n, raw[:size * n]))
    return out
raw = ifd(ifd(struct.unpack(o + 'I', s[4:8])[0])[0xF000][0])
data = s[raw[0xF007][0]:]
if data[:2] != b'IS':
    out.write(struct.pack('<I', 0))
    sys.exit()
black = raw.get(0xF00A, [0])
with rawpy.imread(sys.argv[1]) as r:
    a = r.raw_image
    out.write(struct.pack('<IIIII', 1, a.shape[1], a.shape[0], data[4], len(black)))
    out.write(struct.pack('<%dI' % len(black), *black))
    out.write(a.astype('<u2').tobytes())
"#;

struct LibRaw {
    width: usize,
    height: usize,
    bits: u32,
    black: Vec<u32>,
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
    let (width, height, bits, n) = (
        word(1) as usize,
        word(2) as usize,
        word(3),
        word(4) as usize,
    );
    let black = (5..5 + n).map(word).collect();
    let plane = b[4 * (5 + n)..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|w| u16::from_le_bytes(*w))
        .collect();
    Some(LibRaw {
        width,
        height,
        bits,
        black,
        plane,
    })
}

#[test]
fn compressed_rafs_match_libraw_when_present() {
    let Some(dir) = std::env::var_os("PICTURA_RAW_CORPUS") else {
        eprintln!("SKIP compressed raf corpus: PICTURA_RAW_CORPUS is not set");
        return;
    };
    let rawpy = std::process::Command::new("python3")
        .args(["-c", "import rawpy"])
        .status()
        .is_ok_and(|s| s.success());
    if !rawpy {
        eprintln!("SKIP compressed raf corpus: python3 with rawpy is not available");
        return;
    }
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .expect("corpus directory")
        .map(|e| e.expect("corpus entry").path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("raf")))
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
        let expected = normalize(&want.plane, want.width, &want.black, want.bits);
        let differ = image
            .samples
            .iter()
            .zip(&expected)
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(differ, 0, "{name}: samples differing from LibRaw");
        eprintln!("{name}: {} x {} equal to LibRaw", want.width, want.height);
        compared += 1;
    }
    eprintln!("{compared} compressed RAFs compared");
}
