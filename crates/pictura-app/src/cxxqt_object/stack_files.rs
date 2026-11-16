//! `File > Scripts > Load Files into Stack`: decode each source, build a new
//! document with one native-size layer per file on a canvas sized to the
//! largest source, optionally auto-align the layers and combine them into one
//! embedded smart object.
//!
//! The function is exposed through the `open_jobs` bridge rather than its own,
//! so the bridge-file budget and the one-function bridge gate stay satisfied.
//!
//! [`PictureView`]: super::qobject::PictureView

use super::helpers::{decode_import, memory_gate};
use super::helpers_composite::buffer_to_rgba_bytes;
use super::open_jobs::convert_placed_rgba;
use super::qobject::PictureView;
use crate::history::History;
use crate::layer_target::LayerTarget;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QList, QString, QStringList};
use pictura_core::{BitDepth, ColorMode, Document};
use pictura_render::{
    apply_auto_align, can_auto_align, plan_auto_align, AutoAlignOptions, AutoAlignProjection,
    CompositeScope, DocumentComposite,
};
use std::sync::atomic::{AtomicBool, AtomicU32};

struct Decoded {
    path: String,
    name: String,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

fn decode_file(path: &str, policy: pictura_codec::Policy) -> Option<Decoded> {
    let bytes =
        super::helpers::safe_io::read_capped(path, super::helpers::safe_io::import_file_limit())?;
    let name = std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    if name.is_empty() {
        return None;
    }
    let suffix = std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    let native = matches!(suffix.as_deref(), Some("psd" | "psb"));
    let (rgba, width, height) = if native {
        let doc = pictura_codec::read_psd(&bytes).ok()?;
        let buffer = pictura_render::composite_rgba(&doc);
        (buffer_to_rgba_bytes(&buffer), doc.width, doc.height)
    } else {
        decode_import(&bytes)?
    };
    let rgba = if native {
        rgba
    } else {
        convert_placed_rgba(&bytes, rgba, width, height, policy)
    };
    if width == 0 || height == 0 || rgba.len() < width as usize * height as usize * 4 {
        return None;
    }
    Some(Decoded {
        path: path.to_string(),
        name,
        rgba,
        width,
        height,
    })
}

fn build_document(images: &[Decoded], align: bool, smart_object: bool) -> Document {
    let width = images.iter().map(|image| image.width).max().unwrap_or(0);
    let height = images.iter().map(|image| image.height).max().unwrap_or(0);
    let mut doc = Document::new(width, height, ColorMode::Rgb, BitDepth::Eight);
    let mut layer_paths = Vec::new();
    // The first listed file becomes the topmost layer.
    for image in images.iter().rev() {
        let path = pictura_render::add_raster_layer_from_rgba(
            &mut doc,
            &image.name,
            image.width,
            image.height,
            &image.rgba,
        );
        if !path.is_empty() {
            layer_paths.push(path);
        }
    }
    if align && layer_paths.len() >= 2 {
        align_stack(&mut doc, &layer_paths);
    }
    if smart_object {
        let _ = pictura_render::stack_into_smart_object(&mut doc);
    }
    doc
}

fn align_stack(doc: &mut Document, paths: &[String]) {
    let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    if !can_auto_align(doc, &refs) {
        return;
    }
    let cancel = AtomicBool::new(false);
    let progress = AtomicU32::new(0);
    let options = AutoAlignOptions {
        projection: AutoAlignProjection::Auto,
        vignette_removal: false,
        geometric_distortion: false,
        lens: None,
    };
    if let Ok(plan) = plan_auto_align(doc, &refs, options, &cancel, &progress) {
        apply_auto_align(doc, plan);
    }
}

fn to_qstringlist(items: &[String]) -> QStringList {
    let mut list = QList::<QString>::default();
    for item in items {
        list.append(QString::from(item.as_str()));
    }
    QStringList::from(&list)
}

pub fn load_files_into_stack(
    mut view: Pin<&mut PictureView>,
    paths: &QStringList,
    align: bool,
    smart_object: bool,
) -> QStringList {
    let policy = view.rust().color_policy;
    let mut images = Vec::new();
    let mut failed = Vec::new();
    for path in super::helpers::list_of_strings(paths) {
        match decode_file(&path, policy) {
            Some(image) => images.push(image),
            None => failed.push(path),
        }
    }
    if images.is_empty() {
        return to_qstringlist(&failed);
    }
    let estimate: u64 = images
        .iter()
        .map(|image| u64::from(image.width) * u64::from(image.height) * 4)
        .sum();
    if !memory_gate::admit_fresh(estimate) {
        failed.extend(images.into_iter().map(|image| image.path));
        return to_qstringlist(&failed);
    }
    let mut doc = build_document(&images, align, smart_object);
    if doc.layers.is_empty() {
        failed.extend(images.into_iter().map(|image| image.path));
        return to_qstringlist(&failed);
    }
    let gpu = view.rust().gpu_compute;
    DocumentComposite::new(&mut doc).refresh(CompositeScope::Full, gpu);
    let top = doc.layers.len().saturating_sub(1).to_string();
    let mut target = LayerTarget::default();
    target.select_only(&top);
    let mut history = History::default();
    history.capture_live(&mut doc, &None, &target, "Load Files into Stack");
    {
        let mut rust = view.as_mut().rust_mut();
        rust.replace_document(Some(doc), history, target, None);
        rust.path = None;
        rust.session.mark_saved();
    }
    to_qstringlist(&failed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::PsdRect;

    const W: u32 = 192;
    const H: u32 = 144;

    fn texture(w: u32, h: u32) -> Vec<u8> {
        let mut out = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let fx = f64::from(x);
                let fy = f64::from(y);
                let wave = 128.0
                    + 60.0 * (0.13 * fx + 0.07 * fy).sin()
                    + 50.0 * (0.31 * fx - 0.17 * fy).sin()
                    + 25.0 * (0.9 * fx + 0.4 * fy).sin();
                let n = f64::from((x * 31 + y * 17) % 11) - 5.0;
                let v = (wave + n).clamp(0.0, 255.0) as u8;
                let i = ((y * w + x) * 4) as usize;
                out[i] = v;
                out[i + 1] = v.wrapping_add(37);
                out[i + 2] = v.wrapping_add(91);
                out[i + 3] = 255;
            }
        }
        out
    }

    fn image(path: &str, width: u32, height: u32, rgba: Vec<u8>) -> Decoded {
        Decoded {
            path: path.to_string(),
            name: path.to_string(),
            rgba,
            width,
            height,
        }
    }

    fn two_images() -> Vec<Decoded> {
        vec![
            image("first.png", W, H, texture(W, H)),
            image("second.png", W, H, texture(W, H)),
        ]
    }

    #[test]
    fn canvas_is_largest_source() {
        let images = vec![
            image("big.png", 64, 48, texture(64, 48)),
            image("small.png", 32, 20, texture(32, 20)),
        ];
        let doc = build_document(&images, false, false);
        assert_eq!((doc.width, doc.height), (64, 48));
    }

    #[test]
    fn first_file_is_topmost() {
        let doc = build_document(&two_images(), false, false);
        assert_eq!(doc.layers.len(), 2);
        assert_eq!(doc.layers.last().expect("top").name, "first.png");
        assert_eq!(doc.layers.first().expect("bottom").name, "second.png");
    }

    #[test]
    fn no_align_keeps_positions() {
        let doc = build_document(&two_images(), false, false);
        for layer in &doc.layers {
            assert_eq!((layer.rect.left, layer.rect.top), (0, 0));
        }
    }

    #[test]
    fn aligns_offset_copies() {
        let tex = texture(W, H);
        let mut doc = Document::new(W, H, ColorMode::Rgb, BitDepth::Eight);
        let base = pictura_render::add_raster_layer_from_rgba(&mut doc, "base", W, H, &tex);
        doc.layers[0].lock =
            pictura_core::LockFlags::default().with(pictura_core::LockFlags::POSITION, true);
        let moving = pictura_render::add_raster_layer_from_rgba(&mut doc, "moving", W, H, &tex);
        doc.layers.last_mut().expect("moving").rect = PsdRect {
            left: 14,
            top: -9,
            right: 14 + W as i32,
            bottom: -9 + H as i32,
        };
        align_stack(&mut doc, &[base, moving]);
        let moved = doc.layers.last().expect("moving");
        assert_ne!((moved.rect.left, moved.rect.top), (14, -9));
    }

    #[test]
    fn no_smart_object_keeps_layers() {
        let doc = build_document(&two_images(), false, false);
        assert_eq!(doc.layers.len(), 2);
        assert!(doc.layers.iter().all(|layer| layer.smart_object.is_none()));
    }

    #[test]
    fn smart_object_embeds_stack() {
        let doc = build_document(&two_images(), false, true);
        assert_eq!(doc.layers.len(), 1);
        let layer = doc.layers.last().expect("smart object");
        assert_eq!(
            layer.smart_object.as_ref().map(|so| so.kind),
            Some(pictura_core::SmartObjectKind::Embedded)
        );
        let payload = layer
            .smart_object
            .as_ref()
            .and_then(|so| so.payload())
            .expect("payload");
        let embedded = pictura_codec::read_psd(payload).expect("embedded PSD");
        assert_eq!(embedded.layers.len(), 2);
    }
}
