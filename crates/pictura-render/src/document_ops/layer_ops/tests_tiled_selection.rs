//! A tiled selection read by the clipboard and the layer operations (#1268):
//! each runs without materializing the selection and gives what the same
//! selection held flat (the dense path) gives.

use super::clipboard::{clear_layer, copy_layer, copy_merged, copy_merged_region, paste_clip};
use super::mask_edit::move_mask_selection_content;
use super::move_content::lift_selection;
use super::tests::pixel_layer;
use super::via::{layer_via_copy, layer_via_cut};
use super::PasteMode;
use pictura_core::{
    assert_no_materialization, BitDepth, ColorMode, Document, LayerMask, PixelBuffer, Plane,
    PsdRect,
};

const W: usize = 600;
const H: usize = 530;

fn next(state: &mut u64) -> u8 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 56) as u8
}

/// A seeded soft selection: a solid block that holds whole uniform tiles, an
/// unselected margin, and noise across the tile edges in between. Returned
/// flat and tiled.
fn soft_selection() -> (Plane<u8>, Plane<u8>) {
    let mut state = 0x1268;
    let data: Vec<u8> = (0..W * H)
        .map(|i| {
            let (x, y) = (i % W, i / W);
            let noise = next(&mut state);
            if (40..W - 30).contains(&x) && (30..H - 20).contains(&y) {
                255
            } else if x < 20 || y < 10 {
                0
            } else {
                noise
            }
        })
        .collect();
    let flat = Plane::from(data);
    let mut tiled = flat.clone();
    assert!(tiled.tile(W, H));
    assert!(tiled.is_tiled() && !flat.is_tiled());
    (flat, tiled)
}

fn canvas() -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: H as i32,
        right: W as i32,
    }
}

/// A document with one seeded pixel layer offset from the canvas, so reads
/// cross the layer's edge, with a document-sized raster mask.
fn document() -> Document {
    let mut state = 0x5eed;
    let mut layer = pixel_layer("base", W as u32 - 50, H as u32 - 40, 0);
    layer.rect = PsdRect {
        top: 25,
        left: 30,
        bottom: H as i32 - 15,
        right: W as i32 - 20,
    };
    for channel in layer.channels_mut().iter_mut() {
        for value in channel.data.as_mut_slice() {
            *value = next(&mut state);
        }
    }
    layer.mask = Some(LayerMask {
        rect: canvas(),
        default_color: 255,
        data: Some(
            (0..W * H)
                .map(|i| (i % 251) as u8)
                .collect::<Vec<_>>()
                .into(),
        ),
        ..Default::default()
    });
    let mut doc = Document::new(W as u32, H as u32, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    doc
}

fn selection_mask(data: &Plane<u8>) -> LayerMask {
    LayerMask {
        rect: canvas(),
        default_color: 0,
        data: Some(data.clone()),
        ..Default::default()
    }
}

#[test]
fn clipboard_reads_a_tiled_selection() {
    let (flat, tiled) = soft_selection();
    let doc = document();
    let mut state = 7;
    let composite = PixelBuffer {
        width: W as u32,
        height: H as u32,
        channels: 4,
        data: (0..W * H * 4)
            .map(|_| next(&mut state))
            .collect::<Vec<_>>()
            .into(),
    };
    let read = |region: PsdRect| {
        let (rw, rh) = (region.width() as usize, region.height() as usize);
        let mut out = PixelBuffer::new(rw as u32, rh as u32, 4);
        let plane = out.data.as_mut_slice();
        for c in 0..4 {
            for y in 0..rh {
                for x in 0..rw {
                    let src = c * W * H + (region.top as usize + y) * W + region.left as usize + x;
                    plane[c * rw * rh + y * rw + x] = composite.data[src];
                }
            }
        }
        out
    };

    let copied = assert_no_materialization(|| {
        let (width, height) = (doc.width, doc.height);
        match pictura_core::Coverage::for_canvas(&(tiled), width, height) {
            Some(coverage) => copy_layer(&doc, "0", Some(&coverage)),
            None => None,
        }
    });
    assert_eq!(copied, {
        let (width, height) = (doc.width, doc.height);
        match pictura_core::Coverage::for_canvas(&(flat), width, height) {
            Some(coverage) => copy_layer(&doc, "0", Some(&coverage)),
            None => None,
        }
    });
    assert!(copied.is_some());
    let merged = assert_no_materialization(|| {
        let (width, height) = (doc.width, doc.height);
        match pictura_core::Coverage::for_canvas(&(tiled), width, height) {
            Some(coverage) => copy_merged(&doc, &composite, Some(&coverage)),
            None => None,
        }
    });
    assert_eq!(merged, {
        let (width, height) = (doc.width, doc.height);
        match pictura_core::Coverage::for_canvas(&(flat), width, height) {
            Some(coverage) => copy_merged(&doc, &composite, Some(&coverage)),
            None => None,
        }
    });
    let region = assert_no_materialization(|| {
        copy_merged_region(
            &doc,
            Some(&pictura_core::Coverage::for_canvas(&tiled, W as u32, H as u32).unwrap()),
            read,
        )
    });
    assert_eq!(region, merged);

    let (mut cleared, mut want) = (doc.clone(), doc.clone());
    assert!(assert_no_materialization(|| {
        let (width, height) = (cleared.width, cleared.height);
        match pictura_core::Coverage::for_canvas(&(tiled), width, height) {
            Some(coverage) => clear_layer(&mut cleared, "0", Some(&coverage)),
            None => false,
        }
    }));
    assert!({
        let (width, height) = (want.width, want.height);
        match pictura_core::Coverage::for_canvas(&(flat), width, height) {
            Some(coverage) => clear_layer(&mut want, "0", Some(&coverage)),
            None => false,
        }
    });
    assert_eq!(cleared, want);

    let clip = copied.expect("a clip");
    for mode in [PasteMode::Into, PasteMode::Outside] {
        let (mut pasted, mut want) = (doc.clone(), doc.clone());
        let path = assert_no_materialization(|| {
            let (width, height) = (pasted.width, pasted.height);
            match pictura_core::Coverage::for_canvas(&(tiled), width, height) {
                Some(coverage) => {
                    paste_clip(&mut pasted, "0", &clip, (5, 7), mode, Some(&coverage))
                }
                None => String::new(),
            }
        });
        {
            let (width, height) = (want.width, want.height);
            match pictura_core::Coverage::for_canvas(&(flat), width, height) {
                Some(coverage) => paste_clip(&mut want, "0", &clip, (5, 7), mode, Some(&coverage)),
                None => String::new(),
            }
        };
        assert_eq!(path, "1");
        assert_eq!(pasted, want, "{mode:?}");
    }
}

#[test]
fn layer_ops_read_a_tiled_selection() {
    let (flat, tiled) = soft_selection();
    let doc = document();

    let (mut lifted, mut want) = (doc.clone(), doc.clone());
    let path = assert_no_materialization(|| {
        let (width, height) = (lifted.width, lifted.height);
        match pictura_core::Coverage::for_canvas(&(tiled), width, height) {
            Some(coverage) => lift_selection(&mut lifted, "0", &coverage),
            None => String::new(),
        }
    });
    assert_eq!(path, {
        let (width, height) = (want.width, want.height);
        match pictura_core::Coverage::for_canvas(&(flat), width, height) {
            Some(coverage) => lift_selection(&mut want, "0", &coverage),
            None => String::new(),
        }
    });
    assert_eq!(path, "1");
    assert_eq!(lifted, want, "lift");

    for cut in [false, true] {
        let via = |doc: &mut Document, mask: &LayerMask| {
            if cut {
                layer_via_cut(
                    doc,
                    "0",
                    &mask.edit_coverage().expect("valid mask geometry"),
                )
            } else {
                layer_via_copy(
                    doc,
                    "0",
                    &mask.edit_coverage().expect("valid mask geometry"),
                )
            }
        };
        let (mut got, mut want) = (doc.clone(), doc.clone());
        let mask = selection_mask(&tiled);
        assert_no_materialization(|| via(&mut got, &mask));
        via(&mut want, &selection_mask(&flat));
        assert_eq!(got, want, "via cut={cut}");
    }

    for duplicate in [false, true] {
        let (mut got, mut want) = (doc.clone(), doc.clone());
        assert!(assert_no_materialization(|| {
            {
                let (width, height) = (got.width, got.height);
                match pictura_core::Coverage::for_canvas(&(tiled), width, height) {
                    Some(coverage) => {
                        move_mask_selection_content(&mut got, "0", &coverage, 17, -9, duplicate, 0)
                    }
                    None => false,
                }
            }
        }));
        assert!({
            let (width, height) = (want.width, want.height);
            match pictura_core::Coverage::for_canvas(&(flat), width, height) {
                Some(coverage) => {
                    move_mask_selection_content(&mut want, "0", &coverage, 17, -9, duplicate, 0)
                }
                None => false,
            }
        });
        assert_eq!(got, want, "mask move duplicate={duplicate}");
    }
}
