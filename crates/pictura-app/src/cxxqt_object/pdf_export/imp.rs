use super::*;

use super::super::helpers_composite::current_buffer;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_codec::pdf::{self, Compression, JpegRequest, PdfEnv, PdfOptions};

pub(super) fn parse(options: &QString) -> Option<PdfOptions> {
    pdf::parse_joboptions(&options.to_string())
}

pub(super) fn text(options: &PdfOptions) -> QString {
    QString::from(pdf::write_joboptions(options, false).as_str())
}

/// Encode the view's document as a Photoshop PDF at `path` with `options`.
pub(super) fn save_view(
    mut view: Pin<&mut PictureView>,
    path: &str,
    options: &PdfOptions,
) -> Result<(), String> {
    let gpu_compute = view.rust().gpu_compute;
    let dirty = view.rust().session.dirty;
    let mut options = options.clone();
    if options.compression == Compression::Jpeg2000 {
        let (continuous, deep) = view
            .rust()
            .session
            .doc
            .as_ref()
            .map_or((true, false), |doc| {
                let mode = doc.source.mode().unwrap_or(doc.mode);
                (
                    !matches!(
                        mode,
                        pictura_core::ColorMode::Bitmap | pictura_core::ColorMode::Indexed
                    ),
                    doc.source.depth() == Some(pictura_core::BitDepth::Sixteen),
                )
            });
        if !continuous || (deep && !options.convert_16_to_8) {
            options.compression = Compression::Zip;
        }
    }
    let proof_state = view.rust().proof;
    let proof_icc = crate::cxxqt_object::proofing::proof_device_icc(proof_state.device);
    let proof_components =
        crate::cxxqt_object::proofing::proof_device_components(proof_state.device);
    let proof_intent = crate::cxxqt_object::proofing::proof_intent_from_code(proof_state.intent);
    let bytes = {
        let mut rust = view.as_mut().rust_mut();
        let doc = rust
            .session
            .doc
            .as_mut()
            .ok_or("There is no document to save.")?;
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
            entropy: pictura_codec::os_random(),
            title: std::path::Path::new(path)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            live_text,
            proof: Some(pdf::ProofRequest {
                icc: proof_icc.as_slice(),
                components: proof_components,
                intent: proof_intent,
                bpc: proof_state.bpc,
            }),
            editing_doc: None,
        };
        pdf::write_pdf(&save_doc, &composite, &options, &env).map_err(|e| e.to_string())?
    };
    if super::super::helpers::safe_io::write_atomically(path, &bytes, false).is_err() {
        return Err(format!("Could not write {path}."));
    }
    Ok(())
}

fn pdf_save_impl(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    options: &QString,
    copy: bool,
) -> QString {
    if view.rust().session.read_only() {
        return QString::from("A read-only compatibility document cannot be saved as PDF.");
    }
    let path = path.to_string();
    let Some(mut options) = parse(options) else {
        return QString::from("The PDF settings could not be read.");
    };
    options.normalize();
    if let Err(reason) = save_view(view.as_mut(), &path, &options) {
        return QString::from(reason.as_str());
    }
    let mut rust = view.rust_mut();
    if !copy {
        rust.path = Some(path);
        rust.session.mark_saved();
    }
    rust.session.pdf_options = Some(pdf::write_joboptions(&options, false));
    QString::default()
}

pub fn pdf_save(view: Pin<&mut PictureView>, path: &QString, options: &QString) -> QString {
    pdf_save_impl(view, path, options, false)
}

/// `pdf_save` without recording the path or clearing the modified state.
pub fn pdf_save_copy(view: Pin<&mut PictureView>, path: &QString, options: &QString) -> QString {
    pdf_save_impl(view, path, options, true)
}

/// `PictureView::save` for a `.pdf`/`.pdp` path: the document's last PDF
/// options, else `[Photoshop Default]`. `commit` false writes a copy.
pub(in crate::cxxqt_object) fn save_with_last_options(
    view: Pin<&mut PictureView>,
    path: &QString,
    commit: bool,
) -> bool {
    let options = view
        .rust()
        .session
        .pdf_options
        .clone()
        .unwrap_or_else(|| pdf::write_joboptions(&PdfOptions::default(), false));
    pdf_save_impl(view, path, &QString::from(options.as_str()), !commit).is_empty()
}

pub fn pdf_type_fonts(view: &PictureView) -> QString {
    let fonts = view
        .rust()
        .session
        .doc
        .as_ref()
        .map(pictura_render::pdf_type_fonts)
        .unwrap_or_default();
    QString::from(fonts.join("\n").as_str())
}

pub fn pdf_view_options(view: &PictureView) -> QString {
    QString::from(
        view.rust()
            .session
            .pdf_options
            .as_deref()
            .unwrap_or_default(),
    )
}

pub fn pdf_document_is_16bit(view: &PictureView) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| doc.source.depth() == Some(pictura_core::BitDepth::Sixteen))
}

pub fn pdf_spot_channel_count(view: &PictureView) -> i32 {
    view.rust().session.doc.as_ref().map_or(0, |doc| {
        pictura_codec::spot_channel_indices(doc).len() as i32
    })
}

pub fn pdf_document_is_continuous_tone(view: &PictureView) -> bool {
    view.rust().session.doc.as_ref().is_some_and(|doc| {
        let mode = doc.source.mode().unwrap_or(doc.mode);
        !matches!(
            mode,
            pictura_core::ColorMode::Bitmap | pictura_core::ColorMode::Indexed
        )
    })
}

pub fn pdf_document_is_bitmap(view: &PictureView) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| doc.source.mode().unwrap_or(doc.mode) == pictura_core::ColorMode::Bitmap)
}

pub fn pdf_open(mut view: Pin<&mut PictureView>, path: &QString, password: &QString) -> QString {
    let path = path.to_string();
    let typed = password.to_string();
    // Revert passes none: reuse the password the document was opened with.
    let password = if typed.is_empty() {
        view.rust().session.pdf_password.clone().unwrap_or_default()
    } else {
        typed
    };
    let policy = view.rust().color_policy;
    let gpu_compute = view.rust().gpu_compute;
    match super::super::open_jobs::decode_pdf(&path, &password, policy, gpu_compute) {
        super::super::open_jobs::PdfOpen::Ok(opened) => {
            super::super::open_jobs::apply_opened(&mut view.as_mut().rust_mut(), *opened);
            view.as_mut().session_cancelled();
            QString::from("ok")
        }
        super::super::open_jobs::PdfOpen::None => QString::from("none"),
        super::super::open_jobs::PdfOpen::Password => QString::from("password"),
        super::super::open_jobs::PdfOpen::WrongPassword => QString::from("wrongpassword"),
    }
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
