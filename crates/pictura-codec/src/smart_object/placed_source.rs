use pictura_core::warp::{Warp, WarpStyle};
use pictura_core::{Homography, PsdRect, VectorTransform};

use super::{get_object_item, read_layer_data_descriptor, DescValue};

pub struct PlacedSource {
    pub width: f64,
    pub height: f64,
    pub page: u32,
    pub page_count: u32,
    pub crop: Option<u8>,
    pub transform: Option<VectorTransform>,
}

pub fn read_placed_source(data: &[u8]) -> Option<PlacedSource> {
    let root = read_layer_data_descriptor(data).ok()?;
    let items = object(&root)?;
    let size = object(get_object_item(items, b"Sz  ")?)?;
    let width = number(get_object_item(size, b"Wdth")?)?;
    let height = number(get_object_item(size, b"Hght")?)?;
    if width <= 0.0 || height <= 0.0 || width > f64::from(i32::MAX) || height > f64::from(i32::MAX)
    {
        return None;
    }
    let page = match get_object_item(items, b"PgNm") {
        Some(DescValue::Long(page)) if *page > 0 => *page as u32,
        _ => return None,
    };
    let crop = get_object_item(items, b"Crop").and_then(crop_index);
    let page_count = match get_object_item(items, b"totalPages") {
        Some(DescValue::Long(count)) if *count >= page as i32 => *count as u32,
        _ => page,
    };
    let transform = match get_object_item(items, b"nonAffineTransform")
        .or_else(|| get_object_item(items, b"Trnf"))
    {
        Some(value) => {
            let DescValue::List(values) = value else {
                return None;
            };
            if values.len() != 8 {
                return None;
            }
            let mut quad = [(0.0, 0.0); 4];
            for (point, pair) in quad.iter_mut().zip(values.chunks_exact(2)) {
                *point = (number(&pair[0])?, number(&pair[1])?);
            }
            Homography::from_quads(
                [(0.0, 0.0), (width, 0.0), (width, height), (0.0, height)],
                quad,
            )?;
            let mut source = PsdRect {
                left: 0,
                top: 0,
                right: width.round().max(1.0) as i32,
                bottom: height.round().max(1.0) as i32,
            };
            let warp = if let Some(value) = get_object_item(items, b"warp") {
                let warp = read_warp(object(value)?, &mut source)?;
                (!warp.is_identity()).then_some(warp)
            } else {
                None
            };
            Some(VectorTransform {
                source,
                destination: None,
                quad,
                warp,
            })
        }
        None => None,
    };
    Some(PlacedSource {
        width,
        height,
        page,
        page_count,
        crop,
        transform,
    })
}

fn object(value: &DescValue) -> Option<&[(Vec<u8>, DescValue)]> {
    match value {
        DescValue::Object { items, .. } => Some(items),
        _ => None,
    }
}

fn number(value: &DescValue) -> Option<f64> {
    let value = match value {
        DescValue::Long(value) => f64::from(*value),
        DescValue::Double(value) | DescValue::UnitFloat { value, .. } => *value,
        _ => return None,
    };
    value.is_finite().then_some(value)
}

fn option_value(value: &DescValue) -> Option<&[u8]> {
    match value {
        DescValue::Enum { value, .. } => Some(value),
        DescValue::Text(value) => Some(value.trim_end_matches('\0').as_bytes()),
        _ => None,
    }
}

fn crop_index(value: &DescValue) -> Option<u8> {
    if let DescValue::Long(crop @ 0..=5) = value {
        return Some(*crop as u8);
    }
    match option_value(value)? {
        b"boundingBox" => Some(0),
        b"mediaBox" => Some(1),
        b"cropBox" => Some(2),
        b"bleedBox" => Some(3),
        b"trimBox" => Some(4),
        b"artBox" => Some(5),
        _ => None,
    }
}

fn read_warp(items: &[(Vec<u8>, DescValue)], source: &mut PsdRect) -> Option<Warp> {
    let style = option_value(get_object_item(items, b"warpStyle")?)?;
    let style = WarpStyle::all().find(|candidate| candidate.psd_name().as_bytes() == style)?;
    if style == WarpStyle::None {
        return Some(Warp::none([
            f64::from(source.left),
            f64::from(source.top),
            f64::from(source.right),
            f64::from(source.bottom),
        ]));
    }
    if style == WarpStyle::Custom {
        return None;
    }
    let bounds = object(get_object_item(items, b"bounds")?)?;
    let mut coords = [0.0; 4];
    for (coordinate, key) in coords.iter_mut().zip([b"Left", b"Top ", b"Rght", b"Btom"]) {
        *coordinate = number(get_object_item(bounds, key)?)?;
        if coordinate.abs() >= f64::from(i32::MAX) {
            return None;
        }
    }
    if coords[0] >= coords[2] || coords[1] >= coords[3] {
        return None;
    }
    *source = PsdRect {
        left: coords[0].floor() as i32,
        top: coords[1].floor() as i32,
        right: coords[2].ceil() as i32,
        bottom: coords[3].ceil() as i32,
    };
    Some(Warp {
        style,
        bend: number(get_object_item(items, b"warpValue")?)?,
        h_distort: number(get_object_item(items, b"warpPerspective")?)?,
        v_distort: number(get_object_item(items, b"warpPerspectiveOther")?)?,
        vertical: option_value(get_object_item(items, b"warpRotate")?)? == b"Vrtc",
        bounds: coords,
        mesh: None,
    })
}
