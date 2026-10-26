//! The filter catalogue: one row per filter kind, carrying the engine facts,
//! parameter schema, menu data, gallery rank, and the slot constructor.

use std::ops::RangeInclusive;
use std::sync::Arc;

use crate::filter::{Filter, FilterEngine, FilterKind};
use crate::{
    AwaConstraint, BlurPath, DepthMap, DisplaceMap, LiquifyMesh, PathMotion, SmartBlurMode, VpEdit,
    SHEAR_MAX_POINTS,
};

pub(crate) mod build;
pub(crate) mod rows;

#[cfg(test)]
mod tests;

const MATRIX_SIDE: usize = 5;

pub const GALLERY_TAIL: usize = 13;
pub const SHEAR_SLOTS: usize = 1 + 2 * SHEAR_MAX_POINTS + 1;
pub const LIGHTING_RIG_SLOTS: usize = 9;
pub const LIGHTING_LIGHT_SLOTS: usize = 13;

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterControl {
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
pub enum DialogKind {
    Shared = 0,
    LensFlare = 1,
    LightingEffects = 2,
    PictureFrame = 3,
    ReduceNoise = 4,
    BlurGallery = 5,
    Liquify = 6,
    AdaptiveWideAngle = 7,
    VanishingPoint = 8,
    LensCorrection = 9,
}

pub struct FilterParam {
    pub control: FilterControl,
    pub label: &'static str,
    pub minimum: f64,
    pub maximum: f64,
    pub value: f64,
    pub decimals: i32,
    pub suffix: &'static str,
    pub choices: &'static [&'static str],
    pub initial: &'static [f64],
    pub pixel_distance: bool,
}

impl FilterParam {
    pub fn slots(&self) -> usize {
        match self.control {
            FilterControl::Color => 3,
            FilterControl::Placement | FilterControl::BlurCenter => 2,
            FilterControl::ShearCurve => 1 + 2 * SHEAR_MAX_POINTS,
            FilterControl::Matrix => MATRIX_SIDE * MATRIX_SIDE,
            _ => 1,
        }
    }

    fn defaults_are_in_range(&self) -> bool {
        let in_range = |v: f64| v >= self.minimum && v <= self.maximum;
        match self.control {
            FilterControl::Color | FilterControl::Placement | FilterControl::BlurCenter => {
                !self.initial.is_empty() && self.initial.iter().all(|v| in_range(*v))
            }
            FilterControl::ShearCurve => {
                let Some((count, points)) = self.initial.split_first() else {
                    return false;
                };
                *count >= 0.0
                    && *count <= SHEAR_MAX_POINTS as f64
                    && points.iter().all(|v| (-1.0..=1.0).contains(v))
            }
            FilterControl::Matrix => {
                self.initial.len() == MATRIX_SIDE * MATRIX_SIDE
                    && self.initial.iter().all(|v| in_range(*v))
            }
            _ => in_range(self.value),
        }
    }
}

#[derive(Clone, Copy)]
pub enum AlphaPolicy {
    Never,
    Always,
    SmartBlurNormal,
}

impl AlphaPolicy {
    pub fn is_moving_kind(self) -> bool {
        !matches!(self, Self::Never)
    }

    pub fn moves(self, filter: &Filter) -> bool {
        match self {
            Self::Never => false,
            Self::Always => true,
            Self::SmartBlurNormal => matches!(
                filter,
                Filter::SmartBlur {
                    mode: SmartBlurMode::Normal,
                    ..
                }
            ),
        }
    }
}

#[derive(Clone, Copy)]
pub enum ApronPolicy {
    Fixed(i32),
    RadiusCeil,
    RadiusTimes3,
    RadiusCeilPlus1,
    IntRadius,
    Distance,
    MaxPinBlur,
    TiltBlur,
    PathReach,
    Spin,
    GalleryMax,
    Amount,
    Offset,
}

impl ApronPolicy {
    pub fn resolve(self, filter: &Filter) -> i32 {
        let radius = match self {
            Self::Fixed(value) => return value.clamp(8, 512),
            Self::RadiusCeil => match filter {
                Filter::GaussianBlur { radius }
                | Filter::UnsharpMask { radius, .. }
                | Filter::HighPass { radius } => radius.ceil(),
                _ => 128.0,
            },
            Self::RadiusTimes3 => match filter {
                Filter::SmartSharpen { radius, .. } => (3.0 * radius).ceil(),
                Filter::HdrToning(p) => (3.0 * p.radius).ceil(),
                _ => 128.0,
            },
            Self::RadiusCeilPlus1 => match filter {
                Filter::SmartBlur { radius, .. } => radius.ceil() + 1.0,
                _ => 128.0,
            },
            Self::IntRadius => match filter {
                Filter::BoxBlur { radius }
                | Filter::ShapeBlur { radius, .. }
                | Filter::SurfaceBlur { radius, .. }
                | Filter::LensBlur { radius, .. }
                | Filter::Median { radius }
                | Filter::Maximum { radius }
                | Filter::Minimum { radius } => f64::from(*radius),
                _ => 128.0,
            },
            Self::Distance => match filter {
                Filter::MotionBlur { distance, .. } => f64::from(*distance),
                _ => 128.0,
            },
            Self::MaxPinBlur => match filter {
                Filter::FieldBlur { pins, .. } => {
                    pins.iter().map(|pin| pin.blur).fold(0.0, f64::max)
                }
                Filter::IrisBlur { pins, .. } => {
                    pins.iter().map(|pin| pin.blur).fold(0.0, f64::max)
                }
                _ => 128.0,
            },
            Self::TiltBlur => match filter {
                Filter::TiltShift { pins, .. } => pins
                    .iter()
                    .map(|pin| pin.blur * (1.0 + pin.distortion.abs() / 100.0))
                    .fold(0.0, f64::max),
                _ => 128.0,
            },
            Self::PathReach => match filter {
                Filter::PathBlur { paths, motion, .. } => path_reach(paths, motion),
                _ => 128.0,
            },
            Self::Spin => 512.0,
            Self::GalleryMax => match filter {
                Filter::BlurGallery { spin, .. } if !spin.is_empty() => 512.0,
                Filter::BlurGallery {
                    field,
                    iris,
                    tilt,
                    path,
                    motion,
                    ..
                } => field
                    .iter()
                    .map(|pin| pin.blur)
                    .chain(iris.iter().map(|pin| pin.blur))
                    .chain(
                        tilt.iter()
                            .map(|pin| pin.blur * (1.0 + pin.distortion.abs() / 100.0)),
                    )
                    .fold(0.0, f64::max)
                    .max(path_reach(path, motion)),
                _ => 128.0,
            },
            Self::Amount => match filter {
                Filter::RadialBlur { amount, .. } => *amount,
                _ => 128.0,
            },
            Self::Offset => match filter {
                Filter::Offset {
                    horizontal,
                    vertical,
                    ..
                } => f64::from(horizontal.abs().max(vertical.abs())),
                _ => 128.0,
            },
        };
        (radius.ceil() as i32).clamp(8, 512)
    }
}

fn path_reach(paths: &[BlurPath], motion: &PathMotion) -> f64 {
    paths
        .iter()
        .map(|path| path.start_speed.max(path.end_speed) * motion.speed / 100.0)
        .fold(0.0, f64::max)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GpuPolicy {
    Unsupported,
    Accelerated,
    CpuPreferred,
}

impl GpuPolicy {
    pub fn is_cpu_preferred(self) -> bool {
        matches!(self, Self::CpuPreferred)
    }

    pub fn is_supported(self) -> bool {
        !matches!(self, Self::Unsupported)
    }
}

pub trait FilterResources {
    fn depth_map(&self, id: u32) -> Option<Arc<DepthMap>>;
    fn displace_map(&self, id: u32) -> Option<Arc<DisplaceMap>>;
    fn liquify_mesh(&self, id: u32) -> Option<Arc<LiquifyMesh>>;
    fn vanishing_point(&self, id: u32) -> Option<Arc<VpEdit>>;
    fn awa_constraints(&self, id: u32) -> Option<Arc<Vec<AwaConstraint>>>;
}

pub struct NullResources;

impl FilterResources for NullResources {
    fn depth_map(&self, _id: u32) -> Option<Arc<DepthMap>> {
        None
    }

    fn displace_map(&self, _id: u32) -> Option<Arc<DisplaceMap>> {
        Some(Arc::new(DisplaceMap::neutral()))
    }

    fn liquify_mesh(&self, _id: u32) -> Option<Arc<LiquifyMesh>> {
        Some(Arc::new(LiquifyMesh::new(1, 1)))
    }

    fn vanishing_point(&self, _id: u32) -> Option<Arc<VpEdit>> {
        Some(Arc::new(VpEdit::default()))
    }

    fn awa_constraints(&self, _id: u32) -> Option<Arc<Vec<AwaConstraint>>> {
        Some(Arc::default())
    }
}

pub struct BuildCtx<'a> {
    pub spec: &'a FilterSpec,
    pub params: &'a [f64],
    pub defaults: &'a [f64],
    pub resources: &'a dyn FilterResources,
}

impl BuildCtx<'_> {
    pub fn f(&self, i: usize) -> f64 {
        self.params.get(i).copied().unwrap_or_else(|| self.d(i))
    }

    pub fn d(&self, i: usize) -> f64 {
        self.defaults.get(i).copied().unwrap_or(0.0)
    }

    pub fn u8v(&self, i: usize) -> u8 {
        self.f(i).clamp(0.0, 255.0).round() as u8
    }

    pub fn u32v(&self, i: usize) -> u32 {
        self.f(i).max(0.0).round() as u32
    }

    pub fn i32v(&self, i: usize) -> i32 {
        self.f(i).round() as i32
    }

    pub fn u64v(&self, i: usize) -> u64 {
        self.f(i).max(0.0).round() as u64
    }

    pub fn flag(&self, i: usize) -> bool {
        self.f(i) >= 0.5
    }

    pub fn pick<T: Copy>(&self, all: &[T], i: usize, d: usize) -> T {
        let j = self
            .params
            .get(i)
            .map(|v| v.round().max(0.0) as usize)
            .unwrap_or(d);
        *all.get(j).unwrap_or(&all[d])
    }

    pub fn rgb(&self, base: usize) -> [u8; 3] {
        [self.u8v(base), self.u8v(base + 1), self.u8v(base + 2)]
    }

    pub fn arity(&self) -> Option<()> {
        if !self.params.is_empty() && self.params.len() != self.spec.slots() {
            None
        } else {
            Some(())
        }
    }
}

pub type Build = fn(&BuildCtx<'_>) -> Option<Filter>;

pub struct FilterSpec {
    pub kind: FilterKind,
    pub id: &'static str,
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
    pub alpha: AlphaPolicy,
    pub apron: ApronPolicy,
    pub gpu: GpuPolicy,
    pub gallery: Option<u16>,
    pub build: Build,
    pub engine: FilterEngine,
}

impl FilterSpec {
    pub fn slots(&self) -> usize {
        self.params.iter().map(FilterParam::slots).sum()
    }

    pub fn defaults(&self) -> Vec<f64> {
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

    pub fn range_for_slot(&self, slot: usize) -> Option<RangeInclusive<f64>> {
        let mut offset = 0;
        for param in self.params {
            let slots = param.slots();
            if slot < offset + slots {
                return Some(param.minimum..=param.maximum);
            }
            offset += slots;
        }
        None
    }

    pub fn defaults_are_in_range(&self) -> bool {
        self.params.iter().all(FilterParam::defaults_are_in_range)
    }

    pub fn scale_pixel_params(&self, params: &[f64], scale: f64) -> Vec<f64> {
        let mut out = if params.is_empty() {
            self.defaults()
        } else {
            params.to_vec()
        };
        if !scale.is_finite() || scale <= 0.0 {
            return out;
        }
        let mut slot = 0;
        for param in self.params {
            if param.pixel_distance && slot < out.len() {
                let mut value = out[slot] * scale;
                if param.decimals == 0 {
                    value = value.round();
                }
                out[slot] = value.clamp(param.minimum, param.maximum);
            }
            slot += param.slots();
        }
        out
    }
}

pub fn all() -> impl Iterator<Item = &'static FilterSpec> {
    FilterKind::ALL.iter().map(|kind| kind.spec())
}

pub fn descriptor(id: &str) -> Option<&'static FilterSpec> {
    all().find(|spec| spec.id == id)
}

pub fn arity(id: &str) -> Option<usize> {
    descriptor(id).map(FilterSpec::slots)
}

pub fn scale_pixel_params(id: &str, params: &[f64], scale: f64) -> Vec<f64> {
    descriptor(id).map_or_else(
        || params.to_vec(),
        |spec| spec.scale_pixel_params(params, scale),
    )
}

pub fn spec(kind: FilterKind) -> &'static FilterSpec {
    kind.spec()
}

pub fn build(id: &str, params: &[f64], resources: &dyn FilterResources) -> Option<Filter> {
    let spec = descriptor(id)?;
    let defaults = spec.defaults();
    let ctx = BuildCtx {
        spec,
        params,
        defaults: &defaults,
        resources,
    };
    (spec.build)(&ctx)
}

pub fn default_filter(kind: FilterKind) -> Option<Filter> {
    build(kind.spec().id, &[], &NullResources)
}
