//! Type-setting <-> engine-spec conversion: the placement, character, and
//! paragraph controls as a `TypeSpec`, and the stored path geometry.

use super::attrs::{character_attrs, paragraph_attrs};
use super::ffi::{CharacterSetting, ParagraphSetting, TypeSetting};
use cxx_qt_lib::QString;
use pictura_core::path::PathPoint;
use pictura_core::{TextPath, TextPathKind, TypeSpec};

pub(super) fn spec(
    text: &QString,
    font: &QString,
    placement: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> TypeSpec {
    let mut spec = TypeSpec::new(text.to_string(), font.to_string(), character.size);
    spec.vertical = placement.vertical;
    spec.origin = (placement.x, placement.y);
    spec.matrix = [placement.xx, placement.xy, placement.yx, placement.yy];
    spec.box_bounds = placement.boxed.then_some([
        placement.box_left,
        placement.box_top,
        placement.box_right,
        placement.box_bottom,
    ]);
    if let Some(path) = path_from_setting(placement) {
        if path.kind == TextPathKind::Area {
            let flattened = path.subpath.flattened(0.25);
            if let Some(&(first_x, first_y)) = flattened.points.first() {
                let (mut left, mut top, mut right, mut bottom) =
                    (first_x, first_y, first_x, first_y);
                for &(x, y) in &flattened.points {
                    left = left.min(x);
                    top = top.min(y);
                    right = right.max(x);
                    bottom = bottom.max(y);
                }
                spec.origin = (left, top);
                spec.box_bounds = Some([0.0, 0.0, right - left, bottom - top]);
            }
        } else {
            let flattened = path.subpath.flattened(0.25);
            if let Some((point, _)) = flattened.point_at(path.start) {
                spec.origin = point;
            }
        }
        spec.path = Some(path);
    }
    spec.character = character_attrs(character);
    spec.character.font_family = font.to_string();
    spec.paragraph = paragraph_attrs(paragraph);
    spec.warp = super::warp::text_warp(placement);
    spec
}

pub(super) fn path_from_setting(setting: &TypeSetting) -> Option<TextPath> {
    let kind = match setting.path_kind {
        1 => TextPathKind::OnPath,
        2 => TextPathKind::Area,
        _ => return None,
    };
    let segments: Vec<[(f64, f64); 4]> = setting
        .path_points
        .to_string()
        .split(';')
        .filter(|part| !part.trim().is_empty())
        .filter_map(|part| {
            let values: Vec<f64> = part
                .split_whitespace()
                .filter_map(|value| value.parse().ok())
                .collect();
            (values.len() == 8).then(|| {
                [
                    (values[0], values[1]),
                    (values[2], values[3]),
                    (values[4], values[5]),
                    (values[6], values[7]),
                ]
            })
        })
        .collect();
    if segments.is_empty() {
        return None;
    }
    let mut points = Vec::with_capacity(segments.len() + usize::from(!setting.path_closed));
    for (index, segment) in segments.iter().enumerate() {
        let in_handle = if index == 0 {
            setting
                .path_closed
                .then(|| segments.last().map(|last| last[2]))
                .flatten()
        } else {
            Some(segments[index - 1][2])
        };
        points.push(PathPoint {
            anchor: segment[0],
            in_handle,
            out_handle: Some(segment[1]),
            smooth: true,
        });
    }
    if !setting.path_closed {
        if let Some(last) = segments.last() {
            points.push(PathPoint {
                anchor: last[3],
                in_handle: Some(last[2]),
                out_handle: None,
                smooth: false,
            });
        }
    }
    Some(TextPath {
        kind,
        subpath: pictura_core::path::Subpath {
            points,
            closed: setting.path_closed,
        },
        start: setting.path_start,
    })
}

pub(super) fn path_points(path: &TextPath) -> QString {
    let mut parts = Vec::new();
    for index in 0..path.subpath.segment_count() {
        let Some(segment) = path.subpath.segment(index) else {
            continue;
        };
        let values: Vec<String> = segment
            .iter()
            .flat_map(|(x, y)| [format!("{x}"), format!("{y}")])
            .collect();
        parts.push(values.join(" "));
    }
    QString::from(parts.join(";").as_str())
}

pub(super) fn placement_setting(spec: &TypeSpec) -> TypeSetting {
    let b = spec.box_bounds.unwrap_or_default();
    TypeSetting {
        vertical: spec.vertical,
        x: spec.origin.0,
        y: spec.origin.1,
        xx: spec.matrix[0],
        xy: spec.matrix[1],
        yx: spec.matrix[2],
        yy: spec.matrix[3],
        boxed: spec.box_bounds.is_some(),
        box_left: b[0],
        box_top: b[1],
        box_right: b[2],
        box_bottom: b[3],
        path_kind: spec.path.as_ref().map_or(0, |path| match path.kind {
            TextPathKind::OnPath => 1,
            TextPathKind::Area => 2,
        }),
        path_closed: spec.path.as_ref().is_some_and(|path| path.subpath.closed),
        path_start: spec.path.as_ref().map_or(0.0, |path| path.start),
        path_points: spec
            .path
            .as_ref()
            .map_or_else(QString::default, path_points),
        warp_style: QString::from(if spec.warp.style.is_preset() {
            spec.warp.style.id()
        } else {
            ""
        }),
        warp_bend: spec.warp.bend,
        warp_h: spec.warp.h_distort,
        warp_v: spec.warp.v_distort,
        warp_vertical: spec.warp.vertical,
    }
}
