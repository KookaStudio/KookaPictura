//! Lens Blur's depth sources. The dialog's Source combo is resolved into a
//! `DepthMap` over the active layer's rectangle before every preview and
//! commit, and the `lens-blur` kind's hidden slot names it by id. Unlike
//! Displace's file maps the plane is derived from the document, so a stale id
//! is never produced: the dialog re-resolves on every value read.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use pictura_core::{Layer, LayerMask, PsdRect};
use pictura_filters::DepthMap;

use super::super::helpers::active_pixel_layer;
use super::super::helpers_composite::selection_to_mask;
use super::super::state::PictureViewRust;
#[cfg(test)]
use crate::layer_target::LayerTarget;

/// Preview, Source, Blur Focal Distance, Invert, Shape, Radius, Blade
/// Curvature, Rotation, Specular Brightness, Specular Threshold, Noise Amount,
/// Distribution, Monochromatic, the depth-map id, and the noise seed.
pub(in crate::cxxqt_object) const LENS_BLUR_SLOTS: usize = 15;

/// ponytail: the eight most recent maps stay in memory for the process; a Last
/// Filter that names an older one is refused, and none survive a restart.
const KEPT: usize = 8;

struct Store {
    next: u32,
    maps: VecDeque<(u32, Arc<DepthMap>)>,
}

static STORE: Mutex<Store> = Mutex::new(Store {
    next: 1,
    maps: VecDeque::new(),
});

/// Keep `map` and return its id (never 0). A map equal to the newest one is
/// the same id again, so a slider drag's repeated reads do not churn the store.
pub(in crate::cxxqt_object) fn register(map: DepthMap) -> u32 {
    let mut store = STORE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((id, newest)) = store.maps.back() {
        if newest.as_ref() == &map {
            return *id;
        }
    }
    let id = store.next;
    store.next = store.next.wrapping_add(1).max(1);
    if store.maps.len() == KEPT {
        store.maps.pop_front();
    }
    store.maps.push_back((id, Arc::new(map)));
    id
}

/// A commit's slots. The Source slot is document state, so the depth map is
/// materialized here rather than trusted from the dialog's preview-time id:
/// the store keeps only the eight most recent maps, and a later Lens Blur
/// session can let the committed id go. Last Filter then replays the source
/// against the document as it stands, like a fresh dialog would.
pub(in crate::cxxqt_object) fn commit_slots(
    kind: &str,
    rust: &PictureViewRust,
    params: &[f64],
) -> Vec<f64> {
    let mut params = params.to_vec();
    if kind == "lens-blur" && params.len() == LENS_BLUR_SLOTS && params[1] >= 0.5 {
        params[13] = match materialize(rust, params[1] as i32) {
            Some(map) => f64::from(register(map)),
            None => 0.0,
        };
    }
    params
}

/// The map `id` names, or `None` for id 0 (no map chosen) or an id the store
/// no longer holds.
pub(in crate::cxxqt_object) fn depth_map(id: u32) -> Option<Arc<DepthMap>> {
    if id == 0 {
        return None;
    }
    let store = STORE.lock().unwrap_or_else(|e| e.into_inner());
    store
        .maps
        .iter()
        .find(|(kept, _)| *kept == id)
        .map(|(_, map)| map.clone())
}

/// The active layer's depth plane for `source` (1 Transparency, 2 Layer Mask,
/// 3 Selection), sized to the layer's rectangle. `None` for Source None, an
/// unavailable source (a layer with no mask, no active selection), no
/// document, or a non-pixel active layer.
pub(super) fn materialize(rust: &PictureViewRust, source: i32) -> Option<DepthMap> {
    let doc = rust.session.doc.as_ref()?;
    let layer = active_pixel_layer(doc, rust.layer_target.single())?;
    let lrect = layer.rect;
    if lrect.width() <= 0 || lrect.height() <= 0 {
        return None;
    }
    let (lw, lh) = (lrect.width() as u32, lrect.height() as u32);
    let plane = match source {
        1 => transparency_plane(layer, lw, lh),
        2 => crop_mask(layer.mask.as_ref()?, lrect, lw, lh),
        3 => {
            let mask = selection_to_mask(rust.session.selection.as_ref()?, doc);
            crop_mask(&mask, lrect, lw, lh)
        }
        _ => return None,
    };
    DepthMap::new(lw, lh, plane)
}

/// The layer's transparency channel, or fully opaque when it has none.
fn transparency_plane(layer: &Layer, lw: u32, lh: u32) -> Vec<u8> {
    let n = (lw * lh) as usize;
    layer
        .channels
        .iter()
        .find(|c| c.id == -1 && c.data.len() == n)
        .map(|c| c.data.to_vec())
        .unwrap_or_else(|| vec![255; n])
}

/// `mask`'s coverage over `lrect`, in document coordinates; outside the mask
/// rect (or with no decoded data) the mask's `default_color` applies.
fn crop_mask(mask: &LayerMask, lrect: PsdRect, lw: u32, lh: u32) -> Vec<u8> {
    let n = (lw * lh) as usize;
    let mut plane = vec![mask.default_color; n];
    let Some(data) = &mask.data else {
        return plane;
    };
    let (mw, mh) = (mask.rect.width(), mask.rect.height());
    if mw <= 0 || mh <= 0 {
        return plane;
    }
    for y in 0..lh as i32 {
        for x in 0..lw as i32 {
            let mx = lrect.left + x - mask.rect.left;
            let my = lrect.top + y - mask.rect.top;
            if mx < 0 || my < 0 || mx >= mw || my >= mh {
                continue;
            }
            if let Some(&v) = data.as_slice().get(my as usize * mw as usize + mx as usize) {
                plane[y as usize * lw as usize + x as usize] = v;
            }
        }
    }
    plane
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, Channel, ColorMode, Document, Layer, LockFlags};
    use pictura_select::Selection;

    fn state() -> PictureViewRust {
        let mut doc = Document::new(8, 6, ColorMode::Rgb, BitDepth::Eight);
        let n = 48usize;
        let mut layer = Layer {
            name: "px".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 6,
                right: 8,
            },
            channels: vec![
                Channel {
                    id: 0,
                    data: vec![10; n].into(),
                },
                Channel {
                    id: 1,
                    data: vec![20; n].into(),
                },
                Channel {
                    id: 2,
                    data: vec![30; n].into(),
                },
            ],
            ..Default::default()
        };
        layer.lock = LockFlags::default();
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        PictureViewRust {
            session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
            layer_target: LayerTarget::only("0"),
            ..Default::default()
        }
    }

    #[test]
    fn transparency_is_the_alpha_or_fully_opaque() {
        let rust = state();
        let map = materialize(&rust, 1).expect("opaque transparency");
        assert_eq!(map, DepthMap::new(8, 6, vec![255; 48]).unwrap());

        let mut alpha = state();
        let layer = &mut alpha.session.doc.as_mut().unwrap().layers[0];
        layer.channels.push(Channel {
            id: -1,
            data: (0..48u8).collect::<Vec<_>>().into(),
        });
        let map = materialize(&alpha, 1).expect("alpha transparency");
        assert_eq!(map, DepthMap::new(8, 6, (0..48u8).collect()).unwrap());
    }

    #[test]
    fn a_layer_mask_is_cropped_to_the_layer_rect() {
        let mut rust = state();
        let layer = &mut rust.session.doc.as_mut().unwrap().layers[0];
        layer.mask = Some(LayerMask {
            rect: PsdRect {
                top: 0,
                left: 2,
                bottom: 6,
                right: 8,
            },
            default_color: 17,
            disabled: false,
            flags: 0,
            data: Some(vec![99; 6 * 6].into()),
            extra: Vec::new(),
            ..Default::default()
        });
        let map = materialize(&rust, 2).expect("mask");
        let expected: Vec<u8> = (0..6)
            .flat_map(|_| (0..8).map(|x| if x >= 2 { 99 } else { 17 }))
            .collect();
        assert_eq!(map, DepthMap::new(8, 6, expected).unwrap());
    }

    #[test]
    fn an_unavailable_source_is_refused() {
        let rust = state();
        assert!(materialize(&rust, 0).is_none(), "None is not a plane");
        assert!(materialize(&rust, 2).is_none(), "no layer mask");
        assert!(materialize(&rust, 3).is_none(), "no selection");
    }

    #[test]
    fn a_selection_becomes_the_depth_plane() {
        let mut rust = state();
        rust.session.set_selection(Some(Selection {
            width: 8,
            height: 6,
            data: (0..48u8).map(|i| i * 5).collect(),
        }));
        let map = materialize(&rust, 3).expect("selection");
        let expected: Vec<u8> = (0..48u8).map(|i| i * 5).collect();
        assert_eq!(map, DepthMap::new(8, 6, expected).unwrap());
    }

    #[test]
    fn ids_name_their_map_and_old_maps_are_let_go() {
        assert!(depth_map(0).is_none(), "id 0 is no map");
        let first = register(DepthMap::new(1, 1, vec![7]).unwrap());
        assert_ne!(first, 0);
        assert_eq!(
            *depth_map(first).unwrap(),
            DepthMap::new(1, 1, vec![7]).unwrap()
        );
        // Other tests register too, so push past the cap from here; each map
        // must differ from the newest, or the dedupe returns its id.
        for i in 0..KEPT {
            register(DepthMap::new(1, 1, vec![(i + 1) as u8]).unwrap());
        }
        assert!(depth_map(first).is_none(), "the oldest map was kept");
    }

    #[test]
    fn a_commit_re_materializes_the_source_slot() {
        let mut rust = state();
        let layer = &mut rust.session.doc.as_mut().unwrap().layers[0];
        layer.channels.push(Channel {
            id: -1,
            data: (0..48u8).collect::<Vec<_>>().into(),
        });
        let mut params = vec![0.0; LENS_BLUR_SLOTS];
        params[1] = 1.0; // Transparency
        let slots = commit_slots("lens-blur", &rust, &params);
        assert!(slots[13] > 0.0, "the source was not materialized");
        assert!(depth_map(slots[13] as u32).is_some());

        params[1] = 2.0; // Layer Mask, none present: refused
        assert_eq!(commit_slots("lens-blur", &rust, &params)[13], 0.0);

        params[1] = 0.0; // Source None: no map
        assert_eq!(commit_slots("lens-blur", &rust, &params)[13], 0.0);
    }
}
