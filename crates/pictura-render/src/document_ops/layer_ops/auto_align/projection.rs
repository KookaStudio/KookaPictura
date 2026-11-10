//! Cylindrical and spherical projection spaces for Auto-Align.
//!
//! Feature points are projected about the union centre, a homography is
//! estimated there, and the applied map composes source plane → projection →
//! homography → reference projection → reference plane.

use pictura_core::PsdRect;

use super::super::transform::PlaneMap;
use super::estimate::{ransac_homography, Model, Pair};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Cylindrical,
    Spherical,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ProjectedMap {
    pub h: [f64; 9],
    pub h_inv: [f64; 9],
    pub focal: f64,
    pub centre: (f64, f64),
    pub kind: Kind,
}

impl ProjectedMap {
    pub fn new(kind: Kind, h: [f64; 9], focal: f64, centre: (f64, f64)) -> Option<Self> {
        let h_inv = super::estimate::invert_h(&h)?;
        Some(Self {
            h,
            h_inv,
            focal,
            centre,
            kind,
        })
    }
}

impl PlaneMap for ProjectedMap {
    fn forward(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let p = project(self.kind, x, y, self.focal, self.centre)?;
        let q = super::estimate::apply_h(&self.h, p.0, p.1)?;
        unproject(self.kind, q.0, q.1, self.focal, self.centre)
    }

    fn inverse(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let q = project(self.kind, x, y, self.focal, self.centre)?;
        let p = super::estimate::apply_h(&self.h_inv, q.0, q.1)?;
        unproject(self.kind, p.0, p.1, self.focal, self.centre)
    }

    fn for_rect(&self, _rect: PsdRect) -> Self {
        *self
    }
}

pub(super) fn project(
    kind: Kind,
    x: f64,
    y: f64,
    focal: f64,
    centre: (f64, f64),
) -> Option<(f64, f64)> {
    let dx = x - centre.0;
    let dy = y - centre.1;
    if !dx.is_finite() || !dy.is_finite() || focal <= 0.0 {
        return None;
    }
    let theta = (dx / focal).atan();
    let radius = (dx * dx + focal * focal).sqrt();
    let v = dy / radius;
    Some(match kind {
        Kind::Cylindrical => (focal * theta, focal * v),
        Kind::Spherical => (focal * theta, focal * v.atan()),
    })
}

pub(super) fn unproject(
    kind: Kind,
    u: f64,
    v: f64,
    focal: f64,
    centre: (f64, f64),
) -> Option<(f64, f64)> {
    if !u.is_finite() || !v.is_finite() || focal <= 0.0 {
        return None;
    }
    let theta = u / focal;
    let cos = theta.cos();
    if cos.abs() < 1e-9 {
        return None;
    }
    let x = centre.0 + focal * theta.tan();
    let y = match kind {
        Kind::Cylindrical => centre.1 + (v / focal) * focal / cos,
        Kind::Spherical => centre.1 + focal * (v / focal).tan() / cos,
    };
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// Estimate a homography in the projected space of `kind`.
pub(super) fn estimate(
    kind: Kind,
    pairs: &[Pair],
    focal: f64,
    centre: (f64, f64),
    threshold: f64,
) -> Option<[f64; 9]> {
    let projected: Vec<Pair> = pairs
        .iter()
        .filter_map(|p| {
            Some(Pair {
                a: project(kind, p.a.0, p.a.1, focal, centre)?,
                b: project(kind, p.b.0, p.b.1, focal, centre)?,
            })
        })
        .collect();
    match ransac_homography(&projected, threshold)? {
        Model::Homography { h } => Some(h),
        _ => None,
    }
}
