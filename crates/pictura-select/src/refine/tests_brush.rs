//! Brush-radius tests split from `refine.rs` to keep the code file under its
//! size cap.

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
fn effective_band_radius_matches_the_brush_rule() {
    assert_eq!(effective_band_radius(2, 10, 5, true), 10);
    assert_eq!(effective_band_radius(7, 10, 5, true), 10);
    assert_eq!(effective_band_radius(2, 10, 5, false), 2);
}

#[test]
fn slider_radius_still_refines_outside_a_small_dab() {
    let mask = rect(24, 24, 4, 4, 20, 20);
    // A hard image edge at x = 8 makes the smart radius collapse to 1 there,
    // so only the slider floor can reach 4 px from the contour.
    let image = gray(24, 24, |x, _| if x < 8 { 0 } else { 255 });
    let settings = RefineEdgeSettings {
        smart_radius: true,
        radius: 5,
        ..Default::default()
    };
    let i = 12 * 24 + 8;
    let out = refine_from(
        &mask,
        ImageSource::Buffer(&image),
        &settings,
        &[RefineDab {
            x: 8.0,
            y: 12.0,
            radius: 2.0,
            erase: false,
        }],
    )
    .unwrap();
    assert_ne!(
        out.data[i], mask.data[i],
        "the slider radius still refines four pixels from the contour"
    );
}
