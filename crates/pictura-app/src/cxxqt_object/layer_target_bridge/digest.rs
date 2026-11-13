//! A layer's pixels as one digest, for the control server's `layer_digest`
//! (#1290): a run under a hard memory limit is checked against the same run in
//! RAM without flattening a document too large to flatten.

use super::PictureView;
use cxx_qt::CxxQtType;
use pictura_core::{Plane, PsdRect};

pub(super) fn layer_pixel_digest(view: &PictureView, path: &cxx_qt_lib::QString) -> Vec<i64> {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return Vec::new();
    };
    let Some(layer) = pictura_render::resolve_path(doc, &path.to_string()) else {
        return Vec::new();
    };
    let r = layer.rect;
    vec![
        layer_digest(layer) as i64,
        i64::from(r.left),
        i64::from(r.top),
        i64::from(r.right),
        i64::from(r.bottom),
    ]
}

/// FNV-1a, as [`pictura_core::content_digest`], folded a row at a time.
struct Fnv(u64);

impl Fnv {
    fn feed(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn rect(&mut self, r: PsdRect) {
        for v in [r.left, r.top, r.right, r.bottom] {
            self.feed(&v.to_le_bytes());
        }
    }

    /// Every row of `plane` over `r`, read through the tiles; a row the plane
    /// does not hold is one 0xFF marker, so an empty plane is not a black one.
    fn rows(&mut self, plane: &Plane<u8>, r: PsdRect, row: &mut Vec<u8>) {
        let (w, h) = (r.width().max(0) as usize, r.height().max(0) as usize);
        row.resize(w, 0);
        for y in 0..h {
            if plane.read_row_into(y, w, row) {
                self.feed(row);
            } else {
                self.feed(&[0xFF]);
            }
        }
    }
}

fn layer_digest(layer: &pictura_core::Layer) -> u64 {
    let mut h = Fnv(0xcbf2_9ce4_8422_2325);
    let mut row = Vec::new();
    h.rect(layer.rect);
    for channel in &layer.channels {
        h.feed(&channel.id.to_le_bytes());
        h.rows(&channel.data, layer.rect, &mut row);
    }
    if let Some(mask) = &layer.mask {
        if let Some(data) = &mask.data {
            h.rect(mask.rect);
            h.rows(data, mask.rect, &mut row);
        }
    }
    h.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{Channel, Layer};

    fn layer(fill: u8) -> Layer {
        let rect = PsdRect {
            top: 2,
            left: 3,
            bottom: 6,
            right: 8,
        };
        let mut layer = Layer {
            rect,
            ..Layer::default()
        };
        for id in [-1i16, 0, 1, 2] {
            layer.channels.push(Channel {
                id,
                data: Plane::from(vec![fill; 20]),
            });
        }
        layer
    }

    #[test]
    fn the_digest_sees_every_pixel_and_the_tiling_does_not() {
        let flat = layer(9);
        let mut tiled = layer(9);
        for channel in &mut tiled.channels {
            assert!(channel.data.tile(5, 4));
        }
        assert_eq!(layer_digest(&flat), layer_digest(&tiled));
        let mut changed = layer(9);
        let mut data = vec![9u8; 20];
        data[19] = 10;
        changed.channels[2].data = Plane::from(data);
        assert_ne!(layer_digest(&flat), layer_digest(&changed));
        let mut empty = layer(0);
        empty.channels[0].data = Plane::default();
        assert_ne!(layer_digest(&layer(0)), layer_digest(&empty));
    }
}
