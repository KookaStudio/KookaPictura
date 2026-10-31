use crate::stroke::{layer_rgba, layer_rgba_serial};
use pictura_core::{Channel, Layer, PsdRect};

fn layer_with(rect: PsdRect, channels: &[(i16, u8)]) -> Layer {
    let n = (rect.width().max(0) * rect.height().max(0)) as usize;
    let width = rect.width().max(1) as usize;
    Layer {
        rect,
        channels: channels
            .iter()
            .map(|&(id, value)| {
                let mut channel = Channel {
                    id,
                    data: vec![value; n].into(),
                };
                channel.data.fracture(width);
                channel
            })
            .collect::<Vec<_>>(),
        ..Default::default()
    }
}

#[test]
fn layer_rgba_matches_serial_builder() {
    let full = layer_with(
        PsdRect {
            top: 0,
            left: 0,
            bottom: 6,
            right: 8,
        },
        &[(0, 10), (1, 20), (2, 30), (-1, 40)],
    );
    let offset = layer_with(
        PsdRect {
            top: 2,
            left: 3,
            bottom: 7,
            right: 8,
        },
        &[(0, 11), (1, 21), (2, 31), (-1, 41)],
    );
    let background = layer_with(
        PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 4,
        },
        &[(0, 12), (1, 22), (2, 32)],
    );
    let missing = layer_with(
        PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 4,
        },
        &[(0, 13), (1, 23)],
    );
    let zero = layer_with(
        PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        },
        &[(0, 14), (1, 24), (2, 34), (-1, 44)],
    );
    for layer in [&full, &offset, &background, &missing, &zero] {
        assert_eq!(layer_rgba(layer), layer_rgba_serial(layer));
    }
}
