//! The supported filter kinds, their exact slot counts, and the commit-time
//! slot normalization. Kept beside the mapping so the Rust table and
//! `cpp/filter_commands.cpp` stay in lock-step.

use super::{
    BLUR_GALLERY_SLOTS, CUSTOM_SLOTS, FIELD_BLUR_SLOTS, IRIS_BLUR_SLOTS, LIGHTING_LIGHT_SLOTS,
    LIGHTING_RIG_SLOTS, PATH_BLUR_SLOTS, PICTURE_FRAME_SLOTS, SHEAR_SLOTS, SPIN_BLUR_SLOTS,
    TILT_SHIFT_SLOTS,
};
use crate::cxxqt_object::filter_tools::displace_maps::DISPLACE_SLOTS;
use crate::cxxqt_object::filter_tools::lens_blur_depths::LENS_BLUR_SLOTS;

/// Every supported kind and its exact slot count. This is the guard that
/// keeps the Rust mapping and `cpp/filter_commands.cpp` in lock-step.
pub(in crate::cxxqt_object) const FILTER_ARITIES: &[(&str, usize)] = &[
    ("gaussian-blur", 1),
    ("box-blur", 1),
    ("shape-blur", 2),
    ("surface-blur", 2),
    ("smart-blur", 4),
    ("lens-blur", LENS_BLUR_SLOTS),
    ("field-blur", FIELD_BLUR_SLOTS),
    ("iris-blur", IRIS_BLUR_SLOTS),
    ("tilt-shift", TILT_SHIFT_SLOTS),
    ("path-blur", PATH_BLUR_SLOTS),
    ("spin-blur", SPIN_BLUR_SLOTS),
    ("blur-gallery", BLUR_GALLERY_SLOTS),
    ("motion-blur", 2),
    ("radial-blur", 5),
    ("average", 0),
    ("blur", 0),
    ("blur-more", 0),
    ("median", 1),
    ("despeckle", 0),
    ("reduce-noise", 12),
    ("sharpen", 0),
    ("sharpen-more", 0),
    ("sharpen-edges", 0),
    ("unsharp-mask", 3),
    ("add-noise", 4),
    ("maximum", 1),
    ("minimum", 1),
    ("offset", 7),
    ("high-pass", 1),
    ("custom", CUSTOM_SLOTS),
    ("emboss", 3),
    ("find-edges", 0),
    ("solarize", 0),
    ("diffuse", 1),
    ("glowing-edges", 3),
    ("mosaic", 1),
    ("crystallize", 2),
    ("facet", 0),
    ("fragment", 0),
    ("mezzotint", 2),
    ("pointillize", 5),
    ("color-halftone", 5),
    ("twirl", 1),
    ("pinch", 1),
    ("spherize", 2),
    ("ripple", 2),
    ("wave", 9),
    ("polar-coordinates", 1),
    ("shear", SHEAR_SLOTS),
    ("displace", DISPLACE_SLOTS),
    ("liquify", 1),
    ("zigzag", 3),
    ("ocean-ripple", 3),
    ("diffuse-glow", 4),
    ("glass", 5),
    ("clouds", 8),
    ("difference-clouds", 8),
    ("fibers", 9),
    ("lens-flare", 4),
    ("picture-frame", PICTURE_FRAME_SLOTS),
    (
        "lighting-effects",
        LIGHTING_RIG_SLOTS + LIGHTING_LIGHT_SLOTS,
    ),
    ("colored-pencil", 4),
    ("cutout", 3),
    ("dry-brush", 4),
    ("film-grain", 4),
    ("fresco", 4),
    ("neon-glow", 5),
    ("paint-daubs", 4),
    ("palette-knife", 3),
    ("plastic-wrap", 3),
    ("poster-edges", 3),
    ("rough-pastels", 7),
    ("smudge-stick", 4),
    ("sponge", 4),
    ("underpainting", 8),
    ("watercolor", 4),
    ("accented-edges", 3),
    ("angled-strokes", 3),
    ("crosshatch", 3),
    ("dark-strokes", 3),
    ("ink-outlines", 3),
    ("spatter", 3),
    ("sprayed-strokes", 4),
    ("sumi-e", 3),
    ("bas-relief", 9),
    ("chalk-charcoal", 10),
    ("charcoal", 10),
    ("chrome", 2),
    ("conte-crayon", 14),
    ("graphic-pen", 9),
    ("halftone-pattern", 3),
    ("note-paper", 4),
    ("photocopy", 2),
    ("plaster", 9),
    ("reticulation", 10),
    ("stamp", 8),
    ("torn-edges", 9),
    ("water-paper", 4),
    ("craquelure", 3),
    ("grain", 7),
    ("mosaic-tiles", 4),
    ("patchwork", 3),
    ("stained-glass", 7),
    ("texturizer", 5),
    ("oil-paint", 6),
    ("dust-and-scratches", 2),
    ("extrude", 6),
    ("tiles", 9),
    ("trace-contour", 2),
    ("wind", 2),
    ("smart-sharpen", 12),
];

pub(in crate::cxxqt_object) fn filter_param_arity(kind: &str) -> Option<usize> {
    FILTER_ARITIES
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, n)| *n)
}

/// The slots a commit runs with. Lens Blur's Preview radio only changes the
/// dialog's sampling density, so a commit is always the More Accurate path
/// (docs `FILT-010` edge case).
pub(in crate::cxxqt_object) fn commit_params(kind: &str, params: &[f64]) -> Vec<f64> {
    let mut params = params.to_vec();
    if kind == "lens-blur" && params.len() == LENS_BLUR_SLOTS {
        params[0] = 1.0;
    }
    params
}
