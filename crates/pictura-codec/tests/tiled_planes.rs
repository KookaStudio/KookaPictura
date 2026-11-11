//! PSD read and write stream through tiled planes (#1213).

use pictura_codec::{read_psd, write_psd};
use pictura_core::{assert_no_materialization, Document, LayerMask, Plane, TileState};

const W: u32 = 300;
const H: u32 = 280;

/// A layered RGBA document with varied pixels and a raster mask, every plane
/// flat.
fn layered() -> Document {
    let rgba: Vec<u8> = (0..W * H * 4)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8)
        .collect();
    let mut doc = Document::from_rgba("tiled", W, H, &rgba);
    let layer = doc.layers.get_mut(0).expect("a layer");
    let mask: Vec<u8> = (0..W * H).map(|i| (i % 7 * 40) as u8).collect();
    layer.mask = Some(LayerMask {
        rect: layer.rect,
        data: Some(mask.into()),
        ..Default::default()
    });
    doc
}

/// `plane` tiled at `w × h` with its first tile forked from a held clone.
fn diverge(plane: &mut Plane<u8>, w: usize, h: usize, held: &mut Vec<Plane<u8>>) {
    assert!(plane.tile(w, h));
    held.push(plane.clone());
    let v = plane.sample(0).expect("a sample");
    plane.write_rect_from(0..1, 0..1, w, &[v.wrapping_add(1)]);
    plane.write_rect_from(0..1, 0..1, w, &[v]);
}

// [tp_psd_write]
#[test]
fn psd_write_is_byte_identical_tiled() {
    let flat = layered();
    let expected = write_psd(&flat).expect("flat write");

    let mut tiled = flat.clone();
    let mut held = Vec::new();
    let (w, h) = (W as usize, H as usize);
    diverge(&mut tiled.composite.data, w, h, &mut held);
    for layer in tiled.layers.iter_mut() {
        let (lw, lh) = (layer.rect.width() as usize, layer.rect.height() as usize);
        for channel in &mut layer.channels {
            diverge(&mut channel.data, lw, lh, &mut held);
        }
        if let Some(data) = layer.mask.as_mut().and_then(|m| m.data.as_mut()) {
            diverge(data, lw, lh, &mut held);
        }
    }
    let written = assert_no_materialization(|| write_psd(&tiled).expect("tiled write"));
    assert!(
        written == expected,
        "a tiled document writes the same bytes"
    );
}

// [tp_psd_read]
#[test]
fn reading_fills_tiles() {
    let (w, h) = (600u32, 520u32);
    let mut rgba = vec![255u8; (w * h * 4) as usize];
    for y in 10..20 {
        for x in 10..20 {
            let i = ((y * w + x) * 4) as usize;
            rgba[i..i + 3].copy_from_slice(&[200, 30, 40]);
        }
    }
    let bytes = write_psd(&Document::from_rgba("white", w, h, &rgba)).expect("write");
    let doc = read_psd(&bytes).expect("read");
    let layer = doc.layers.get(0).expect("a layer");
    for channel in &layer.channels {
        assert!(channel.data.is_tiled(), "channel {} is tiled", channel.id);
        let last = channel.data.tile_count() - 1;
        assert!(
            matches!(
                channel.data.tile_state(last),
                Some(TileState::Uniform(_) | TileState::Empty)
            ),
            "channel {}: a white region holds no allocation",
            channel.id
        );
    }
    let red = &layer.channels.iter().find(|c| c.id == 0).expect("red").data;
    assert_eq!(red.sample(10 * w as usize + 10), Some(200));
    assert!(matches!(red.tile_state(0), Some(TileState::Dense(_))));
}

/// The merged image a document without a stored composite is saved with
/// (#1216): band-written tiled planes whose uniform tiles hold no allocation.
/// Every compression writes the same bytes as the flat composite.
// [rc_save]
#[test]
fn streamed_merged_image_is_byte_identical() {
    let (w, h) = (W as usize, H as usize);
    let mut flat = Document::from_rgba("merged", W, H, &vec![255u8; w * h * 4]);
    // A painted square over white: most tiles stay uniform.
    let mut rgba = vec![255u8; w * h * 4];
    for y in 40..170 {
        for x in 30..260 {
            let i = (y * w + x) * 4;
            rgba[i..i + 4].copy_from_slice(&[(x % 251) as u8, (y % 241) as u8, 9, 255]);
        }
    }
    let planar: Vec<u8> = (0..4)
        .flat_map(|c| (0..w * h).map(move |i| (c, i)))
        .map(|(c, i)| rgba[i * 4 + c])
        .collect();
    flat.composite.data = planar.clone().into();

    let mut banded = Plane::filled(w, h, 4, 0u8);
    for band in (0..h).step_by(256) {
        let rows = (h - band).min(256);
        for c in 0..4 {
            let start = c * w * h + band * w;
            banded.write_rect_from(
                0..w,
                c * h + band..c * h + band + rows,
                w,
                &planar[start..start + rows * w],
            );
        }
        banded.settle();
    }
    assert!(
        banded.allocated_bytes() < w * h * 4,
        "uniform tiles hold no allocation"
    );
    for compression in [
        pictura_core::Compression::Rle,
        pictura_core::Compression::Raw,
        pictura_core::Compression::Zip,
    ] {
        flat.composite_compression = compression;
        let mut streamed = flat.clone();
        streamed.composite.data = banded.clone();
        let written = assert_no_materialization(|| write_psd(&streamed).expect("tiled write"));
        assert!(
            written == write_psd(&flat).expect("flat write"),
            "{compression:?}: the band-written merged image writes the same bytes"
        );
    }
}

/// An application open of a layered 8-bit RGB file skips the merged image's
/// colour planes, which it would only drop (#1216); the layers and the extra
/// channels are read as without the option.
#[test]
fn a_layered_merged_image_is_skipped_on_request() {
    let mut doc = layered();
    doc.channels.push(pictura_core::Channel {
        id: 0,
        data: (0..W * H)
            .map(|i| (i % 253) as u8)
            .collect::<Vec<u8>>()
            .into(),
    });
    for compression in [
        pictura_core::Compression::Rle,
        pictura_core::Compression::Raw,
    ] {
        doc.composite_compression = compression;
        let bytes = write_psd(&doc).expect("write");
        let full = read_psd(&bytes).expect("read");
        let skipped = pictura_codec::read_psd_with_options(
            &bytes,
            &pictura_codec::OpenOptions {
                skip_layered_merged: true,
                ..pictura_codec::OpenOptions::default()
            },
        )
        .expect("read skipping the merged image");
        assert!(skipped.merged_composite_present, "{compression:?}");
        assert!(
            skipped.composite.data.is_empty(),
            "{compression:?}: no merged planes"
        );
        assert!(
            skipped.layers == full.layers,
            "{compression:?}: the same layers"
        );
        assert!(
            skipped.channels == full.channels,
            "{compression:?}: the same extra channels"
        );
    }
}
