//! Select > Refine Edge: the dialog's preview mask and its OK. Free functions
//! over a [`PictureView`] (their own bridge, so the `PictureView` declaration
//! list does not grow). Ports CS6 Refine Edge (SEL-003) onto
//! [`pictura_select::refine`].
//!
//! [`PictureView`]: super::qobject::PictureView

use super::super::helpers::active_pixel_layer;
use super::super::helpers_composite::{current_buffer, selection_to_mask};
use super::super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::{LayerMask, PsdRect};
use pictura_select::{refine, OutputTarget, RefineEdgeSettings, Selection};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Whether Refine Edge can run: a document with a non-empty selection.
        fn refine_edge_available(view: &PictureView) -> bool;

        /// The refined coverage mask (one byte a pixel, row-major) scaled to fit
        /// `max_side`, with its size in `width` / `height`. Empty when
        /// unavailable.
        #[allow(clippy::too_many_arguments)]
        fn refine_edge_preview(
            view: &PictureView,
            smart_radius: bool,
            radius: i32,
            smooth: i32,
            feather: i32,
            contrast: i32,
            shift_edge: i32,
            max_side: i32,
            width: &mut i32,
            height: &mut i32,
        ) -> Vec<u8>;

        /// Apply the settings once: refine the active selection and write the
        /// result to the chosen `output` target (0 Selection, 1 Layer Mask, 2
        /// New Layer, 3 New Layer with Mask). `decontaminate` / `amount` add
        /// colour decontamination for the new-layer targets. One undo state;
        /// false when unavailable or refused.
        #[allow(clippy::too_many_arguments)]
        fn refine_edge_apply(
            view: Pin<&mut PictureView>,
            smart_radius: bool,
            radius: i32,
            smooth: i32,
            feather: i32,
            contrast: i32,
            shift_edge: i32,
            decontaminate: bool,
            amount: i32,
            output: i32,
        ) -> bool;
    }
}

/// The engine settings the dialog passes. View-mode and view toggles are a view
/// concern and do not reach the engine.
#[derive(Clone, Copy)]
struct EngineParams {
    smart_radius: bool,
    radius: u32,
    smooth: u32,
    feather: u32,
    contrast: u32,
    shift_edge: i32,
}

impl EngineParams {
    fn settings(self) -> RefineEdgeSettings {
        RefineEdgeSettings {
            smart_radius: self.smart_radius,
            radius: self.radius,
            smooth: self.smooth,
            feather: self.feather,
            contrast: self.contrast,
            shift_edge: self.shift_edge,
            ..Default::default()
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn params(
    smart_radius: bool,
    radius: i32,
    smooth: i32,
    feather: i32,
    contrast: i32,
    shift_edge: i32,
) -> EngineParams {
    EngineParams {
        smart_radius,
        radius: radius.max(0) as u32,
        smooth: smooth.max(0) as u32,
        feather: feather.max(0) as u32,
        contrast: contrast.max(0) as u32,
        shift_edge: shift_edge.clamp(-100, 100),
    }
}

fn refine_edge_available(view: &PictureView) -> bool {
    let rust = view.rust();
    rust.doc.is_some()
        && rust
            .selection
            .as_ref()
            .is_some_and(|s| s.data.iter().any(|&v| v > 0))
}

/// Nearest-neighbour shrink of a planar buffer so its longer side is at most
/// `max_side`; the preview needs the look, not every pixel.
fn shrink(buffer: pictura_core::PixelBuffer, max_side: u32) -> pictura_core::PixelBuffer {
    let longer = buffer.width.max(buffer.height);
    if max_side == 0 || longer <= max_side {
        return buffer;
    }
    let scale = max_side as f64 / longer as f64;
    let w = ((buffer.width as f64 * scale).round() as u32).max(1);
    let h = ((buffer.height as f64 * scale).round() as u32).max(1);
    let (sw, sh) = (buffer.width as usize, buffer.height as usize);
    let plane = sw * sh;
    let mut data = vec![0u8; w as usize * h as usize * buffer.channels as usize];
    for c in 0..buffer.channels as usize {
        for y in 0..h as usize {
            let sy = (y * sh / h as usize).min(sh - 1);
            for x in 0..w as usize {
                let sx = (x * sw / w as usize).min(sw - 1);
                data[c * w as usize * h as usize + y * w as usize + x] =
                    buffer.data[c * plane + sy * sw + sx];
            }
        }
    }
    pictura_core::PixelBuffer {
        width: w,
        height: h,
        channels: buffer.channels,
        data: data.into(),
    }
}

/// Shrink a selection mask to a proxy the same way [`shrink`] shrinks a buffer.
fn shrink_selection(sel: &Selection, w: u32, h: u32) -> Selection {
    let (sw, sh) = (sel.width as usize, sel.height as usize);
    let mut data = vec![0u8; w as usize * h as usize];
    for y in 0..h as usize {
        let sy = (y * sh / h as usize).min(sh - 1);
        for x in 0..w as usize {
            let sx = (x * sw / w as usize).min(sw - 1);
            data[y * w as usize + x] = sel.data[sy * sw + sx];
        }
    }
    Selection {
        width: w,
        height: h,
        data,
    }
}

#[allow(clippy::too_many_arguments)]
fn refine_edge_preview(
    view: &PictureView,
    smart_radius: bool,
    radius: i32,
    smooth: i32,
    feather: i32,
    contrast: i32,
    shift_edge: i32,
    max_side: i32,
    width: &mut i32,
    height: &mut i32,
) -> Vec<u8> {
    *width = 0;
    *height = 0;
    if !refine_edge_available(view) {
        return Vec::new();
    }
    let rust = view.rust();
    let (Some(doc), Some(selection)) = (rust.doc.as_ref(), rust.selection.as_ref()) else {
        return Vec::new();
    };
    let image = shrink(
        current_buffer(doc, rust.gpu_compute),
        max_side.max(0) as u32,
    );
    let mask = shrink_selection(selection, image.width, image.height);
    let settings = params(smart_radius, radius, smooth, feather, contrast, shift_edge).settings();
    match refine(&mask, &image, &settings) {
        Ok(refined) => {
            *width = refined.width as i32;
            *height = refined.height as i32;
            refined.data
        }
        Err(_) => Vec::new(),
    }
}

/// Write `data` into the layer at `path` as a document-sized raster mask.
fn set_layer_mask(doc: &mut pictura_core::Document, path: &str, data: Vec<u8>) {
    let (w, h) = (doc.width as i32, doc.height as i32);
    if let Some(layer) = pictura_render::resolve_path_mut(doc, path) {
        layer.mask = Some(LayerMask {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: h,
                right: w,
            },
            default_color: 0,
            disabled: false,
            flags: 0,
            data: Some(data.into()),
            extra: Vec::new(),
        });
    }
}

/// Overwrite the RGB of the document-sized layer at `path` with `decon` where
/// the refined coverage is partly selected. Alpha is left as the copy set it.
fn write_decontaminated(
    doc: &mut pictura_core::Document,
    path: &str,
    coverage: &Selection,
    decon: &pictura_core::PixelBuffer,
) {
    let (w, h) = (doc.width as usize, doc.height as usize);
    let n = w * h;
    let Some(layer) = pictura_render::resolve_path_mut(doc, path) else {
        return;
    };
    let rect = layer.rect;
    for c in 0..3usize {
        let Some(channel) = layer.channels.iter_mut().find(|ch| ch.id == c as i16) else {
            continue;
        };
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if coverage.data[i] == 0 {
                    continue;
                }
                let sx = x as i32 - rect.left;
                let sy = y as i32 - rect.top;
                if sx < 0 || sy < 0 || sx >= rect.width() || sy >= rect.height() {
                    continue;
                }
                let si = sy as usize * rect.width() as usize + sx as usize;
                if si < channel.data.len() {
                    let decon_i = c * n + i;
                    if decon_i < decon.data.len() {
                        channel.data[si] = decon.data[decon_i];
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn refine_edge_apply(
    mut view: Pin<&mut PictureView>,
    smart_radius: bool,
    radius: i32,
    smooth: i32,
    feather: i32,
    contrast: i32,
    shift_edge: i32,
    decontaminate: bool,
    amount: i32,
    output: i32,
) -> bool {
    if !refine_edge_available(&view) {
        return false;
    }
    let Some(target) = OutputTarget::from_index(output) else {
        return false;
    };
    let settings = params(smart_radius, radius, smooth, feather, contrast, shift_edge).settings();
    // Decontamination writes colour, so it only combines with a colour target.
    if decontaminate && !target.is_color_output() {
        return false;
    }

    // Compute the refined mask (and, for a colour target, the decontaminated
    // pixels) against the composite before mutating anything.
    let prepared = {
        let rust = view.rust();
        let (Some(doc), Some(selection)) = (rust.doc.as_ref(), rust.selection.as_ref()) else {
            return false;
        };
        let image = current_buffer(doc, rust.gpu_compute);
        let refined = match refine(selection, &image, &settings) {
            Ok(refined) => refined,
            Err(_) => return false,
        };
        let decon = if decontaminate {
            match pictura_select::decontaminate(&image, &refined, amount.clamp(0, 100) as u32) {
                Ok(buffer) => Some(buffer),
                Err(_) => return false,
            }
        } else {
            None
        };
        (refined, decon)
    };
    let (refined, decon) = prepared;

    let active = view.rust().active_layer.clone();
    let source_is_pixel = view
        .rust()
        .doc
        .as_ref()
        .is_some_and(|doc| active_pixel_layer(doc, active.as_deref()).is_some());
    let mut new_layer_path: Option<String> = None;

    let applied = {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.doc.as_mut() else {
            return false;
        };
        match target {
            OutputTarget::Selection => true,
            OutputTarget::LayerMask => {
                let Some(path) = active.clone() else {
                    return false;
                };
                if active_pixel_layer(doc, Some(&path)).is_none() {
                    return false;
                }
                set_layer_mask(doc, &path, refined.data.clone());
                true
            }
            OutputTarget::NewLayer | OutputTarget::NewLayerWithMask => {
                if !source_is_pixel {
                    return false;
                }
                let Some(path) = active.clone() else {
                    return false;
                };
                let mask = selection_to_mask(&refined, doc);
                let new_path = pictura_render::layer_via_copy(doc, &path, &mask);
                if new_path.is_empty() {
                    return false;
                }
                if let Some(decon) = decon.as_ref() {
                    write_decontaminated(doc, &new_path, &refined, decon);
                }
                if target == OutputTarget::NewLayerWithMask {
                    set_layer_mask(doc, &new_path, refined.data.clone());
                }
                new_layer_path = Some(new_path);
                true
            }
        }
    };
    if !applied {
        return false;
    }

    {
        let mut rust = view.as_mut().rust_mut();
        if target == OutputTarget::Selection {
            rust.selection = Some(refined);
        }
        if let Some(path) = new_layer_path {
            rust.active_layer = Some(path);
        }
    }
    view.as_mut().recomposite();
    view.as_mut().record("Refine Edge");
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrink_selection_keeps_size_and_samples_nearest() {
        let mut sel = Selection::none(8, 4);
        for x in 0..8u32 {
            sel.data[x as usize] = (x * 10) as u8;
        }
        let small = shrink_selection(&sel, 4, 2);
        assert_eq!((small.width, small.height), (4, 2));
        assert_eq!(&small.data[..4], &[0, 20, 40, 60]);
    }

    #[test]
    fn output_index_mapping() {
        assert!(matches!(
            OutputTarget::from_index(0),
            Some(OutputTarget::Selection)
        ));
        assert!(matches!(
            OutputTarget::from_index(3),
            Some(OutputTarget::NewLayerWithMask)
        ));
        assert!(OutputTarget::from_index(9).is_none());
    }

    #[test]
    fn engine_params_clamp() {
        let p = params(true, -5, 200, 3, 101, -300);
        assert_eq!(p.radius, 0);
        assert_eq!(p.smooth, 200);
        assert_eq!(p.shift_edge, -100);
        assert!(p.smart_radius);
    }
}
