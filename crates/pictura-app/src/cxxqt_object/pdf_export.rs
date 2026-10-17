//! The Photoshop PDF bridge (`interop/pdf-export`): save the document through
//! `pictura_codec::pdf` with JPEG encoded at the Qt boundary, reopen a
//! Pictura Photoshop PDF from its embedded PSD, and the option, preset, and
//! `.joboptions` helpers the Save Adobe PDF dialog edits through. Options
//! travel as canonical `.joboptions` text (design D3). Free functions over a
//! [`PictureView`] (their own bridge, so the `PictureView` list does not grow).
//!
//! [`PictureView`]: super::qobject::PictureView

mod import;

use super::helpers::topmost_pixel_layer_index;
use super::helpers_composite::{current_buffer, store_composite};
use super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_codec::pdf::{self, JpegRequest, PdfEnv, PdfOptions};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("encode_image.h");
        /// Encode interleaved 8-bit samples (1 gray, 3 RGB, 4 CMYK) as a JPEG at `quality` 0–100; empty on failure.
        fn encode_jpeg(
            samples: &[u8],
            width: i32,
            height: i32,
            components: i32,
            quality: i32,
        ) -> Vec<u8>;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Write the document to `path` as a Photoshop PDF with the canonical `options` text; on success record the path, clear the modified state, and remember the options. Returns an empty string on success, else the reason.
        fn pdf_save(view: Pin<&mut PictureView>, path: &QString, options: &QString) -> QString;

        /// The font family of every type layer, one per line, for the shell to register before a save sets them as live text.
        fn pdf_type_fonts(view: &PictureView) -> QString;

        /// The options this document was last saved to PDF with; empty when never.
        fn pdf_view_options(view: &PictureView) -> QString;

        /// Open a Photoshop PDF with its layers (Pictura's embedded PSD, or Photoshop's own editing data rebuilt into a PSD) into the view; false, unchanged, for any other file.
        fn pdf_open(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// `[Photoshop Default]` as canonical options text.
        fn pdf_default_options() -> QString;

        /// One field of `options` as text (enums as their list index); empty for an unknown field.
        fn pdf_option(options: &QString, field: &QString) -> QString;

        /// `options` with `field` set to `value` and the gating re-applied; unchanged on refusal.
        fn pdf_set_option(options: &QString, field: &QString, value: &QString) -> QString;

        /// Whether the standard or compatibility level forces `field`.
        fn pdf_option_locked(options: &QString, field: &QString) -> bool;

        /// The Summary page text: one setting or warning per line.
        fn pdf_summary(options: &QString) -> QString;

        /// The built-in preset names, one per line.
        fn pdf_builtin_presets() -> QString;

        /// A built-in preset as canonical options text; empty for an unknown name.
        fn pdf_builtin_preset(name: &QString) -> QString;

        /// Parse a `.joboptions` file into canonical options text; empty when it holds no parameters.
        fn pdf_parse_joboptions(text: &QString) -> QString;

        /// The `.joboptions` text a preset file stores (no Save options, no Security).
        fn pdf_preset_text(options: &QString) -> QString;

        /// Whether two option texts agree on every preset setting.
        fn pdf_same_preset(a: &QString, b: &QString) -> bool;
    }
}

fn parse(options: &QString) -> Option<PdfOptions> {
    pdf::parse_joboptions(&options.to_string())
}

fn text(options: &PdfOptions) -> QString {
    QString::from(pdf::write_joboptions(options, false).as_str())
}

/// 32 bytes of OS-seeded randomness for the file ID and encryption salts:
/// std's `RandomState` keys come from the OS, so no new dependency.
fn os_entropy() -> [u8; 32] {
    use std::hash::{BuildHasher, Hasher};
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let mut seed = [0u8; 32];
    for (i, chunk) in seed.chunks_mut(8).enumerate() {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u128(nanos);
        hasher.write_usize(i);
        chunk.copy_from_slice(&hasher.finish().to_le_bytes());
    }
    seed
}

fn write_atomically(path: &str, bytes: &[u8]) -> bool {
    let tmp = format!("{path}.tmp");
    if std::fs::write(&tmp, bytes).is_err() {
        return false;
    }
    if std::fs::rename(&tmp, path).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return false;
    }
    true
}

/// Encode the view's document as a Photoshop PDF at `path` with `options`.
fn save_view(
    mut view: Pin<&mut PictureView>,
    path: &str,
    options: &PdfOptions,
) -> Result<(), String> {
    let gpu_compute = view.rust().gpu_compute;
    let dirty = view.rust().dirty;
    let bytes = {
        let mut rust = view.as_mut().rust_mut();
        let doc = rust.doc.as_mut().ok_or("There is no document to save.")?;
        if dirty {
            pictura_render::refresh_native_composite(doc);
        }
        let composite = current_buffer(doc, gpu_compute);
        let live_text =
            pictura_render::pdf_live_type(doc, options.text_policy()).map(|live| pdf::LiveText {
                background: current_buffer(&live.hidden, gpu_compute),
                fonts: live.fonts,
                texts: live.texts,
            });
        let save_doc = pictura_render::save_view(doc);
        let jpeg = |request: &JpegRequest| {
            let bytes = ffi::encode_jpeg(
                request.samples,
                request.width as i32,
                request.height as i32,
                i32::from(request.components),
                i32::from(request.quality),
            );
            (!bytes.is_empty()).then_some(bytes)
        };
        let env = PdfEnv {
            jpeg: &jpeg,
            now: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or_default(),
            entropy: os_entropy(),
            title: std::path::Path::new(path)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            live_text,
        };
        pdf::write_pdf(&save_doc, &composite, options, &env).map_err(|e| e.to_string())?
    };
    if !write_atomically(path, &bytes) {
        return Err(format!("Could not write {path}."));
    }
    Ok(())
}

pub fn pdf_save(mut view: Pin<&mut PictureView>, path: &QString, options: &QString) -> QString {
    let path = path.to_string();
    let Some(mut options) = parse(options) else {
        return QString::from("The PDF settings could not be read.");
    };
    options.normalize();
    if let Err(reason) = save_view(view.as_mut(), &path, &options) {
        return QString::from(reason.as_str());
    }
    let mut rust = view.rust_mut();
    rust.path = Some(path);
    rust.dirty = false;
    rust.pdf_options = Some(pdf::write_joboptions(&options, false));
    QString::default()
}

/// `PictureView::save` for a `.pdf`/`.pdp` path: the document's last PDF
/// options, else `[Photoshop Default]`.
pub(super) fn save_with_last_options(view: Pin<&mut PictureView>, path: &QString) -> bool {
    let options = view
        .rust()
        .pdf_options
        .clone()
        .unwrap_or_else(|| pdf::write_joboptions(&PdfOptions::default(), false));
    pdf_save(view, path, &QString::from(options.as_str())).is_empty()
}

pub fn pdf_type_fonts(view: &PictureView) -> QString {
    let fonts = view
        .rust()
        .doc
        .as_ref()
        .map(pictura_render::pdf_type_fonts)
        .unwrap_or_default();
    QString::from(fonts.join("\n").as_str())
}

pub fn pdf_view_options(view: &PictureView) -> QString {
    QString::from(view.rust().pdf_options.as_deref().unwrap_or_default())
}

pub fn pdf_open(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    let policy = view.rust().color_policy;
    let Some(mut doc) = std::fs::read(&path)
        .ok()
        .and_then(|bytes| pdf::embedded_psd(&bytes).or_else(|| pdf::photoshop_psd(&bytes)))
        .and_then(|psd| pictura_codec::read_psd_with(&psd, policy).ok())
    else {
        return false;
    };
    let gpu_compute = view.rust().gpu_compute;
    let rendered = current_buffer(&doc, gpu_compute);
    store_composite(&mut doc, &rendered);
    let mut rust = view.as_mut().rust_mut();
    rust.active_layer = topmost_pixel_layer_index(&doc).map(|i| i.to_string());
    // The document goes in first: reset_edit_state rebuilds the display
    // pyramid from it (before, the canvas showed the previous, or no, document).
    rust.doc = Some(doc);
    rust.reset_edit_state();
    let rust = &mut *rust;
    if let Some(doc) = rust.doc.as_mut() {
        rust.history.capture_live(doc, &None, "Open");
    }
    rust.path = Some(path);
    rust.dirty = false;
    rust.source_format = "pdf".into();
    true
}

pub fn pdf_default_options() -> QString {
    text(&PdfOptions::default())
}

pub fn pdf_option(options: &QString, field: &QString) -> QString {
    parse(options)
        .and_then(|o| o.get(&field.to_string()))
        .map(|v| QString::from(v.as_str()))
        .unwrap_or_default()
}

pub fn pdf_set_option(options: &QString, field: &QString, value: &QString) -> QString {
    match parse(options) {
        Some(mut o) => {
            o.set(&field.to_string(), &value.to_string());
            text(&o)
        }
        None => options.clone(),
    }
}

pub fn pdf_option_locked(options: &QString, field: &QString) -> bool {
    parse(options).is_some_and(|o| o.locked(&field.to_string()))
}

pub fn pdf_summary(options: &QString) -> QString {
    parse(options)
        .map(|o| QString::from(o.summary().join("\n").as_str()))
        .unwrap_or_default()
}

pub fn pdf_builtin_presets() -> QString {
    QString::from(pdf::BUILTIN_PRESETS.join("\n").as_str())
}

pub fn pdf_builtin_preset(name: &QString) -> QString {
    pdf::builtin_preset(&name.to_string())
        .map(|o| text(&o))
        .unwrap_or_default()
}

pub fn pdf_parse_joboptions(text_in: &QString) -> QString {
    parse(text_in).map(|o| text(&o)).unwrap_or_default()
}

pub fn pdf_preset_text(options: &QString) -> QString {
    parse(options)
        .map(|o| QString::from(pdf::write_joboptions(&o, true).as_str()))
        .unwrap_or_default()
}

pub fn pdf_same_preset(a: &QString, b: &QString) -> bool {
    match (parse(a), parse(b)) {
        (Some(a), Some(b)) => pdf::same_preset(&a, &b),
        _ => false,
    }
}
