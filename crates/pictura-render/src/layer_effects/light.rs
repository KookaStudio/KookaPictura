//! Resolve the document global light into the effects that read it.
//!
//! Effects decode their stored local `lagl`/`Lald` once; a document-wide light
//! change must not invalidate that decode cache, so the override is applied to a
//! clone at pass-build time instead.

use pictura_core::Document;

use super::LayerEffects;

/// `effects` with the document's global light applied to every effect whose
/// `use_global_angle` is true: Drop Shadow and Inner Shadow take the angle,
/// Bevel & Emboss takes the angle and altitude. Satin keeps its stored angle.
pub(crate) fn effective(effects: &LayerEffects, doc: &Document) -> LayerEffects {
    let mut out = effects.clone();
    let light = doc.global_light;
    if let Some(shadow) = out
        .drop_shadow
        .as_mut()
        .filter(|effect| effect.use_global_angle)
    {
        shadow.angle_deg = light.angle as f32;
    }
    if let Some(shadow) = out
        .inner_shadow
        .as_mut()
        .filter(|effect| effect.use_global_angle)
    {
        shadow.angle_deg = light.angle as f32;
    }
    if let Some(bevel) = out.bevel.as_mut().filter(|effect| effect.use_global_angle) {
        bevel.angle_deg = light.angle as f32;
        bevel.altitude_deg = light.altitude as f32;
    }
    out
}
