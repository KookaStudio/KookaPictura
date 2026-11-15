//! Document save: native PSD/PSB through the worker job, GIF through the web
//! encoder, raster with the per-document profile choice, and the engine Save As
//! writers. A copy writes the same bytes without recording the path or clearing
//! modified.

use super::*;
use crate::cxxqt_object::PictureViewRust;
use pictura_codec::{ColorReduction, Dither, PaletteOptions};
use pictura_core::{ColorMode, Samples};
use std::path::Path;

/// Why the bytes are written. Only `Copy` changes the engine bookkeeping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SaveIntent {
    Save,
    Copy,
}

impl qobject::PictureView {
    pub fn save(mut self: Pin<&mut Self>, path: &QString) -> bool {
        self.as_mut().save_with_intent(path, SaveIntent::Save)
    }

    pub fn save_copy(mut self: Pin<&mut Self>, path: &QString) -> bool {
        self.as_mut().save_with_intent(path, SaveIntent::Copy)
    }

    fn save_with_intent(mut self: Pin<&mut Self>, path: &QString, intent: SaveIntent) -> bool {
        self.as_mut().settle_pending_composite();
        let path_string = path.to_string();
        if Path::new(&path_string)
            .extension()
            .is_none_or(|extension| extension.is_empty())
        {
            return false;
        }
        let suffix = format_for_path(&path_string);
        let commit = intent != SaveIntent::Copy;
        if matches!(suffix.as_str(), "pdf" | "pdp") {
            return super::super::pdf_export::save_with_last_options(
                self,
                &QString::from(path_string.as_str()),
                commit,
            );
        }
        if matches!(suffix.as_str(), "psd" | "psb") {
            if !super::save_jobs::save_start_rust(
                &mut self.as_mut().rust_mut(),
                &path_string,
                !commit,
            ) {
                return false;
            }
            return super::save_jobs::save_finish_rust(&mut self.as_mut().rust_mut());
        }
        if suffix == "gif" {
            let written = write_gif(self.as_mut().rust_mut().get_mut(), &path_string);
            if written && commit {
                let mut rust = self.as_mut().rust_mut();
                rust.path = Some(path_string);
                rust.source_format = "gif".to_string();
                rust.session.mark_saved();
            }
            return written;
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
                return self.save_engine(kind, &path_string, commit);
            }
        }
        if suffix != "webp" && suffix != "jpg" && suffix != "jpeg" && suffix != "jpe" {
            return false;
        }
        let format = raster_writer_for_suffix(&suffix).unwrap_or("JPG");
        self.as_mut().save_qt_raster(&path_string, format, commit)
    }

    fn save_qt_raster(
        mut self: Pin<&mut Self>,
        path_string: &str,
        format: &'static str,
        commit: bool,
    ) -> bool {
        let native = false;
        let embed = {
            let rust = self.rust();
            rust.save_embed_profile
                .unwrap_or_else(|| super::super::color_management::save_embed_default(rust, native))
        };
        let dirty = self.rust().session.dirty;
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
                metadata.icc = Some(if doc.source_mode == Some(ColorMode::Cmyk) {
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
            path_string,
            format,
            &metadata,
            &options,
        ) {
            return false;
        }
        if commit {
            let mut view = self.as_mut().rust_mut();
            view.path = Some(path_string.to_string());
            view.session.mark_saved();
        }
        true
    }

    fn save_engine(
        mut self: Pin<&mut Self>,
        kind: pictura_codec::SaveKind,
        path: &str,
        commit: bool,
    ) -> bool {
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
        let mut doc = doc_ref.clone();
        if crate::history::unreadable_tiles(&doc) > 0 {
            super::super::helpers::memory_gate::refuse_for_scratch();
            return false;
        }
        let unreadable_before = pictura_core::memory::eviction::unreadable_reads();
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
        let source_mode = doc.source_mode.unwrap_or(doc.mode);
        let srgb = matches!(
            kind,
            pictura_codec::SaveKind::Png
                | pictura_codec::SaveKind::Bmp
                | pictura_codec::SaveKind::Tga
                | pictura_codec::SaveKind::Gif
                | pictura_codec::SaveKind::Eps
                | pictura_codec::SaveKind::Dcs
        ) && source_mode == ColorMode::Rgb;
        doc.composite = save_composite(&doc, gpu_compute, srgb);
        if embed_tiff {
            let mode = doc.source_mode.unwrap_or(doc.mode);
            options.icc = match mode {
                ColorMode::Cmyk => {
                    pictura_codec::document_cmyk_profile(&doc).map(|profile| profile.to_icc())
                }
                ColorMode::Rgb => Some(
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
        let guard = || {
            if pictura_core::memory::eviction::unreadable_reads() != unreadable_before
                && crate::history::unreadable_tiles(&doc) > 0
            {
                super::super::helpers::memory_gate::refuse_for_scratch();
                false
            } else {
                true
            }
        };
        if pictura_codec::save_document_with_jpeg_guarded(
            kind,
            &doc,
            &options,
            Path::new(path),
            Some(&jpeg),
            Some(&guard),
        )
        .is_err()
        {
            return false;
        }
        if commit {
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
        }
        true
    }
}

/// The composite a save writer consumes. sRGB-only writers convert a profiled
/// working-space composite; TIFF/Raw retain the working/mode samples.
pub(super) fn save_composite(
    doc: &pictura_core::Document,
    gpu_compute: bool,
    srgb: bool,
) -> pictura_core::PixelBuffer {
    let buffer = current_buffer(doc, gpu_compute);
    if srgb && doc.document_icc.is_some() {
        pictura_codec::buffer_to_srgb(doc, &buffer).into_owned()
    } else {
        buffer
    }
}

/// Write `path` as a single-frame GIF89a. The retained Indexed source is used
/// only while it still expands to the current composite; otherwise the 8-bit
/// sRGB composite is quantized with the remembered palette settings.
fn write_gif(rust: &mut PictureViewRust, path: &str) -> bool {
    let gpu_compute = rust.gpu_compute;
    let dirty = rust.session.dirty;
    let options = rust.save_gif_options.unwrap_or_default();
    let bytes = {
        let session = &mut rust.session;
        let Some(doc) = session.doc.as_mut() else {
            return false;
        };
        if doc.width == 0 || doc.height == 0 || doc.width > 65_535 || doc.height > 65_535 {
            return false;
        }
        if dirty {
            pictura_render::refresh_native_composite(doc);
        }
        if pictura_codec::indexed_source_is_current(doc) {
            let Some(palette) = doc.source_palette else {
                return false;
            };
            let Some(planes) = doc.source_planes.as_ref() else {
                return false;
            };
            let Samples::U8(indices) = &planes.samples else {
                return false;
            };
            if indices.len() != doc.width as usize * doc.height as usize {
                return false;
            }
            let highest = indices.iter().copied().max().unwrap_or(0) as usize;
            let count = (highest + 1).clamp(2, 256);
            let palette = (0..count)
                .map(|i| [palette[i], palette[256 + i], palette[512 + i], 255])
                .collect();
            pictura_codec::encode_gif(
                &pictura_codec::Indexed {
                    width: doc.width,
                    height: doc.height,
                    palette,
                    indices: indices.to_vec(),
                },
                options.interlaced,
            )
        } else {
            let buffer = current_buffer(doc, gpu_compute);
            let srgb = pictura_codec::buffer_to_srgb(doc, &buffer);
            let rgba = buffer_to_rgba_bytes(&srgb);
            let indexed =
                pictura_codec::quantize(&rgba, doc.width, doc.height, gif_options(&options));
            pictura_codec::encode_gif(&indexed, options.interlaced)
        }
    };
    super::super::helpers::safe_io::write_atomically(path, &bytes, false).is_ok()
}

fn gif_options(options: &super::super::save_format::SaveGifOptions) -> PaletteOptions {
    let reduction = match options.palette {
        0 => ColorReduction::Adaptive,
        1 => ColorReduction::Restrictive,
        2 => ColorReduction::Perceptual,
        3 => ColorReduction::Selective,
        _ => ColorReduction::Adaptive,
    };
    let colors = match options.palette {
        0 => 256,
        1 => 216,
        _ => options.colors,
    };
    let dither = match (options.palette, options.dither) {
        (0, _) => Dither::None,
        (_, 1) => Dither::Diffusion,
        (_, 2) => Dither::Pattern,
        (_, 3) => Dither::Noise,
        _ => Dither::None,
    };
    PaletteOptions {
        reduction,
        colors: colors.clamp(2, 256) as u16,
        dither,
        amount: options.amount.clamp(0, 100) as u8,
        transparency: false,
        mapped_transparency: false,
        matte: [255, 255, 255],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn save_composite_converts_profiled_rgb_for_srgb_writers() {
        let mut doc = pictura_core::Document::from_rgba("srgb", 1, 1, &[200, 100, 50, 255]);
        pictura_codec::assign_document_profile(&mut doc, &pictura_codec::Profile::adobe_rgb());
        let native = save_composite(&doc, false, false);
        let srgb = save_composite(&doc, false, true);
        assert_ne!(native.data, srgb.data, "the working profile is converted");
        let expected = pictura_codec::buffer_to_srgb(&doc, &native).into_owned();
        assert_eq!(srgb.data, expected.data);
    }


    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn path(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!(
            "pictura-gif-save-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name).to_string_lossy().into_owned()
    }

    fn rgb_doc(width: u32, height: u32) -> pictura_core::Document {
        let rgba: Vec<u8> = (0..width * height)
            .flat_map(|i| {
                [
                    (i % 251) as u8,
                    ((i * 7) % 251) as u8,
                    ((i * 13) % 251) as u8,
                    255,
                ]
            })
            .collect();
        pictura_core::Document::from_rgba("Layer 0", width, height, &rgba)
    }

    fn options() -> PaletteOptions {
        PaletteOptions {
            reduction: ColorReduction::Selective,
            colors: 16,
            dither: Dither::None,
            amount: 0,
            transparency: false,
            mapped_transparency: false,
            matte: [255, 255, 255],
        }
    }

    fn write(rust: &mut PictureViewRust, name: &str) -> String {
        let target = path(name);
        assert!(write_gif(rust, &target));
        assert_eq!(&std::fs::read(&target).unwrap()[..6], b"GIF89a");
        let _ = std::fs::remove_file(&target);
        target
    }

    #[test]
    fn gif_bytes_prefer_the_indexed_source() {
        let mut doc = rgb_doc(4, 4);
        pictura_render::convert_to_indexed(&mut doc, options()).unwrap();
        assert!(pictura_codec::indexed_source_is_current(&doc));

        let palette = doc.source_palette.expect("retained palette");
        let indices = {
            let planes = doc.source_planes.as_ref().expect("retained planes");
            let Samples::U8(indices) = &planes.samples else {
                panic!("expected an 8-bit index plane");
            };
            indices.to_vec()
        };
        let highest = indices.iter().copied().max().unwrap_or(0) as usize;
        let count = (highest + 1).clamp(2, 256);
        let palette: Vec<[u8; 4]> = (0..count)
            .map(|i| [palette[i], palette[256 + i], palette[512 + i], 255])
            .collect();
        let expected = pictura_codec::encode_gif(
            &pictura_codec::Indexed {
                width: doc.width,
                height: doc.height,
                palette,
                indices,
            },
            false,
        );

        let mut rust = PictureViewRust::default();
        rust.session.doc = Some(doc);
        rust.session.dirty = true;
        let target = path("indexed.gif");
        assert!(write_gif(&mut rust, &target));
        assert_eq!(std::fs::read(&target).unwrap(), expected);
        let _ = std::fs::remove_file(&target);
    }

    #[test]
    fn edited_indexed_source_is_stale() {
        let mut doc = rgb_doc(4, 4);
        pictura_render::convert_to_indexed(&mut doc, options()).unwrap();
        doc.composite.data.as_mut_slice()[0] ^= 0xFF;
        doc.layers[0].channels[0].data.as_mut_slice()[0] ^= 0xFF;
        assert!(!pictura_codec::indexed_source_is_current(&doc));
        let mut rust = PictureViewRust::default();
        rust.session.doc = Some(doc);
        rust.session.dirty = true;
        write(&mut rust, "edited.gif");
    }

    #[test]
    fn gif_bytes_quantize_the_composite() {
        let mut rust = PictureViewRust::default();
        rust.session.doc = Some(rgb_doc(8, 8));
        rust.session.dirty = true;
        write(&mut rust, "quantized.gif");
    }

    #[test]
    fn gif_bytes_reject_oversized_dimensions() {
        let mut rust = PictureViewRust::default();
        rust.session.doc = Some(rgb_doc(70_000, 1));
        rust.session.dirty = true;
        let target = path("wide.gif");
        assert!(!write_gif(&mut rust, &target));
        assert!(!std::path::Path::new(&target).exists());
    }

    #[test]
    fn gif_write_failure_preserves_previous_file() {
        let mut rust = PictureViewRust::default();
        rust.session.doc = Some(rgb_doc(4, 4));
        rust.session.dirty = true;
        let dir = path("locked.gif");
        std::fs::create_dir_all(&dir).unwrap();
        let target = format!("{dir}/out.gif");
        std::fs::write(&target, b"old bytes").unwrap();
        let mut permissions = std::fs::metadata(&dir).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&dir, permissions).unwrap();
        let written = write_gif(&mut rust, &target);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(&dir, permissions).unwrap();
        }
        #[cfg(not(unix))]
        {
            let mut permissions = std::fs::metadata(&dir).unwrap().permissions();
            permissions.set_readonly(false);
            std::fs::set_permissions(&dir, permissions).unwrap();
        }
        if written {
            // A privileged test runner can write through the directory mode.
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }
        assert_eq!(std::fs::read(&target).unwrap(), b"old bytes");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
