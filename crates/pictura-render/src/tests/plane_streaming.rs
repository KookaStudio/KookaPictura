//! The compositing, Move and Transform paths read diverged banded planes in
//! place (#1212): each runs on a document whose planes are fractured with a
//! forked band, under `assert_no_materialization`, and must match the same
//! operation on flat copies of those planes byte for byte.

use super::*;
use pictura_core::{assert_no_materialization, Plane};

const W: u32 = 24;
const H: u32 = 200;

fn varied(n: usize, seed: usize) -> Vec<u8> {
    (0..n).map(|i| ((i * 31 + seed * 7) % 251) as u8).collect()
}

/// A layer at `r` with varied colour and an alpha that is clear outside a
/// margin, so a trim has something to drop.
fn varied_layer(r: PsdRect) -> Layer {
    let (w, h) = (r.width() as usize, r.height() as usize);
    let mut layer = solid("varied", r, (0, 0, 0), 255, BlendMode::Normal, 255);
    for (k, channel) in layer.channels.iter_mut().enumerate() {
        let mut data = varied(w * h, k);
        if channel.id == -1 {
            for (i, a) in data.iter_mut().enumerate() {
                let (x, y) = (i % w, i / w);
                if x < 3 || y < 70 || y >= h - 5 {
                    *a = 0;
                }
            }
        }
        channel.data = data.into();
    }
    layer
}

fn with_mask(mut layer: Layer) -> Layer {
    let r = layer.rect;
    let n = (r.width() * r.height()) as usize;
    layer.mask = Some(LayerMask {
        rect: r,
        default_color: 0,
        data: Some(varied(n, 9).into()),
        ..Default::default()
    });
    layer
}

/// Every plane of `planes` fractured at `width` with band 1 forked from a held
/// clone, so a whole-plane read would have to materialize it.
fn diverge_plane(plane: &mut Plane<u8>, width: usize, held: &mut Vec<Plane<u8>>) {
    plane.fracture(width);
    held.push(plane.clone());
    if let Some(band) = plane.write_band(1) {
        band[0] = band[0].wrapping_add(1);
    }
}

/// The document with every layer plane diverged, the clones that keep them
/// diverged, and a reference copy whose planes are flat.
fn diverged(d: Document) -> (Document, Vec<Plane<u8>>, Document) {
    let mut banded = d;
    let mut held = Vec::new();
    for layer in banded.layers.iter_mut() {
        let w = layer.rect.width() as usize;
        for channel in &mut layer.channels {
            diverge_plane(&mut channel.data, w, &mut held);
        }
        if let Some(mask) = layer.mask.as_mut() {
            let mw = mask.rect.width() as usize;
            if let Some(data) = mask.data.as_mut() {
                diverge_plane(data, mw, &mut held);
            }
        }
    }
    let mut flat = banded.clone();
    for layer in flat.layers.iter_mut() {
        for channel in &mut layer.channels {
            channel.data = channel.data.as_slice().to_vec().into();
        }
        if let Some(data) = layer.mask.as_mut().and_then(|m| m.data.as_mut()) {
            *data = data.as_slice().to_vec().into();
        }
    }
    (banded, held, flat)
}

fn assert_same_layers(a: &Document, b: &Document) {
    assert_eq!(a.layers.len(), b.layers.len());
    for (x, y) in a.layers.iter().zip(b.layers.iter()) {
        assert_eq!(x.rect, y.rect, "{}", x.name);
        for (cx, cy) in x.channels.iter().zip(&y.channels) {
            assert_eq!(cx.id, cy.id);
            assert!(cx.data == cy.data, "{} channel {}", x.name, cx.id);
        }
    }
}

#[test]
fn composite_reads_a_diverged_mask_and_layer_in_place() {
    let base = solid(
        "base",
        full(W, H),
        (10, 20, 30),
        255,
        BlendMode::Normal,
        255,
    );
    let top = with_mask(varied_layer(rect(4, 2, H as i32 - 4, W as i32 - 1)));
    let (banded, _held, flat) = diverged(doc(W, H, vec![base, top]));
    let got = assert_no_materialization(|| crate::composite_rgba(&banded));
    assert!(got.data == crate::composite_rgba(&flat).data);
}

#[test]
fn trim_and_untrim_read_diverged_rows_in_place() {
    let (mut banded, _held, mut flat) = diverged(doc(W, H, vec![varied_layer(full(W, H))]));
    let trimmed = assert_no_materialization(|| crate::trim_to_content(&mut banded, "0"));
    assert!(trimmed && crate::trim_to_content(&mut flat, "0"));
    assert_same_layers(&banded, &flat);

    let (mut banded, _held, mut flat) = diverged(banded);
    let grown = assert_no_materialization(|| crate::untrim_to(&mut banded, "0", full(W, H)));
    assert!(grown && crate::untrim_to(&mut flat, "0", full(W, H)));
    assert_same_layers(&banded, &flat);
}

#[test]
fn moving_selected_content_merges_into_diverged_rows_in_place() {
    let base = solid(
        "base",
        full(W, H),
        (10, 20, 30),
        255,
        BlendMode::Normal,
        255,
    );
    let layers = vec![base, varied_layer(full(W, H))];
    let (mut banded, _held, mut flat) = diverged(doc(W, H, layers));
    let mut coverage = vec![0u8; (W * H) as usize];
    for y in 60..150 {
        for x in 4..20 {
            coverage[y * W as usize + x] = 255;
        }
    }
    let selection = LayerMask {
        rect: full(W, H),
        default_color: 0,
        data: Some(coverage.into()),
        ..Default::default()
    };
    let moved = assert_no_materialization(|| {
        crate::move_selection_content(&mut banded, "1", &selection, 2, 70, false)
    });
    assert!(moved && crate::move_selection_content(&mut flat, "1", &selection, 2, 70, false));
    assert_same_layers(&banded, &flat);
}

#[test]
fn free_transform_resamples_a_diverged_layer_in_place() {
    let base = solid(
        "base",
        full(W, H),
        (10, 20, 30),
        255,
        BlendMode::Normal,
        255,
    );
    let top = with_mask(varied_layer(full(W, H)));
    let (mut banded, _held, mut flat) = diverged(doc(W, H, vec![base, top]));
    let transform = crate::LayerTransform {
        scale_x: 1.5,
        scale_y: 1.5,
        angle_radians: 15f64.to_radians(),
        dx: 0.0,
        dy: 0.0,
    };
    let interpolation = crate::Interpolation::Bicubic;
    let done = assert_no_materialization(|| {
        crate::transform_layer_with(&mut banded, "1", transform, interpolation)
    });
    assert!(done && crate::transform_layer_with(&mut flat, "1", transform, interpolation));
    assert_same_layers(&banded, &flat);
}
