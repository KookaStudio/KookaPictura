//! Document save: native PSD/PSB through the worker job, raster with the
//! per-document profile choice, and the engine Save As writers.

use super::*;
use std::path::Path;

impl qobject::PictureView {
    pub fn save(mut self: Pin<&mut Self>, path: &QString) -> bool {
        self.as_mut().settle_pending_composite();
        let path_string = path.to_string();
        if Path::new(&path_string).extension().is_none() {
            return false;
        }
        let suffix = format_for_path(&path_string);
        if matches!(suffix.as_str(), "pdf" | "pdp") {
            return super::super::pdf_export::save_with_last_options(
                self,
                &QString::from(path_string.as_str()),
            );
        }
        if matches!(suffix.as_str(), "psd" | "psb") {
            if !super::save_jobs::save_start_rust(&mut self.as_mut().rust_mut(), &path_string) {
                return false;
            }
            return super::save_jobs::save_finish_rust(&mut self.as_mut().rust_mut());
        }
        let mut resolved = pictura_codec::SaveKind::from_suffix(&suffix);
        if suffix == "eps" {
            if let Some(settings) = self.rust().save_options.as_ref() {
                if let Some(kind) = pictura_codec::SaveKind::from_code(settings.kind) {
                    if matches!(
                        kind,
                        pictura_codec::SaveKind::Eps | pictura_codec::SaveKind::Dcs
                    ) {
                        resolved = Some(kind);
                    }
                }
            }
        }
        if let Some(kind) = resolved {
            if kind != pictura_codec::SaveKind::Jpeg {
                return self.save_engine(kind, &path_string);
            }
        }
        if suffix != "webp" && suffix != "jpg" && suffix != "jpeg" && suffix != "jpe" {
            return false;
        }
        let native = false;
        let embed = {
            let rust = self.rust();
            rust.save_embed_profile
                .unwrap_or_else(|| super::super::color_management::save_embed_default(rust, native))
        };
        let dirty = self.rust().session.dirty;
        let format = raster_writer_for_suffix(&suffix).unwrap_or("JPG");
        let gpu_compute = self.rust().gpu_compute;
        let options = self.rust().save_raster_options.unwrap_or_default();
        let keeps_working = format == "JPG";
        let (rgba, width, height, metadata) = {
            let mut rust = self.as_mut().rust_mut();
            let session = &mut rust.session;
            let Some(doc) = session.doc.as_mut() else {
                return false;
            };
            if dirty {
                pictura_render::refresh_native_composite(doc);
            }
            let buffer = current_buffer(doc, gpu_compute);
            let rgba = if keeps_working {
                buffer_to_rgba_bytes(&buffer)
            } else {
                let srgb = pictura_codec::buffer_to_srgb(doc, &buffer);
                buffer_to_rgba_bytes(&srgb)
            };
            let mut metadata = pictura_codec::raster_metadata(doc);
            metadata.vanishing_point = store_planes(session.vanishing_planes.as_ref(), doc);
            if keeps_working && embed {
                metadata.icc = Some(if doc.source_mode == Some(pictura_core::ColorMode::Cmyk) {
                    pictura_codec::Profile::srgb().to_icc()
                } else {
                    doc.document_icc
                        .clone()
                        .unwrap_or_else(|| pictura_codec::Profile::srgb().to_icc().into())
                        .to_vec()
                });
            }
            (rgba, buffer.width, buffer.height, metadata)
        };
        if !super::super::export::write_raster_with_metadata(
            &rgba,
            width,
            height,
            &path_string,
            format,
            &metadata,
            &options,
        ) {
            return false;
        }
        let mut view = self.as_mut().rust_mut();
        view.path = Some(path_string);
        view.session.mark_saved();
        true
    }

    fn save_engine(mut self: Pin<&mut Self>, kind: pictura_codec::SaveKind, path: &str) -> bool {
        let settings = self.rust().save_options.clone();
        let mut options = match settings {
            Some(settings) if settings.kind == kind.code() => settings.options,
            _ => {
                let old = self.rust().save_raster_options.unwrap_or_default();
                let mut options = pictura_codec::SaveOptions::default().with_defaults();
                options.png_compression = (old.compression.clamp(0, 100) / 11) as u8;
                match old.compression {
                    0 => options.tiff_compression = 0,
                    1 => options.tiff_compression = 2,
                    5 => options.tiff_compression = 4,
                    _ => {}
                }
                options.tiff_jpeg_quality = (old.quality.clamp(0, 100) * 12 / 100) as u8;
                options.tiff_transparency = old.matte != 0xFFFFFF;
                options
            }
        };
        let embed_tiff = if kind == pictura_codec::SaveKind::Tiff {
            let rust = self.rust();
            rust.save_embed_profile
                .unwrap_or_else(|| super::super::color_management::save_embed_default(rust, false))
        } else {
            false
        };
        let dirty = self.rust().session.dirty;
        let gpu_compute = self.rust().gpu_compute;
        let mut rust = self.as_mut().rust_mut();
        let Some(doc_ref) = rust.session.doc.as_ref() else {
            return false;
        };
        // The engine writers consume the flattened composite. Composite the
        // layer stack here, as Export As does: a document that keeps no stored
        // composite (or whose stored copy is stale) must not save blank.
        let mut doc = doc_ref.clone();
        if dirty {
            pictura_render::refresh_native_composite(&mut doc);
        }
        let vanishing =
            super::super::filter_tools::store_planes(rust.session.vanishing_planes.as_ref(), &doc);
        pictura_codec::set_vanishing_point_data(&mut doc, vanishing.as_deref());
        let Ok(_reservation) = pictura_core::memory::governor().reserve(
            super::super::helpers::memory_gate::save_bytes(&doc),
            "a save",
        ) else {
            return false;
        };
        doc.composite = super::super::helpers_composite::current_buffer(&doc, gpu_compute);
        if embed_tiff {
            let mode = doc.source_mode.unwrap_or(doc.mode);
            options.icc = match mode {
                pictura_core::ColorMode::Cmyk => {
                    pictura_codec::document_cmyk_profile(&doc).map(|profile| profile.to_icc())
                }
                pictura_core::ColorMode::Rgb => Some(
                    doc.document_icc
                        .clone()
                        .unwrap_or_else(|| pictura_codec::Profile::srgb().to_icc().into())
                        .to_vec(),
                ),
                _ => None,
            };
        }
        let jpeg = |samples: &[u8], width: u32, height: u32, components: u8, quality: u8| {
            let bytes = super::super::pdf_export::ffi::encode_jpeg(
                samples,
                width as i32,
                height as i32,
                i32::from(components),
                i32::from(quality),
            );
            (!bytes.is_empty()).then_some(bytes)
        };
        if pictura_codec::save_document_with_jpeg(
            kind,
            &doc,
            &options,
            Path::new(path),
            Some(&jpeg),
        )
        .is_err()
        {
            return false;
        }
        rust.path = Some(path.to_string());
        rust.source_format = if kind == pictura_codec::SaveKind::Dcs {
            "dcs".to_string()
        } else {
            Path::new(path)
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_else(|| "psd".to_string())
        };
        rust.session.mark_saved();
        true
    }
}
