//! The filter parameter descriptor: one table of every filter kind's dialog
//! controls, canonical slot count and opening defaults.
//!
//! The table is split into five contiguous runs so each module stays well
//! under the file-size cap. The rows are the C++ table's former rows, in the
//! same order; the order is what `filterCommands()` preserves.

mod blur;
mod distort;
mod paint;
mod render;
mod stylize;
mod video;

use pictura_filters::SHEAR_MAX_POINTS;

const MATRIX_SIDE: usize = 5;

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FilterControl {
    Slider = 0,
    Angle = 1,
    Choice = 2,
    Radio = 3,
    CheckBox = 4,
    Color = 5,
    Placement = 6,
    BlurCenter = 7,
    ShearCurve = 8,
    Hidden = 9,
    Number = 10,
    Matrix = 11,
    Shape = 12,
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DialogKind {
    Shared = 0,
    LensFlare = 1,
    LightingEffects = 2,
    PictureFrame = 3,
    ReduceNoise = 4,
    BlurGallery = 5,
    Liquify = 6,
    AdaptiveWideAngle = 7,
    VanishingPoint = 8,
}

pub(crate) struct FilterParam {
    pub control: FilterControl,
    pub label: &'static str,
    pub minimum: f64,
    pub maximum: f64,
    pub value: f64,
    pub decimals: i32,
    pub suffix: &'static str,
    pub choices: &'static [&'static str],
    pub initial: &'static [f64],
}

impl FilterParam {
    pub(crate) fn slots(&self) -> usize {
        match self.control {
            FilterControl::Color => 3,
            FilterControl::Placement | FilterControl::BlurCenter => 2,
            FilterControl::ShearCurve => 1 + 2 * SHEAR_MAX_POINTS,
            FilterControl::Matrix => MATRIX_SIDE * MATRIX_SIDE,
            _ => 1,
        }
    }
}

pub(crate) struct FilterDescriptor {
    pub kind: &'static str,
    pub path: &'static [&'static str],
    pub menu: bool,
    pub label: &'static str,
    pub params: &'static [FilterParam],
    pub whole_layer: bool,
    pub dialog: DialogKind,
    pub map_after_ok: bool,
    pub preview_pane: bool,
    pub stacked: bool,
    pub preview_below: bool,
    pub live_preview: bool,
}

impl FilterDescriptor {
    pub(crate) fn slots(&self) -> usize {
        self.params.iter().map(FilterParam::slots).sum()
    }

    /// The dialog-opening values, one per slot.
    pub(crate) fn defaults(&self) -> Vec<f64> {
        let mut out = Vec::with_capacity(self.slots());
        for param in self.params {
            if param.initial.is_empty() {
                out.resize(out.len() + param.slots(), param.value);
            } else {
                debug_assert_eq!(param.initial.len(), param.slots());
                out.extend_from_slice(param.initial);
            }
        }
        out
    }
}

static TABLE: &[&[FilterDescriptor]] = &[
    paint::ROWS,
    blur::ROWS,
    distort::ROWS,
    render::ROWS,
    stylize::ROWS,
    video::ROWS,
];

pub(crate) fn all() -> impl Iterator<Item = &'static FilterDescriptor> {
    TABLE.iter().flat_map(|rows| *rows)
}

pub(crate) fn descriptor(kind: &str) -> Option<&'static FilterDescriptor> {
    all().find(|descriptor| descriptor.kind == kind)
}

pub(crate) fn arity(kind: &str) -> Option<usize> {
    descriptor(kind).map(FilterDescriptor::slots)
}

/// The slots a commit runs with. Lens Blur's Preview radio only changes the
/// dialog's sampling density, so a commit always uses the More Accurate path
/// (docs `FILT-010` edge case).
pub(crate) fn commit_params(kind: &str, params: &[f64]) -> Vec<f64> {
    let mut params = params.to_vec();
    if kind == "lens-blur" && params.len() == arity(kind).unwrap_or(0) {
        params[0] = 1.0;
    }
    params
}

#[cfg(test)]
mod tests;
