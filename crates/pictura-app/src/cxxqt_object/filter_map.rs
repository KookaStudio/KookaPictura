//! The app's filter slot decoder: the catalogue builds the filter; this module
//! resolves the session resources the catalogue cannot own (Lens Blur depth
//! maps, Displace maps, Liquify meshes, Adaptive Wide Angle constraints, and
//! Vanishing Point edits).

use std::sync::Arc;

use pictura_filters::catalogue::{self, FilterResources};
use pictura_filters::{AwaConstraint, DepthMap, DisplaceMap, Filter, LiquifyMesh, VpEdit};

pub(crate) use pictura_filters::catalogue as descriptor;
pub(super) use pictura_filters::catalogue::arity as filter_param_arity;
#[cfg(test)]
pub(super) use pictura_filters::catalogue::SHEAR_SLOTS;

struct AppResources;

impl FilterResources for AppResources {
    fn depth_map(&self, id: u32) -> Option<Arc<DepthMap>> {
        super::filter_tools::lens_blur_depths::depth_map(id)
    }

    fn displace_map(&self, id: u32) -> Option<Arc<DisplaceMap>> {
        super::filter_tools::displace_maps::displace_map(id)
    }

    fn liquify_mesh(&self, id: u32) -> Option<Arc<LiquifyMesh>> {
        super::filter_tools::liquify::liquify_mesh(id)
    }

    fn vanishing_point(&self, id: u32) -> Option<Arc<VpEdit>> {
        super::filter_tools::vanishing_point::vp_edit(id)
    }

    fn awa_constraints(&self, id: u32) -> Option<Arc<Vec<AwaConstraint>>> {
        super::filter_tools::adaptive_wide_angle::awa_constraints(id)
    }
}

pub(crate) fn filter_from_kind_params(kind: &str, params: &[f64]) -> Option<Filter> {
    catalogue::build(kind, params, &AppResources)
}

pub(crate) fn is_filter_command(kind: &str) -> bool {
    descriptor::descriptor(kind).is_some_and(|spec| !spec.path.is_empty())
}

/// The slots a commit runs with. Lens Blur's Preview radio only changes the
/// dialog's sampling density, so a commit always uses the More Accurate path
/// (docs `FILT-010` edge case).
pub(crate) fn commit_params(kind: &str, params: &[f64]) -> Vec<f64> {
    let mut params = params.to_vec();
    if kind == "lens-blur" && params.len() == descriptor::arity(kind).unwrap_or(0) {
        params[0] = 1.0;
    }
    params
}

#[cfg(test)]
#[path = "filter_map_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "filter_map_resource_tests.rs"]
mod resource_tests;
