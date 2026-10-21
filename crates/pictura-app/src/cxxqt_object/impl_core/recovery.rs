//! Crash-recovery snapshots (`docs/11-cross-cutting/crash-recovery-and-autosave.md`):
//! write the document to a recovery file off the UI thread without touching its
//! path or dirty flag, and open such a file back as an unsaved, untitled
//! document. Free functions over a [`PictureView`] (their own bridge); the frame
//! owns where the files live.
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::Document;
use std::io::Write;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Start writing the document as a PSD (PSB past the PSD size limit) to `path` on a worker thread, after waiting for any earlier write. The path and dirty flag are untouched. False without a document.
        fn recovery_write(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Wait for the write `recovery_write` started; whether it landed (true when none was pending).
        fn recovery_wait(view: Pin<&mut PictureView>) -> bool;

        /// Open the snapshot at `path` as an untitled document with unsaved changes, so Save asks for a destination. False when it cannot be read.
        fn recovery_open(view: Pin<&mut PictureView>, path: &QString) -> bool;
    }
}

fn recovery_write(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    recovery_wait(view.as_mut());
    let Some(doc) = view.rust().session.doc.clone() else {
        return false;
    };
    let path = path.to_string();
    let handle = std::thread::spawn(move || write_snapshot(doc, &path).is_ok());
    view.rust_mut().recovery_write = Some(handle);
    true
}

fn recovery_wait(view: Pin<&mut PictureView>) -> bool {
    match view.rust_mut().recovery_write.take() {
        Some(handle) => handle.join().unwrap_or(false),
        None => true,
    }
}

fn recovery_open(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    if !view.as_mut().open(path) {
        return false;
    }
    let mut rust = view.rust_mut();
    rust.path = None;
    rust.session.dirty = true;
    true
}

/// Encode `doc` as the native save does and land it at `path` atomically: a
/// synced sibling temp file renamed into place, so a crash mid-write keeps the
/// previous snapshot.
fn write_snapshot(mut doc: Document, path: &str) -> std::io::Result<()> {
    pictura_render::refresh_native_composite(&mut doc);
    let bytes = pictura_codec::write_psd(&pictura_render::save_view(&doc))
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    let tmp = format!("{path}.tmp");
    let written = std::fs::File::create(&tmp).and_then(|mut file| {
        file.write_all(&bytes)?;
        file.sync_all()
    });
    if let Err(e) = written.and_then(|()| std::fs::rename(&tmp, path)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::path::{PathPoint, Subpath};
    use pictura_core::{BitDepth, ColorMode};

    #[test]
    fn a_snapshot_round_trips_layers_and_the_work_path() {
        let mut doc = Document::from_rgba("Layer 0", 4, 3, &[200u8; 4 * 3 * 4]);
        doc.work_path.add_subpath(Subpath {
            points: vec![
                PathPoint {
                    anchor: (1.0, 1.0),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
                PathPoint {
                    anchor: (3.0, 2.0),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
            ],
            closed: false,
        });
        let dir = std::env::temp_dir().join(format!("pictura-recovery-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("1.psd");
        let path = path.to_str().unwrap();
        write_snapshot(doc.clone(), path).expect("write");
        let back = pictura_codec::read_psd(&std::fs::read(path).unwrap()).expect("read");
        let tmp_left = std::path::Path::new(&format!("{path}.tmp")).exists();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!tmp_left);
        assert_eq!((back.width, back.height), (4, 3));
        assert_eq!(back.layers.len(), 1);
        assert_eq!(back.layers[0].name, "Layer 0");
        assert_eq!(back.work_path.subpaths.len(), 1);
        assert_eq!(back.work_path.subpaths[0].points.len(), 2);
    }

    #[test]
    fn an_unwritable_destination_fails_and_leaves_no_temp_file() {
        let doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        let dir =
            std::env::temp_dir().join(format!("pictura-recovery-missing-{}", std::process::id()));
        let path = dir.join("nested").join("1.psd");
        assert!(write_snapshot(doc, path.to_str().unwrap()).is_err());
        assert!(!dir.exists());
    }
}
