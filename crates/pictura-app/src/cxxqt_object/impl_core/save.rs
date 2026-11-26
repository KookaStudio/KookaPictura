//! Document save: native PSD/PSB through the worker job, GIF through the web
//! encoder, raster with the per-document profile choice, and the engine Save As
//! writers. A copy writes the same bytes without recording the path or clearing
//! modified.

use super::*;
use crate::cxxqt_object::PictureViewRust;
use pictura_codec::{SaveFormat, SaveRoute};
use pictura_core::{ColorMode, Samples};
use std::path::Path;

/// Why the bytes are written. Only `Copy` changes the engine bookkeeping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SaveIntent {
    Save,
    Copy,
}

/// Resolve the save format for a call. An explicit catalog id wins; otherwise
/// the path suffix decides. The remembered `save_options.kind` is deliberately
/// not an input: a stale EPS/DCS choice must not route a later path.
pub(super) fn save_format_for_save(format_id: i32, suffix: &str) -> Option<SaveFormat> {
    let suffix_format = SaveFormat::from_suffix(suffix);
    if format_id == -1 {
        return suffix_format;
    }
    let format = SaveFormat::from_id(format_id)?;
    suffix_format.is_some().then_some(format)
}

impl qobject::PictureView {
    pub fn save(mut self: Pin<&mut Self>, path: &QString) -> bool {
        self.as_mut().save_with_intent(path, SaveIntent::Save, -1)
    }

    pub fn save_copy(mut self: Pin<&mut Self>, path: &QString) -> bool {
        self.as_mut().save_with_intent(path, SaveIntent::Copy, -1)
    }

    /// Save with a format the shell resolved. `format_id` is a catalog ordinal;
    /// `-1` resolves from the path.
    pub(in crate::cxxqt_object) fn save_as_format(
        mut self: Pin<&mut Self>,
        path: &QString,
        format_id: i32,
        copy: bool,
    ) -> bool {
        let intent = if copy {
            SaveIntent::Copy
        } else {
            SaveIntent::Save
        };
        self.as_mut().save_with_intent(path, intent, format_id)
    }

    fn save_with_intent(
        mut self: Pin<&mut Self>,
        path: &QString,
        intent: SaveIntent,
        format_id: i32,
    ) -> bool {
        if self.rust().session.read_only() {
            return false;
        }
        self.as_mut().settle_pending_composite();
        let path_string = path.to_string();
        self.as_mut().rust_mut().session.skipped_xmp_mirror = false;
        self.as_mut().rust_mut().session.history_log_save_warning = None;
        if crate::history_log::destination_conflicts(&path_string)
            || super::super::open_jobs::compatibility::destination_is_protected(Path::new(
                &path_string,
            ))
        {
            return false;
        }
        if Path::new(&path_string)
            .extension()
            .is_none_or(|extension| extension.is_empty())
        {
            return false;
        }
        let suffix = format_for_path(&path_string);
        let commit = intent != SaveIntent::Copy;
        let Some(format) = save_format_for_save(format_id, &suffix) else {
            return false;
        };
        match format.route() {
            SaveRoute::Pdf => super::super::pdf_export::save_with_last_options(
                self,
                &QString::from(path_string.as_str()),
                commit,
            ),
            SaveRoute::Native => {
                if !super::save_jobs::save_start_rust(
                    &mut self.as_mut().rust_mut(),
                    &path_string,
                    format.id(),
                    !commit,
                ) {
                    crate::diagnostic_logging::outcome("document.save", false);
                    return false;
                }
                super::save_jobs::save_finish_rust(&mut self.as_mut().rust_mut())
            }
            SaveRoute::AppGif => {
                let written = write_gif(self.as_mut().rust_mut().get_mut(), &path_string);
                crate::diagnostic_logging::outcome("document.save", written);
                if written && commit {
                    let mut rust = self.as_mut().rust_mut();
                    rust.path = Some(path_string);
                    rust.source_format = "gif".to_string();
                    rust.session.mark_saved();
                }
                written
            }
            SaveRoute::Engine(kind) => {
                let written = self.save_engine(kind, &path_string, commit);
                crate::diagnostic_logging::outcome("document.save", written);
                written
            }
            SaveRoute::QtRaster(writer) => {
                let written = self.as_mut().save_qt_raster(&path_string, writer, commit);
                crate::diagnostic_logging::outcome("document.save", written);
                written
            }
        }
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
        let (rgba, width, height, metadata, history, omitted) = {
            let mut rust = self.as_mut().rust_mut();
            let session = &mut rust.session;
            let Some(doc) = session.doc.as_mut() else {
                return false;
            };
            super::super::helpers::memory_gate::clear();
            // ponytail: Qt's encoders take the whole image; the gate counts it.
            let Some(_reservation) = super::super::helpers::save_gate::reserve_save(
                &super::super::helpers::save_gate::qt_raster_cost(doc),
            ) else {
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
            let (history, omitted) = if keeps_working {
                match session
                    .history_log
                    .stage_bytes(metadata.xmp.as_deref(), pictura_codec::JPEG_XMP_MAX_BYTES)
                {
                    Ok(history) => {
                        if let Some(staged) = &history {
                            metadata.xmp = Some(staged.packet().as_bytes().to_vec());
                        }
                        (history, false)
                    }
                    Err(()) => (None, true),
                }
            } else {
                (None, false)
            };
            metadata.vanishing_point = store_planes(session.vanishing_planes.as_ref(), doc);
            if keeps_working && embed {
                metadata.icc = Some(if doc.source.mode() == Some(ColorMode::Cmyk) {
                    pictura_codec::Profile::srgb().to_icc()
                } else {
                    doc.source
                        .working_profile()
                        .clone()
                        .unwrap_or_else(|| pictura_codec::Profile::srgb().to_icc().into())
                        .to_vec()
                });
            }
            (
                rgba,
                buffer.width,
                buffer.height,
                metadata,
                history,
                omitted,
            )
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
        let mut view = self.as_mut().rust_mut();
        view.session.history_log_save_warning =
            omitted.then_some(crate::history_log::APPEND_OMITTED);
        if commit {
            acknowledge_raster_history(&mut view, history, path_string);
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
        let mut rust = self.as_mut().rust_mut();
        let Some(doc_ref) = rust.session.doc.as_ref() else {
            return false;
        };
        let mut doc = doc_ref.clone();
        let (history, omitted) = if kind == pictura_codec::SaveKind::Tiff {
            match rust.session.history_log.stage_document(&mut doc) {
                Ok(history) => (history, false),
                Err(()) => (None, true),
            }
        } else {
            (None, false)
        };
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
        super::super::helpers::memory_gate::clear();
        let Some(_reservation) = super::super::helpers::save_gate::reserve_save(
            &super::super::helpers::save_gate::raster_save_cost(&doc, kind, &options),
        ) else {
            return false;
        };
        let source_mode = doc.source.mode().unwrap_or(doc.mode);
        let srgb = matches!(
            kind,
            pictura_codec::SaveKind::Png
                | pictura_codec::SaveKind::Bmp
                | pictura_codec::SaveKind::Tga
                | pictura_codec::SaveKind::Gif
                | pictura_codec::SaveKind::Eps
                | pictura_codec::SaveKind::Dcs
                | pictura_codec::SaveKind::Pict
        ) && source_mode == ColorMode::Rgb;
        doc.composite
            .replace(save_composite(&doc, srgb))
            .expect("valid composite");
        if embed_tiff {
            let mode = doc.source.mode().unwrap_or(doc.mode);
            options.icc = match mode {
                ColorMode::Cmyk => {
                    pictura_codec::document_cmyk_profile(&doc).map(|profile| profile.to_icc())
                }
                ColorMode::Rgb => Some(
                    doc.source
                        .working_profile()
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
        rust.session.history_log_save_warning =
            omitted.then_some(crate::history_log::APPEND_OMITTED);
        if commit {
            acknowledge_raster_history(&mut rust, history, path);
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

fn acknowledge_raster_history(
    rust: &mut PictureViewRust,
    staged: Option<crate::history_log::StagedHistory>,
    path: &str,
) {
    let session = &mut rust.session;
    if let (Some(history), Some(doc)) = (staged, session.doc.as_mut()) {
        if !session.history_log.acknowledge(doc, history) {
            session.history_log_save_warning = Some(crate::history_log::REBASE_FAILED);
        }
    }
    session.history_log.set_filename(path);
}

/// The composite a save writer consumes, composited a band at a time into a
/// tiled planar buffer (#1320): uniform tiles cost nothing and dense ones can
/// be packed or spilled while the writer reads them. sRGB-only writers convert
/// a profiled working-space composite band by band; TIFF/Raw retain the
/// working/mode samples. A layerless document keeps its stored channel layout.
pub(in crate::cxxqt_object) fn save_composite(
    doc: &pictura_core::Document,
    srgb: bool,
) -> pictura_core::PixelBuffer {
    let convert = srgb && doc.source.working_profile().is_some();
    let (w, h) = (doc.width as usize, doc.height as usize);
    let channels = if doc.layers.is_empty() {
        doc.composite.channels as usize
    } else {
        4
    };
    let mut data = pictura_core::Plane::filled(w, h, channels.max(1), 0u8);
    let mut put = |offset: usize, band: &pictura_core::PixelBuffer| {
        let band = if convert {
            pictura_codec::buffer_to_srgb(doc, band)
        } else {
            std::borrow::Cow::Borrowed(band)
        };
        let rows = band.height as usize;
        let n = w * rows;
        let mut samples = vec![0u8; n];
        for c in 0..channels.min(band.channels as usize) {
            band.data.read_into(c * n, &mut samples);
            let top = c * h + offset;
            data.write_rect_from(0..w, top..top + rows, w, &samples);
        }
        data.settle();
    };
    let rows = pictura_render::band_rows(w) as usize;
    if doc.layers.is_empty() {
        if !convert {
            return doc.composite.buffer().clone();
        }
        let mut band_data = Vec::new();
        for y0 in (0..h).step_by(rows.max(1)) {
            let band_rows = rows.min(h - y0);
            band_data.clear();
            let mut plane = vec![0u8; w * band_rows];
            for c in 0..channels {
                doc.composite.data.read_rect_into(
                    0..w,
                    c * h + y0..c * h + y0 + band_rows,
                    w,
                    &mut plane,
                );
                band_data.extend_from_slice(&plane);
            }
            let band = pictura_core::PixelBuffer {
                width: doc.width,
                height: band_rows as u32,
                channels: channels as u8,
                data: std::mem::take(&mut band_data).into(),
            };
            put(y0, &band);
        }
    } else {
        let whole = pictura_core::PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        };
        pictura_render::stream_composite(doc, whole, rows as u32, |offset, band| {
            put(offset as usize, &band)
        });
    }
    pictura_core::PixelBuffer {
        width: doc.width,
        height: doc.height,
        channels: channels.max(1) as u8,
        data,
    }
}

/// Write `path` as a single-frame GIF89a. The retained Indexed source is used
/// only while it still expands to the current composite; otherwise the 8-bit
/// sRGB composite is quantized with the remembered palette settings.
pub(in crate::cxxqt_object) fn write_gif(rust: &mut PictureViewRust, path: &str) -> bool {
    let gpu_compute = rust.gpu_compute;
    let dirty = rust.session.dirty;
    let options = rust.save_gif_options.unwrap_or_default();
    let unreadable_before = pictura_core::memory::eviction::unreadable_reads();
    let bytes = {
        let session = &mut rust.session;
        let Some(doc) = session.doc.as_mut() else {
            return false;
        };
        if doc.width == 0 || doc.height == 0 || doc.width > 65_535 || doc.height > 65_535 {
            return false;
        }
        super::super::helpers::memory_gate::clear();
        // ponytail: the quantizer reads every pixel at once; the gate counts it.
        let Some(_reservation) = super::super::helpers::save_gate::reserve_save(
            &super::super::helpers::save_gate::gif_cost(doc),
        ) else {
            return false;
        };
        // A tile lost from scratch reads back as defaults; committing those
        // bytes would overwrite a good GIF with a damaged one (#1217).
        if crate::history::unreadable_tiles(doc) > 0 {
            super::super::helpers::memory_gate::refuse_for_scratch();
            return false;
        }
        if dirty {
            pictura_render::refresh_native_composite(doc);
        }
        let bytes = if pictura_codec::gif_indexed_source_is_current(doc) {
            let Some(palette) = doc.source.palette() else {
                return false;
            };
            let Some(planes) = doc.source.planes().as_ref() else {
                return false;
            };
            let Samples::U8(samples) = &planes.samples else {
                return false;
            };
            let plane = doc.width as usize * doc.height as usize;
            let Some(indices) = samples.as_slice().get(..plane) else {
                return false;
            };
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
                pictura_codec::quantize(&rgba, doc.width, doc.height, options.palette_options());
            pictura_codec::encode_gif(&indexed, options.interlaced)
        };
        // A tile can be lost while encoding, after the preflight. Reject the
        // encoded bytes before the atomic write for the same reason.
        if pictura_core::memory::eviction::unreadable_reads() != unreadable_before
            && crate::history::unreadable_tiles(doc) > 0
        {
            return false;
        }
        bytes
    };
    super::super::helpers::safe_io::write_atomically(path, &bytes, false).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_codec::{ColorReduction, Dither, PaletteOptions};
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn save_composite_converts_profiled_rgb_for_srgb_writers() {
        let mut doc = pictura_core::Document::from_rgba("srgb", 1, 1, &[200, 100, 50, 255]);
        pictura_codec::assign_document_profile(&mut doc, &pictura_codec::Profile::adobe_rgb());
        let native = save_composite(&doc, false);
        let srgb = save_composite(&doc, true);
        assert_ne!(native.data, srgb.data, "the working profile is converted");
        let expected = pictura_codec::buffer_to_srgb(&doc, &native).into_owned();
        assert_eq!(srgb.data, expected.data);
    }

    #[test]
    fn save_composite_equals_the_whole_composite() {
        let (w, h) = (37u32, 300u32);
        let rgba: Vec<u8> = (0..w * h * 4).map(|i| (i * 7 % 251) as u8).collect();
        let mut doc = pictura_core::Document::from_rgba("Layer 0", w, h, &rgba);
        let mut top = doc.layers[0].clone();
        top.opacity = 100;
        for channel in top.channels_mut() {
            let flipped: Vec<u8> = channel.data.iter().copied().rev().collect();
            channel.data = flipped.into();
        }
        doc.layers.push(top).expect("a second layer");
        pictura_codec::assign_document_profile(&mut doc, &pictura_codec::Profile::adobe_rgb());
        let whole = current_buffer(&doc, false);
        for srgb in [false, true] {
            let expected = if srgb {
                pictura_codec::buffer_to_srgb(&doc, &whole).into_owned()
            } else {
                whole.clone()
            };
            let banded = save_composite(&doc, srgb);
            assert_eq!(banded.channels, expected.channels);
            assert_eq!(banded.data.to_vec(), expected.data.to_vec(), "srgb {srgb}");
        }
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
    fn explicit_format_selects_the_route() {
        assert_eq!(save_format_for_save(-1, "png"), Some(SaveFormat::Png));
        assert_eq!(
            save_format_for_save(SaveFormat::Dcs.id(), "png"),
            Some(SaveFormat::Dcs)
        );
        assert_eq!(save_format_for_save(i32::MAX, "png"), None);
        assert_eq!(save_format_for_save(-2, "png"), None);
        assert_eq!(save_format_for_save(SaveFormat::Png.id(), "xyz"), None);
        assert_eq!(save_format_for_save(-1, "eps"), Some(SaveFormat::Eps));
        assert_eq!(save_format_for_save(-1, "dcs"), None);
        assert_eq!(save_format_for_save(-1, ""), None);
        assert_eq!(
            SaveFormat::Dcs.route(),
            SaveRoute::Engine(pictura_codec::SaveKind::Dcs)
        );
    }

    #[test]
    fn gif_bytes_prefer_the_indexed_source() {
        let mut doc = rgb_doc(4, 4);
        pictura_render::convert_to_indexed(&mut doc, options()).unwrap();
        assert!(pictura_codec::indexed_source_is_current(&doc));

        let palette = doc.source.palette().expect("retained palette");
        let plane = doc.width as usize * doc.height as usize;
        let indices = {
            let mut planes = doc.source.planes().clone().expect("retained planes");
            let Samples::U8(samples) = &mut planes.samples else {
                panic!("expected an 8-bit index plane");
            };
            let mut samples = samples.to_vec();
            // The merged store also keeps alpha and document channels after the
            // index plane; GIF reads only the first plane.
            samples.extend(std::iter::repeat_n(0xA5, plane));
            planes.samples = Samples::U8(samples.clone().into());
            doc.source
                .replace_planes(Some(planes))
                .expect("retained index planes");
            samples[..plane].to_vec()
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

        // A full refresh stores the working RGBA frame; GIF still accepts it
        // while the RGB planes match and alpha stays opaque.
        let mut refreshed = doc.composite.data.get(..3 * plane).unwrap().to_vec();
        refreshed.extend(std::iter::repeat_n(255, plane));
        doc.composite
            .replace(pictura_core::PixelBuffer {
                width: doc.width,
                height: doc.height,
                channels: 4,
                data: refreshed.into(),
            })
            .expect("valid composite");
        assert!(!pictura_codec::indexed_source_is_current(&doc));
        assert!(pictura_codec::gif_indexed_source_is_current(&doc));
        let mut transparent = doc.clone();
        transparent
            .composite
            .edit_data(|data| {
                data.as_mut_slice()[3 * plane] = 254;
            })
            .expect("pixel edit preserves geometry");
        assert!(!pictura_codec::gif_indexed_source_is_current(&transparent));

        let mut rust = PictureViewRust::default();
        rust.session.doc.set(Some(doc));
        rust.session.dirty = true;
        let target = path("indexed.gif");
        assert!(write_gif(&mut rust, &target));
        assert_eq!(std::fs::read(&target).unwrap(), expected);
        let _ = std::fs::remove_file(&target);
    }

    #[test]
    fn gif_refuses_unreadable_tiles() {
        use crate::cxxqt_object::scratch_rollback::tests::{dense_doc, lose_tiles};
        let mut rust = PictureViewRust::default();
        rust.session.doc.set(Some(dense_doc()));
        rust.session.dirty = true;
        lose_tiles(rust.session.doc.as_ref().unwrap());
        super::super::super::helpers::memory_gate::clear();
        let target = path("unreadable.gif");
        assert!(!write_gif(&mut rust, &target));
        assert!(super::super::super::helpers::memory_gate::edit_refused_for_scratch());
        assert!(
            !std::path::Path::new(&target).exists(),
            "nothing was written"
        );
    }

    #[test]
    fn edited_indexed_source_is_stale() {
        let mut doc = rgb_doc(4, 4);
        pictura_render::convert_to_indexed(&mut doc, options()).unwrap();
        doc.composite
            .edit_data(|data| {
                data.as_mut_slice()[0] ^= 0xFF;
            })
            .expect("pixel edit preserves geometry");
        doc.layers[0].channels_mut()[0].data.as_mut_slice()[0] ^= 0xFF;
        assert!(!pictura_codec::indexed_source_is_current(&doc));
        let mut rust = PictureViewRust::default();
        rust.session.doc.set(Some(doc));
        rust.session.dirty = true;
        write(&mut rust, "edited.gif");
    }

    #[test]
    fn gif_bytes_quantize_the_composite() {
        let mut rust = PictureViewRust::default();
        rust.session.doc.set(Some(rgb_doc(8, 8)));
        rust.session.dirty = true;
        write(&mut rust, "quantized.gif");
    }

    #[test]
    fn gif_bytes_reject_oversized_dimensions() {
        let mut rust = PictureViewRust::default();
        rust.session.doc.set(Some(rgb_doc(70_000, 1)));
        rust.session.dirty = true;
        let target = path("wide.gif");
        assert!(!write_gif(&mut rust, &target));
        assert!(!std::path::Path::new(&target).exists());
    }

    #[test]
    fn gif_write_failure_preserves_previous_file() {
        let mut rust = PictureViewRust::default();
        rust.session.doc.set(Some(rgb_doc(4, 4)));
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
