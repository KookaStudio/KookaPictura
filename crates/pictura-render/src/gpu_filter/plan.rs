use pictura_filters::blur::{BLUR_MORE_RADIUS, BLUR_RADIUS};
use pictura_filters::kernel::{gaussian_kernel, sigma_from_radius};
use pictura_filters::sharpen::{SHARPEN_AMOUNT, SHARPEN_MORE_AMOUNT, SHARPEN_RADIUS};
use pictura_filters::{Filter, Quality, RadialMethod};

use super::MAX_WEIGHTS;

/// Largest Surface Blur radius the GPU runs; measured on a 1024² buffer, the
/// CPU overtakes the shader between radius 16 and 30.
const SURFACE_GPU_MAX_RADIUS: u32 = 16;

/// One GPU execution plan for a filter, mirroring the CPU kernel construction.
pub(super) enum Plan {
    Kernel {
        weights: Vec<f32>,
        k: u32,
        norm: f32,
        offset: f32,
    },
    Separable(SepPlan),
    Motion {
        angle: f32,
        taps: u32,
    },
    /// Radial Blur's Zoom: `samples` taps along the pixel's ray from `center`
    /// (normalized to the image), spanning `span` of its radius.
    RadialZoom {
        samples: u32,
        span: f32,
        center: (f32, f32),
    },
    Surface {
        radius: u32,
        threshold: u32,
    },
    Morph {
        radius: u32,
        dilate: bool,
    },
    Median {
        radius: u32,
    },
    OilPaint {
        along: i32,
        perp: i32,
        stop: f32,
        scale_t: f32,
        bristle: f32,
        relief_mag: f32,
        light: [f32; 3],
        half: [f32; 3],
        shine_t: f32,
    },
}

pub(super) enum Combine {
    None,
    HighPass,
    Unsharp { gain: f32, thr: f32 },
}

pub(super) struct SepPlan {
    pub(super) weights: Vec<f32>,
    pub(super) support: i32,
    pub(super) norm: f32,
    pub(super) combine: Combine,
}

/// Mirror `pictura_filters::apply`'s parameter validation and kernel choice.
/// `None` means "no GPU kernel for this filter or these parameters": fall back
/// to the CPU so the oracle still decides the bytes.
pub(super) fn plan(filter: &Filter) -> Option<Plan> {
    match filter {
        // Blur and the two Sharpens are a Gaussian and an Unsharp Mask at fixed
        // settings on the CPU, so they take those filters' plans.
        Filter::Blur => plan(&Filter::GaussianBlur {
            radius: BLUR_RADIUS,
        }),
        Filter::BlurMore => plan(&Filter::GaussianBlur {
            radius: BLUR_MORE_RADIUS,
        }),
        Filter::Sharpen => plan(&unsharp(SHARPEN_AMOUNT)),
        Filter::SharpenMore => plan(&unsharp(SHARPEN_MORE_AMOUNT)),
        Filter::GaussianBlur { radius } => {
            if !radius.is_finite() || *radius < 0.0 || *radius == 0.0 {
                return None;
            }
            separable_sigma(*radius, Combine::None)
        }
        Filter::BoxBlur { radius } => {
            if *radius == 0 {
                return None;
            }
            let r = *radius as usize;
            if 2 * r + 1 > MAX_WEIGHTS {
                return None;
            }
            Some(Plan::Separable(SepPlan {
                weights: vec![1.0; 2 * r + 1],
                support: r as i32,
                norm: (2 * r + 1) as f32,
                combine: Combine::None,
            }))
        }
        Filter::MotionBlur { angle, distance } => {
            if !angle.is_finite() || !(-360.0..=360.0).contains(angle) {
                return None;
            }
            if !(1..=999).contains(distance) {
                return None;
            }
            Some(Plan::Motion {
                angle: *angle as f32,
                taps: *distance,
            })
        }
        Filter::RadialBlur {
            method: RadialMethod::Zoom,
            amount,
            center,
            quality,
        } => {
            // Zero is the identity; the CPU returns it untouched.
            if !amount.is_finite() || *amount <= 0.0 || *amount > 100.0 {
                return None;
            }
            if !center.0.is_finite() || !center.1.is_finite() {
                return None;
            }
            Some(Plan::RadialZoom {
                samples: match quality {
                    Quality::Draft => 8,
                    Quality::Good => 16,
                    Quality::Best => 32,
                },
                span: (amount / 100.0) as f32,
                center: (
                    center.0.clamp(0.0, 1.0) as f32,
                    center.1.clamp(0.0, 1.0) as f32,
                ),
            })
        }
        Filter::UnsharpMask {
            amount,
            radius,
            threshold,
        } => {
            if !(1.0..=500.0).contains(amount) {
                return None;
            }
            if !radius.is_finite() || *radius <= 0.0 {
                return None;
            }
            separable_sigma(
                *radius,
                Combine::Unsharp {
                    gain: (*amount / 100.0) as f32,
                    thr: f32::from(*threshold),
                },
            )
        }
        Filter::HighPass { radius } => {
            if !radius.is_finite() || *radius <= 0.0 {
                return None;
            }
            separable_sigma(*radius, Combine::HighPass)
        }
        Filter::SurfaceBlur { radius, threshold } => {
            if !(1..=100).contains(radius) || *threshold == 0 {
                return None;
            }
            // The CPU slides a histogram (O(r) per pixel) where the shader
            // scans the window (O(r²)); past this radius the CPU is faster.
            if *radius > SURFACE_GPU_MAX_RADIUS {
                return None;
            }
            Some(Plan::Surface {
                radius: *radius,
                threshold: u32::from(*threshold),
            })
        }
        Filter::Maximum { radius } => {
            let r = (*radius).min(100);
            if r == 0 {
                return None;
            }
            Some(Plan::Morph {
                radius: r,
                dilate: true,
            })
        }
        Filter::Minimum { radius } => {
            let r = (*radius).min(100);
            if r == 0 {
                return None;
            }
            Some(Plan::Morph {
                radius: r,
                dilate: false,
            })
        }
        Filter::Median { radius } => {
            if *radius == 0 || *radius > 100 {
                return None;
            }
            Some(Plan::Median { radius: *radius })
        }
        Filter::Custom {
            kernel,
            scale,
            offset,
        } => {
            let scale32 = *scale as f32;
            let offset32 = *offset as f32;
            if !scale.is_finite() || scale32 == 0.0 || !scale32.is_finite() {
                return None;
            }
            if !offset.is_finite() || !offset32.is_finite() {
                return None;
            }
            if kernel.iter().flatten().any(|k| !(*k as f32).is_finite()) {
                return None;
            }
            Some(Plan::Kernel {
                weights: kernel.iter().flatten().map(|&v| v as f32).collect(),
                k: 5,
                norm: scale32,
                offset: offset32,
            })
        }
        Filter::OilPaint {
            stylization,
            cleanliness,
            scale,
            bristle_detail,
            angular_direction,
            shine,
        } => {
            if [stylization, cleanliness, scale, bristle_detail, shine]
                .into_iter()
                .any(|v| !unit(*v))
            {
                return None;
            }
            if !angular_direction.is_finite() || !(0.0..=360.0).contains(angular_direction) {
                return None;
            }
            let along = (1.0 + cleanliness * 0.9).round().max(1.0) as i32;
            let perp = (stylization * 0.4).round() as i32;
            let azimuth = angular_direction.to_radians();
            let light = normalize3((azimuth.cos(), azimuth.sin(), 0.6));
            let half = normalize3((light.0, light.1, light.2 + 1.0));
            Some(Plan::OilPaint {
                along,
                perp,
                stop: (10.0 + stylization * 8.0) as f32,
                scale_t: (scale / 10.0) as f32,
                bristle: (bristle_detail / 10.0) as f32,
                relief_mag: (1.0 + scale * 1.5) as f32,
                light: [light.0 as f32, light.1 as f32, light.2 as f32],
                half: [half.0 as f32, half.1 as f32, half.2 as f32],
                shine_t: (shine / 10.0) as f32,
            })
        }
        _ => None,
    }
}

fn unit(v: f64) -> bool {
    v.is_finite() && (0.0..=10.0).contains(&v)
}

fn normalize3(v: (f64, f64, f64)) -> (f64, f64, f64) {
    let len = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
    if len < 1e-9 {
        (0.0, 0.0, 1.0)
    } else {
        (v.0 / len, v.1 / len, v.2 / len)
    }
}

fn unsharp(amount: f64) -> Filter {
    Filter::UnsharpMask {
        amount,
        radius: SHARPEN_RADIUS,
        threshold: 0,
    }
}

fn separable_sigma(radius: f64, combine: Combine) -> Option<Plan> {
    let kernel = gaussian_kernel(sigma_from_radius(radius));
    let support = (kernel.len() / 2) as i32;
    if kernel.len() > MAX_WEIGHTS {
        return None;
    }
    Some(Plan::Separable(SepPlan {
        weights: kernel.into_iter().map(|w| w as f32).collect(),
        support,
        norm: 1.0,
        combine,
    }))
}
