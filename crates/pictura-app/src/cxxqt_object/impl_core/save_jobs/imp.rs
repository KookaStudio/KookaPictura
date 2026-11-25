use super::*;

use super::super::super::color_management::save_embed_default;
use super::super::super::filter_tools::store_planes;
use super::super::super::state::PictureViewRust;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_error::{guard, Error, ErrorCode};
use std::io::Write;
use std::panic::AssertUnwindSafe;
use std::path::Path;

use super::ffi::NativeSaveResult;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn can_omit_merged(doc: &pictura_core::Document) -> bool {
    !doc.layers.is_empty() && doc.channels.is_empty()
}

/// The live native-save worker and the document identity it snapshotted.
pub(in crate::cxxqt_object) struct SaveJob {
    handle: std::thread::JoinHandle<(SaveOutcome, Result<bool, Error>)>,
    progress: Arc<AtomicI32>,
    path: String,
    start_history: Option<u64>,
    start_revision: u64,
    print_revision: u64,
    start_metadata: u64,
    generation: u64,
    copy: bool,
    history_log: Option<crate::history_log::StagedHistory>,
    history_log_omitted: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
enum SaveOutcome {
    Failed = 0,
    Saved = 1,
    Uncertain = 2,
}

pub(in crate::cxxqt_object) fn save_start_rust(
    rust: &mut PictureViewRust,
    path: &str,
    format_id: i32,
    copy: bool,
) -> bool {
    save_start_with_compatibility(rust, path, format_id, copy, None)
}

fn save_start_with_compatibility(
    rust: &mut PictureViewRust,
    path: &str,
    format_id: i32,
    copy: bool,
    compatibility: Option<bool>,
) -> bool {
    super::super::super::helpers::memory_gate::clear();
    rust.last_native_save = NativeSaveResult {
        code: ErrorCode::InvalidArg.status().raw(),
        ..Default::default()
    };
    rust.session.skipped_xmp_mirror = false;
    rust.session.history_log_save_warning = None;
    if rust.session.read_only()
        || crate::history_log::destination_conflicts(path)
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
    let Some(format) = super::super::save::save_format_for_save(format_id, &suffix) else {
        return false;
    };
    if format.route() != pictura_codec::SaveRoute::Native {
        return false;
    }
    let session = &rust.session;
    let Some(doc_ref) = session.doc.as_ref() else {
        return false;
    };
    // A document missing tiles lost from scratch is never written (#1217).
    if crate::history::unreadable_tiles(doc_ref) > 0 {
        super::super::super::helpers::memory_gate::refuse_for_scratch();
        rust.last_native_save.code = ErrorCode::Scratch.status().raw();
        rust.last_native_save.admission = 2;
        return false;
    }
    // Held by the worker until its write lands, so other edits see the room
    // the save is using.
    use super::super::super::helpers::save_gate::{reserve_save_checked, save_cost, SaveAdmission};
    let reservation = match reserve_save_checked(&save_cost(doc_ref)) {
        Ok(reservation) => reservation,
        Err(reason) => {
            let (code, admission) = match reason {
                SaveAdmission::Memory => (ErrorCode::Host, 1),
                SaveAdmission::ScratchFull => (ErrorCode::Scratch, 3),
            };
            rust.last_native_save.code = code.status().raw();
            rust.last_native_save.admission = admission;
            return false;
        }
    };
    let fault = std::mem::take(&mut rust.save_fault_for_test);
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
    let mut doc = doc_ref.clone();
    let (history_log, history_log_omitted) = match session.history_log.stage_document(&mut doc) {
        Ok(history) => (history, false),
        Err(()) => (None, true),
    };
    let print_resources = session.print_resources.clone();
    let delay = rust.save_delay_for_test;
    let sync_failure = rust.save_sync_failure_for_test;
    let psb = format == pictura_codec::SaveFormat::Psb;
    let worker_path = path.to_string();
    let job_path = path.to_string();
    let progress = Arc::new(AtomicI32::new(0));
    let worker_progress = progress.clone();
    let handle = std::thread::Builder::new()
        .name(crate::worker_threads::SAVE.into())
        .spawn(move || {
            let _reservation = reservation;
            let mut renamed = false;
            // Only this worker owns the snapshot and temp writer. The rename marker survives unwind.
            let result = guard(AssertUnwindSafe(|| {
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
                // The resolved catalog id chooses the PSD or PSB container.
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
                let mut scratch_failure = false;
                let result = write_durable_with(
                    Path::new(&worker_path),
                    |out| {
                        if fault == 3 {
                            out.write_all(b"8BPS")?;
                            panic!("injected panic while writing a native save");
                        }
                        if fault == 1 {
                            doc.to_mut().depth = pictura_core::BitDepth::Sixteen;
                        }
                        let encoded = if fault == 2 {
                            let mut empty = [];
                            let mut full_stream = std::io::Cursor::new(&mut empty[..]);
                            pictura_codec::write_versioned_to_checked(
                                &mut full_stream,
                                &doc,
                                psb,
                                Some(&cache),
                                env!("CARGO_PKG_VERSION"),
                                compatibility.is_some(),
                            )
                        } else {
                            pictura_codec::write_versioned_to_checked(
                                out,
                                &doc,
                                psb,
                                Some(&cache),
                                env!("CARGO_PKG_VERSION"),
                                compatibility.is_some(),
                            )
                        };
                        skipped_xmp_mirror = encoded.map_err(|error| match error {
                            pictura_codec::NativeWriteError::Codec(error) => {
                                std::io::Error::other(error)
                            }
                            pictura_codec::NativeWriteError::Io(error) => error,
                        })?;
                        // A tile lost while encoding wrote defaults in its place.
                        if pictura_core::memory::eviction::unreadable_reads() != unreadable_before
                            && crate::history::unreadable_tiles(&doc) > 0
                        {
                            scratch_failure = true;
                            return Err(std::io::Error::other("tiles lost while encoding"));
                        }
                        worker_progress.store(85, Ordering::Relaxed);
                        if delay > 0 {
                            std::thread::sleep(Duration::from_millis(delay));
                        }
                        Ok(())
                    },
                    |parent| {
                        renamed = true;
                        if fault == 4 {
                            panic!("injected panic after native save rename");
                        }
                        if sync_failure {
                            return Err(std::io::Error::other("directory sync fault"));
                        }
                        sync_directory(parent)
                    },
                );
                result.map_err(|error| {
                    let cause = match error {
                        DurableWriteError::BeforeRename(cause)
                        | DurableWriteError::AfterRename(cause) => cause,
                    };
                    let code = if scratch_failure {
                        ErrorCode::Scratch
                    } else if cause
                        .get_ref()
                        .is_some_and(|source| source.is::<pictura_codec::PsdError>())
                    {
                        ErrorCode::Codec
                    } else {
                        ErrorCode::Io
                    };
                    Error::with_source(code, cause.to_string(), cause)
                })?;
                Ok(skipped_xmp_mirror)
            }));
            let publication = if result.is_ok() {
                worker_progress.store(100, Ordering::Relaxed);
                SaveOutcome::Saved
            } else if renamed {
                SaveOutcome::Uncertain
            } else {
                SaveOutcome::Failed
            };
            (publication, result)
        });
    let Ok(handle) = handle else {
        rust.last_native_save.code = ErrorCode::Host.status().raw();
        return false;
    };
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
        history_log,
        history_log_omitted,
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

fn finish(rust: &mut PictureViewRust) -> NativeSaveResult {
    let Some(job) = rust.save_job.take() else {
        return NativeSaveResult {
            code: ErrorCode::InvalidArg.status().raw(),
            ..Default::default()
        };
    };
    let (outcome, worker_result) = job.handle.join().unwrap_or_else(|payload| {
        std::mem::forget(payload);
        (
            SaveOutcome::Failed,
            Err(Error::new(
                ErrorCode::Panic,
                "the save stopped unexpectedly",
            )),
        )
    });
    crate::diagnostic_logging::outcome("document.save", outcome == SaveOutcome::Saved);
    let replaced = rust.session.document_generation != job.generation;
    let result = NativeSaveResult {
        code: worker_result
            .as_ref()
            .map_or_else(|error| error.code(), |_| ErrorCode::Ok)
            .status()
            .raw(),
        publication: outcome as i32,
        admission: 0,
        replaced,
    };
    rust.last_native_save = result;
    if replaced {
        return result;
    }
    let session = &mut rust.session;
    session.skipped_xmp_mirror = worker_result.unwrap_or(false);
    session.history_log_save_warning = (outcome == SaveOutcome::Saved && job.history_log_omitted)
        .then_some(crate::history_log::APPEND_OMITTED);
    if job.copy {
        return result;
    }
    if outcome == SaveOutcome::Uncertain {
        session.save_publication_uncertain = true;
        session.dirty = true;
        session.capture_recovery(true);
    }
    if outcome != SaveOutcome::Saved {
        return result;
    }

    if let (Some(history), Some(doc)) = (job.history_log, session.doc.as_mut()) {
        if !session.history_log.acknowledge(doc, history) {
            session.history_log_save_warning = Some(crate::history_log::REBASE_FAILED);
        }
    }
    session.history_log.set_filename(&job.path);
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
    result
}

pub(in crate::cxxqt_object) fn save_finish_rust(rust: &mut PictureViewRust) -> bool {
    let result = finish(rust);
    result.publication == SaveOutcome::Saved as i32 && !result.replaced
}

pub(super) fn save_progress(view: &PictureView) -> i32 {
    view.rust()
        .save_job
        .as_ref()
        .map_or(-1, |job| job.progress.load(Ordering::Relaxed))
}

pub(super) fn save_finish_outcome(mut view: Pin<&mut PictureView>) -> i32 {
    let result = finish(&mut view.as_mut().rust_mut());
    match (result.replaced, result.publication) {
        (false, publication) => publication,
        (true, 1) => 3,
        (true, _) => 4,
    }
}

pub(super) fn save_finish_result(mut view: Pin<&mut PictureView>) -> NativeSaveResult {
    finish(&mut view.as_mut().rust_mut())
}

pub(super) fn save_last_result(view: &PictureView) -> NativeSaveResult {
    view.rust().last_native_save
}

pub(super) fn save_set_fault_for_test(mut view: Pin<&mut PictureView>, fault: i32) {
    view.as_mut().rust_mut().save_fault_for_test = fault;
}

pub(super) fn save_set_sync_failure(mut view: Pin<&mut PictureView>, fail: bool) {
    view.as_mut().rust_mut().save_sync_failure_for_test = fail;
}

pub(super) fn save_start(mut view: Pin<&mut PictureView>, path: &QString, format_id: i32) -> bool {
    view.as_mut().settle_pending_composite();
    save_start_rust(
        &mut view.as_mut().rust_mut(),
        &path.to_string(),
        format_id,
        false,
    )
}

pub(super) fn save_copy_start(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    format_id: i32,
) -> bool {
    view.as_mut().settle_pending_composite();
    save_start_rust(
        &mut view.as_mut().rust_mut(),
        &path.to_string(),
        format_id,
        true,
    )
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
    format_id: i32,
    copy: bool,
    include: bool,
) -> bool {
    view.as_mut().settle_pending_composite();
    save_start_with_compatibility(
        &mut view.as_mut().rust_mut(),
        &path.to_string(),
        format_id,
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
mod tests;
