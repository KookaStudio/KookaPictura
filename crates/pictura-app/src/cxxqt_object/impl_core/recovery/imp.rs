use super::*;

use super::super::super::filter_tools::store_planes;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::Document;
use std::sync::Arc;

pub(super) fn recovery_write(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    recovery_wait(view.as_mut());
    let session = &view.rust().session;
    let Some(doc_ref) = session.doc.as_ref() else {
        return false;
    };
    pictura_codec::warm_encode_idents(doc_ref);
    let mut doc = doc_ref.clone();
    let planes = store_planes(session.vanishing_planes.as_ref(), &doc);
    pictura_codec::set_vanishing_point_data(&mut doc, planes.as_deref());
    let path = path.to_string();
    let cache = Arc::clone(&session.encode_cache);
    let handle = std::thread::spawn(move || write_snapshot(doc, &path, &cache).is_ok());
    view.rust_mut().recovery_write = Some(handle);
    true
}

pub(super) fn recovery_wait(view: Pin<&mut PictureView>) -> bool {
    match view.rust_mut().recovery_write.take() {
        Some(handle) => handle.join().unwrap_or(false),
        None => true,
    }
}

pub(super) fn recovery_open(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    if !view.as_mut().open(path) {
        return false;
    }
    let mut rust = view.rust_mut();
    rust.path = None;
    rust.session.mark_unsaved();
    true
}

/// Encode `doc` as the native save does and land it at `path` atomically: a
/// synced sibling temp file renamed into place, so a crash mid-write keeps the
/// previous snapshot.
pub(super) fn write_snapshot(
    mut doc: Document,
    path: &str,
    cache: &pictura_codec::EncodeCache,
) -> std::io::Result<()> {
    pictura_render::refresh_native_composite(&mut doc);
    let bytes = pictura_codec::write_psd_cached(&pictura_render::save_view(&doc), cache)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    super::super::super::helpers::safe_io::write_atomically(path, &bytes, true)
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
        write_snapshot(doc.clone(), path, &pictura_codec::EncodeCache::new()).expect("write");
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
        assert!(write_snapshot(
            doc,
            path.to_str().unwrap(),
            &pictura_codec::EncodeCache::new()
        )
        .is_err());
        assert!(!dir.exists());
    }

    #[test]
    fn a_snapshot_reuses_the_encode_cache() {
        let rgba = vec![200u8; 4 * 3 * 4];
        let mut doc = Document::from_rgba("Layer 0", 4, 3, &rgba);
        let original = doc.layers.get(0).expect("layer").clone();
        let mut second = original.clone();
        second.name = "Layer 1".to_string();
        second.channels[0].data.as_mut_slice().fill(7);
        doc.layers = pictura_core::LayerStack::from_layers(vec![second, original]);

        let dir =
            std::env::temp_dir().join(format!("pictura-recovery-cache-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cache = pictura_codec::EncodeCache::new();
        let first = dir.join("1.psd");
        pictura_codec::warm_encode_idents(&doc);
        write_snapshot(doc.clone(), first.to_str().unwrap(), &cache).expect("first snapshot");
        let before = cache.stats().layers_encoded;
        doc.layers.get_mut(0).expect("layer").channels[0]
            .data
            .as_mut_slice()
            .fill(1);
        let second = dir.join("2.psd");
        pictura_codec::warm_encode_idents(&doc);
        write_snapshot(doc, second.to_str().unwrap(), &cache).expect("second snapshot");
        let back = pictura_codec::read_psd(&std::fs::read(&second).unwrap()).expect("reads");
        let edited = back
            .layers
            .iter()
            .find(|layer| layer.name == "Layer 1")
            .expect("edited layer");
        assert!(edited
            .channels
            .iter()
            .find(|channel| channel.id == 0)
            .expect("red channel")
            .data
            .iter()
            .all(|&byte| byte == 1));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(cache.stats().layers_encoded - before, 1);
    }
}
