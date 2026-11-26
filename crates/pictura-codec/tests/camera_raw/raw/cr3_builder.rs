//! Synthetic Canon CR3s for the vendor reader tests: an ISO media file of
//! brand `crx ` with Canon's `moov/uuid` box (`CMT1` IFD0, `CMT3` maker
//! note), raw `CRAW` tracks and a `CTMD` metadata track, samples in `mdat`.

use super::builder::{write_tiff, IfdSpec, Val};

const CANON_UUID: [u8; 16] = [
    0x85, 0xc0, 0xb6, 0x87, 0x82, 0x0f, 0x11, 0xe0, 0x81, 0x11, 0xf4, 0xce, 0x46, 0x2b, 0x6a, 0x48,
];

pub enum Track {
    /// A raw track: its `CMP1` content and samples.
    Raw {
        cmp1: Vec<u8>,
        samples: Vec<Vec<u8>>,
    },
    /// A timed-metadata track of one sample.
    Ctmd(Vec<u8>),
}

#[derive(Default)]
pub struct Cr3 {
    pub cmt1: Option<Vec<u8>>,
    pub cmt3: Option<Vec<u8>>,
    pub tracks: Vec<Track>,
    /// Leaves the movie box out.
    pub no_movie: bool,
}

pub fn boxed(name: &[u8; 4], content: &[u8]) -> Vec<u8> {
    let mut b = ((content.len() + 8) as u32).to_be_bytes().to_vec();
    b.extend(name);
    b.extend(content);
    b
}

fn full(name: &[u8; 4], content: &[u8]) -> Vec<u8> {
    boxed(name, &[&[0u8; 4][..], content].concat())
}

/// IFD0 (`CMT1`) naming `make`, `model` and `orientation`.
pub fn cmt1(make: &'static str, model: &'static str, orientation: u16) -> Vec<u8> {
    write_tiff(
        &IfdSpec {
            tags: vec![
                (271, Val::Ascii(make)),
                (272, Val::Ascii(model)),
                (274, Val::Short(vec![orientation])),
            ],
            ..IfdSpec::default()
        },
        false,
        false,
    )
}

/// A maker note as a TIFF file whose IFD0 holds `tags`.
pub fn note(tags: Vec<(u16, Val)>) -> Vec<u8> {
    write_tiff(
        &IfdSpec {
            tags,
            ..IfdSpec::default()
        },
        false,
        false,
    )
}

/// The maker note's sensor information for a `w × h` frame with the
/// inclusive active area `[left, top, right, bottom]`.
pub fn sensor_info(w: usize, h: usize, area: [u16; 4]) -> (u16, Val) {
    let mut info = vec![0u16; 17];
    info[0] = 34;
    info[1] = w as u16;
    info[2] = h as u16;
    info[5..9].copy_from_slice(&area);
    (0x00E0, Val::Short(info))
}

/// A `CTMD` sample: a record of type 5, then one of type 8 whose block
/// `0x927C` is a maker note with colour data `color`.
pub fn ctmd(color: &[u16]) -> Vec<u8> {
    let mut out = record(5, &[0; 10]);
    let note = note(vec![(0x4001, Val::Short(color.to_vec()))]);
    let mut block = ((note.len() + 8) as u32).to_le_bytes().to_vec();
    block.extend(0x927Cu32.to_le_bytes());
    block.extend(note);
    out.extend(record(8, &block));
    out
}

/// A `CTMD` record: little-endian size, type, six reserved bytes, body.
pub fn record(kind: u16, body: &[u8]) -> Vec<u8> {
    let mut r = ((body.len() + 12) as u32).to_le_bytes().to_vec();
    r.extend(kind.to_le_bytes());
    r.extend([0; 6]);
    r.extend(body);
    r
}

fn trak(handler: &[u8; 4], entry: Vec<u8>, sizes: &[usize], offset: u64) -> Vec<u8> {
    let hdlr = full(b"hdlr", &[&[0u8; 4][..], handler, &[0; 13]].concat());
    let stsd = full(b"stsd", &[&1u32.to_be_bytes()[..], &entry].concat());
    let mut stsz = vec![0u8; 4];
    stsz.extend((sizes.len() as u32).to_be_bytes());
    for s in sizes {
        stsz.extend((*s as u32).to_be_bytes());
    }
    let stsz = full(b"stsz", &stsz);
    let co64 = full(
        b"co64",
        &[&1u32.to_be_bytes()[..], &offset.to_be_bytes()].concat(),
    );
    let stsc = full(
        b"stsc",
        &[
            &1u32.to_be_bytes()[..],
            &1u32.to_be_bytes(),
            &(sizes.len() as u32).to_be_bytes(),
            &1u32.to_be_bytes(),
        ]
        .concat(),
    );
    let stbl = boxed(b"stbl", &[stsd, stsz, stsc, co64].concat());
    let minf = boxed(b"minf", &stbl);
    let mdia = boxed(b"mdia", &[hdlr, minf].concat());
    boxed(b"trak", &[full(b"tkhd", &[0; 80]), mdia].concat())
}

fn craw_entry(cmp1: &[u8]) -> Vec<u8> {
    let mut head = vec![0u8; 82];
    let (w, h) = (
        u32::from_be_bytes(cmp1[8..12].try_into().unwrap()),
        u32::from_be_bytes(cmp1[12..16].try_into().unwrap()),
    );
    head[24..26].copy_from_slice(&(w as u16).to_be_bytes());
    head[26..28].copy_from_slice(&(h as u16).to_be_bytes());
    boxed(
        b"CRAW",
        &[head, boxed(b"CMP1", cmp1), boxed(b"free", &[0; 4])].concat(),
    )
}

fn movie(spec: &Cr3, mdat_at: u64) -> Vec<u8> {
    let mut canon = CANON_UUID.to_vec();
    canon.extend(boxed(b"CNCV", b"CanonCR3_001/00.10.00/00.00.00"));
    if let Some(c) = &spec.cmt1 {
        canon.extend(boxed(b"CMT1", c));
    }
    if let Some(c) = &spec.cmt3 {
        canon.extend(boxed(b"CMT3", c));
    }
    let mut moov = full(b"mvhd", &[0; 96]);
    moov.extend(boxed(b"uuid", &canon));
    let mut at = mdat_at;
    for t in &spec.tracks {
        let (handler, entry, sizes) = match t {
            Track::Raw { cmp1, samples } => (
                b"vide",
                craw_entry(cmp1),
                samples.iter().map(Vec::len).collect::<Vec<_>>(),
            ),
            Track::Ctmd(s) => (b"meta", boxed(b"CTMD", &[0; 8]), vec![s.len()]),
        };
        moov.extend(trak(handler, entry, &sizes, at));
        at += sizes.iter().sum::<usize>() as u64;
    }
    boxed(b"moov", &moov)
}

pub fn cr3(spec: &Cr3) -> Vec<u8> {
    let mut out = boxed(b"ftyp", b"crx \0\0\0\x01crx isom");
    let movie_len = if spec.no_movie {
        0
    } else {
        movie(spec, 0).len()
    };
    let mdat_at = (out.len() + movie_len + 8) as u64;
    if !spec.no_movie {
        out.extend(movie(spec, mdat_at));
    }
    let mut mdat = Vec::new();
    for t in &spec.tracks {
        match t {
            Track::Raw { samples, .. } => samples.iter().for_each(|s| mdat.extend(s)),
            Track::Ctmd(s) => mdat.extend(s),
        }
    }
    out.extend(boxed(b"mdat", &mdat));
    out
}
