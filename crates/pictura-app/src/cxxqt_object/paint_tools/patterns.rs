use super::{
    ffi::PaintTip,
    imp::{begin, stamp_config},
    PictureView,
};
use core::pin::Pin;
use cxx_qt_lib::QString;
use pictura_paint::{pattern, stamp::StampSource, Stroke};

pub(super) fn begin_pattern_stamp(
    view: Pin<&mut PictureView>,
    tip: &PaintTip,
    opacity: i32,
    flow: i32,
    mode: &QString,
    pattern: i32,
    origin_x: i32,
    origin_y: i32,
) -> bool {
    let Some(pattern) = usize::try_from(pattern)
        .ok()
        .and_then(pictura_render::builtin_pattern)
    else {
        return false;
    };
    let tile = pictura_paint::healing::RgbaImage {
        width: pattern.width as i32,
        height: pattern.height as i32,
        data: pattern.rgba.as_chunks::<4>().0.to_vec(),
    };
    let cfg = stamp_config(tip, opacity, flow, mode);
    begin(view, "Pattern Stamp", cfg, |doc, path, _| {
        let source = StampSource::pattern(tile, (origin_x, origin_y), doc.width, doc.height)?;
        Stroke::begin_source(doc, path, cfg, source).ok()
    })
}

pub(super) fn begin_pattern_preset(
    view: Pin<&mut PictureView>,
    tip: &PaintTip,
    opacity: i32,
    flow: i32,
    mode: &QString,
    file: &[u8],
    origin_x: i32,
    origin_y: i32,
) -> bool {
    use pictura_codec::preset_library::{decode, Format, Kind, Payload};
    let Ok(mut set) = decode(Kind::Patterns, Format::Native, file) else {
        return false;
    };
    let Some(Payload::Pattern(pattern)) = set.library.entries.pop().map(|row| row.payload) else {
        return false;
    };
    let tile = pictura_paint::healing::RgbaImage {
        width: pattern.width as i32,
        height: pattern.height as i32,
        data: pattern.rgba.as_chunks::<4>().0.to_vec(),
    };
    let cfg = stamp_config(tip, opacity, flow, mode);
    begin(view, "Pattern Stamp", cfg, |doc, path, _| {
        let source = StampSource::pattern(tile, (origin_x, origin_y), doc.width, doc.height)?;
        Stroke::begin_source(doc, path, cfg, source).ok()
    })
}

pub(super) fn stamp_pattern_count() -> i32 {
    pictura_paint::pattern::PATTERN_NAMES.len() as i32
}

pub(super) fn managed_pattern_count() -> i32 {
    pictura_render::layer_style_pattern_names().len() as i32
}

pub(super) fn stamp_pattern_name(index: i32) -> QString {
    usize::try_from(index)
        .ok()
        .and_then(|i| pictura_render::layer_style_pattern_names().get(i).copied())
        .map_or_else(QString::default, QString::from)
}

pub(super) fn stamp_pattern_id(index: i32) -> QString {
    if (0..managed_pattern_count()).contains(&index) {
        QString::from(pictura_render::builtin_pattern_id(index as usize).as_str())
    } else {
        QString::default()
    }
}

pub(super) fn stamp_pattern_tile(index: i32) -> Vec<u8> {
    let Some(tile) = usize::try_from(index)
        .ok()
        .and_then(pictura_render::builtin_pattern)
    else {
        return Vec::new();
    };
    let side = pattern::TILE as u32;
    let mut result = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            let at = ((y % tile.height * tile.width + x % tile.width) * 4) as usize;
            result.extend_from_slice(&tile.rgba[at..at + 4]);
        }
    }
    result
}

pub(super) fn stamp_pattern_side() -> i32 {
    pattern::TILE
}
