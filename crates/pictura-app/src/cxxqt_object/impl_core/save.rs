//! Document save: PSD/PSB and raster with the per-document profile choice.

use super::*;

impl qobject::PictureView {
    pub fn save(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let path = path.to_string();
        let suffix = format_for_path(&path);
        if matches!(suffix.as_str(), "pdf" | "pdp") {
            return super::super::pdf_export::save_with_last_options(
                self,
                &QString::from(path.as_str()),
            );
        }
        let native = raster_writer_for_suffix(&suffix).is_none();
        let embed = {
            let rust = self.rust();
            rust.save_embed_profile
                .unwrap_or_else(|| super::super::color_management::save_embed_default(rust, native))
        };
        let dirty = self.rust().session.dirty;
        match raster_writer_for_suffix(&suffix) {
            None => {
                let bytes = {
                    let mut rust = self.as_mut().rust_mut();
                    let session = &mut rust.session;
                    let Some(doc) = session.doc.as_mut() else {
                        return false;
                    };
                    let planes = store_planes(session.vanishing_planes.as_ref(), doc);
                    if dirty {
                        pictura_render::refresh_native_composite(doc);
                    }
                    // A `.psb` path forces a version-2 PSB container; every other
                    // native suffix (including an unknown one) writes a PSD.
                    let mut doc = pictura_render::save_view(doc);
                    pictura_codec::set_vanishing_point_data(doc.to_mut(), planes.as_deref());
                    if !embed {
                        pictura_codec::remove_document_profile(doc.to_mut());
                    }
                    let written = if suffix == "psb" {
                        pictura_codec::write_psb(&doc)
                    } else {
                        pictura_codec::write_psd(&doc)
                    };
                    let Ok(bytes) = written else {
                        return false;
                    };
                    bytes
                };
                if super::super::helpers::safe_io::write_atomically(&path, &bytes, false).is_err() {
                    return false;
                }
            }
            Some(format) => {
                let gpu_compute = self.rust().gpu_compute;
                let options = self.rust().save_raster_options.unwrap_or_default();
                let keeps_working = matches!(
                    format.to_ascii_lowercase().as_str(),
                    "jpg" | "jpeg" | "jpe" | "tif" | "tiff"
                );
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
                        // The raster pixels are the working RGB values; a CMYK
                        // source keeps its profile only in PSD/PSB.
                        metadata.icc =
                            Some(if doc.source_mode == Some(pictura_core::ColorMode::Cmyk) {
                                pictura_codec::Profile::srgb().to_icc()
                            } else {
                                doc.document_icc
                                    .clone()
                                    .unwrap_or_else(|| pictura_codec::Profile::srgb().to_icc())
                            });
                    }
                    (rgba, buffer.width, buffer.height, metadata)
                };
                if !super::super::export::write_raster_with_metadata(
                    &rgba, width, height, &path, format, &metadata, &options,
                ) {
                    return false;
                }
            }
        }
        let mut view = self.as_mut().rust_mut();
        view.path = Some(path);
        view.session.mark_saved();
        true
    }
}
