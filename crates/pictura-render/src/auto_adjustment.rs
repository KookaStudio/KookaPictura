use pictura_core::{AdjustmentData, AutoOptions, BitDepth, Document, LevelsChannel, PsdRect};



pub use pictura_adjust::AutoHistogram;

/// RGB samples from the layer input. The copy shares all the actual layer
/// storage. Only ancestors of the selected adjustment are structurally edited.
pub fn adjustment_auto_source(doc: &Document, path: &str) -> Option<AutoHistogram> {
    let indices: Vec<usize> = path.split('/').map(str::parse).collect::<Result<_, _>>().ok()?;
    let (last, parents) = indices.split_last()?;
    crate::resolve_path(doc, path)?;
    let mut input = doc.clone();
    for (depth, &index) in parents.iter().enumerate() {
        input.layers.truncate(&parents[..depth], index + 1);
    }
    input.layers.truncate(parents, *last);
    let native = matches!(input.source.depth(), Some(BitDepth::Sixteen | BitDepth::ThirtyTwo));
    let mut histogram = AutoHistogram::new(if native { u16::MAX } else { 255 });
    if !input.layers.is_empty() {
        let rect = PsdRect {
            top: 0,
            left: 0,
            bottom: input.height as i32,
            right: input.width as i32,
        };
        let rows = (262_144 / input.width.max(1)).clamp(1, pictura_core::TILE as u32);
        if native {
            let plan = crate::composite_plan::CompositePlan::build(&input, rect, None);
            for top in (0..input.height).step_by(rows as usize) {
                let height = rows.min(input.height - top);
                let mut canvas = crate::composite::Canvas::new_region(
                    0, top as i32, input.width as usize, height as usize,
                );
                canvas.clip = rect;
                crate::composite::composite_layers(&mut canvas, &input, &plan);
                let r: Vec<f32> = canvas.px.iter().map(|p| p.r).collect();
                let g: Vec<f32> = canvas.px.iter().map(|p| p.g).collect();
                let b: Vec<f32> = canvas.px.iter().map(|p| p.b).collect();
                histogram.add_planes([&r, &g, &b], |i| canvas.px[i].a > 0.0);
            }
        } else {
            crate::stream_composite(&input, rect, rows, |_, band| histogram.add_planar(&band));
        }
    }
    Some(histogram)
}

/// Encode the fitted parameters in the adjustment's existing layer format.
pub fn auto_adjustment_block(
    opening: &AdjustmentData,
    histogram: &AutoHistogram,
    options: &AutoOptions,
) -> Option<AdjustmentData> {
    let fit = histogram.fit(options);
    if fit.iter().all(Option::is_none) {
        return None;
    }
    const IDENTITY: LevelsChannel = LevelsChannel {
        input_black: 0,
        input_white: 255,
        output_black: 0,
        output_white: 255,
        gamma: 1.0,
    };
    match &opening.key {
        b"levl" => {
            let mut data = opening.data.to_vec();
            // Some readers accept a block that has only its composite record.
            for offset in (data.len().saturating_sub(2) / 10)..4 {
                data.truncate(2 + offset * 10);
                data.resize(2 + (offset + 1) * 10, 0);
            }
            let records = [IDENTITY, fit[0].map_or(IDENTITY, |f| f.levels()),
                fit[1].map_or(IDENTITY, |f| f.levels()), fit[2].map_or(IDENTITY, |f| f.levels())];
            for (index, record) in records.into_iter().enumerate() {
                let values = [u16::from(record.input_black), u16::from(record.input_white),
                    u16::from(record.output_black), u16::from(record.output_white),
                    (record.gamma * 100.0).round() as u16];
                for (offset, value) in values.into_iter().enumerate() {
                    let start = 2 + index * 10 + offset * 2;
                    data[start..start + 2].copy_from_slice(&value.to_be_bytes());
                }
            }
            Some(AdjustmentData { key: opening.key, data: data.into() })
        }
        b"curv" => {
            let [r, g, b] = fit.map(|transfer| transfer.map(|f| f.curve()));
            Some(crate::encode_curves(&[(0, 0), (255, 255)], r.as_deref(), g.as_deref(), b.as_deref()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, Channel, ColorMode, Layer, LayerContent, LayerStack, PixelBuffer};

    fn fixture(kind: crate::AdjustmentKind, width: u32, height: u32, gray: Vec<u8>) -> Document {
        let rect = PsdRect { top: 0, left: 0, bottom: height as i32, right: width as i32 };
        let mut base = Layer::new(LayerContent::Pixel);
        base.rect = rect;
        base.set_channels([0, 1, 2, -1].into_iter().map(|id| Channel {
            id,
            data: if id == -1 { vec![255; gray.len()].into() } else { gray.clone().into() },
        }).collect());
        let mut edit = Layer::new(LayerContent::Adjustment(None));
        edit.rect = rect;
        pictura_codec::edit_layer(&mut edit, |parts| {
            parts.adjustment = crate::layer_default(kind, crate::Defaults::rgb());
        });
        let mut doc = Document::new(width, height, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(vec![base, edit]);
        doc
    }

    #[test]
    fn configured_auto_streams_tiled_histogram() {
        let (width, height) = (1040u32, 400u32);
        let mut gray = vec![100; (width * height) as usize];
        gray[..(width * height / 2) as usize].fill(40);
        gray[7] = 5;
        let last = gray.len() - 8;
        gray[last] = 246;
        let doc = fixture(crate::AdjustmentKind::Levels, width, height, gray);
        let source = adjustment_auto_source(&doc, "1").unwrap();
        let mut options = AutoOptions { shadow_clip: 0.0, highlight_clip: 0.0, ..AutoOptions::default() };
        let range = |cfg: &AutoOptions| {
            let record = source.fit(cfg)[0].unwrap().levels();
            (record.input_black, record.input_white)
        };
        assert_eq!(range(&options), (5, 246));
        options.shadow_clip = 0.1;
        options.highlight_clip = 0.1;
        assert_eq!(range(&options), (40, 100));
    }

    #[test]
    fn configured_auto_fits_native_composite_precision() {
        use pictura_core::{Samples, SourceChannels};
        let mut doc = fixture(crate::AdjustmentKind::Levels, 4, 1, vec![4, 4, 5, 5]);
        doc.source.set_native(Some(BitDepth::Sixteen), doc.source.planes().clone()).unwrap();
        let base = &mut doc.layers[0];
        let values = vec![1200u16, 1223, 1277, 1300];
        base.source_channels = Some(SourceChannels::new(BitDepth::Sixteen, base.rect,
            (0..3).map(|id| (id, Samples::U16(values.clone().into()))).collect()));
        let histogram = adjustment_auto_source(&doc, "1").unwrap();
        let options = AutoOptions { shadow_clip: 0.0, highlight_clip: 0.0, ..AutoOptions::default() };
        let transfer = histogram.fit(&options)[0].expect("distinct native samples");
        assert!(transfer.map(1200.0 / 65535.0) < 0.001);
        assert!(transfer.map(1300.0 / 65535.0) > 0.999);
        // The PSD Levels record has byte-sized endpoints and cannot encode this range.
        assert_eq!(transfer.levels().input_black, 0);
        assert_eq!(transfer.levels().input_white, 255);
        assert_eq!(transfer.curve(), vec![(0, 0), (255, 255)]);
    }

    #[test]
    fn auto_adjustment_blocks_roundtrip() {
        for kind in [crate::AdjustmentKind::Levels, crate::AdjustmentKind::Curves] {
            let mut doc = fixture(kind, 4, 1, vec![40, 60, 110, 170]);
            let source = adjustment_auto_source(&doc, "1").unwrap();
            let block = auto_adjustment_block(doc.layers[1].adjustment().unwrap(), &source,
                &AutoOptions::default()).unwrap();
            pictura_codec::edit_layer(&mut doc.layers[1], |parts| parts.adjustment = Some(block.clone()));
            let before: PixelBuffer = crate::composite_rgba(&doc);
            assert_eq!((before.data[0], before.data[3]), (0, 255));
            for encoded in [pictura_codec::write_psd(&doc), pictura_codec::write_psb(&doc)] {
                let reopened = pictura_codec::read_psd(&encoded.unwrap()).unwrap();
                assert_eq!(reopened.layers[1].adjustment(), Some(&block));
                assert_eq!(crate::composite_rgba(&reopened).data.as_slice(), before.data.as_slice());
            }
        }
    }
}
