//! Refine Edge (SEL-003): improve the quality of a selection's edges so an
//! object can be extracted more cleanly.
//!
//! The observable behavior is taken from the fetched CS6 corpus
//! (`docs/08-selection/refine-edge.md`); Adobe's edge-estimation and matting
//! algorithms are closed, so this is an implementation-agnostic approximation
//! (behavioral parity only), marked where inferred.
//!
//! [`refine`] takes the selection coverage and the composite the selection was
//! made from and returns a refined coverage. [`decontaminate`] replaces colour
//! fringes at soft edges with nearby foreground colour and returns a new pixel
//! buffer.

use pictura_core::{PixelBuffer, Plane};

use crate::{SelectError, Selection};

const MAX_RADIUS: u32 = 100;
const MAX_AMOUNT: u32 = 100;
const MAX_SHIFT: i32 = 100;

/// How the refined result is displayed in the dialog preview. Only the mask
/// affects the engine; the rest is a view choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    MarchingAnts,
    Overlay,
    OnBlack,
    OnWhite,
    BlackAndWhite,
    OnLayers,
    RevealLayer,
}

impl ViewMode {
    pub const ALL: [ViewMode; 7] = [
        ViewMode::MarchingAnts,
        ViewMode::Overlay,
        ViewMode::OnBlack,
        ViewMode::OnWhite,
        ViewMode::BlackAndWhite,
        ViewMode::OnLayers,
        ViewMode::RevealLayer,
    ];

    pub fn from_index(index: i32) -> Option<ViewMode> {
        Self::ALL.get(usize::try_from(index).ok()?).copied()
    }
}

/// Where the refined result is written. Mirrors the CS6 "Output To" list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputTarget {
    Selection,
    LayerMask,
    NewLayer,
    NewLayerWithMask,
}

impl OutputTarget {
    pub const ALL: [OutputTarget; 4] = [
        OutputTarget::Selection,
        OutputTarget::LayerMask,
        OutputTarget::NewLayer,
        OutputTarget::NewLayerWithMask,
    ];

    pub fn from_index(index: i32) -> Option<OutputTarget> {
        Self::ALL.get(usize::try_from(index).ok()?).copied()
    }

    /// Whether the target writes pixel colour, which Decontaminate Colors
    /// requires (CS6 forbids in-place output while it is on).
    pub fn is_color_output(self) -> bool {
        matches!(
            self,
            OutputTarget::NewLayer | OutputTarget::NewLayerWithMask
        )
    }
}

/// The dialog's full parameter set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefineEdgeSettings {
    pub view_mode: ViewMode,
    pub show_original: bool,
    pub show_radius: bool,
    pub smart_radius: bool,
    pub radius: u32,
    pub smooth: u32,
    pub feather: u32,
    pub contrast: u32,
    pub shift_edge: i32,
    pub decontaminate: bool,
    pub amount: u32,
    pub output_to: OutputTarget,
}

impl Default for RefineEdgeSettings {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::MarchingAnts,
            show_original: false,
            show_radius: false,
            smart_radius: false,
            radius: 0,
            smooth: 0,
            feather: 0,
            contrast: 0,
            shift_edge: 0,
            decontaminate: false,
            amount: 50,
            output_to: OutputTarget::Selection,
        }
    }
}

/// Per-pixel radius for the edge band.
///
/// With `smart_radius` off every boundary pixel uses `radius`. With it on the
/// radius narrows toward `1` where the image edge is hard (a steep local
/// gradient) and stays near `radius` where it is soft, scaled by `radius`.
fn smart_band_radius(
    selection: &Selection,
    image: &PixelBuffer,
    x: usize,
    y: usize,
    radius: u32,
) -> u32 {
    if radius == 0 {
        return 0;
    }
    let w = selection.width as usize;
    let h = selection.height as usize;
    if !(1..w.saturating_sub(1)).contains(&x) || !(1..h.saturating_sub(1)).contains(&y) {
        return radius;
    }
    let lum = |xi: usize, yi: usize| {
        let i = yi * w + xi;
        let n = w * h;
        let (r, g, b) = if image.channels >= 3 {
            (
                image.data[i] as i32,
                image.data[n + i] as i32,
                image.data[2 * n + i] as i32,
            )
        } else {
            let v = image.data[i] as i32;
            (v, v, v)
        };
        (r * 299 + g * 587 + b * 114) / 1000
    };
    // Sobel magnitude: a hard edge has a large gradient.
    let gx = (lum(x + 1, y - 1) + 2 * lum(x + 1, y) + lum(x + 1, y + 1))
        - (lum(x - 1, y - 1) + 2 * lum(x - 1, y) + lum(x - 1, y + 1));
    let gy = (lum(x - 1, y + 1) + 2 * lum(x, y + 1) + lum(x + 1, y + 1))
        - (lum(x - 1, y - 1) + 2 * lum(x, y - 1) + lum(x + 1, y - 1));
    let magnitude = ((gx * gx + gy * gy) as f64).sqrt().min(1020.0);
    // magnitude 0 (soft) -> full radius, magnitude >= 510 (hard) -> radius 1.
    let softness = (1.0 - magnitude / 510.0).clamp(0.0, 1.0);
    ((radius as f64 * (0.15 + 0.85 * softness)).round() as u32).max(1)
}

/// Chamfer distance (in pixels) from every pixel to the nearest zero/nonzero
/// transition of the 50 % threshold of `sel`. Two passes, 3-4 chamfer weights
/// rounded to 3 and 4.
fn boundary_distance(sel: &Selection) -> Vec<u32> {
    let (w, h) = (sel.width as usize, sel.height as usize);
    let inside: Vec<bool> = sel.data.iter().map(|&v| v >= 128).collect();
    let mut dist = vec![u32::MAX; w * h];
    let is_edge = |x: usize, y: usize| {
        let i = y * w + x;
        let me = inside[i];
        let mut edge = false;
        if x > 0 && inside[i - 1] != me {
            edge = true;
        }
        if x + 1 < w && inside[i + 1] != me {
            edge = true;
        }
        if y > 0 && inside[i - w] != me {
            edge = true;
        }
        if y + 1 < h && inside[i + w] != me {
            edge = true;
        }
        edge
    };
    for y in 0..h {
        for x in 0..w {
            if is_edge(x, y) {
                dist[y * w + x] = 0;
            }
        }
    }
    // Forward pass.
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut d = dist[i];
            if x > 0 {
                d = d.min(dist[i - 1].saturating_add(3));
            }
            if y > 0 {
                d = d.min(dist[i - w].saturating_add(3));
            }
            if x > 0 && y > 0 {
                d = d.min(dist[i - w - 1].saturating_add(4));
            }
            if x + 1 < w && y > 0 {
                d = d.min(dist[i - w + 1].saturating_add(4));
            }
            dist[i] = d;
        }
    }
    // Backward pass.
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            let mut d = dist[i];
            if x + 1 < w {
                d = d.min(dist[i + 1].saturating_add(3));
            }
            if y + 1 < h {
                d = d.min(dist[i + w].saturating_add(3));
            }
            if x + 1 < w && y + 1 < h {
                d = d.min(dist[i + w + 1].saturating_add(4));
            }
            if x > 0 && y + 1 < h {
                d = d.min(dist[i + w - 1].saturating_add(4));
            }
            dist[i] = d;
        }
    }
    // Convert chamfer units (max 4 per diagonal) to pixels (max 1.41): /4.
    dist.iter()
        .map(|&d| if d == u32::MAX { u32::MAX } else { d / 4 })
        .collect()
}

/// Refine the coverage of `mask` against the composite `image`.
///
/// Pipeline (CS6's documented order): edge-band re-estimation, then Smooth,
/// Feather, Contrast, and Shift Edge. Confined to the boundary band: no pixel
/// farther than the effective radius from the input contour changes. Defaults
/// are the identity.
pub fn refine(
    mask: &Selection,
    image: &PixelBuffer,
    settings: &RefineEdgeSettings,
) -> Result<Selection, SelectError> {
    if mask.width != image.width || mask.height != image.height {
        return Err(SelectError::SizeMismatch(format!(
            "{}x{} vs image {}x{}",
            mask.width, mask.height, image.width, image.height
        )));
    }
    if settings.radius > MAX_RADIUS
        || settings.smooth > MAX_AMOUNT
        || settings.contrast > MAX_AMOUNT
        || settings.amount > MAX_AMOUNT
        || settings.feather > MAX_RADIUS
        || !(-MAX_SHIFT..=MAX_SHIFT).contains(&settings.shift_edge)
    {
        return Err(SelectError::InvalidParams("refine settings".into()));
    }

    let identity = settings.radius == 0
        && settings.smooth == 0
        && settings.feather == 0
        && settings.contrast == 0
        && settings.shift_edge == 0;
    if identity {
        return Ok(mask.clone());
    }

    let (w, h) = (mask.width as usize, mask.height as usize);
    let mut coverage: Vec<f64> = mask.data.iter().map(|&v| v as f64).collect();

    // 1. Edge band: re-estimate coverage from the image inside the band, blended
    // back to the input so the band edge stays continuous.
    if settings.radius > 0 {
        let dist = boundary_distance(mask);
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let r = if settings.smart_radius {
                    smart_band_radius(mask, image, x, y, settings.radius)
                } else {
                    settings.radius
                };
                if r == 0 || dist[i] > r {
                    continue;
                }
                // The band edge blends from the re-estimated value at the
                // contour back to the input at distance r.
                let t = dist[i] as f64 / r as f64;
                let estimated = estimate_coverage(image, w, h, x, y, mask.data[i]);
                coverage[i] = estimated * (1.0 - t) + coverage[i] * t;
            }
        }
    }

    let mut sel = Selection {
        width: mask.width,
        height: mask.height,
        data: coverage
            .iter()
            .map(|&v| v.round().clamp(0.0, 255.0) as u8)
            .collect(),
    };

    // 2/3. Smooth and Feather use the existing modify operations.
    if settings.smooth > 0 {
        sel = sel.smooth(settings.smooth);
    }
    if settings.feather > 0 {
        sel = sel.feather(settings.feather as f64);
    }

    // 4. Contrast: steepen the coverage ramp about 128.
    if settings.contrast > 0 {
        let c = settings.contrast as f64 / 100.0;
        for v in &mut sel.data {
            let x = *v as f64 / 255.0;
            // S-curve whose slope at 0.5 grows with c; c=0 keeps the identity.
            let steepened = if x < 0.5 {
                (0.5 * (2.0 * x).powf(1.0 + 2.0 * c)).clamp(0.0, 1.0)
            } else {
                (1.0 - 0.5 * (2.0 * (1.0 - x)).powf(1.0 + 2.0 * c)).clamp(0.0, 1.0)
            };
            *v = (steepened * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }

    // 5. Shift Edge: move the 50 % contour along the signed distance field.
    if settings.shift_edge != 0 {
        sel = shift_edge(&sel, &signed_distance(&sel), settings.shift_edge);
    }

    Ok(sel)
}

/// A re-estimated alpha inside the band. The image edge gives a sub-pixel
/// position; coverage moves toward the image where the local gradient is
/// strong, otherwise it stays near the input. `input` is the current coverage.
fn estimate_coverage(
    image: &PixelBuffer,
    w: usize,
    h: usize,
    x: usize,
    y: usize,
    input: u8,
) -> f64 {
    let lum = |xi: usize, yi: usize| {
        let i = yi * w + xi;
        let n = w * h;
        let v = if image.channels >= 3 {
            (image.data[i] as i32 * 299
                + image.data[n + i] as i32 * 587
                + image.data[2 * n + i] as i32 * 114)
                / 1000
        } else {
            image.data[i] as i32
        };
        v as f64
    };
    let x0 = x.saturating_sub(1);
    let y0 = y.saturating_sub(1);
    let x1 = (x + 1).min(w - 1);
    let y1 = (y + 1).min(h - 1);
    let gx = lum(x1, y) - lum(x0, y);
    let gy = lum(x, y1) - lum(x, y0);
    let magnitude = (gx * gx + gy * gy).sqrt();
    // A strong gradient pulls partial coverage toward the mid-point; a weak one
    // leaves the input unchanged.
    let strength = (magnitude / 255.0).clamp(0.0, 1.0);
    let mid = 127.5;
    let base = input as f64;
    base + (mid - base) * strength * 0.5
}

/// Signed distance from the 50 % contour: positive inside, negative outside.
fn signed_distance(sel: &Selection) -> Vec<i32> {
    let dist = boundary_distance(sel);
    sel.data
        .iter()
        .zip(&dist)
        .map(|(&v, &d)| {
            let d = if d == u32::MAX {
                i32::MAX / 2
            } else {
                d as i32
            };
            if v >= 128 {
                d
            } else {
                -d
            }
        })
        .collect()
}

/// Shift the 50 % contour by `percent` percent of a fixed reference band along
/// the signed distance field. Negative moves inward (erode), positive outward
/// (dilate). Partial coverage on the ramp is re-thresholded by the shifted
/// distance, so the shift is monotonic in `percent`.
fn shift_edge(sel: &Selection, signed: &[i32], percent: i32) -> Selection {
    if percent == 0 {
        return sel.clone();
    }
    let (w, h) = (sel.width as usize, sel.height as usize);
    // Reference band: 16 px maps to 100 %, so the contour moves by
    // `percent / 100 * 16` px.
    let shift = (percent.abs() as f64 / 100.0 * 16.0).round() as i32;
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            // Boundary-relative signed distance after the shift.
            let moved = if percent > 0 {
                signed[i] + shift
            } else {
                signed[i] - shift
            };
            // A 1 px ramp at the new 50 % line keeps soft coverage soft; the
            // original value magnifies the shape of the existing edge.
            let ramp = ((moved as f64 + 0.5) * 255.0).clamp(0.0, 255.0);
            let value = sel.data[i] as f64;
            // Blend: mostly the new coverage, retaining a little of the input so
            // a feathered selection stays feathered.
            out[i] = (0.75 * ramp + 0.25 * value).round().clamp(0.0, 255.0) as u8;
        }
    }
    Selection {
        width: sel.width,
        height: sel.height,
        data: out,
    }
}

/// Replace colour fringes at soft edges with the colour of nearby fully
/// selected pixels.
///
/// For each pixel with partial alpha, estimate the foreground colour from
/// fully-opaque selected neighbours and lerp the colour toward it by
/// `(1 - alpha) * amount`. Returns a new buffer; the input is never mutated.
pub fn decontaminate(
    image: &PixelBuffer,
    alpha: &Selection,
    amount: u32,
) -> Result<PixelBuffer, SelectError> {
    if image.width != alpha.width || image.height != alpha.height {
        return Err(SelectError::SizeMismatch(format!(
            "alpha {}x{} vs image {}x{}",
            alpha.width, alpha.height, image.width, image.height
        )));
    }
    if amount > MAX_AMOUNT {
        return Err(SelectError::InvalidParams("amount".into()));
    }
    let (w, h) = (image.width as usize, image.height as usize);
    if w == 0 || h == 0 {
        return Ok(image.clone());
    }
    let channels = image.channels as usize;
    let n = w * h;
    let blend = amount as f64 / 100.0;
    // Radius in which we look for fully-opaque foreground neighbours.
    let search = 2isize;

    let sample = |c: usize, x: usize, y: usize| -> f64 {
        if channels >= 3 {
            image.data[c * n + y * w + x] as f64
        } else {
            image.data[y * w + x] as f64
        }
    };

    let mut out = image.data.to_vec();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let a = alpha.data[i] as f64 / 255.0;
            if a <= 0.0 || a >= 1.0 {
                continue;
            }
            // Gather fully-opaque selected neighbours.
            let mut sum = [0.0f64; 3];
            let mut count = 0u32;
            for dy in -search..=search {
                let yy = y as isize + dy;
                if yy < 0 || yy >= h as isize {
                    continue;
                }
                for dx in -search..=search {
                    let xx = x as isize + dx;
                    if xx < 0 || xx >= w as isize {
                        continue;
                    }
                    let j = yy as usize * w + xx as usize;
                    if alpha.data[j] < 255 {
                        continue;
                    }
                    let (xu, yu) = (xx as usize, yy as usize);
                    if channels >= 3 {
                        for (c, value) in sum.iter_mut().enumerate() {
                            *value += sample(c, xu, yu);
                        }
                    } else {
                        let g = sample(0, xu, yu);
                        for value in &mut sum {
                            *value += g;
                        }
                    }
                    count += 1;
                }
            }
            if count == 0 {
                // No usable foreground: leave the fringe rather than invent one.
                continue;
            }
            let foreground = [
                sum[0] / count as f64,
                sum[1] / count as f64,
                sum[2] / count as f64,
            ];
            let weight = (1.0 - a) * blend;
            let chans = channels.min(3);
            for c in 0..chans {
                let original = sample(c, x, y);
                let mixed = original + (foreground[c] - original) * weight;
                out[c * n + i] = mixed.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    Ok(PixelBuffer {
        width: image.width,
        height: image.height,
        channels: image.channels,
        data: out.into(),
    })
}

/// Build a full-frame [`Plane`] from refined coverage (used by the app to write
/// a layer mask).
pub fn coverage_plane(sel: &Selection) -> Plane<u8> {
    sel.data.clone().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gray(w: u32, h: u32, f: impl Fn(u32, u32) -> u8) -> PixelBuffer {
        let mut data = vec![0u8; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                data[(y * w + x) as usize] = f(x, y);
            }
        }
        PixelBuffer {
            width: w,
            height: h,
            channels: 1,
            data: data.into(),
        }
    }

    fn rect(w: u32, h: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> Selection {
        let mut s = Selection::none(w, h);
        for y in y0..y1 {
            for x in x0..x1 {
                s.data[(y * w + x) as usize] = 255;
            }
        }
        s
    }

    #[test]
    fn defaults_are_identity() {
        let mask = rect(10, 10, 2, 2, 7, 7);
        let image = gray(10, 10, |_, _| 128);
        let out = refine(&mask, &image, &RefineEdgeSettings::default()).unwrap();
        assert_eq!(out, mask);
    }

    #[test]
    fn larger_radius_reaches_farther_but_stays_bounded() {
        let mask = rect(20, 20, 5, 5, 15, 15);
        // A soft ramp so re-estimation changes coverage near the edge.
        let image = gray(20, 20, |x, _| (x * 12).min(255) as u8);
        let mut settings = RefineEdgeSettings {
            radius: 2,
            ..Default::default()
        };
        let small = refine(&mask, &image, &settings).unwrap();
        settings.radius = 6;
        let large = refine(&mask, &image, &settings).unwrap();
        let changed = |s: &Selection| {
            s.data
                .iter()
                .zip(&mask.data)
                .map(|(a, b)| a != b)
                .collect::<Vec<_>>()
        };
        let small_changed = changed(&small);
        let large_changed = changed(&large);
        assert!(large_changed.iter().filter(|&&c| c).count() > 0);
        // Superset: everything small changed, large changed too.
        for (i, &c) in small_changed.iter().enumerate() {
            if c {
                assert!(large_changed[i], "large radius must cover small at {i}");
            }
        }
        // Bounded: no changed pixel farther than the radius from the square edge.
        for (i, &c) in large_changed.iter().enumerate() {
            if !c {
                continue;
            }
            let (x, y) = ((i as u32) % 20, (i as u32) / 20);
            let dx = if x < 5 {
                5 - x as i32
            } else if x >= 15 {
                x as i32 - 14
            } else {
                0
            };
            let dy = if y < 5 {
                5 - y as i32
            } else if y >= 15 {
                y as i32 - 14
            } else {
                0
            };
            assert!(dx.max(dy) <= 6, "changed pixel {x},{y} escaped the band");
        }
    }

    #[test]
    fn out_of_range_parameters_are_rejected() {
        let mask = rect(8, 8, 2, 2, 6, 6);
        let image = gray(8, 8, |_, _| 100);
        for settings in [
            RefineEdgeSettings {
                radius: 101,
                ..Default::default()
            },
            RefineEdgeSettings {
                smooth: 101,
                ..Default::default()
            },
            RefineEdgeSettings {
                contrast: 101,
                ..Default::default()
            },
            RefineEdgeSettings {
                amount: 101,
                ..Default::default()
            },
            RefineEdgeSettings {
                shift_edge: -101,
                ..Default::default()
            },
        ] {
            assert!(matches!(
                refine(&mask, &image, &settings),
                Err(SelectError::InvalidParams(_))
            ));
        }
    }

    #[test]
    fn size_mismatch_is_an_error() {
        let mask = rect(8, 8, 0, 0, 4, 4);
        let image = gray(6, 6, |_, _| 0);
        assert!(matches!(
            refine(&mask, &image, &RefineEdgeSettings::default()),
            Err(SelectError::SizeMismatch(_))
        ));
        let alpha = rect(4, 4, 0, 0, 3, 3);
        assert!(matches!(
            decontaminate(&image, &alpha, 50),
            Err(SelectError::SizeMismatch(_))
        ));
    }

    #[test]
    fn contrast_is_monotonically_more_bimodal() {
        let mask = rect(20, 20, 5, 5, 15, 15).feather(4.0);
        let image = gray(20, 20, |_, _| 100);
        let intermediates = |contrast| {
            refine(
                &mask,
                &image,
                &RefineEdgeSettings {
                    contrast,
                    ..Default::default()
                },
            )
            .unwrap()
            .data
            .iter()
            .filter(|&&v| v > 0 && v < 255)
            .count()
        };
        let none = intermediates(0);
        let some = intermediates(50);
        let more = intermediates(100);
        assert!(some <= none, "contrast 50 must not add intermediates");
        assert!(more <= some, "contrast 100 must not add intermediates");
        assert!(more < none, "contrast must remove intermediates");
    }

    #[test]
    fn shift_edge_moves_the_contour() {
        let mask = rect(24, 24, 8, 8, 16, 16).feather(3.0);
        let image = gray(24, 24, |_, _| 100);
        let inward = refine(
            &mask,
            &image,
            &RefineEdgeSettings {
                shift_edge: -50,
                ..Default::default()
            },
        )
        .unwrap();
        let outward = refine(
            &mask,
            &image,
            &RefineEdgeSettings {
                shift_edge: 50,
                ..Default::default()
            },
        )
        .unwrap();
        // Count coverage mass on each side of the 50 % line near the left edge.
        let mass = |s: &Selection| -> i64 { s.data.iter().map(|&v| v as i64).sum() };
        assert!(mass(&inward) < mass(&outward));
    }

    #[test]
    fn smooth_removes_boundary_specks() {
        let mut mask = rect(24, 24, 6, 6, 18, 18);
        // Isolated specks just outside the boundary.
        mask.data[(5 * 24 + 5) as usize] = 255;
        mask.data[(5 * 24 + 18) as usize] = 255;
        let image = gray(24, 24, |_, _| 100);
        let specks = |s: &Selection| {
            s.data
                .iter()
                .enumerate()
                .filter(|(i, &v)| {
                    v > 0
                        && (i % 24) >= 5
                        && (i % 24) <= 18
                        && (i / 24) >= 5
                        && (i / 24) <= 18
                        && (i % 24 < 6 || i % 24 > 17 || i / 24 < 6 || i / 24 > 17)
                })
                .count()
        };
        let before = specks(&mask);
        let after = refine(
            &mask,
            &image,
            &RefineEdgeSettings {
                smooth: 3,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(specks(&after) < before, "smooth must remove specks");
    }

    #[test]
    fn decontaminate_moves_fringe_toward_foreground() {
        // A red foreground at left, a green fringe at right, alpha 128 on fringe.
        let w = 8u32;
        let h = 1u32;
        let mut data = vec![0u8; (w * h) as usize * 3];
        for x in 0..w {
            let (r, g) = if x < 4 { (255, 0) } else { (0, 255) };
            data[x as usize] = r;
            data[(w + x) as usize] = g;
        }
        let image = PixelBuffer {
            width: w,
            height: h,
            channels: 3,
            data: data.into(),
        };
        let mut alpha = Selection::none(w, h);
        for x in 0..4u32 {
            alpha.data[x as usize] = 255;
        }
        alpha.data[4] = 128;
        let out = decontaminate(&image, &alpha, 100).unwrap();
        // The fringe pixel at x=4 moves toward red.
        assert!(out.data[4] > 0, "red channel rises");
        assert!(out.data[w as usize + 4] < 255, "green channel falls");
        // Fully opaque / fully transparent pixels are untouched.
        assert_eq!(out.data[0], image.data[0]);
        assert_eq!(out.data[w as usize], image.data[w as usize]);
        let full = Selection::all(w, h);
        let untouched = decontaminate(&image, &full, 100).unwrap();
        assert_eq!(untouched, image);
    }

    #[test]
    fn decontaminate_rejects_bad_amount() {
        let image = gray(4, 4, |_, _| 0);
        let alpha = rect(4, 4, 0, 0, 2, 2);
        assert!(matches!(
            decontaminate(&image, &alpha, 101),
            Err(SelectError::InvalidParams(_))
        ));
    }

    #[test]
    fn view_mode_and_output_indexing() {
        assert_eq!(ViewMode::from_index(0), Some(ViewMode::MarchingAnts));
        assert_eq!(ViewMode::from_index(6), Some(ViewMode::RevealLayer));
        assert_eq!(ViewMode::from_index(7), None);
        assert_eq!(
            OutputTarget::from_index(3),
            Some(OutputTarget::NewLayerWithMask)
        );
        assert_eq!(OutputTarget::from_index(4), None);
        assert!(!OutputTarget::Selection.is_color_output());
        assert!(OutputTarget::NewLayer.is_color_output());
    }
}
