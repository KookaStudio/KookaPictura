//! Displace's map files. CS6 asks for the map after the dialog's OK, and the
//! filter is a kind plus number slots, so a loaded map is kept here and the
//! `displace` kind's last slot names it by id. Last Filter replays the same id,
//! so it reuses the map without asking again, as CS6 does.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use pictura_core::ColorMode;
use pictura_filters::DisplaceMap;

use super::super::helpers_composite::current_buffer;

/// ponytail: the eight most recent maps stay in memory for the process; a
/// Last Filter that names an older one is refused, and none survive a restart.
const KEPT: usize = 8;

struct Store {
    next: u32,
    maps: VecDeque<(u32, Arc<DisplaceMap>)>,
}

static STORE: Mutex<Store> = Mutex::new(Store {
    next: 1,
    maps: VecDeque::new(),
});

/// Keep `map` and return its id (never 0).
fn register(map: DisplaceMap) -> u32 {
    let mut store = STORE.lock().unwrap_or_else(|e| e.into_inner());
    let id = store.next;
    store.next = store.next.wrapping_add(1).max(1);
    if store.maps.len() == KEPT {
        store.maps.pop_front();
    }
    store.maps.push_back((id, Arc::new(map)));
    id
}

/// The map `id` names. Id 0 — no map chosen yet, as in a dialog's defaults —
/// is the neutral map, which moves nothing; an id no longer kept is `None`.
pub(in crate::cxxqt_object) fn displace_map(id: u32) -> Option<Arc<DisplaceMap>> {
    if id == 0 {
        return Some(Arc::new(DisplaceMap::neutral()));
    }
    let store = STORE.lock().unwrap_or_else(|e| e.into_inner());
    store
        .maps
        .iter()
        .find(|(kept, _)| *kept == id)
        .map(|(_, map)| map.clone())
}

/// Read the PSD at `path` and keep it as a map.
pub(super) fn load(path: &str) -> Result<u32, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    map_from_psd(&bytes).map(register)
}

/// The flattened file's first two channels; a one-channel file gives its plane
/// to both axes. The embedded profile is ignored: the samples are distances,
/// not colours.
/// ponytail: CMYK, Lab and Indexed files are read as RGB, so their first two
/// channels are red and green rather than the file's own first two.
fn map_from_psd(bytes: &[u8]) -> Result<DisplaceMap, String> {
    let doc = pictura_codec::read_psd_with(bytes, pictura_codec::Policy::Off)
        .map_err(|e| e.to_string())?;
    if doc.source_mode == Some(ColorMode::Bitmap) || doc.mode == ColorMode::Bitmap {
        return Err("a Bitmap-mode file cannot be a displacement map".into());
    }
    let flat = if doc.merged_composite_present {
        doc.composite.clone()
    } else {
        current_buffer(&doc, false)
    };
    let n = flat.pixel_count();
    if flat.channels == 0 || flat.data.len() < n * flat.channels as usize {
        return Err("the file has no image data".into());
    }
    let horizontal = flat.data[..n].to_vec();
    let vertical = if doc.mode.color_channels() >= 2 && flat.channels >= 2 {
        flat.data[n..2 * n].to_vec()
    } else {
        horizontal.clone()
    };
    DisplaceMap::new(flat.width, flat.height, horizontal, vertical)
        .ok_or_else(|| "the file is empty".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, Document};

    fn psd(mode: ColorMode, planes: &[&[u8]]) -> Vec<u8> {
        let mut doc = Document::new(2, 2, mode, BitDepth::Eight);
        let data: Vec<u8> = planes.concat();
        doc.composite.data = data.into();
        pictura_codec::write_psd(&doc).expect("writes")
    }

    #[test]
    fn an_rgb_file_gives_red_across_and_green_down() {
        let bytes = psd(
            ColorMode::Rgb,
            &[&[0, 64, 128, 255], &[10, 20, 30, 40], &[9; 4]],
        );
        let map = map_from_psd(&bytes).expect("reads");
        let expected = DisplaceMap::new(2, 2, vec![0, 64, 128, 255], vec![10, 20, 30, 40]).unwrap();
        assert_eq!(map, expected);
    }

    #[test]
    fn a_grayscale_file_drives_both_axes_with_its_one_plane() {
        let bytes = psd(ColorMode::Grayscale, &[&[0, 64, 128, 255]]);
        let map = map_from_psd(&bytes).expect("reads");
        let plane = vec![0, 64, 128, 255];
        assert_eq!(map, DisplaceMap::new(2, 2, plane.clone(), plane).unwrap());
    }

    #[test]
    fn a_bitmap_file_is_refused() {
        // A raw 8×1 depth-1 Bitmap PSD, one packed byte of pixels.
        let mut p = b"8BPS".to_vec();
        p.extend_from_slice(&1u16.to_be_bytes());
        p.extend_from_slice(&[0; 6]);
        p.extend_from_slice(&1u16.to_be_bytes()); // channels
        p.extend_from_slice(&1u32.to_be_bytes()); // height
        p.extend_from_slice(&8u32.to_be_bytes()); // width
        p.extend_from_slice(&1u16.to_be_bytes()); // depth
        p.extend_from_slice(&0u16.to_be_bytes()); // mode Bitmap
        p.extend_from_slice(&[0; 12]); // empty mode data, resources, layers
        p.extend_from_slice(&0u16.to_be_bytes()); // raw
        p.push(0b1010_1010);
        let error = map_from_psd(&p).expect_err("a Bitmap map was accepted");
        assert!(error.contains("Bitmap"), "{error}");
    }

    #[test]
    fn garbage_is_refused() {
        assert!(map_from_psd(b"not a psd").is_err());
    }

    #[test]
    fn ids_name_their_map_and_old_maps_are_let_go() {
        assert_eq!(*displace_map(0).unwrap(), DisplaceMap::neutral());
        let first = register(DisplaceMap::new(1, 1, vec![1], vec![2]).unwrap());
        assert_ne!(first, 0);
        assert_eq!(displace_map(first).unwrap().width(), 1);
        // Other tests register too, so push past the cap from here.
        for _ in 0..KEPT {
            register(DisplaceMap::neutral());
        }
        assert!(displace_map(first).is_none(), "the oldest map was kept");
    }
}
