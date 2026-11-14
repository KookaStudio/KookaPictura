//! The Red Eye tool: take the red out of a flash reflection inside a dragged
//! box. Only pixels where red genuinely dominates are touched, so the box can
//! be dragged loosely over an eye without draining the skin around it.
//!
//! Behavioural parity only: CS6's detector is closed; this is photorust's
//! red-dominance test (`docs/03-tools/healing-brushes.md` lists the tool in
//! the `J` group without an algorithm).
//!
//! Ported from photorust's `core/src/healing.rs` (`red_eye_region`)
//! (<https://github.com/perfecto25/photorust>).

use super::layer::{heal_layer, HealError};
use super::RgbaImage;
use pictura_core::{Document, PsdRect};
use rayon::prelude::*;

/// CS6's Pupil Size and Darken Amount defaults (both 0–100).
pub const RED_EYE_DEFAULT_PUPIL: u32 = 50;
pub const RED_EYE_DEFAULT_DARKEN: u32 = 50;

/// Neutralise red-eye in the pixel layer at `path` inside `rect` (document
/// pixels). `pupil` is Pupil Size (0–100): how readily a pixel counts as red.
/// `darken` is Darken Amount (0–100), applied to whatever is neutralised.
/// Returns the document rectangle changed, or `None` when the box held no red.
pub fn red_eye_layer(
    doc: &mut Document,
    path: &str,
    rect: PsdRect,
    pupil: u32,
    darken: u32,
) -> Result<Option<PsdRect>, HealError> {
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };
    let Some(rect) = rect.intersect(canvas) else {
        return Ok(None);
    };
    // Red Eye is a per-pixel box test, so it carries no coverage mask.
    heal_layer(doc, path, rect, &[], Some(rect), |img, local, _| {
        red_eye_region(img, local, pupil, darken)
    })
}

fn red_eye_region(
    img: &mut RgbaImage,
    region: PsdRect,
    pupil: u32,
    darken: u32,
) -> Option<PsdRect> {
    let work = region.intersect(img.rect())?;
    // Pupil Size widens the net: at 0 only strongly red pixels qualify, at 100
    // anything where red merely leads.
    let ratio = 1.8 - (pupil.min(100) as f32 / 100.0) * 0.75;
    let keep = 1.0 - (darken.min(100) as f32 / 100.0) * 0.6;
    let to_u8 = |v: f32| v.round().clamp(0.0, 255.0) as u8;
    let width = img.width as usize;
    let (top, bottom) = (work.top as usize, work.bottom as usize);
    let (left, right) = (work.left as usize, work.right as usize);
    let touched = img.data[top * width..bottom * width]
        .par_chunks_mut(width)
        .map(|row| {
            let mut touched = false;
            for pixel in &mut row[left..right] {
                let [r, g, b, a] = *pixel;
                let (rf, gf, bf) = (r as f32, g as f32, b as f32);
                if rf < gf.max(bf).max(1.0) * ratio {
                    continue;
                }
                // Red becomes the green/blue level, which is what the pupil
                // would have been without the flash; then everything is
                // darkened.
                *pixel = [
                    to_u8((gf + bf) / 2.0 * keep),
                    to_u8(gf * keep),
                    to_u8(bf * keep),
                    a,
                ];
                touched = true;
            }
            touched
        })
        .reduce(|| false, |a, b| a || b);
    touched.then_some(work)
}
