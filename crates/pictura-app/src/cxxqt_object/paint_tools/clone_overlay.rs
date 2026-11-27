use super::super::helpers::active_pixel_layer;
use super::super::helpers_composite::{layer_buffer_within, level_display_image_data};
use super::super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QImage;
use pictura_core::{Document, PixelBuffer, PsdRect};
use pictura_paint::healing::RgbaImage;
use pictura_paint::stamp::{sample_scope, CloneSampling};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceKey {
    generation: u64,
    revision: u64,
    canvas_revision: u64,
    path: String,
    scope: i32,
    ignore_adjustments: bool,
    snapshot_id: usize,
}

#[derive(Clone)]
struct Request {
    key: SourceKey,
    needed: PsdRect,
    level: usize,
    snapshot: Option<Arc<RgbaImage>>,
}

struct OverlayImage {
    width: i32,
    height: i32,
    data: Vec<u8>,
}

struct Running {
    request: Request,
    area: PsdRect,
    handle: JoinHandle<Option<OverlayImage>>,
    cancelled: Arc<AtomicBool>,
}

struct Cached {
    key: SourceKey,
    area: PsdRect,
    level: usize,
    id: u64,
    image: OverlayImage,
    delivered: bool,
    _snapshot: Option<Arc<RgbaImage>>,
}

#[derive(Default)]
pub(in crate::cxxqt_object) struct OverlayJobs {
    running: Option<Running>,
    cache: Option<Cached>,
    failed: Option<Request>,
    serial: u64,
    delay_for_test: u64,
}

impl OverlayJobs {
    fn cancel(&mut self) {
        if let Some(running) = self.running.as_ref() {
            running.cancelled.store(true, Ordering::Relaxed);
        }
        self.cache = None;
        self.failed = None;
    }
}

impl Drop for OverlayJobs {
    fn drop(&mut self) {
        if let Some(running) = self.running.as_ref() {
            running.cancelled.store(true, Ordering::Relaxed);
        }
    }
}

fn contains(area: PsdRect, needed: PsdRect) -> bool {
    area.left <= needed.left
        && area.top <= needed.top
        && area.right >= needed.right
        && area.bottom >= needed.bottom
}

fn serves(key: &SourceKey, area: PsdRect, level: usize, request: &Request) -> bool {
    *key == request.key && level <= request.level && contains(area, request.needed)
}

fn can_land(
    key: &SourceKey,
    area: PsdRect,
    level: usize,
    cancelled: bool,
    request: &Request,
) -> bool {
    !cancelled && serves(key, area, level, request)
}

fn expanded(request: &Request, width: u32, height: u32) -> PsdRect {
    let rect = request.needed;
    let dx = rect.width().max(4) / 4;
    let dy = rect.height().max(4) / 4;
    let step = 1i32 << request.level;
    let left = rect.left.saturating_sub(dx).max(0) / step;
    let top = rect.top.saturating_sub(dy).max(0) / step;
    let right = rect
        .right
        .saturating_add(dx)
        .min(width as i32)
        .saturating_add(step - 1)
        / step;
    let bottom = rect
        .bottom
        .saturating_add(dy)
        .min(height as i32)
        .saturating_add(step - 1)
        / step;
    PsdRect {
        left: left * step,
        top: top * step,
        right: right * step,
        bottom: bottom * step,
    }
}

fn render(
    doc: Document,
    request: Request,
    area: PsdRect,
    cancelled: &AtomicBool,
) -> Option<OverlayImage> {
    if cancelled.load(Ordering::Relaxed) {
        return None;
    }
    let scope = if request.snapshot.is_none() && request.key.scope != 0 {
        Some(sample_scope(
            &doc,
            &request.key.path,
            CloneSampling::from_i32(request.key.scope).unwrap_or_default(),
            request.key.ignore_adjustments,
        )?)
    } else {
        None
    };
    let layer = if request.snapshot.is_none() && scope.is_none() {
        Some(active_pixel_layer(&doc, Some(&request.key.path))?)
    } else {
        None
    };
    let read = |part: PsdRect| {
        if let Some(scope) = &scope {
            let buffer = pictura_render::composite_region(scope, part);
            return pictura_codec::buffer_to_srgb(&doc, &buffer).into_owned();
        }
        let (w, h) = (part.width() as usize, part.height() as usize);
        let plane = w * h;
        let mut data = vec![0u8; plane * 4];
        if let Some(snapshot) = &request.snapshot {
            for y in 0..h {
                for x in 0..w {
                    let i = ((part.top as usize + y) * snapshot.width as usize)
                        + part.left as usize
                        + x;
                    if let Some(pixel) = snapshot.data.get(i) {
                        for c in 0..4 {
                            data[c * plane + y * w + x] = pixel[c];
                        }
                    }
                }
            }
        } else if let Some(layer) = layer {
            if let Some(intersection) = part.intersect(layer.rect) {
                if let Some(src) = layer_buffer_within(layer, intersection) {
                    let sw = src.width as usize;
                    let sh = src.height as usize;
                    let ox = (intersection.left - part.left) as usize;
                    let oy = (intersection.top - part.top) as usize;
                    for c in 0..4 {
                        let channel = if src.channels == 2 {
                            if c == 3 {
                                1
                            } else {
                                0
                            }
                        } else {
                            c
                        };
                        for y in 0..sh {
                            let row = src.data.row(channel * sh + y, sw).unwrap_or_default();
                            let to = c * plane + (oy + y) * w + ox;
                            if row.len() == sw {
                                data[to..to + sw].copy_from_slice(&row);
                            }
                        }
                    }
                }
            }
        }
        let buffer = PixelBuffer {
            width: w as u32,
            height: h as u32,
            channels: 4,
            data: data.into(),
        };
        pictura_codec::buffer_to_srgb(&doc, &buffer).into_owned()
    };
    let step = 1i32 << request.level;
    let reduced = PsdRect {
        left: area.left / step,
        top: area.top / step,
        right: area.right / step,
        bottom: area.bottom / step,
    };
    let width = reduced.width();
    let height = reduced.height();
    let mut output = vec![0u8; (width as usize) * (height as usize) * 4];
    let side = (512i32 >> request.level.min(9)).max(1);
    let mut y = reduced.top;
    while y < reduced.bottom {
        let bottom = (y + side).min(reduced.bottom);
        let mut x = reduced.left;
        while x < reduced.right {
            if cancelled.load(Ordering::Relaxed) {
                return None;
            }
            let right = (x + side).min(reduced.right);
            let tile = pictura_render::region_level(
                &read,
                (doc.width, doc.height),
                PsdRect {
                    left: x,
                    top: y,
                    right,
                    bottom,
                },
                request.level,
            );
            let row_bytes = (right - x) as usize * 4;
            for row in 0..(bottom - y) as usize {
                let from = row * row_bytes;
                let to = (((y - reduced.top) as usize + row) * width as usize
                    + (x - reduced.left) as usize)
                    * 4;
                output[to..to + row_bytes].copy_from_slice(&tile.data()[from..from + row_bytes]);
            }
            x = right;
        }
        y = bottom;
    }
    Some(OverlayImage {
        width,
        height,
        data: output,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn clone_overlay_source(
    mut view: Pin<&mut PictureView>,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    scope: i32,
    ignore_adjustments: bool,
    snapshot: bool,
    actual_left: &mut i32,
    actual_top: &mut i32,
    actual_step: &mut i32,
    image_id: &mut u64,
    busy: &mut bool,
) -> QImage {
    *image_id = 0;
    *busy = false;
    let mut rust = view.as_mut().rust_mut();
    let Some(doc) = rust.session.doc.as_ref() else {
        rust.clone_overlay_jobs.cancel();
        return QImage::default();
    };
    let Some(path) = rust.layer_target.single() else {
        rust.clone_overlay_jobs.cancel();
        return QImage::default();
    };
    let canvas = PsdRect {
        left: 0,
        top: 0,
        right: doc.width as i32,
        bottom: doc.height as i32,
    };
    let requested = PsdRect {
        left,
        top,
        right: left.saturating_add(width.max(0)),
        bottom: top.saturating_add(height.max(0)),
    };
    let Some(needed) = requested.intersect(canvas) else {
        rust.clone_overlay_jobs.cancel();
        return QImage::default();
    };
    let saved = if snapshot {
        let Some(image) = rust
            .session
            .live
            .stroke()
            .and_then(|s| s.shared_source_image())
        else {
            rust.clone_overlay_jobs.cancel();
            return QImage::default();
        };
        Some(image)
    } else {
        None
    };
    let mut level = 0usize;
    while (needed.width().max(needed.height()) as u32).div_ceil(1u32 << level) > 2048 && level < 20
    {
        level += 1;
    }
    let key = SourceKey {
        generation: rust.session.document_generation,
        revision: if snapshot {
            0
        } else {
            rust.session.content_revision
        },
        canvas_revision: if snapshot { 0 } else { rust.canvas_revision },
        path: path.to_owned(),
        scope,
        ignore_adjustments,
        snapshot_id: saved
            .as_ref()
            .map_or(0, |image| Arc::as_ptr(image) as usize),
    };
    let request = Request {
        key,
        needed,
        level,
        snapshot: saved,
    };
    let dimensions = (doc.width, doc.height);
    let jobs = &mut rust.clone_overlay_jobs;
    if let Some(running) = jobs.running.as_ref() {
        if !serves(
            &running.request.key,
            running.area,
            running.request.level,
            &request,
        ) {
            running.cancelled.store(true, Ordering::Relaxed);
        }
    }
    if jobs
        .running
        .as_ref()
        .is_some_and(|r| r.handle.is_finished())
    {
        let running = jobs.running.take().expect("finished worker");
        let result = running.handle.join().ok().flatten();
        if can_land(
            &running.request.key,
            running.area,
            running.request.level,
            running.cancelled.load(Ordering::Relaxed),
            &request,
        ) {
            if let Some(image) = result {
                jobs.serial = jobs.serial.wrapping_add(1).max(1);
                jobs.cache = Some(Cached {
                    key: running.request.key,
                    area: running.area,
                    level: running.request.level,
                    id: jobs.serial,
                    image,
                    delivered: false,
                    _snapshot: running.request.snapshot,
                });
                jobs.failed = None;
            } else {
                jobs.failed = Some(running.request);
            }
        }
    }
    if let Some(cache) = jobs.cache.as_mut() {
        if serves(&cache.key, cache.area, cache.level, &request) {
            *actual_left = cache.area.left;
            *actual_top = cache.area.top;
            *actual_step = 1 << cache.level;
            *image_id = cache.id;
            if cache.delivered {
                return QImage::default();
            }
            cache.delivered = true;
            return level_display_image_data(
                std::mem::take(&mut cache.image.data),
                cache.image.width,
                cache.image.height,
            );
        }
    }
    let failed = jobs.failed.as_ref().is_some_and(|f| {
        f.key == request.key && f.level == request.level && contains(f.needed, request.needed)
    });
    if jobs.running.is_none() && !failed {
        let area = expanded(&request, dimensions.0, dimensions.1);
        let Some(doc) = rust.session.doc.snapshot() else {
            rust.clone_overlay_jobs.cancel();
            return QImage::default();
        };
        let work = request.clone();
        let delay = rust.clone_overlay_jobs.delay_for_test;
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let handle = std::thread::spawn(move || {
            let result = render(doc, work, area, &worker_cancelled);
            if delay > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
            result
        });
        rust.clone_overlay_jobs.running = Some(Running {
            request,
            area,
            handle,
            cancelled,
        });
    }
    *busy = !failed;
    QImage::default()
}

pub(super) fn clone_overlay_cancel(mut view: Pin<&mut PictureView>) {
    let mut rust = view.as_mut().rust_mut();
    rust.clone_overlay_jobs.cancel();
}

pub(super) fn clone_overlay_test_delay(mut view: Pin<&mut PictureView>, milliseconds: u64) {
    view.as_mut().rust_mut().clone_overlay_jobs.delay_for_test = milliseconds;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> SourceKey {
        SourceKey {
            generation: 1,
            revision: 4,
            canvas_revision: 5,
            path: "0".into(),
            scope: 0,
            ignore_adjustments: false,
            snapshot_id: 0,
        }
    }

    fn rect(left: i32, right: i32) -> PsdRect {
        PsdRect {
            left,
            top: 0,
            right,
            bottom: 16,
        }
    }

    #[test]
    fn source_cache_reuses_only_the_same_source_and_covered_region() {
        let base = key();
        let mut request = Request {
            key: base.clone(),
            needed: rect(4, 12),
            level: 1,
            snapshot: None,
        };
        assert!(serves(&base, rect(0, 16), 1, &request));
        assert!(can_land(&base, rect(0, 16), 1, false, &request));
        assert!(!can_land(&base, rect(0, 16), 1, true, &request));
        request.needed = rect(8, 20);
        assert!(!serves(&base, rect(0, 16), 1, &request));
        request.needed = rect(4, 12);
        assert!(!serves(&base, rect(0, 16), 2, &request));
        request.key.scope = 1;
        assert!(!can_land(&base, rect(0, 16), 1, false, &request));
        assert!(!serves(&base, rect(0, 16), 1, &request));
        request.key = base.clone();
        request.key.ignore_adjustments = true;
        assert!(!serves(&base, rect(0, 16), 1, &request));
        request.key = base.clone();
        request.key.path = "1".into();
        assert!(!serves(&base, rect(0, 16), 1, &request));
        request.key = base.clone();
        request.key.generation += 1;
        assert!(!serves(&base, rect(0, 16), 1, &request));
        request.key = base.clone();
        request.key.revision += 1;
        assert!(!serves(&base, rect(0, 16), 1, &request));
        request.key = base.clone();
        request.key.canvas_revision += 1;
        assert!(!serves(&base, rect(0, 16), 1, &request));
        request.key = base.clone();
        request.key.snapshot_id = 7;
        assert!(!serves(&base, rect(0, 16), 1, &request));
        let snapshot = request.key.clone();
        request.key.snapshot_id = 8;
        assert!(!serves(&snapshot, rect(0, 16), 1, &request));
    }

    fn one_pixel_request(snapshot: Option<Arc<RgbaImage>>) -> Request {
        Request {
            key: key(),
            needed: PsdRect {
                left: 0,
                top: 0,
                right: 1,
                bottom: 1,
            },
            level: 0,
            snapshot,
        }
    }

    #[test]
    fn grayscale_layer_populates_every_color_channel() {
        use pictura_core::{BitDepth, Channel, ColorMode, LayerParts};
        let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
        let area = PsdRect {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        };
        doc.layers.push(pictura_codec::layer_from_parts(LayerParts {
            name: "gray".into(),
            rect: area,
            channels: vec![
                Channel {
                    id: 0,
                    data: vec![128].into(),
                },
                Channel {
                    id: -1,
                    data: vec![128].into(),
                },
            ],
            ..Default::default()
        }));
        let result = render(doc, one_pixel_request(None), area, &AtomicBool::new(false)).unwrap();
        assert_eq!(result.data, vec![64, 64, 64, 128]);
    }

    #[test]
    fn profiled_snapshot_uses_the_document_display_transform() {
        use pictura_core::{BitDepth, ColorMode};
        let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
        doc.source
            .set_working_profile(Some(pictura_codec::Profile::adobe_rgb().to_icc().into()));
        let input = PixelBuffer {
            width: 1,
            height: 1,
            channels: 4,
            data: vec![200, 100, 50, 255].into(),
        };
        let expected = pictura_codec::buffer_to_srgb(&doc, &input).into_owned();
        assert_ne!(expected.data, input.data);
        let snapshot = Arc::new(RgbaImage {
            width: 1,
            height: 1,
            data: vec![[200, 100, 50, 255]],
        });
        let area = PsdRect {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        };
        let result = render(
            doc,
            one_pixel_request(Some(snapshot)),
            area,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(
            result.data,
            vec![expected.data[0], expected.data[1], expected.data[2], 255]
        );
    }

    #[test]
    fn cancelling_jobs_marks_running_work_and_discards_cached_result() {
        let flag = Arc::new(AtomicBool::new(false));
        let request = one_pixel_request(None);
        let area = request.needed;
        let mut jobs = OverlayJobs::default();
        jobs.running = Some(Running {
            request: request.clone(),
            area,
            handle: std::thread::spawn(|| None),
            cancelled: Arc::clone(&flag),
        });
        jobs.cancel();
        assert!(flag.load(Ordering::Relaxed));
        assert!(!can_land(
            &request.key,
            area,
            0,
            flag.load(Ordering::Relaxed),
            &request
        ));
        assert!(jobs.cache.is_none());
    }
}
