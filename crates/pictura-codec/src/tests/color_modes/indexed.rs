use super::*;

pub(super) fn indexed_palette() -> [u8; 768] {
    let mut palette = [0u8; 768];
    for i in 0..256 {
        palette[i] = i as u8;
        palette[256 + i] = (255 - i) as u8;
        palette[512 + i] = ((i * 7) % 256) as u8;
    }
    palette
}

#[test]
fn indexed_document_writes_back_as_indexed() {
    // An unedited flat Indexed composite is retained on read and re-emitted
    // byte-identically on write, so re-reading reproduces the working RGB.
    let palette = indexed_palette();
    let indices = [0u8, 1, 2, 3];
    let p = flat_psd_with_data(8, 2, 1, 4, 1, &palette, &[&indices]);
    let doc = read_psd(&p).unwrap();
    assert_eq!(doc.mode, ColorMode::Rgb);
    assert_eq!(doc.source.mode(), Some(ColorMode::Indexed));
    assert!(doc.color_mode_data.is_empty(), "palette consumed");
    assert_eq!(doc.source.palette(), Some(palette));
    assert_eq!(
        doc.source
            .planes()
            .as_ref()
            .expect("index plane retained")
            .samples
            .to_bytes(),
        indices.to_vec(),
        "the index plane is retained"
    );

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[12..14].try_into().unwrap()),
        1,
        "one index channel"
    );
    assert_eq!(
        u16::from_be_bytes(out[22..24].try_into().unwrap()),
        8,
        "output depth stays 8"
    );
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        2,
        "output header color mode is Indexed"
    );
    assert_eq!(&out[30..30 + 768], &palette[..], "palette re-emitted");

    let back = read_psd(&out).unwrap();
    assert_eq!(back.mode, ColorMode::Rgb);
    assert_eq!(back.source.mode(), Some(ColorMode::Indexed));
    assert_eq!(back.source.palette(), Some(palette));
    assert_eq!(
        back.source
            .planes()
            .as_ref()
            .expect("index retained")
            .samples
            .to_bytes(),
        indices.to_vec(),
        "the written index plane equals the source exactly"
    );
    assert_eq!(
        back.composite, doc.composite,
        "re-reading the palette reproduces the working RGB exactly"
    );
}

#[test]
fn indexed_layer_color_channel_writes_back_as_indexed() {
    let palette = indexed_palette();
    let p = layered_psd_with_data(8, 2, 1, &palette, &[&[2]], &[(0, &[5]), (-1, &[200])]);
    let doc = read_psd(&p).unwrap();
    assert_eq!(doc.source.mode(), Some(ColorMode::Indexed));
    let layer = &doc.layers[0];
    assert_eq!(
        layer.channels().iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![0, 1, 2, -1],
        "the index layer becomes RGB plus transparency"
    );
    let retained = layer.source_channels.as_ref().expect("index retained");
    assert_eq!(retained.depth, BitDepth::Eight);
    assert_eq!(retained.planes, vec![(0, Samples::U8(vec![5].into()))]);

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        2,
        "output header color mode is Indexed"
    );

    let back = read_psd(&out).unwrap();
    let back_layer = &back.layers[0];
    assert_eq!(
        back_layer
            .channels()
            .iter()
            .map(|c| c.id)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, -1]
    );
    for c in 0..3 {
        assert_eq!(
            back_layer.channels()[c].data,
            layer.channels()[c].data,
            "layer color channel {c} is exact"
        );
    }
    assert_eq!(
        back_layer.channels()[3].data,
        vec![200],
        "transparency untouched"
    );
    assert_eq!(back.composite, doc.composite);
}

#[test]
fn grouped_indexed_layer_round_trips() {
    // A pixel layer nested in a group must have its index channel retained and
    // re-emitted like a top-level one.
    let palette = indexed_palette();
    let p = grouped_psd_with_data(2, 1, &palette, &[&[2]], &[(0, &[5])]);
    let doc = read_psd(&p).unwrap();
    assert_eq!(doc.mode, ColorMode::Rgb);
    assert_eq!(doc.source.mode(), Some(ColorMode::Indexed));
    assert!(doc.layers[0].is_group());
    let child = &doc.layers[0].children()[0];
    assert_eq!(
        child.channels().iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        child.source_channels.as_ref().unwrap().planes,
        vec![(0, Samples::U8(vec![5].into()))],
        "the nested index channel is retained"
    );

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        2,
        "output header color mode is Indexed"
    );

    let back = read_psd(&out).unwrap();
    let back_child = &back.layers[0].children()[0];
    for c in 0..3 {
        assert_eq!(
            back_child.channels()[c].data,
            child.channels()[c].data,
            "nested channel {c} after save"
        );
    }
    assert_eq!(back.composite, doc.composite);
}

#[test]
fn edited_indexed_composite_falls_back_to_rgb() {
    // An edit cannot be re-quantized: the whole document writes the working RGB.
    let palette = indexed_palette();
    let p = flat_psd_with_data(8, 2, 1, 2, 1, &palette, &[&[1, 2]]);
    let mut doc = read_psd(&p).unwrap();
    doc.composite
        .edit_data(|data| {
            data[0] ^= 1;
        })
        .expect("pixel edit preserves geometry");

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        3,
        "an edited composite writes RGB, not an invented quantization"
    );
    let back = read_psd(&out).unwrap();
    assert_eq!(back.mode, ColorMode::Rgb);
    assert_eq!(back.source.mode(), None);
    assert_eq!(back.composite, doc.composite);
}

#[test]
fn edited_indexed_layer_falls_back_to_rgb() {
    // One edited layer flips the document-wide header mode to RGB.
    let palette = indexed_palette();
    let p = layered_psd_with_data(8, 2, 1, &palette, &[&[2]], &[(0, &[5]), (-1, &[200])]);
    let mut doc = read_psd(&p).unwrap();
    doc.layers[0].channels_mut()[0].data[0] ^= 1;

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        3,
        "an edited layer writes the whole document as RGB"
    );
    let back = read_psd(&out).unwrap();
    assert_eq!(back.mode, ColorMode::Rgb);
    assert_eq!(back.source.mode(), None);
    assert_eq!(
        back.layers[0].channels()[0].data,
        doc.layers[0].channels()[0].data,
        "the edited layer round-trips in RGB"
    );
}

#[test]
fn indexed_layer_with_raster_mask_round_trips_as_indexed() {
    // A layer's raster mask (channel -2) is not palette-encoded, so it must ride
    // through an Indexed save unchanged alongside the re-emitted index channel.
    let palette = indexed_palette();
    let p = layered_psd_with_data(8, 2, 1, &palette, &[&[2]], &[(0, &[5]), (-2, &[222])]);
    let doc = read_psd(&p).unwrap();
    assert_eq!(doc.source.mode(), Some(ColorMode::Indexed));
    assert_eq!(
        doc.layers[0].mask.as_ref().and_then(|m| m.data.as_deref()),
        Some(&[222u8][..]),
        "the raster mask is read from channel -2"
    );

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        2,
        "output header color mode is Indexed"
    );

    let back = read_psd(&out).unwrap();
    assert_eq!(back.source.mode(), Some(ColorMode::Indexed));
    assert_eq!(
        back.layers[0].mask.as_ref().and_then(|m| m.data.as_deref()),
        Some(&[222u8][..]),
        "the mask bytes survive the Indexed save"
    );
    assert_eq!(
        back.layers[0].source_channels.as_ref().unwrap().planes,
        vec![(0, Samples::U8(vec![5].into()))],
        "the index channel is byte-identical"
    );
}

#[test]
fn indexed_document_without_a_merged_composite_writes_indexed() {
    // "Maximize Compatibility" off: the read retains the palette and each layer
    // index but no composite plane. The writer must still choose Indexed, skip
    // the composite requirement, and emit no image-data section.
    let palette = indexed_palette();
    let p = layered_psd_with_data(8, 2, 1, &palette, &[&[2]], &[(0, &[5])]);
    let mut doc = read_psd(&p).unwrap();
    doc.composite.set_merged_output(false);
    doc.source
        .replace_planes(None)
        .expect("valid source planes");

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[12..14].try_into().unwrap()),
        1,
        "one index channel"
    );
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        2,
        "output header color mode is Indexed"
    );
    assert_eq!(&out[30..30 + 768], &palette[..], "palette re-emitted");

    let back = read_psd(&out).unwrap();
    assert!(
        !back.composite.merged_output(),
        "no image-data section is written"
    );
    assert_eq!(back.source.mode(), Some(ColorMode::Indexed));
    assert_eq!(back.source.palette(), Some(palette));
    assert_eq!(
        back.layers[0].source_channels.as_ref().unwrap().planes,
        vec![(0, Samples::U8(vec![5].into()))],
        "the layer index channel is byte-identical"
    );
}

#[test]
fn added_empty_layer_keeps_indexed_on_save() {
    // An added layer with no RGB color channels (an empty/adjustment layer)
    // participates in nothing, so the document stays Indexed.
    let palette = indexed_palette();
    let p = layered_psd_with_data(8, 2, 1, &palette, &[&[2]], &[(0, &[5]), (-1, &[200])]);
    let mut doc = read_psd(&p).unwrap();
    {
        let _ = doc
            .layers
            .push(crate::layer_from_parts(pictura_core::LayerParts {
                name: "adjustment".into(),
                rect: PsdRect {
                    top: 0,
                    left: 0,
                    bottom: 1,
                    right: 1,
                },
                ..Default::default()
            }));
    };

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        2,
        "an added empty layer keeps the document Indexed"
    );

    let back = read_psd(&out).unwrap();
    assert_eq!(back.source.mode(), Some(ColorMode::Indexed));
    assert_eq!(back.layers.len(), 2, "both layers survive the save");
    assert_eq!(
        back.layers[0].source_channels.as_ref().unwrap().planes,
        vec![(0, Samples::U8(vec![5].into()))],
        "the original index channel is byte-identical"
    );
}
