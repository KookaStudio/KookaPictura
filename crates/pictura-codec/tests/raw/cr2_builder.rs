//! Synthetic Canon CR2s for the vendor reader tests: the CR2 header, IFD0
//! with an Exif IFD and a Canon maker note, and the raw IFD holding one
//! sliced lossless JPEG strip.

use super::builder::{write_ifd, IfdSpec, Plane, Val};
use super::ljpeg_encode;

pub struct Cr2 {
    /// The whole sensor plane, masked border included.
    pub plane: Plane,
    pub precision: u8,
    pub components: usize,
    /// Tag `0xC640`: `[full slices, slice width, last slice width]`.
    pub slices: Option<[u16; 3]>,
    /// Inclusive `[left, top, right, bottom]` of the active area.
    pub active: Option<[u16; 4]>,
    pub compression: u16,
    /// Sensor rows per JPEG frame row; above 1 the raw IFD carries
    /// `ImageWidth` and `ImageLength` (the EOS 5DS layout).
    pub rows_per_frame_row: usize,
    pub ifd0_tags: Vec<(u16, Val)>,
    /// The maker note's colour data, tag `0x4001`.
    pub color_data: Option<Vec<u16>>,
}

impl Cr2 {
    pub fn new(plane: Plane, precision: u8) -> Self {
        Self {
            plane,
            precision,
            components: 2,
            slices: None,
            active: None,
            compression: 6,
            rows_per_frame_row: 1,
            ifd0_tags: Vec::new(),
            color_data: None,
        }
    }
}

/// The plane in the order the JPEG stream carries it: slice by slice, each
/// top to bottom.
fn sliced(plane: &Plane, slices: Option<[u16; 3]>) -> Vec<u16> {
    let Some([n, w, last]) = slices else {
        return plane.samples.clone();
    };
    let mut widths = vec![w as usize; n as usize];
    widths.push(last as usize);
    let mut out = Vec::with_capacity(plane.samples.len());
    let mut x0 = 0;
    for sw in widths {
        for y in 0..plane.height {
            let row = &plane.samples[y * plane.width..][..plane.width];
            out.extend(row.iter().skip(x0).take(sw));
        }
        x0 += sw;
    }
    out
}

pub fn cr2(spec: &Cr2) -> Vec<u8> {
    let plane = &spec.plane;
    let mut out = b"II\x2a\x00\0\0\0\0CR\x02\x00\0\0\0\0".to_vec();

    let mut note_tags = Vec::new();
    if let Some([left, top, right, bottom]) = spec.active {
        let mut info = vec![0u16; 17];
        info[0] = 34;
        info[1] = plane.width as u16;
        info[2] = plane.height as u16;
        info[5..9].copy_from_slice(&[left, top, right, bottom]);
        note_tags.push((0x00E0, Val::Short(info)));
    }
    if let Some(data) = &spec.color_data {
        note_tags.push((0x4001, Val::Short(data.clone())));
    }
    let mut exif_tags = Vec::new();
    if !note_tags.is_empty() {
        let count = (2 + 12 * note_tags.len() + 4) as u32;
        let note = IfdSpec {
            tags: note_tags,
            ..IfdSpec::default()
        };
        let at = write_ifd(&mut out, &note, false, false) as u32;
        exif_tags.push((0x927C, Val::At { typ: 7, count, at }));
    }
    let exif = IfdSpec {
        tags: exif_tags,
        ..IfdSpec::default()
    };
    let exif_at = write_ifd(&mut out, &exif, false, false) as u32;
    let mut ifd0_tags = vec![(34665, Val::Long(vec![exif_at]))];
    ifd0_tags.extend(spec.ifd0_tags.iter().cloned());
    let ifd0 = IfdSpec {
        tags: ifd0_tags,
        ..IfdSpec::default()
    };
    let ifd0_at = write_ifd(&mut out, &ifd0, false, false) as u32;

    let stream = sliced(plane, spec.slices);
    let jpeg = ljpeg_encode::encode(
        &stream,
        plane.width * spec.rows_per_frame_row / spec.components,
        plane.height / spec.rows_per_frame_row,
        spec.components,
        spec.precision,
        1,
        0,
    );
    let mut raw_tags = vec![(259, Val::Short(vec![spec.compression]))];
    if spec.rows_per_frame_row > 1 {
        raw_tags.push((256, Val::Short(vec![plane.width as u16])));
        raw_tags.push((257, Val::Short(vec![plane.height as u16])));
    }
    if let Some(s) = spec.slices {
        raw_tags.push((0xC640, Val::Short(s.to_vec())));
    }
    let raw = IfdSpec {
        tags: raw_tags,
        chunks: Some((273, 279, vec![jpeg])),
        subs: Vec::new(),
    };
    let raw_at = write_ifd(&mut out, &raw, false, false) as u32;
    out[4..8].copy_from_slice(&ifd0_at.to_le_bytes());
    out[12..16].copy_from_slice(&raw_at.to_le_bytes());
    out
}

/// Marks the first frame component as 2×1 subsampled (a small raw).
pub fn subsample(bytes: &mut [u8]) {
    let sof = bytes
        .windows(2)
        .position(|w| w == [0xFF, 0xC3])
        .expect("SOF3");
    bytes[sof + 2 + 2 + 6 + 1] = 0x21;
}
