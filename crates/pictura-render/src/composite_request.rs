//! One composite request over the region x backend cross-product.
//!
//! [`CompositeRequest`] is the policy entry point: it clamps the region, tries
//! the GPU when enabled and available, and falls back to the CPU oracle inside.
//! [`crate::composite_rgba`] remains the CPU oracle; the GPU compositor entry
//! points stay internal to this crate.

use pictura_core::{Document, PixelBuffer, PsdRect};

use crate::gpu::{self, Backend};

/// A request to composite `doc` over `region` through the requested backend.
///
/// `region: None` composites the whole document. `gpu_enabled` only permits the
/// GPU; the returned [`Backend`] reports what actually ran.
pub struct CompositeRequest<'a> {
    pub doc: &'a Document,
    pub region: Option<PsdRect>,
    pub gpu_enabled: bool,
}

impl<'a> CompositeRequest<'a> {
    /// The whole-document request.
    pub fn whole(doc: &'a Document, gpu_enabled: bool) -> Self {
        Self {
            doc,
            region: None,
            gpu_enabled,
        }
    }

    /// A region request; [`composite`] clamps the region to the document.
    pub fn region(doc: &'a Document, region: PsdRect, gpu_enabled: bool) -> Self {
        Self {
            doc,
            region: Some(region),
            gpu_enabled,
        }
    }
}

/// Composite a request, clamping the region and falling back to the CPU.
///
/// The CPU result is byte-identical to [`crate::composite_rgba`] for a
/// whole-document request and to the matching slice for a region request. A GPU
/// result comes from the existing GPU kernel, within ±1 LSB of the CPU oracle.
pub fn composite(request: CompositeRequest<'_>) -> (PixelBuffer, Backend) {
    let doc = request.doc;
    let Some((x0, y0, w, h)) = clamp(request.region, doc) else {
        return (PixelBuffer::new(0, 0, 4), Backend::Cpu);
    };
    let whole = x0 == 0 && y0 == 0 && w == doc.width && h == doc.height;
    if request.gpu_enabled && gpu::gpu_available() {
        if let Ok(buffer) = gpu::composite_gpu_region(doc, x0, y0, w, h) {
            return (buffer, Backend::Gpu);
        }
    }
    let buffer = if whole {
        crate::composite::composite_rgba(doc)
    } else {
        crate::composite::composite_rgba_region(doc, x0, y0, w, h)
    };
    (buffer, Backend::Cpu)
}

fn clamp(region: Option<PsdRect>, doc: &Document) -> Option<(u32, u32, u32, u32)> {
    let rect = region.unwrap_or(PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height.min(i32::MAX as u32) as i32,
        right: doc.width.min(i32::MAX as u32) as i32,
    });
    let x0 = rect.left.max(0);
    let y0 = rect.top.max(0);
    let x1 = rect.right.min(doc.width.min(i32::MAX as u32) as i32);
    let y1 = rect.bottom.min(doc.height.min(i32::MAX as u32) as i32);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some((x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32))
}
