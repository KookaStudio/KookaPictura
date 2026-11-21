//! The camera raw entry point (#399): names a container by its signature and
//! reads any supported one into the same normalized CFA image as `read_dng`.
//! Also the helpers the TIFF-based vendor containers share.

use super::camera_matrices::CAMERAS;
use super::levels::Area;
use super::tiff::{Ifd, Tiff};
use super::{cr2, read_dng, to_usize, DngColor, DngImage, Tags, DNG_VERSION};
use crate::error::PsdError;
use crate::probe::ImageBudget;

const EXIF_IFD: u16 = 34665;
const MAKER_NOTE: u16 = 0x927C;
const ILLUMINANT_A: u16 = 17;
const ILLUMINANT_D65: u16 = 21;

/// A camera raw container `read_raw` reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawFormat {
    Dng,
    Cr2,
}

/// Names the raw container `bytes` hold, from its signature and, for a DNG,
/// IFD0's `DNGVersion` tag.
pub fn sniff_raw(bytes: &[u8]) -> Option<RawFormat> {
    if cr2::is_cr2(bytes) {
        return Some(RawFormat::Cr2);
    }
    let (tiff, first) = Tiff::parse(bytes).ok()?;
    let ifd0 = tiff.ifd(first).ok()?;
    ifd0.get(DNG_VERSION).map(|_| RawFormat::Dng)
}

/// Whether `prefix`, the start of a file, is a supported raw container. Only
/// IFD0's entry table is read, so the prefix need not hold its values.
pub fn sniff_raw_prefix(prefix: &[u8]) -> bool {
    if cr2::is_cr2(prefix) {
        return true;
    }
    Tiff::parse(prefix).is_ok_and(|(tiff, first)| tiff.lists_tag(first, DNG_VERSION))
}

/// Reads any supported raw container's full-resolution CFA plane, normalized
/// and cropped to its active area, charging declared sizes to `budget` first.
pub fn read_raw(bytes: &[u8], budget: &ImageBudget) -> Result<DngImage, PsdError> {
    match sniff_raw(bytes) {
        Some(RawFormat::Dng) => read_dng(bytes, budget),
        Some(RawFormat::Cr2) => cr2::read_cr2(bytes, budget),
        None => Err(PsdError::Unsupported("camera raw format".into())),
    }
}

/// The maker note of IFD0's Exif IFD read as a bare IFD at its value, with
/// offsets relative to the file start (Canon's layout).
pub(super) fn maker_note_ifd(ifd0: &Tags) -> Result<Option<Ifd>, PsdError> {
    let Some(exif) = ifd0.uint(EXIF_IFD)? else {
        return Ok(None);
    };
    let exif = ifd0.tiff.ifd(exif)?;
    exif.get(MAKER_NOTE)
        .map(|note| ifd0.tiff.ifd(note.at as u64))
        .transpose()
}

/// The mean of the photosites in `mask` (raw-plane coordinates) for each
/// position of the 2 × 2 phase measured from `origin`, row-major; 0 where a
/// position has no photosite.
pub(super) fn masked_black(
    raw: &[u16],
    raw_width: usize,
    mask: Area,
    origin: (usize, usize),
) -> Vec<f64> {
    let mut sum = [0f64; 4];
    let mut count = [0u64; 4];
    for y in mask.top..mask.top + mask.height {
        let row = &raw[y * raw_width..][..raw_width];
        let py = (y as isize - origin.1 as isize).rem_euclid(2) as usize;
        for (x, &v) in row.iter().enumerate().skip(mask.left).take(mask.width) {
            let p = py * 2 + (x as isize - origin.0 as isize).rem_euclid(2) as usize;
            sum[p] += v as f64;
            count[p] += 1;
        }
    }
    (0..4)
        .map(|p| {
            if count[p] == 0 {
                0.0
            } else {
                sum[p] / count[p] as f64
            }
        })
        .collect()
}

/// `value` as a plane coordinate.
pub(super) fn coord(value: f64) -> Result<usize, PsdError> {
    if value < 0.0 || value.fract() != 0.0 {
        return Err(PsdError::Invalid(format!("raw coordinate {value}")));
    }
    to_usize(value as u64)
}

/// A make reduced for matching: its first word, upper-cased (`NIKON
/// CORPORATION` → `NIKON`). Kept in step with `scripts/gen-camera-matrices.py`.
fn make_key(make: &str) -> String {
    make.split_whitespace().next().unwrap_or("").to_uppercase()
}

/// A model reduced for matching, so a camera's market names meet one key:
/// upper-cased, `-` as a space, without a leading make word or the words
/// `EOS` and `DIGITAL` (`Canon EOS 400D DIGITAL` → `400D`). Kept in step with
/// `scripts/gen-camera-matrices.py`.
fn model_key(make: &str, model: &str) -> String {
    let make = make_key(make);
    let model = model.to_uppercase().replace('-', " ");
    let mut words: Vec<&str> = model.split_whitespace().collect();
    if words.first() == Some(&make.as_str()) {
        words.remove(0);
    }
    words.retain(|w| !matches!(*w, "EOS" | "DIGITAL"));
    words.join(" ")
}

/// The colour tags of a raw file that embeds no matrix: its camera's fitted
/// matrices under illuminants A and D65 when the table knows the camera, none
/// otherwise, and the as-shot neutral the file recorded.
pub(super) fn camera_color(
    make: Option<&str>,
    model: Option<&str>,
    as_shot_neutral: Option<[f64; 3]>,
) -> DngColor {
    let mut color = DngColor {
        as_shot_neutral,
        ..DngColor::default()
    };
    let (Some(make), Some(model)) = (make, model) else {
        return color;
    };
    let (make_key, model_key) = (make_key(make), model_key(make, model));
    if let Some(camera) = CAMERAS
        .iter()
        .find(|c| c.make == make_key && c.models.contains(&model_key.as_str()))
    {
        color.color_matrix = [Some(camera.illuminant_a), Some(camera.d65)];
        color.calibration_illuminant = [Some(ILLUMINANT_A), Some(ILLUMINANT_D65)];
    }
    color
}
