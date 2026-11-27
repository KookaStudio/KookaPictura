//! What an ORF's maker note records (#1471): levels, crop, white balances
//! and the camera's colour matrix. Built from ExifTool's public Olympus tag
//! documentation and observation of real files.
//!
//! The camera's matrix takes white-balanced camera RGB to sRGB; it was read
//! so by comparing files with their own camera JPEGs, so colour is approximate.

use pictura_color::chroma::{self, Mat3, D50_XY};

use super::orf_decode::{Params, PARAM_TAGS};
use super::tiff::{Entry, Ifd, Tiff};
use super::vendor::maker_note;
use super::{DngColor, DngCrop, Tags};
use crate::error::PsdError;

const IMAGE_PROCESSING: u16 = 0x2040;
const RAW_INFO: u16 = 0x3000;
/// In ImageProcessing and RawInfo alike.
const WB_LEVELS: u16 = 0x0100;
const MATRIX: u16 = 0x0200;
const BLACK: u16 = 0x0600;
const VALID_BITS: u16 = 0x0611;
const CROP: [u16; 4] = [0x0612, 0x0613, 0x0614, 0x0615];
const WB_6600K: u16 = 0x010C;
const SENSOR_CALIBRATION: u16 = 0x0805;
const WB_FINE_WEATHER: u16 = 0x0122;
/// The main note's tags, on bodies older than those directories.
const OLD_MATRIX: u16 = 0x1011;
const OLD_BLACK: u16 = 0x1012;
const OLD_RED: u16 = 0x1017;
const OLD_BLUE: u16 = 0x1018;
const OLD_VALID_BITS: u16 = 0x102C;

const ILLUMINANT_D55: u16 = 20;
const ILLUMINANT_D65: u16 = 21;
const D55_XY: [f64; 2] = [0.33242, 0.34743];
const D65_XY: [f64; 2] = [0.31271, 0.32902];
const SRGB_PRIMARIES: [[f64; 2]; 3] = [[0.64, 0.33], [0.30, 0.60], [0.15, 0.06]];

#[derive(Default)]
pub(super) struct OrfMeta {
    pub bits: Option<u32>,
    /// One per 2 × 2 position, at the precision of `bits`.
    pub black: Option<[f64; 4]>,
    pub white: Option<f64>,
    pub crop: Option<DngCrop>,
    /// The compressed stream's coding, when ImageProcessing records any of it.
    pub params: Option<Params>,
    as_shot: Option<[f64; 3]>,
    /// The daylight neutral and the EXIF code of its illuminant.
    daylight: Option<([f64; 3], u16)>,
    matrix: Option<Mat3>,
}

struct MakerNote<'a> {
    tiff: Tiff<'a>,
    main: Ifd,
    processing: Option<Ifd>,
    raw_info: Option<Ifd>,
}

impl<'a> MakerNote<'a> {
    fn read(ifd0: &Tags<'_, 'a>) -> Result<Option<Self>, PsdError> {
        let Some(note) = maker_note(ifd0)? else {
            return Ok(None);
        };
        let file = ifd0.tiff.bytes();
        let bytes = file.get(note.at..).ok_or(PsdError::Truncated)?;
        let order = ifd0.tiff.order;
        let (tiff, at) = if bytes.starts_with(b"OLYMPUS\0") {
            (Tiff::view(bytes, order), 12)
        } else if bytes.starts_with(b"OM SYSTEM\0") {
            (Tiff::view(bytes, order), 16)
        } else if bytes.starts_with(b"OLYMP\0") {
            (Tiff::view(file, order), note.at as u64 + 8)
        } else {
            return Ok(None);
        };
        let main = tiff.ifd_skipping_strays(at)?;
        // A directory is named by an offset or stored as the entry's value.
        let directory = |entry: &Entry| -> Result<Ifd, PsdError> {
            let at = match tiff.raw(entry).len() {
                4 => tiff.uint(entry)?,
                _ => entry.at as u64,
            };
            tiff.ifd_skipping_strays(at)
        };
        let processing = main.get(IMAGE_PROCESSING).map(directory).transpose()?;
        let raw_info = main.get(RAW_INFO).map(directory).transpose()?;
        Ok(Some(Self {
            tiff,
            main,
            processing,
            raw_info,
        }))
    }

    /// `tag` of `ifd`, when it holds at least `N` numbers that are not all 0.
    fn set<const N: usize>(&self, ifd: &Ifd, tag: u16) -> Result<Option<[f64; N]>, PsdError> {
        Ok(self
            .values(ifd, tag)?
            .filter(|v: &[f64; N]| v.iter().any(|&x| x != 0.0)))
    }

    fn values<const N: usize>(&self, ifd: &Ifd, tag: u16) -> Result<Option<[f64; N]>, PsdError> {
        let Some(entry) = ifd.get(tag) else {
            return Ok(None);
        };
        Ok(self
            .tiff
            .reals(entry)?
            .get(..N)
            .and_then(|v| v.try_into().ok()))
    }

    /// The directories that share the ImageProcessing tag numbers, preferred
    /// first.
    fn directories(&self) -> impl Iterator<Item = &Ifd> {
        self.processing.iter().chain(&self.raw_info)
    }

    /// `tag` from those directories, else `old` from the main note.
    fn find<const N: usize>(&self, tag: u16, old: u16) -> Result<Option<[f64; N]>, PsdError> {
        for ifd in self.directories() {
            if let Some(v) = self.values(ifd, tag)? {
                return Ok(Some(v));
            }
        }
        self.values(&self.main, old)
    }

    // ponytail: the green levels are taken as 256; they differ by under 2 %.
    fn as_shot(&self) -> Result<Option<[f64; 3]>, PsdError> {
        for ifd in self.directories() {
            if let Some([r, b]) = self.set(ifd, WB_LEVELS)? {
                return Ok(neutral(r, b));
            }
        }
        match (
            self.set(&self.main, OLD_RED)?,
            self.set(&self.main, OLD_BLUE)?,
        ) {
            (Some([r]), Some([b])) => Ok(neutral(r, b)),
            _ => Ok(None),
        }
    }

    fn daylight(&self) -> Result<Option<([f64; 3], u16)>, PsdError> {
        let sources = [
            (&self.processing, WB_6600K, ILLUMINANT_D65),
            (&self.raw_info, WB_FINE_WEATHER, ILLUMINANT_D55),
        ];
        for (ifd, tag, illuminant) in sources {
            if let Some(ifd) = ifd {
                if let Some(n) = self.set(ifd, tag)?.and_then(|[r, b]| neutral(r, b)) {
                    return Ok(Some((n, illuminant)));
                }
            }
        }
        Ok(None)
    }

    /// The coding parameters, LibRaw's 0 for each tag that is absent.
    fn params(&self) -> Result<Option<Params>, PsdError> {
        let Some(ifd) = &self.processing else {
            return Ok(None);
        };
        let mut params = [0; 14];
        let mut any = false;
        for (p, &tag) in params.iter_mut().zip(&PARAM_TAGS) {
            if let Some([v]) = self.values::<1>(ifd, tag)? {
                *p = v as u32;
                any = true;
            }
        }
        Ok(any.then_some(params))
    }

    fn crop(&self) -> Result<Option<DngCrop>, PsdError> {
        for ifd in self.directories() {
            let value = |i: usize| self.values::<1>(ifd, CROP[i]);
            if let (Some([x]), Some([y]), Some([width]), Some([height])) =
                (value(0)?, value(1)?, value(2)?, value(3)?)
            {
                if width > 0.0 && height > 0.0 {
                    return Ok(Some(DngCrop {
                        x,
                        y,
                        width,
                        height,
                    }));
                }
            }
        }
        Ok(None)
    }
}

fn neutral(red: f64, blue: f64) -> Option<[f64; 3]> {
    (red > 0.0 && blue > 0.0).then(|| [256.0 / red, 1.0, 256.0 / blue])
}

/// The stored matrix with each row summing to 1, so white stays white. Its
/// entries are signed, whatever their tag type says.
fn matrix(stored: [f64; 9]) -> Option<Mat3> {
    let v = stored.map(|v| if v > 32767.0 { v - 65536.0 } else { v });
    let sums: [f64; 3] = std::array::from_fn(|r| v[3 * r..][..3].iter().sum());
    sums.iter()
        .all(|&s| s > 0.0)
        .then(|| std::array::from_fn(|r| std::array::from_fn(|c| v[3 * r + c] / sums[r])))
}

fn flat(m: &Mat3) -> [f64; 9] {
    std::array::from_fn(|i| m[i / 3][i % 3])
}

impl OrfMeta {
    pub fn read(ifd0: &Tags) -> Result<Self, PsdError> {
        let Some(note) = MakerNote::read(ifd0)? else {
            return Ok(Self::default());
        };
        let white = match &note.processing {
            Some(ifd) => note.set(ifd, SENSOR_CALIBRATION)?.map(|[max]| max),
            None => None,
        };
        Ok(Self {
            bits: note
                .find(VALID_BITS, OLD_VALID_BITS)?
                .map(|[bits]| bits as u32),
            black: note.find(BLACK, OLD_BLACK)?,
            white,
            crop: note.crop()?,
            params: note.params()?,
            as_shot: note.as_shot()?,
            daylight: note.daylight()?,
            matrix: note.find(MATRIX, OLD_MATRIX)?.and_then(matrix),
        })
    }

    /// The colour tags equivalent to the camera's matrix, anchored on its
    /// daylight balance, else on the as-shot one as D65 (a nominal Kelvin
    /// readout). Without either, or without a usable matrix, no matrices.
    pub fn color(&self) -> DngColor {
        let mut color = DngColor {
            as_shot_neutral: self.as_shot,
            ..DngColor::default()
        };
        let anchor = self.daylight.or(self.as_shot.map(|n| (n, ILLUMINANT_D65)));
        let (Some(m), Some((daylight, illuminant))) = (self.matrix, anchor) else {
            return color;
        };
        let srgb = chroma::rgb_to_xyz(SRGB_PRIMARIES, D65_XY);
        let (Some(to_camera), Some(to_srgb)) = (chroma::invert(&m), chroma::invert(&srgb)) else {
            return color;
        };
        let white = if illuminant == ILLUMINANT_D55 {
            D55_XY
        } else {
            D65_XY
        };
        let forward = chroma::mul(&chroma::bradford(D65_XY, D50_XY), &chroma::mul(&srgb, &m));
        let xyz_to_camera = chroma::mul(
            &chroma::diag(daylight),
            &chroma::mul(
                &to_camera,
                &chroma::mul(&to_srgb, &chroma::bradford(white, D65_XY)),
            ),
        );
        color.color_matrix = [Some(flat(&xyz_to_camera)), None];
        color.forward_matrix = [Some(flat(&forward)), None];
        color.calibration_illuminant = [Some(illuminant), None];
        color
    }
}
