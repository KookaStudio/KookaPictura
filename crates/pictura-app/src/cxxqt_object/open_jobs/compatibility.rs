//! Compatibility state attached to a native document open.

use crate::cxxqt_object::helpers_composite::{
    buffer_to_rgba_bytes, crop_planar, premultiplied_display_image,
};
use crate::cxxqt_object::qobject;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QString};
use pictura_codec::{buffer_to_srgb, DocumentVersion, SourcePin, VersionAccess};
use pictura_core::{Document, LayerStack, PixelBuffer, PsdRect};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};

static PROTECTED_SOURCES: Mutex<Vec<Weak<ProtectedSource>>> = Mutex::new(Vec::new());

struct ProtectedSource {
    path: PathBuf,
    canonical: PathBuf,
    metadata: Option<(u64, Option<std::time::SystemTime>)>,
    #[cfg(unix)]
    file_id: Option<(u64, u64)>,
}

impl ProtectedSource {
    fn new(path: &Path) -> Arc<Self> {
        let source = Arc::new(Self {
            path: path.to_owned(),
            canonical: path.canonicalize().unwrap_or_else(|_| resolved_path(path)),
            metadata: source_metadata(path),
            #[cfg(unix)]
            file_id: file_id(path),
        });
        let mut open = PROTECTED_SOURCES
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        open.retain(|entry| entry.strong_count() != 0);
        open.push(Arc::downgrade(&source));
        source
    }

    fn unchanged(&self, pin: Option<&SourcePin>) -> bool {
        if self.metadata.is_none()
            || self.metadata != source_metadata(&self.path)
            || pin.is_some_and(SourcePin::rewritten)
        {
            return false;
        }
        #[cfg(unix)]
        if self.file_id.is_some() && self.file_id != file_id(&self.path) {
            return false;
        }
        true
    }
}

fn source_metadata(path: &Path) -> Option<(u64, Option<std::time::SystemTime>)> {
    let metadata = path.metadata().ok()?;
    Some((metadata.len(), metadata.modified().ok()))
}

fn resolved_path(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    if let Some((parent, name)) = path.parent().zip(path.file_name()) {
        if let Ok(parent) = parent.canonicalize() {
            return parent.join(name);
        }
    }
    std::path::absolute(path).unwrap_or_else(|_| path.to_owned())
}

#[cfg(unix)]
fn file_id(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let file = path.metadata().ok()?;
    Some((file.dev(), file.ino()))
}

pub(in crate::cxxqt_object) fn destination_is_protected(path: &Path) -> bool {
    let open = PROTECTED_SOURCES
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let mut sources = open.iter().filter_map(Weak::upgrade).peekable();
    if sources.peek().is_none() {
        return false;
    }
    let resolved = resolved_path(path);
    #[cfg(unix)]
    let identity = file_id(path);
    sources.any(|source| {
        if source.path == path
            || source.canonical == resolved
            || resolved_path(&source.path) == resolved
        {
            return true;
        }
        #[cfg(unix)]
        if identity.is_some() && (identity == source.file_id || identity == file_id(&source.path)) {
            return true;
        }
        false
    })
}

#[derive(Clone, Default)]
pub(in crate::cxxqt_object) struct NativeCompatibility {
    pub(in crate::cxxqt_object) access: VersionAccess,
    source: Option<Arc<ProtectedSource>>,
    pin: Option<Arc<SourcePin>>,
    pixels: Option<PixelBuffer>,
}

impl NativeCompatibility {
    pub(in crate::cxxqt_object) fn new(
        access: VersionAccess,
        path: &Path,
        pin: Option<Arc<SourcePin>>,
        pixels: Option<PixelBuffer>,
    ) -> Self {
        let source = access.is_read_only().then(|| ProtectedSource::new(path));
        Self {
            access,
            source,
            pin,
            pixels,
        }
    }

    pub(in crate::cxxqt_object) fn activate(&self) {
        if let Some(source) = &self.source {
            let mut open = PROTECTED_SOURCES
                .lock()
                .unwrap_or_else(|err| err.into_inner());
            if !open
                .iter()
                .any(|entry| entry.ptr_eq(&Arc::downgrade(source)))
            {
                open.push(Arc::downgrade(source));
            }
        }
    }

    pub(in crate::cxxqt_object) fn deactivate(&self) {
        if let Some(source) = &self.source {
            let weak = Arc::downgrade(source);
            PROTECTED_SOURCES
                .lock()
                .unwrap_or_else(|err| err.into_inner())
                .retain(|entry| !entry.ptr_eq(&weak));
        }
    }

    pub(in crate::cxxqt_object) fn notice(&self) -> String {
        let Some(reason) = self.access.notice() else {
            return String::new();
        };
        if !self.access.is_read_only() {
            return reason.into();
        }
        let mut notice = format!("Read-only: {reason}");
        if let Err(reason) = self.pixels() {
            notice.push_str(" A faithful preview is unavailable. ");
            notice.push_str(reason);
        }
        notice
    }

    fn pixels(&self) -> Result<&PixelBuffer, &'static str> {
        if self
            .source
            .as_ref()
            .is_some_and(|source| !source.unchanged(self.pin.as_deref()))
        {
            return Err("The document source has changed since it was opened.");
        }
        self.pixels
            .as_ref()
            .ok_or("The document has no usable original merged image.")
    }

    pub(in crate::cxxqt_object) fn image(&self, level: i32, rect: PsdRect) -> QImage {
        let Ok(pixels) = self.pixels() else {
            return QImage::default();
        };
        let step = 1i32.checked_shl(level.clamp(0, 30) as u32).unwrap_or(1);
        let left = rect.left.saturating_mul(step).clamp(0, pixels.width as i32);
        let top = rect.top.saturating_mul(step).clamp(0, pixels.height as i32);
        let right = rect
            .right
            .saturating_mul(step)
            .clamp(left, pixels.width as i32);
        let bottom = rect
            .bottom
            .saturating_mul(step)
            .clamp(top, pixels.height as i32);
        if right == left || bottom == top {
            return QImage::default();
        }
        let buffer = crop_planar(
            pixels,
            left,
            top,
            (right - left) as u32,
            (bottom - top) as u32,
        );
        let image = premultiplied_display_image(&buffer);
        if step == 1 {
            return image;
        }
        image.scaled(
            ((right - left) as u32).div_ceil(step as u32) as i32,
            ((bottom - top) as u32).div_ceil(step as u32) as i32,
            cxx_qt_lib::AspectRatioMode::IgnoreAspectRatio,
            cxx_qt_lib::TransformationMode::SmoothTransformation,
        )
    }
}

pub(in crate::cxxqt_object) fn native_access(
    doc: &Document,
    version: &DocumentVersion,
) -> VersionAccess {
    if version.access.is_read_only() {
        return version.access.clone();
    }
    fn unsupported_layer_content(layers: &LayerStack) -> bool {
        use pictura_render::ContentRoute;

        layers.iter().any(|layer| {
            matches!(
                pictura_render::content_route(layer),
                ContentRoute::UndecodedAdjustment | ContentRoute::UndecodedVectorFill
            ) || unsupported_layer_content(layer.children())
        })
    }
    if let Some(reason) = pictura_codec::unsupported_document_feature(doc) {
        return VersionAccess::ReadOnly(reason);
    }
    if unsupported_layer_content(&doc.layers) {
        return VersionAccess::ReadOnly(
            "This document uses an unsupported Photoshop adjustment or vector fill.",
        );
    }
    version.access.clone()
}

pub(in crate::cxxqt_object) fn source_pixels(
    doc: &Document,
    merged_alpha: bool,
) -> Option<PixelBuffer> {
    if !doc.composite.merged_output() || !doc.has_stored_composite() {
        return None;
    }
    let mut pixels = buffer_to_srgb(doc, doc.composite.buffer()).into_owned();
    if merged_alpha && matches!(pixels.channels, 1 | 3) {
        let alpha = &doc.channels.first()?.data;
        if alpha.len() != pixels.pixel_count() {
            return None;
        }
        let mut data = Vec::new();
        data.try_reserve_exact(pixels.data.len().checked_add(alpha.len())?)
            .ok()?;
        data.extend_from_slice(&pixels.data);
        data.extend_from_slice(alpha);
        pixels.data = data.into();
        pixels.channels += 1;
    }
    Some(pixels)
}

pub fn compatibility_reason(view: &qobject::PictureView) -> QString {
    let state = &view.rust().session.compatibility;
    match state.access {
        VersionAccess::ReadOnly(reason) => QString::from(reason),
        _ => QString::default(),
    }
}

pub fn compatibility_preview_available(view: &qobject::PictureView) -> bool {
    view.rust().session.compatibility.pixels().is_ok()
}

pub fn save_compatibility_copy(
    view: Pin<&mut qobject::PictureView>,
    path: &QString,
    confirmed: bool,
) -> QString {
    let state = &view.rust().session.compatibility;
    if !confirmed || !state.access.is_read_only() {
        return QString::from("Confirm a flattened copy before saving.");
    }
    let path = path.to_string();
    let path_ref = Path::new(&path);
    if !path_ref
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
    {
        return QString::from("A read-only compatibility copy must be a PNG.");
    }
    if destination_is_protected(path_ref) {
        return QString::from("The destination is an open read-only source file.");
    }
    let pixels = match state.pixels() {
        Ok(pixels) => pixels,
        Err(reason) => return QString::from(reason),
    };
    let rgba = buffer_to_rgba_bytes(pixels);
    let bytes = crate::cxxqt_object::export::ffi::encode_image_bytes(
        &rgba,
        pixels.width as i32,
        pixels.height as i32,
        "PNG",
        -1,
        100,
        -1,
        false,
        false,
        -1,
    );
    if let Err(reason) = state.pixels() {
        return QString::from(reason);
    }
    if bytes.is_empty()
        || crate::cxxqt_object::helpers::safe_io::write_atomically(&path, &bytes, false).is_err()
    {
        return QString::from("The flattened compatibility copy could not be written.");
    }
    QString::default()
}

pub fn native_save_warning(view: &qobject::PictureView) -> QString {
    let session = &view.rust().session;
    let mut messages = Vec::new();
    if session.skipped_xmp_mirror {
        messages.push("The document was saved. Its XMP version mirror could not be written.");
    }
    if let Some(warning) = session.history_log_save_warning {
        messages.push(warning);
    }
    QString::from(messages.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{Channel, LayerBlock};

    #[test]
    fn vector_fill_compatibility() {
        let mut doc = Document::from_rgba("shape", 1, 1, &[1, 2, 3, 255]);
        let fill = pictura_render::encode_solid_color_fill([10, 20, 30]);
        let mut data = b"SoCo".to_vec();
        data.extend_from_slice(&fill.data);
        let base = pictura_core::LayerParts::from(doc.layers.first().unwrap().clone());
        for (data, read_only) in [(data, false), (vec![0], true)] {
            let mut parts = base.clone();
            parts.extra_blocks.push(LayerBlock {
                key: *b"vscg",
                data: data.into(),
            });
            *doc.layers.first_mut().unwrap() = pictura_codec::layer_from_parts(parts);
            assert_eq!(
                native_access(&doc, &pictura_codec::document_version(&doc)).is_read_only(),
                read_only
            );
        }
    }

    #[test]
    fn merged_transparency_ignores_saved_selection() {
        let mut doc = Document::from_rgba("layer", 1, 1, &[30, 60, 90, 255]);
        doc.channels.push(Channel {
            id: 0,
            data: vec![0].into(),
        });
        let view = pictura_render::save_view(&doc);
        for psb in [false, true] {
            let bytes = pictura_codec::write_versioned(&view, psb, None, "test", false)
                .unwrap()
                .bytes;
            let read = pictura_codec::read_psd(&bytes).unwrap();
            assert!(!pictura_codec::native_merged_has_transparency(&bytes, psb));
            let pixels = source_pixels(&read, false).unwrap();
            assert_eq!(pixels.channels, 3);
            assert_eq!(buffer_to_rgba_bytes(&pixels), vec![30, 60, 90, 255]);
        }
    }
}
