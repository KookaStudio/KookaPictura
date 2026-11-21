use super::ffi::{CanvasFrameMeasurement, FrameCropRect};
use super::PictureView;
use crate::cxxqt_object::metrics::{self, FrameRecord};
use cxx_qt_lib::QImage;
use pictura_core::PsdRect;
use pictura_render::FrameTileSamples;

pub struct FrameCropSamples(FrameTileSamples);

fn bounds(rect: FrameCropRect) -> PsdRect {
    PsdRect {
        left: rect.x,
        top: rect.y,
        right: rect.x.saturating_add(rect.w),
        bottom: rect.y.saturating_add(rect.h),
    }
}

pub fn frame_capture_generation() -> u64 {
    metrics::frame_generation()
}

pub fn frame_capture_record(generation: u64, record: CanvasFrameMeasurement) {
    metrics::record_frame(
        generation,
        FrameRecord {
            canvas_id: record.canvas_id,
            input_count: record.input_count,
            input_ns: record.input_ns,
            tile_gather_ns: record.tile_gather_ns,
            paint_ns: record.paint_ns,
            interval_ns: u64::try_from(record.interval_ns).ok(),
            gpu_ns: u64::try_from(record.gpu_ns).ok(),
        },
    );
}

pub fn frame_samples_new() -> Box<FrameCropSamples> {
    Box::new(FrameCropSamples(FrameTileSamples::default()))
}

pub fn frame_samples_crop(
    view: &PictureView,
    level: i32,
    rect: FrameCropRect,
    samples: &mut FrameCropSamples,
) -> QImage {
    let image =
        view.canvas_display_crop(level, rect.x, rect.y, rect.w, rect.h, Some(&mut samples.0));
    samples.0.retain_generation(frame_capture_generation());
    image
}

pub fn frame_samples_copy(
    target: &mut FrameCropSamples,
    source: &FrameCropSamples,
    area: FrameCropRect,
) {
    target.0.copy_clipped_from(&source.0, bounds(area), 1);
}

pub fn frame_samples_erase(samples: &mut FrameCropSamples, area: FrameCropRect) {
    samples.0.overwrite(bounds(area), None);
}

pub fn frame_samples_empty(samples: &FrameCropSamples) -> bool {
    samples.0.is_empty()
}

pub fn frame_samples_prune_empty(samples: &mut FrameCropSamples, generation: u64) -> bool {
    samples.0.retain_generation(generation);
    samples.0.is_empty()
}

pub fn frame_samples_rects(samples: &mut FrameCropSamples, generation: u64) -> Vec<FrameCropRect> {
    samples.0.retain_generation(generation);
    samples
        .0
        .rects()
        .map(|rect| FrameCropRect {
            x: rect.left,
            y: rect.top,
            w: rect.width(),
            h: rect.height(),
        })
        .collect()
}

pub fn frame_samples_take(samples: &FrameCropSamples, index: usize, generation: u64) -> i64 {
    samples
        .0
        .take(index, generation)
        .map_or(-1, |ns| ns.min(i64::MAX as u64) as i64)
}

pub fn frame_samples_seed_for_test(samples: &mut FrameCropSamples, rect: FrameCropRect, ns: u64) {
    let sample = metrics::new_frame_sample(Some(ns));
    samples.0.overwrite(bounds(rect), sample.as_ref());
}
