//! The scroll/zoom/pan crop-vs-rescale profile (#1066). Split out of
//! `tests_profiles.rs` to keep that file under its size cap.

use super::{ms, VIEW};
use pictura_core::{BitDepth, ColorMode, Document, PixelBuffer, PsdRect};
use pictura_render::{Planes, ViewPyramid};

/// The C++ `ImageView::presentLevelForZoom` formula, so the profile crops the
/// level the canvas would. ponytail: mirrored, not shared across the FFI.
fn present_level(zoom: f64, level_count: usize) -> usize {
    let mut level = 0;
    while level + 1 < level_count && 1.0 / (1u64 << (level + 1)) as f64 >= zoom {
        level += 1;
    }
    level
}

/// A naive nearest-neighbour rescale of the full-resolution composite to a
/// viewport-sized buffer: the old present path that read every source row.
fn naive_rescale(src: &Planes<'_>, w: u32, h: u32) -> Vec<u8> {
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        let sy = (y as u64 * src.height as u64 / h as u64) as usize;
        for x in 0..w {
            let sx = (x as u64 * src.width as u64 / w as u64) as usize;
            let i = sy * src.width as usize + sx;
            let o = ((y * w + x) * 4) as usize;
            out[o] = src.r[i];
            out[o + 1] = src.g[i];
            out[o + 2] = src.b[i];
            out[o + 3] = src.a[i];
        }
    }
    out
}

fn scroll_zoom_pan_profile(n: u32) {
    let plane = n as usize * n as usize;
    let mut data = vec![0u8; plane * 4];
    // Sparse touch so the pages are resident and the premultiply/average is not
    // a no-op, without an O(n²) fill. ponytail: sparse fill, real pixels if the
    // timing ever needs texture.
    for i in (0..plane).step_by(4096) {
        data[i] = (i % 251) as u8;
        data[3 * plane + i] = 255;
    }
    let mut doc = Document::new(n, n, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = PixelBuffer {
        width: n,
        height: n,
        channels: 4,
        data: data.into(),
    };
    let planes = Planes {
        width: n,
        height: n,
        r: &doc.composite.data[..plane],
        g: &doc.composite.data[plane..2 * plane],
        b: &doc.composite.data[2 * plane..3 * plane],
        a: &doc.composite.data[3 * plane..4 * plane],
    };

    let t = std::time::Instant::now();
    let pyramid = ViewPyramid::rebuild(planes);
    ms(&format!("scroll_zoom_pan_profile_{n} rebuild"), t.elapsed());

    let zoom = f64::min(VIEW.0 as f64 / n as f64, VIEW.1 as f64 / n as f64);
    let level = present_level(zoom, pyramid.level_count());
    let (lw, lh) = pyramid.level_size(level);
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: lh as i32,
        right: lw as i32,
    };
    let t = std::time::Instant::now();
    let crop = pyramid.crop(planes, level, rect);
    ms(
        &format!(
            "scroll_zoom_pan_profile_{n} level_crop {}x{}",
            crop.width(),
            crop.height()
        ),
        t.elapsed(),
    );

    let t = std::time::Instant::now();
    let _rescaled = naive_rescale(&planes, VIEW.0, VIEW.1);
    ms(
        &format!("scroll_zoom_pan_profile_{n} full_rescale"),
        t.elapsed(),
    );

    // A zoom-in past 50 % presents level 0: the viewport is cropped from the
    // planar level 0, the cost the pan/zoom crop reuse stops re-paying whole
    // every frame (#1066).
    let vw = VIEW.0.min(n) as i32;
    let vh = VIEW.1.min(n) as i32;
    let view0 = PsdRect {
        top: 0,
        left: 0,
        bottom: vh,
        right: vw,
    };
    let t = std::time::Instant::now();
    let crop0 = pyramid.crop(planes, 0, view0);
    ms(
        &format!(
            "scroll_zoom_pan_profile_{n} level0_crop {}x{}",
            crop0.width(),
            crop0.height()
        ),
        t.elapsed(),
    );

    // The reuse path crops only the exposed band of a 200 px pan.
    let band = PsdRect {
        top: 0,
        left: 0,
        bottom: vh,
        right: 200.min(vw),
    };
    let t = std::time::Instant::now();
    let crop_band = pyramid.crop(planes, 0, band);
    ms(
        &format!(
            "scroll_zoom_pan_profile_{n} level0_pan_band {}x{}",
            crop_band.width(),
            crop_band.height()
        ),
        t.elapsed(),
    );
}

/// Print-only evidence that a viewport-sized pyramid-level crop is cheaper than
/// rescaling the full-resolution composite. No pass/fail budget (the reference
/// machine is not pinned).
#[test]
#[ignore = "4000x4000 crop-vs-rescale profile; run explicitly with --ignored --nocapture"]
fn scroll_zoom_pan_profile_4000() {
    scroll_zoom_pan_profile(4000);
}

#[test]
#[ignore = "16000x16000 crop-vs-rescale profile; run explicitly with --ignored --nocapture"]
fn scroll_zoom_pan_profile_16000() {
    scroll_zoom_pan_profile(16000);
}
