use super::*;

use super::super::super::color_management::save_embed_default;
use super::super::super::filter_tools::store_planes;
use super::super::super::state::PictureViewRust;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use std::path::Path;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn can_omit_merged(doc: &pictura_core::Document) -> bool {
    !doc.layers.is_empty() && doc.channels.is_empty()
}

/// The live native-save worker and the document identity it snapshotted.
pub(in crate::cxxqt_object) struct SaveJob {
    handle: std::thread::JoinHandle<(SaveOutcome, bool)>,
    progress: Arc<AtomicI32>,
    path: String,
    start_history: Option<u64>,
    start_revision: u64,
    print_revision: u64,
    start_metadata: u64,
    generation: u64,
    copy: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
enum SaveOutcome {
    Failed = 0,
    Saved = 1,
    Uncertain = 2,
    Replaced = 3,
    ReplacedFailure = 4,
}

pub(in crate::cxxqt_object) fn save_start_rust(
    rust: &mut PictureViewRust,
    path: &str,
    copy: bool,
) -> bool {
    save_start_with_compatibility(rust, path, copy, None)
}

fn save_start_with_compatibility(
    rust: &mut PictureViewRust,
    path: &str,
    copy: bool,
    compatibility: Option<bool>,
) -> bool {
    super::super::super::helpers::memory_gate::clear();
    rust.session.skipped_xmp_mirror = false;
    if rust.session.read_only()
        || super::super::super::open_jobs::compatibility::destination_is_protected(Path::new(path))
    {
        return false;
    }
    if rust.save_job.is_some() {
        return false;
    }
    let suffix = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| !extension.is_empty())
        .map(|extension| extension.to_ascii_lowercase());
    let Some(suffix) = suffix else {
        return false;
    };
    if !matches!(suffix.as_str(), "psd" | "psb") {
        return false;
    }
    let session = &rust.session;
    let Some(doc_ref) = session.doc.as_ref() else {
        return false;
    };
    // A document missing tiles lost from scratch is never written (#1217).
    if crate::history::unreadable_tiles(doc_ref) > 0 {
        super::super::super::helpers::memory_gate::refuse_for_scratch();
        return false;
    }
    // Held by the worker until its write lands, so other edits see the room
    // the save is using.
    let Some(reservation) = super::super::super::helpers::save_gate::reserve_save(
        &super::super::super::helpers::save_gate::save_cost(doc_ref),
    ) else {
        return false;
    };
    let embed = rust
        .save_embed_profile
        .unwrap_or_else(|| save_embed_default(rust, true));
    let dirty = session.dirty;
    let planes = store_planes(session.vanishing_planes.as_ref(), doc_ref);
    let start_history = session.history.current_id();
    let start_revision = session.content_revision;
    let print_revision = session.print_revision;
    let start_metadata = session.persisted_metadata_revision;
    let generation = session.document_generation;
    let cache = Arc::clone(&session.encode_cache);
    pictura_codec::warm_encode_idents(doc_ref);
    let doc = doc_ref.clone();
    let print_resources = session.print_resources.clone();
    let delay = rust.save_delay_for_test;
    let sync_failure = rust.save_sync_failure_for_test;
    let psb = suffix == "psb";
    let worker_path = path.to_string();
    let job_path = path.to_string();
    let progress = Arc::new(AtomicI32::new(0));
    let worker_progress = progress.clone();
    let handle = std::thread::spawn(move || {
        let _reservation = reservation;
        let _efficiency = pictura_core::scratch::operation();
        let unreadable_before = pictura_core::memory::eviction::unreadable_reads();
        let mut doc = doc;
        print_resources.apply(&mut doc);
        if let Some(include) = compatibility {
            prepare_compatibility_save(&mut doc, include, dirty);
        } else {
            prepare_native_save(&mut doc, dirty);
        }
        worker_progress.store(30, Ordering::Relaxed);
        // A `.psb` path forces a version-2 PSB container; every other native
        // suffix writes a PSD.
        let mut doc = pictura_render::save_view(&doc);
        pictura_codec::set_vanishing_point_data(doc.to_mut(), planes.as_deref());
        if !embed {
            pictura_codec::remove_document_profile(doc.to_mut());
        }
        use super::super::super::helpers::safe_io::{
            sync_directory, write_durable_with, DurableWriteError,
        };
        // The file streams into the temp file a band of rows at a time (#1320).
        let mut skipped_xmp_mirror = false;
        let result = write_durable_with(
            Path::new(&worker_path),
            |out| {
                skipped_xmp_mirror = pictura_codec::write_versioned_to(
                    out,
                    &doc,
                    psb,
                    Some(&cache),
                    env!("CARGO_PKG_VERSION"),
                    compatibility.is_some(),
                )
                .map_err(|e| std::io::Error::other(e.to_string()))?;
                // A tile lost while encoding wrote defaults in its place.
                if pictura_core::memory::eviction::unreadable_reads() != unreadable_before
                    && crate::history::unreadable_tiles(&doc) > 0
                {
                    return Err(std::io::Error::other("tiles lost while encoding"));
                }
                worker_progress.store(85, Ordering::Relaxed);
                if delay > 0 {
                    std::thread::sleep(Duration::from_millis(delay));
                }
                Ok(())
            },
            |parent| {
                if sync_failure {
                    return Err(std::io::Error::other("directory sync fault"));
                }
                sync_directory(parent)
            },
        );
        match result {
            Ok(()) => {
                worker_progress.store(100, Ordering::Relaxed);
                (SaveOutcome::Saved, skipped_xmp_mirror)
            }
            Err(DurableWriteError::AfterRename(_)) => (SaveOutcome::Uncertain, false),
            Err(DurableWriteError::BeforeRename(_)) => (SaveOutcome::Failed, false),
        }
    });
    rust.save_job = Some(SaveJob {
        handle,
        progress,
        path: job_path,
        start_history,
        start_revision,
        print_revision,
        start_metadata,
        generation,
        copy,
    });
    true
}

pub(in crate::cxxqt_object) fn prepare_native_save(doc: &mut pictura_core::Document, dirty: bool) {
    if !doc.composite.merged_output() {
        if !can_omit_merged(doc) {
            prepare_compatibility_save(doc, true, dirty);
        } else if !pictura_render::DocumentComposite::is_stored(doc) {
            let channels = doc.mode.color_channels();
            doc.composite
                .replace(pictura_core::PixelBuffer {
                    width: doc.width,
                    height: doc.height,
                    channels,
                    data: pictura_core::Plane::filled(
                        doc.width as usize,
                        doc.height as usize,
                        channels as usize,
                        0,
                    ),
                })
                .expect("valid omitted composite geometry");
        }
    } else if dirty {
        pictura_render::refresh_native_composite(doc);
    }
}

fn prepare_compatibility_save(doc: &mut pictura_core::Document, include: bool, dirty: bool) {
    use pictura_core::{PixelBuffer, Plane};

    if !include && can_omit_merged(doc) {
        // The codec requires a dimensioned buffer even when its pixels are not written.
        let channels = doc.mode.color_channels();
        doc.composite
            .replace(PixelBuffer {
                width: doc.width,
                height: doc.height,
                channels,
                data: Plane::filled(
                    doc.width as usize,
                    doc.height as usize,
                    channels as usize,
                    0,
                ),
            })
            .expect("valid omitted composite geometry");
        doc.composite.set_merged_output(false);
        return;
    }

    let restore = !doc.composite.merged_output() && !doc.layers.is_empty();
    doc.composite.set_merged_output(true);
    if restore {
        if !pictura_render::restore_native_composite(doc) {
            let mut buffer = pictura_render::composite_tiled(doc);
            if doc.mode == pictura_core::ColorMode::Grayscale {
                let mut gray = vec![0; doc.width as usize * doc.height as usize];
                buffer.data.read_into(0, &mut gray);
                buffer.channels = 1;
                buffer.data = gray.into();
            }
            doc.composite
                .replace(buffer)
                .expect("valid restored composite");
        }
    } else if dirty {
        pictura_render::refresh_native_composite(doc);
    }
}

pub(in crate::cxxqt_object) fn save_running_rust(rust: &PictureViewRust) -> bool {
    rust.save_job
        .as_ref()
        .is_some_and(|job| !job.handle.is_finished())
}

fn finish(rust: &mut PictureViewRust) -> SaveOutcome {
    let Some(job) = rust.save_job.take() else {
        return SaveOutcome::Failed;
    };
    let (outcome, skipped_xmp_mirror) = job.handle.join().unwrap_or((SaveOutcome::Failed, false));
    let session = &mut rust.session;
    if session.document_generation != job.generation {
        return if outcome == SaveOutcome::Saved {
            SaveOutcome::Replaced
        } else {
            SaveOutcome::ReplacedFailure
        };
    }
    session.skipped_xmp_mirror = skipped_xmp_mirror;
    if job.copy {
        return outcome;
    }
    if outcome == SaveOutcome::Uncertain {
        session.save_publication_uncertain = true;
        session.dirty = true;
        session.capture_recovery(true);
    }
    if outcome != SaveOutcome::Saved {
        return outcome;
    }

    rust.path = Some(job.path);
    session.saved_id = job.start_history;
    session.saved_metadata_revision = job.start_metadata;
    session.saved_print_revision = job.print_revision;
    let current = session.history.current_id() == job.start_history
        && session.content_revision == job.start_revision
        && session.persisted_metadata_revision == job.start_metadata
        && session.print_revision == job.print_revision;
    if current {
        session.save_publication_uncertain = false;
    }
    session.dirty = !current || session.save_publication_uncertain;
    outcome
}

pub(in crate::cxxqt_object) fn save_finish_rust(rust: &mut PictureViewRust) -> bool {
    finish(rust) == SaveOutcome::Saved
}

pub(super) fn save_progress(view: &PictureView) -> i32 {
    view.rust()
        .save_job
        .as_ref()
        .map_or(-1, |job| job.progress.load(Ordering::Relaxed))
}

pub(super) fn save_finish_outcome(mut view: Pin<&mut PictureView>) -> i32 {
    finish(&mut view.as_mut().rust_mut()) as i32
}

pub(super) fn save_set_sync_failure(mut view: Pin<&mut PictureView>, fail: bool) {
    view.as_mut().rust_mut().save_sync_failure_for_test = fail;
}

pub(super) fn save_start(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    view.as_mut().settle_pending_composite();
    save_start_rust(&mut view.as_mut().rust_mut(), &path.to_string(), false)
}

pub(super) fn save_copy_start(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    view.as_mut().settle_pending_composite();
    save_start_rust(&mut view.as_mut().rust_mut(), &path.to_string(), true)
}

pub(super) fn native_save_can_omit_merged(view: &PictureView) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(can_omit_merged)
}

pub(super) fn native_compatibility_save_start(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    copy: bool,
    include: bool,
) -> bool {
    view.as_mut().settle_pending_composite();
    save_start_with_compatibility(
        &mut view.as_mut().rust_mut(),
        &path.to_string(),
        copy,
        Some(include),
    )
}
pub(super) fn save_running(view: &PictureView) -> bool {
    save_running_rust(view.rust())
}

pub(super) fn save_finish(mut view: Pin<&mut PictureView>) -> bool {
    save_finish_rust(&mut view.as_mut().rust_mut())
}

pub(super) fn save_set_delay(mut view: Pin<&mut PictureView>, ms: u64) {
    view.as_mut().rust_mut().save_delay_for_test = ms;
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, ColorMode, Document, LayerStack};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn view() -> PictureViewRust {
        let rgba = vec![200u8; 8 * 8 * 4];
        let doc = Document::from_rgba("Layer 0", 8, 8, &rgba);
        let mut rust = PictureViewRust::default();
        rust.session.doc.set(Some(doc));
        rust.session.dirty = true;
        rust
    }

    fn path(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!(
            "pictura-save-job-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name).to_string_lossy().into_owned()
    }

    fn wait(rust: &PictureViewRust) {
        while save_running_rust(rust) {
            std::thread::yield_now();
        }
    }

    fn save_compatible(doc: Document, name: &str, include: bool, dirty: bool) -> Vec<u8> {
        let mut rust = PictureViewRust::default();
        rust.session.doc.set(Some(doc));
        rust.session.dirty = dirty;
        let target = path(name);
        assert!(save_start_with_compatibility(
            &mut rust,
            &target,
            false,
            Some(include)
        ));
        assert!(save_finish_rust(&mut rust));
        let bytes = std::fs::read(&target).unwrap();
        std::fs::remove_dir_all(std::path::Path::new(&target).parent().unwrap()).unwrap();
        bytes
    }

    fn colored_document() -> Document {
        let rgba: Vec<u8> = (0..4096)
            .flat_map(|i| [(i % 251) as u8, (i % 173) as u8, 84, 255])
            .collect();
        let mut doc = Document::from_rgba("Color", 64, 64, &rgba);
        let mut hidden = doc.layers[0].clone();
        hidden.name = "Hidden".into();
        hidden.visible = false;
        doc.layers = LayerStack::from_layers(vec![doc.layers[0].clone(), hidden]);
        doc
    }

    #[test]
    fn native_save_respects_merged_output_choice() {
        for name in ["color.psd", "color.psb"] {
            let mut source = colored_document();
            let expected = pictura_render::composite_rgba(&source);
            source.composite.detach_plane();
            let include = save_compatible(source.clone(), name, true, false);
            let omit = save_compatible(source, name, false, false);
            assert!(omit.len() < include.len());
            let include = pictura_codec::read_psd(&include).unwrap();
            let omit = pictura_codec::read_psd(&omit).unwrap();
            assert!(include.composite.merged_output());
            assert!(!omit.composite.merged_output());
            assert_eq!(omit.is_psb, name.ends_with(".psb"));
            assert_eq!(include.layers, omit.layers);
            assert_eq!(
                &include.composite.data[..64 * 64 * 3],
                &expected.data[..64 * 64 * 3]
            );
        }
    }

    #[test]
    fn native_save_restores_merged_image() {
        for name in ["restore.psd", "restore.psb"] {
            let omitted = save_compatible(colored_document(), name, false, false);
            let mut doc = pictura_codec::read_psd(&omitted).unwrap();
            doc.layers[0].channels_mut()[0].data.as_mut_slice().fill(88);
            let expected = pictura_render::composite_rgba(&doc);
            let restored = save_compatible(doc, name, true, true);
            let restored = pictura_codec::read_psd(&restored).unwrap();
            assert!(restored.composite.merged_output());
            assert!(restored.channels.is_empty());
            assert!(can_omit_merged(&restored));
            assert_eq!(
                &restored.composite.data[..64 * 64 * 3],
                &expected.data[..64 * 64 * 3]
            );
            assert_eq!(restored.layers.len(), 2);
        }
    }

    #[test]
    fn native_restore_keeps_merged_transparency_separate() {
        for name in ["transparent.psd", "transparent.psb"] {
            let rgba = [40, 90, 150, 80].repeat(64);
            let source = Document::from_rgba("Transparent", 8, 8, &rgba);
            let omitted = save_compatible(source, name, false, false);
            let doc = pictura_codec::read_psd(&omitted).unwrap();
            let expected = pictura_render::composite_rgba(&doc);
            let restored =
                pictura_codec::read_psd(&save_compatible(doc, name, true, true)).unwrap();
            assert!(restored.channels.is_empty());
            assert!(can_omit_merged(&restored));
            assert_eq!(restored.composite.channels, 4);
            assert_eq!(restored.composite.data.as_ref(), expected.data.as_ref());
        }
    }

    #[test]
    fn signed_alpha_survives_direct_and_noninteractive_resave() {
        for name in ["signed.psd", "signed.psb"] {
            let source = Document::from_rgba("Transparent", 8, 8, &[40, 90, 150, 80].repeat(64));
            let encoded = save_compatible(source, name, true, true);
            let mut signed = pictura_codec::read_psd(&encoded).unwrap();
            assert!(signed.composite.signed_merged_alpha());
            signed.channels.push(pictura_core::Channel {
                id: 0,
                data: vec![37; 64].into(),
            });
            let direct = if name.ends_with(".psb") {
                pictura_codec::write_psb(&signed).unwrap()
            } else {
                pictura_codec::write_psd(&signed).unwrap()
            };
            let lazy = pictura_codec::read_psd_with_options(
                &direct,
                &pictura_codec::OpenOptions {
                    skip_layered_merged: true,
                    ..Default::default()
                },
            )
            .unwrap();
            assert!(lazy.composite.signed_merged_alpha());
            assert_eq!(lazy.channels.len(), 1);
            assert_eq!(lazy.channels[0].data.as_ref(), &[37; 64]);
            let mut reopened = pictura_codec::read_psd(&direct).unwrap();
            assert!(reopened.composite.signed_merged_alpha());
            assert_eq!(reopened.composite.data[3 * 64], 80);
            assert_eq!(reopened.channels.len(), 1);
            assert_eq!(reopened.channels[0].data.as_ref(), &[37; 64]);
            reopened.composite.detach_plane();
            let mut rust = PictureViewRust::default();
            rust.session.doc.set(Some(reopened));
            let target = path(name);
            assert!(save_start_rust(&mut rust, &target, false));
            assert!(save_finish_rust(&mut rust));
            let reopened = pictura_codec::read_psd(&std::fs::read(&target).unwrap()).unwrap();
            assert!(reopened.composite.signed_merged_alpha());
            assert_eq!(reopened.composite.data[3 * 64], 80);
            assert_eq!(reopened.channels.len(), 1);
            assert_eq!(reopened.channels[0].data.as_ref(), &[37; 64]);
            std::fs::remove_dir_all(std::path::Path::new(&target).parent().unwrap()).unwrap();
        }
    }

    #[test]
    fn legacy_save_stages_detached_omitted_composite() {
        let encoded = save_compatible(colored_document(), "omitted.psd", false, false);
        let mut doc = pictura_codec::read_psd(&encoded).unwrap();
        doc.composite.detach_plane();
        let mut rust = PictureViewRust::default();
        rust.session.doc.set(Some(doc));
        let target = path("legacy.psd");
        assert!(save_start_rust(&mut rust, &target, false));
        assert!(save_finish_rust(&mut rust));
        let reopened = pictura_codec::read_psd(&std::fs::read(&target).unwrap()).unwrap();
        assert!(!reopened.composite.merged_output());
        assert_eq!(reopened.layers.len(), 2);
        std::fs::remove_dir_all(std::path::Path::new(&target).parent().unwrap()).unwrap();
    }

    #[test]
    fn legacy_save_preserves_channels_from_marked_placeholder() {
        use pictura_codec::{encode_image_resources, frame_image_resource};
        let mut source = colored_document();
        source.channels.push(pictura_core::Channel {
            id: 0,
            data: vec![37; 64 * 64].into(),
        });
        let version_info = [0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        let marker = frame_image_resource(1057, "", &version_info);
        source.image_resources = encode_image_resources(std::slice::from_ref(&marker)).into();
        let mut encoded = pictura_codec::write_psd(&pictura_render::save_view(&source)).unwrap();
        let offset = encoded
            .windows(marker.raw.len())
            .position(|bytes| bytes == marker.raw)
            .unwrap();
        let flag = marker.raw.len() - marker.data.len() - marker.data.len() % 2 + 4;
        encoded[offset + flag] = 0;
        let mut doc = pictura_codec::read_psd(&encoded).unwrap();
        assert!(!doc.composite.merged_output());
        assert_eq!(doc.channels[0].data.as_ref(), &[37; 64 * 64]);
        doc.composite.detach_plane();
        let mut rust = PictureViewRust::default();
        rust.session.doc.set(Some(doc));
        let target = path("with-channel.psd");
        assert!(save_start_rust(&mut rust, &target, false));
        assert!(save_finish_rust(&mut rust));
        let reopened = pictura_codec::read_psd(&std::fs::read(&target).unwrap()).unwrap();
        assert!(reopened.composite.merged_output());
        assert_eq!(reopened.channels[0].data.as_ref(), &[37; 64 * 64]);
        std::fs::remove_dir_all(std::path::Path::new(&target).parent().unwrap()).unwrap();
    }

    #[test]
    fn direct_writer_synchronizes_existing_version_info() {
        use pictura_codec::{decode_image_resources, encode_image_resources, frame_image_resource};
        let mut doc = colored_document();
        let marker = frame_image_resource(1057, "", &[0, 0, 0, 1, 0]);
        let other = frame_image_resource(1005, "", &[1, 2, 3, 4]);
        doc.image_resources = encode_image_resources(&[marker, other.clone()]).into();
        for bytes in [
            pictura_codec::write_psd(&doc).unwrap(),
            pictura_codec::write_psb(&doc).unwrap(),
        ] {
            let back = pictura_codec::read_psd(&bytes).unwrap();
            let resources = decode_image_resources(&back);
            assert_eq!(resources.iter().find(|r| r.id == 1057).unwrap().data[4], 1);
            assert_eq!(
                resources.iter().find(|r| r.id == 1005).unwrap().raw,
                other.raw
            );
            assert!(back.composite.merged_output());
        }
    }

    #[test]
    fn native_save_preserves_required_image_data() {
        for name in ["protected.psd", "protected.psb"] {
            let flat =
                Document::with_uniform_composite(8, 8, ColorMode::Grayscale, BitDepth::Eight, 44);
            let flat = pictura_codec::read_psd(&save_compatible(flat, name, false, false)).unwrap();
            assert!(flat.composite.merged_output());
            assert_eq!(flat.composite.data.as_ref(), &[44; 64]);

            let mut layered = colored_document();
            layered.channels.push(pictura_core::Channel {
                id: 0,
                data: vec![37; 64 * 64].into(),
            });
            let layered =
                pictura_codec::read_psd(&save_compatible(layered, name, false, false)).unwrap();
            assert!(layered.composite.merged_output());
            assert_eq!(layered.layers.len(), 2);
            assert_eq!(layered.channels[0].data.as_ref(), &[37; 64 * 64]);
        }
    }

    #[test]
    fn native_save_restores_adobe_omitted_fixture() {
        use pictura_codec::{decode_image_resources, encode_image_resources, frame_image_resource};

        // Generated from the observed structure of the local Photoshop 2021
        // reference: resource 1057 with false, plus a white 3-channel RLE image.
        let mut doc = colored_document();
        doc.composite
            .replace(pictura_core::PixelBuffer {
                width: 64,
                height: 64,
                channels: 3,
                data: vec![255; 64 * 64 * 3].into(),
            })
            .unwrap();
        let version_info = [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        let marker = frame_image_resource(1057, "", &version_info);
        doc.image_resources = encode_image_resources(std::slice::from_ref(&marker)).into();
        let mut encoded = pictura_codec::write_psd(&doc).unwrap();
        let mut real_marker = marker.raw.clone();
        let flag = real_marker.len() - marker.data.len() - marker.data.len() % 2 + 4;
        real_marker[flag] = 1;
        let offset = encoded
            .windows(real_marker.len())
            .position(|bytes| bytes == real_marker)
            .unwrap();
        encoded[offset + flag] = 0;
        let mut doc = pictura_codec::read_psd(&encoded).unwrap();
        assert!(!doc.composite.merged_output());
        doc.layers[0].channels_mut()[0].data.as_mut_slice().fill(80);
        let output = save_compatible(doc, "adobe.psd", true, true);
        let back = pictura_codec::read_psd(&output).unwrap();
        assert!(back.composite.merged_output());
        assert_eq!(back.composite.data[0], 80);
        let resource = decode_image_resources(&back)
            .into_iter()
            .find(|resource| resource.id == 1057)
            .unwrap();
        assert_eq!(resource.data[4], 1);
    }

    fn native_layered(mode: ColorMode, depth: BitDepth) -> Document {
        use pictura_core::{PixelBuffer, Samples, SourceChannels};

        let color_count = mode.color_channels();
        let mut doc = Document::from_rgba("Native", 8, 8, &[100, 100, 100, 255].repeat(64));
        doc.mode = mode;
        let layer = &mut doc.layers[0];
        layer
            .channels_mut()
            .retain(|channel| channel.id < i16::from(color_count));
        let planes = layer
            .channels()
            .iter()
            .map(|channel| {
                let samples = match (depth, channel.id) {
                    (BitDepth::Sixteen, -1) => Samples::U16(vec![u16::MAX; 64].into()),
                    (BitDepth::Sixteen, _) => Samples::U16(vec![25_701; 64].into()),
                    (BitDepth::ThirtyTwo, -1) => Samples::F32(vec![1.0; 64].into()),
                    _ => Samples::F32(vec![0.393; 64].into()),
                };
                (channel.id, samples)
            })
            .collect();
        layer.source_channels = Some(SourceChannels::new(depth, layer.rect, planes));
        doc.source.set_native(Some(depth), None).unwrap();
        doc.composite
            .replace(PixelBuffer {
                width: 8,
                height: 8,
                channels: color_count,
                data: vec![100; 64 * usize::from(color_count)].into(),
            })
            .unwrap();
        doc
    }

    #[test]
    fn native_save_restores_missing_native_composite() {
        for mode in [ColorMode::Rgb, ColorMode::Grayscale] {
            for depth in [BitDepth::Sixteen, BitDepth::ThirtyTwo] {
                for name in ["native.psd", "native.psb"] {
                    let source = native_layered(mode, depth);
                    let encoded = save_compatible(source, name, false, false);
                    let omitted = pictura_codec::read_psd(&encoded).unwrap();
                    assert!(!omitted.composite.merged_output());
                    assert!(omitted.source.planes().is_none());
                    for dirty in [false, true] {
                        let mut doc = omitted.clone();
                        if dirty {
                            doc.layers[0].name = "Edited".into();
                        }
                        let expected = doc.authoritative_plane(&doc.layers[0], 0).unwrap().clone();
                        let encoded = save_compatible(doc, name, true, dirty);
                        let restored = pictura_codec::read_psd(&encoded).unwrap();
                        assert!(restored.composite.merged_output());
                        assert_eq!(restored.source.depth(), Some(depth));
                        let native = restored.source.planes().as_ref().unwrap();
                        assert_eq!(native.samples.slice(0..64).unwrap(), expected);
                    }
                }
            }
        }
    }

    #[test]
    fn native_depth_save_regressions() {
        use pictura_core::{Channel, Samples, SourcePlanes};

        fn psd_layer_section_and_image(bytes: &[u8]) -> (&[u8], &[u8]) {
            let end = |start: usize| {
                start + 4 + u32::from_be_bytes(bytes[start..start + 4].try_into().unwrap()) as usize
            };
            let layers = end(end(26));
            let image = end(layers);
            (&bytes[layers..image], &bytes[image..])
        }

        let mut doc = native_layered(ColorMode::Rgb, BitDepth::Sixteen);
        assert!(pictura_render::restore_native_composite(&mut doc));
        let unchanged =
            pictura_codec::read_psd(&save_compatible(doc.clone(), "native.psd", true, false))
                .unwrap();
        assert!(unchanged.channels.is_empty());
        assert_eq!(unchanged.source.depth(), Some(BitDepth::Sixteen));
        let edited = save_compatible(doc.clone(), "native.psd", true, true);
        let native = pictura_codec::read_psd(&edited).unwrap();
        assert_eq!(
            native
                .source
                .planes()
                .as_ref()
                .unwrap()
                .samples
                .slice(0..64),
            doc.source.planes().as_ref().unwrap().samples.slice(0..64)
        );

        let mut with_channel = doc.clone();
        with_channel.channels.push(Channel {
            id: 0,
            data: vec![98; 64].into(),
        });
        let native =
            pictura_codec::read_psd(&save_compatible(with_channel, "channel.psd", true, true))
                .unwrap();
        assert_eq!(native.channels.len(), 1);
        assert_eq!(native.channels[0].data.as_ref(), &[98; 64]);

        let mut converted = native_layered(ColorMode::Rgb, BitDepth::Sixteen);
        converted
            .source
            .set_encoding(Some(ColorMode::Cmyk), None)
            .unwrap();
        converted
            .source
            .replace_planes(Some(SourcePlanes {
                depth: BitDepth::Sixteen,
                width: 8,
                height: 8,
                samples: Samples::U16(vec![25_701; 4 * 64].into()),
            }))
            .unwrap();
        let expected = pictura_codec::write_psd(&converted).unwrap();
        let saved = save_compatible(converted, "converted.psd", true, true);
        assert_eq!(&saved[..26], &expected[..26]);
        assert_eq!(
            psd_layer_section_and_image(&saved),
            psd_layer_section_and_image(&expected)
        );

        doc.layers = LayerStack::default();
        let expected = pictura_codec::write_psd(&doc).unwrap();
        let saved = save_compatible(doc, "native.psd", true, true);
        assert_eq!(&saved[..26], &expected[..26]);
        assert_eq!(
            psd_layer_section_and_image(&saved).1,
            psd_layer_section_and_image(&expected).1
        );
        let reopened = pictura_codec::read_psd(&saved).unwrap();
        assert!(reopened.composite.signed_merged_alpha());
        assert!(reopened.channels.is_empty());
    }

    #[test]
    fn native_save_codec_preconditions() {
        let mut doc = colored_document();
        doc.composite.set_merged_output(false);
        let bytes = pictura_codec::write_psd(&doc).unwrap();
        let omitted = pictura_codec::read_psd(&bytes).unwrap();
        assert!(!omitted.composite.merged_output());
        assert!(pictura_codec::decode_image_resources(&omitted)
            .iter()
            .all(|resource| resource.id != 1057));
        doc.composite.detach_plane();
        assert!(pictura_codec::write_psd(&doc).is_err());
    }

    #[test]
    fn native_save_stamps_document_versions() {
        for (name, native_version) in [("version.psd", 1), ("version.psb", 2)] {
            let mut rust = view();
            let target = path(name);
            assert!(save_start_rust(&mut rust, &target, false));
            wait(&rust);
            assert!(save_finish_rust(&mut rust));
            let bytes = std::fs::read(&target).unwrap();
            assert_eq!(bytes[5], native_version);
            let doc = pictura_codec::read_psd(&bytes).unwrap();
            let read = pictura_codec::document_version(&doc);
            let stamp = read.stamp.unwrap();
            assert_eq!(
                stamp.op_document_version,
                pictura_codec::CURRENT_DOCUMENT_VERSION
            );
            assert_eq!(stamp.min_reader_version, stamp.op_document_version);
            assert_eq!(stamp.writer_version, env!("CARGO_PKG_VERSION"));
            assert_eq!(read.access, pictura_codec::VersionAccess::Editable);
            assert!(read.version_info.unwrap().has_real_merged_data);
            let packet = pictura_codec::read_metadata(&doc).xmp;
            assert!(packet.contains("opDocumentVersion"));
            assert!(packet.contains("minReaderVersion"));
            assert!(packet.contains("writerVersion"));
            let _ = std::fs::remove_file(target);
        }
    }

    #[test]
    fn save_absent_composite_preserves_merged_output_policy() {
        let mut rust = view();
        let expected = pictura_render::composite_rgba(rust.session.doc.as_ref().unwrap());
        rust.session.doc.as_mut().unwrap().composite.detach_plane();
        assert!(pictura_codec::write_psd(rust.session.doc.as_ref().unwrap()).is_err());
        let target = path("absent.psd");
        assert!(save_start_rust(&mut rust, &target, false));
        wait(&rust);
        assert!(save_finish_rust(&mut rust));
        let saved = pictura_codec::read_psd(&std::fs::read(&target).unwrap()).unwrap();
        assert!(saved.composite.merged_output());
        assert!(saved.composite.signed_merged_alpha());
        assert_eq!(saved.composite.buffer(), &expected);
        assert!(saved.channels.is_empty());
        let doc = rust.session.doc.as_mut().unwrap();
        doc.composite.replace(expected).unwrap();
        doc.composite.set_merged_output(false);
        assert!(save_start_rust(&mut rust, &target, false));
        wait(&rust);
        assert!(save_finish_rust(&mut rust));
        let saved = pictura_codec::read_psd(&std::fs::read(&target).unwrap()).unwrap();
        assert!(!saved.composite.merged_output());
        let _ = std::fs::remove_file(target);
    }

    #[test]
    fn a_started_save_finishes_writes_and_clears_modified() {
        let mut rust = view();
        let path = path("save.psd");
        assert!(save_start_rust(&mut rust, &path, false));
        wait(&rust);
        assert!(save_finish_rust(&mut rust));
        assert_eq!(rust.path.as_deref(), Some(path.as_str()));
        assert!(!rust.session.dirty);
        assert!(std::path::Path::new(&path).is_file());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn print_metadata_survives_undo_and_concurrent_save() {
        use pictura_codec::print_resources::{
            read_print_settings, update_print_settings, PrintResourceEdits,
        };

        let mut rust = view();
        let target = crate::layer_target::LayerTarget::default();
        rust.session.record(&target, "Open");
        rust.session.mark_saved();
        rust.session.record(&target, "Image change");
        let stage = |rust: &mut PictureViewRust, border| {
            let doc = rust.session.doc.as_ref().unwrap();
            let mut edited = doc.clone();
            rust.session.print_resources.apply(&mut edited);
            let old = read_print_settings(&edited);
            let mut new = old.clone();
            new.border = Some(border);
            update_print_settings(&mut edited, &old, &new).unwrap();
            let changes = PrintResourceEdits::between(doc, &edited);
            rust.session.stage_print_resources(changes);
        };
        let image_history = rust.session.history.current_id();
        stage(&mut rust, 4.0);
        assert_eq!(rust.session.history.current_id(), image_history);
        let restored = rust.session.history.undo().unwrap();
        rust.session.doc.set(Some(restored.doc));
        assert!(rust.session.derived_dirty());
        let path = path("print.psb");
        assert!(save_start_rust(&mut rust, &path, false));
        stage(&mut rust, 6.0);
        wait(&rust);
        assert!(save_finish_rust(&mut rust));
        assert!(rust.session.dirty);
        let first = pictura_codec::read_psd(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(read_print_settings(&first).border, Some(4.0));
        assert!(save_start_rust(&mut rust, &path, false));
        wait(&rust);
        assert!(save_finish_rust(&mut rust));
        assert!(!rust.session.dirty);
        let second = pictura_codec::read_psd(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(read_print_settings(&second).border, Some(6.0));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_refuses_unreadable_tiles() {
        use crate::cxxqt_object::scratch_rollback::tests::{dense_doc, lose_tiles};
        let mut rust = PictureViewRust::default();
        rust.session.doc.set(Some(dense_doc()));
        rust.session.dirty = true;
        lose_tiles(rust.session.doc.as_ref().unwrap());
        let path = path("unreadable.psd");
        assert!(!save_start_rust(&mut rust, &path, false));
        assert!(crate::cxxqt_object::helpers::memory_gate::edit_refused_for_scratch());
        assert!(!std::path::Path::new(&path).exists(), "nothing was written");
    }

    #[test]
    fn a_second_start_while_running_is_refused() {
        let mut rust = view();
        // Keep the worker provably alive after its encode; an 8×8 document can
        // otherwise finish inside `thread::spawn`'s window and the second start
        // legitimately succeeds.
        rust.save_delay_for_test = 200;
        let path = path("twice.psd");
        assert!(save_start_rust(&mut rust, &path, false));
        assert!(!save_start_rust(&mut rust, &path, false));
        wait(&rust);
        assert!(save_finish_rust(&mut rust));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_edit_during_the_save_keeps_the_document_modified() {
        let mut rust = view();
        let path = path("stale.psd");
        let snapshot = rust.session.doc.snapshot().expect("document");
        assert!(save_start_rust(&mut rust, &path, false));
        rust.session.content_revision = rust.session.content_revision.wrapping_add(1);
        wait(&rust);
        assert!(save_finish_rust(&mut rust));
        assert!(rust.session.dirty);
        assert_eq!(rust.path.as_deref(), Some(path.as_str()));
        let written = std::fs::read(&path).expect("saved file");
        let expected = pictura_codec::write_versioned(
            &snapshot,
            false,
            None,
            env!("CARGO_PKG_VERSION"),
            false,
        )
        .expect("snapshot bytes")
        .bytes;
        assert_eq!(written, expected, "the file holds the pre-edit snapshot");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_replaced_document_keeps_its_path_and_dirty_state() {
        let mut rust = view();
        rust.path = Some("original.psd".to_string());
        let path = path("replaced.psd");
        assert!(save_start_rust(&mut rust, &path, false));
        rust.session
            .doc
            .set(Some(Document::from_rgba("Other", 4, 4, &[1u8; 4 * 4 * 4])));
        rust.session.document_generation = rust.session.document_generation.wrapping_add(1);
        wait(&rust);
        assert!(!save_finish_rust(&mut rust));
        assert_eq!(rust.path.as_deref(), Some("original.psd"));
        assert!(rust.session.dirty);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_failed_write_leaves_the_document_untouched() {
        let mut rust = view();
        let path = path("save.psd");
        std::fs::create_dir_all(&path).unwrap();
        assert!(save_start_rust(&mut rust, &path, false));
        wait(&rust);
        assert!(!save_finish_rust(&mut rust));
        assert!(rust.path.is_none());
        assert!(rust.session.dirty);
        let dir = std::path::Path::new(&path).parent().expect("parent");
        let leftovers = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0, "a failed save leaves no temp file");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn native_save_distinguishes_publication_failures() {
        let mut rust = view();
        let target = path("publication.psd");
        rust.path = Some("prior.psd".into());
        std::fs::write(&target, b"prior snapshot").unwrap();
        let bad_target = format!("{target}/missing.psd");
        assert!(save_start_rust(&mut rust, &bad_target, false));
        wait(&rust);
        assert!(!save_finish_rust(&mut rust));
        assert!(!rust.session.save_publication_uncertain);
        assert_eq!(std::fs::read(&target).unwrap(), b"prior snapshot");
        rust.save_sync_failure_for_test = true;
        assert!(save_start_rust(&mut rust, &target, false));
        wait(&rust);
        assert!(!save_finish_rust(&mut rust));
        assert!(rust.session.save_publication_uncertain);
        assert_eq!(rust.path.as_deref(), Some("prior.psd"));
        assert!(rust.session.dirty);
        assert!(pictura_codec::read_psd(&std::fs::read(&target).unwrap()).is_ok());
        let _ = std::fs::remove_file(target);
    }

    #[test]
    fn a_synchronous_save_writes_the_worker_bytes() {
        let mut rust = view();
        let path = path("sync.psd");
        assert!(save_start_rust(&mut rust, &path, false));
        wait(&rust);
        assert!(save_finish_rust(&mut rust));
        let written = std::fs::read(&path).expect("saved file");
        let doc = rust.session.doc.as_ref().expect("document");
        let expected =
            pictura_codec::write_versioned(doc, false, None, env!("CARGO_PKG_VERSION"), false)
                .expect("fresh encode")
                .bytes;
        assert_eq!(written, expected);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_layered_document_round_trips_through_the_job() {
        let rgba = vec![10u8; 4 * 4 * 4];
        let mut doc = Document::from_rgba("Layer 0", 4, 4, &rgba);
        let mut layer = doc.layers.get(0).expect("layer").clone();
        layer.name = "Layer 1".to_string();
        layer.channels_mut()[1].data.as_mut_slice().fill(9);
        let original = doc.layers.get(0).expect("layer").clone();
        doc.layers = LayerStack::from_layers(vec![layer, original]);
        doc.composite
            .replace(pictura_core::PixelBuffer {
                width: 4,
                height: 4,
                channels: 4,
                data: vec![10u8; 4 * 4 * 4].into(),
            })
            .expect("valid composite");
        doc.mode = ColorMode::Rgb;
        doc.depth = BitDepth::Eight;
        let mut rust = PictureViewRust::default();
        rust.session.doc.set(Some(doc));
        rust.session.dirty = true;
        let path = path("layers.psd");
        assert!(save_start_rust(&mut rust, &path, false));
        wait(&rust);
        assert!(save_finish_rust(&mut rust));
        let back = pictura_codec::read_psd(&std::fs::read(&path).unwrap()).expect("reads");
        assert_eq!(back.layers.len(), 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn copy_intent_skips_path_and_dirty() {
        let mut rust = view();
        rust.path = Some("original.psd".to_string());
        let path = path("copy.psd");
        assert!(save_start_rust(&mut rust, &path, true));
        wait(&rust);
        assert!(save_finish_rust(&mut rust));
        assert_eq!(rust.path.as_deref(), Some("original.psd"));
        assert!(rust.session.dirty);
        assert!(std::path::Path::new(&path).is_file());
        let _ = std::fs::remove_file(&path);
    }
}
