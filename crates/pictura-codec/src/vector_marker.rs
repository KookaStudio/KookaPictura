use pictura_core::{LayerParts, VectorPlacement};

pub(crate) const KEY: [u8; 4] = *b"PiVs";

pub(crate) fn marker_data(placement: VectorPlacement, authored: bool) -> [u8; 28] {
    let mut data = [0; 28];
    data[0] = 2;
    data[1] = placement.crop;
    data[2..6].copy_from_slice(&placement.page.to_be_bytes());
    data[6..10].copy_from_slice(&placement.page_count.to_be_bytes());
    data[10..18].copy_from_slice(&placement.width_points.to_be_bytes());
    data[18..26].copy_from_slice(&placement.height_points.to_be_bytes());
    data[26] = u8::from(authored);
    data
}

fn parse(data: &[u8]) -> Option<(VectorPlacement, bool)> {
    let authored = match (data.first()?, data.len()) {
        (1, 26) => false,
        (2, 28) if data[26] <= 1 && data[27] == 0 => data[26] == 1,
        _ => return None,
    };
    if data[1] > 5 {
        return None;
    }
    let page = u32::from_be_bytes(data[2..6].try_into().ok()?);
    let page_count = u32::from_be_bytes(data[6..10].try_into().ok()?);
    let width_points = f64::from_be_bytes(data[10..18].try_into().ok()?);
    let height_points = f64::from_be_bytes(data[18..26].try_into().ok()?);
    (page != 0
        && page <= page_count
        && width_points.is_finite()
        && height_points.is_finite()
        && width_points > 0.0
        && height_points > 0.0)
        .then_some((
            VectorPlacement {
                page,
                page_count,
                crop: data[1],
                width_points,
                height_points,
            },
            authored,
        ))
}

pub(crate) fn take_marker(layer: &mut LayerParts) -> Option<(VectorPlacement, bool)> {
    let (index, placement) = layer
        .extra_blocks
        .iter()
        .enumerate()
        .find_map(|(index, block)| {
            if block.key == KEY {
                parse(&block.data).map(|placement| (index, placement))
            } else {
                None
            }
        })?;
    layer.extra_blocks.remove(index);
    Some(placement)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_page_and_crop_round_trip() {
        let selection = VectorPlacement {
            page: 2,
            page_count: 3,
            crop: 4,
            width_points: 257.6,
            height_points: 120.0,
        };
        let mut bytes = marker_data(selection, true);
        assert_eq!(parse(&bytes), Some((selection, true)));
        bytes[0] = 1;
        assert_eq!(parse(&bytes[..26]), Some((selection, false)));
        assert_eq!(parse(&bytes), None);
    }
}
