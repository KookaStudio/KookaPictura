//! The Pictura-private `PiSt` layer block that carries a smart object's stack
//! mode. A recognized marker is modelled; an unknown version stays verbatim.

use pictura_core::StackMode;

pub(crate) const KEY: [u8; 4] = *b"PiSt";
const VERSION: u8 = 1;

/// The block payload for `mode`: the marker version and the stable mode code.
pub(crate) fn marker_data(mode: StackMode) -> [u8; 2] {
    [VERSION, mode.code()]
}

fn parse(data: &[u8]) -> Option<StackMode> {
    if data.first().copied()? != VERSION {
        return None;
    }
    StackMode::from_code(*data.get(1)?)
}

/// Take the first recognized marker from `layer` and remove it from the
/// unmodelled blocks; an unrecognized version or code stays in place.
#[cfg(test)]
pub(crate) fn take_stack_marker(layer: &mut pictura_core::Layer) -> Option<StackMode> {
    crate::edit_layer(layer, take_stack_marker_from_parts)
}

/// Read a candidate's marker without consuming losing preservation data.
pub(crate) fn stack_marker_from_parts(parts: &pictura_core::LayerParts) -> Option<StackMode> {
    parts
        .extra_blocks
        .iter()
        .filter(|block| block.key == KEY)
        .find_map(|block| parse(&block.data))
}

pub(crate) fn take_stack_marker_from_parts(
    parts: &mut pictura_core::LayerParts,
) -> Option<StackMode> {
    let index = parts
        .extra_blocks
        .iter()
        .position(|block| block.key == KEY && parse(&block.data).is_some())?;
    let mode = parse(&parts.extra_blocks[index].data).expect("checked recognized");
    parts.extra_blocks.remove(index);
    Some(mode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{
        BitDepth, Channel, ColorMode, Document, Layer, LayerBlock, LayerStack, PsdRect,
        SmartObject, SmartObjectKind,
    };

    use crate::{read_psd, write_psd};

    fn rect() -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 4,
        }
    }

    fn proxy() -> Vec<Channel> {
        [0i16, 1, 2, -1]
            .map(|id| Channel {
                id,
                data: vec![90u8; 16].into(),
            })
            .to_vec()
    }

    fn stack_layer(mode: Option<StackMode>, blocks: Vec<LayerBlock>) -> Layer {
        let mut so = SmartObject::default();
        so.kind = SmartObjectKind::Embedded;
        so.filename = "source.psd".into();
        so.filetype = *b"8BPB";
        so.creator = *b"8BIM";
        so.set_payload(vec![0xAA, 0xBB, 0xCC, 0xDD]);
        so.stack_mode = mode;
        crate::layer_from_parts(pictura_core::LayerParts {
            name: "stack".into(),
            rect: rect(),
            channels: proxy(),
            smart_object: Some(so),
            extra_blocks: blocks,
            ..Default::default()
        })
    }

    fn host(layer: Layer) -> Document {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(vec![layer]);
        doc
    }

    fn marker_count(bytes: &[u8]) -> usize {
        bytes
            .windows(4)
            .filter(|window| *window == KEY.as_slice())
            .count()
    }

    #[test]
    fn stack_mode_marker_round_trips() {
        let bytes = write_psd(&host(stack_layer(Some(StackMode::Median), Vec::new())))
            .expect("host writes");
        let back = read_psd(&bytes).expect("host reads");
        let layer = &back.layers[0];
        let so = layer.smart_object().expect("resolved");
        assert_eq!(so.stack_mode, Some(StackMode::Median));
        assert_eq!(so.payload(), Some([0xAA, 0xBB, 0xCC, 0xDD].as_slice()));
        let descriptor = so.config_descriptor.clone();
        assert!(!descriptor.is_empty());

        let again = write_psd(&back).expect("second write");
        let reread = read_psd(&again).expect("second read");
        let so = reread.layers[0].smart_object().expect("resolved again");
        assert_eq!(so.stack_mode, Some(StackMode::Median));
        assert_eq!(so.config_descriptor, descriptor);
    }

    #[test]
    fn stack_mode_marker_clears() {
        let mut back = read_psd(
            &write_psd(&host(stack_layer(Some(StackMode::Mean), Vec::new()))).expect("writes"),
        )
        .expect("reads");
        crate::edit_layer(&mut back.layers[0], |parts| {
            parts.smart_object.as_mut().expect("resolved").stack_mode = None;
        });
        let bytes = write_psd(&back).expect("cleared write");
        let reread = read_psd(&bytes).expect("cleared read");
        assert_eq!(
            reread.layers[0]
                .smart_object()
                .expect("resolved")
                .stack_mode,
            None
        );
        assert!(reread.layers[0]
            .extra_blocks()
            .iter()
            .all(|block| block.key != KEY));
        assert_eq!(marker_count(&bytes), 0);
    }

    #[test]
    fn unknown_stack_mode_marker_is_preserved() {
        let unknown = LayerBlock {
            key: KEY,
            data: vec![9, 4].into(),
        };
        let bytes = write_psd(&host(stack_layer(None, vec![unknown]))).expect("host writes");
        let back = read_psd(&bytes).expect("host reads");
        assert_eq!(
            back.layers[0].smart_object().expect("resolved").stack_mode,
            None
        );
        let block = back.layers[0]
            .extra_blocks()
            .iter()
            .find(|block| block.key == KEY)
            .expect("the unknown marker survives");
        assert_eq!(block.data.as_slice(), [9, 4]);
    }

    #[test]
    fn unknown_stack_mode_marker_is_replaced_by_a_modelled_mode() {
        let unknown = LayerBlock {
            key: KEY,
            data: vec![9, 4].into(),
        };
        let bytes = write_psd(&host(stack_layer(Some(StackMode::Maximum), vec![unknown])))
            .expect("host writes");
        assert_eq!(marker_count(&bytes), 1, "one authored marker");
        let back = read_psd(&bytes).expect("host reads");
        assert_eq!(
            back.layers[0].smart_object().expect("resolved").stack_mode,
            Some(StackMode::Maximum)
        );
        assert!(back.layers[0]
            .extra_blocks()
            .iter()
            .all(|block| block.key != KEY));
    }

    #[test]
    fn duplicate_unknown_stack_mode_markers_collapse_on_write() {
        let unknown_a = LayerBlock {
            key: KEY,
            data: vec![9, 4].into(),
        };
        let unknown_b = LayerBlock {
            key: KEY,
            data: vec![9, 5].into(),
        };
        let bytes =
            write_psd(&host(stack_layer(None, vec![unknown_a, unknown_b]))).expect("host writes");
        assert_eq!(marker_count(&bytes), 1, "at most one preserved marker");
    }

    #[test]
    fn take_stack_marker_scans_past_an_unknown_marker() {
        let unknown = LayerBlock {
            key: KEY,
            data: vec![9, 4].into(),
        };
        let recognized = LayerBlock {
            key: KEY,
            data: marker_data(StackMode::Kurtosis).to_vec().into(),
        };
        let base = stack_layer(None, Vec::new());
        let content = base.content().clone();
        let mut parts = pictura_core::LayerParts::from(base);
        parts.extra_blocks = vec![unknown, recognized];
        let mut layer = Layer::from_parts(parts, content);
        assert_eq!(take_stack_marker(&mut layer), Some(StackMode::Kurtosis));
        assert_eq!(layer.extra_blocks().len(), 1);
        assert_eq!(layer.extra_blocks()[0].data.as_slice(), [9, 4]);
    }

    #[test]
    fn stack_mode_leaves_the_proxy_untouched() {
        let channels = proxy();
        let bytes = write_psd(&host(stack_layer(Some(StackMode::Entropy), Vec::new())))
            .expect("host writes");
        let back = read_psd(&bytes).expect("host reads");
        for channel in &channels {
            let got = back.layers[0]
                .channels()
                .iter()
                .find(|candidate| candidate.id == channel.id)
                .expect("channel survives");
            assert_eq!(got.data.as_slice(), channel.data.as_slice());
        }
    }
    #[test]
    fn losing_smart_object_keeps_original_stack_markers() {
        use pictura_core::{LayerKind, LayerParts};
        let original =
            read_psd(&write_psd(&host(stack_layer(Some(StackMode::Median), Vec::new()))).unwrap())
                .unwrap();
        for kind in [LayerKind::Fill, LayerKind::Adjustment, LayerKind::Group] {
            let mut parts = LayerParts::from(original.layers[0].clone());
            parts.extra_blocks.extend([
                LayerBlock {
                    key: *b"zzzz",
                    data: vec![4, 5, 6, 0].into(),
                },
                LayerBlock {
                    key: KEY,
                    data: vec![1, StackMode::Median.code(), 37, 112].into(),
                },
                LayerBlock {
                    key: KEY,
                    data: vec![99, 12].into(),
                },
                LayerBlock {
                    key: *b"yyyy",
                    data: vec![7, 8, 9, 0].into(),
                },
            ]);
            match kind {
                LayerKind::Fill => {
                    parts.adjustment = Some(crate::encode_solid_color_fill([10, 20, 30]))
                }
                LayerKind::Adjustment => parts.adjustment = Some(crate::encode_invert()),
                LayerKind::Group => parts.is_group = true,
                _ => unreachable!(),
            }
            let expected = parts.extra_blocks.clone();
            let mut doc = original.clone();
            doc.layers = LayerStack::from_layers(vec![crate::layer_from_parts(parts)]);
            for bytes in [write_psd(&doc).unwrap(), crate::write_psb(&doc).unwrap()] {
                let back = read_psd(&bytes).unwrap();
                assert_eq!(back.layers[0].kind(), kind);
                assert_eq!(back.layers[0].extra_blocks(), &expected);
                let again = if back.is_psb {
                    crate::write_psb(&back)
                } else {
                    write_psd(&back)
                }
                .unwrap();
                let second = read_psd(&again).unwrap();
                assert_eq!(second.layers[0].extra_blocks(), &expected);
                assert_eq!(second.layer_section_extra, back.layer_section_extra);
            }
        }
    }
}
