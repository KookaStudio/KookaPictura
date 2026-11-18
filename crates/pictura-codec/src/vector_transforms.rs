use pictura_core::warp::{BezierMesh, Warp, WarpStyle};
use pictura_core::{LayerParts, PsdRect, VectorTransform};

use crate::common::Reader;

pub(crate) const KEY: [u8; 4] = *b"PiVt";
const MAX_STEPS: usize = 256;
const MAX_KNOTS: usize = 128;

fn push_floats(out: &mut Vec<u8>, values: impl IntoIterator<Item = f64>) {
    for value in values {
        out.extend_from_slice(&value.to_be_bytes());
    }
}

fn push_rect(out: &mut Vec<u8>, rect: PsdRect) {
    for value in [rect.top, rect.left, rect.bottom, rect.right] {
        out.extend_from_slice(&value.to_be_bytes());
    }
}

pub(crate) fn encode(transforms: &[VectorTransform]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&2u32.to_be_bytes());
    out.extend_from_slice(&(transforms.len() as u32).to_be_bytes());
    for step in transforms {
        push_rect(&mut out, step.source);
        out.push(u8::from(step.destination.is_some()));
        if let Some(destination) = step.destination {
            push_rect(&mut out, destination);
        }
        push_floats(&mut out, step.quad.into_iter().flat_map(|(x, y)| [x, y]));
        let Some(warp) = &step.warp else {
            out.push(0);
            continue;
        };
        let style = WarpStyle::all()
            .position(|style| style == warp.style)
            .unwrap_or(0);
        out.push(style as u8 + 1);
        out.push(u8::from(warp.vertical));
        push_floats(&mut out, [warp.bend, warp.h_distort, warp.v_distort]);
        push_floats(&mut out, warp.bounds);
        if let Some(mesh) = &warp.mesh {
            for knots in [&mesh.us, &mesh.vs] {
                out.extend_from_slice(&(knots.len() as u32).to_be_bytes());
                push_floats(&mut out, knots.iter().copied());
            }
            push_floats(&mut out, mesh.points.iter().flatten().copied());
        } else {
            out.extend_from_slice(&0u32.to_be_bytes());
        }
    }
    out
}

fn finite(reader: &mut Reader<'_>) -> Option<f64> {
    let value = f64::from_bits(reader.u64().ok()?);
    value.is_finite().then_some(value)
}

fn knots(reader: &mut Reader<'_>, count: usize) -> Option<Vec<f64>> {
    if !(2..=MAX_KNOTS).contains(&count) {
        return None;
    }
    let values: Vec<f64> = (0..count).map(|_| finite(reader)).collect::<Option<_>>()?;
    (values[0] == 0.0 && values[count - 1] == 1.0 && values.windows(2).all(|w| w[0] < w[1]))
        .then_some(values)
}

fn read_rect(reader: &mut Reader<'_>) -> Option<PsdRect> {
    let rect = PsdRect {
        top: reader.u32().ok()? as i32,
        left: reader.u32().ok()? as i32,
        bottom: reader.u32().ok()? as i32,
        right: reader.u32().ok()? as i32,
    };
    (rect.right.checked_sub(rect.left)? > 0 && rect.bottom.checked_sub(rect.top)? > 0)
        .then_some(rect)
}

fn step(reader: &mut Reader<'_>, has_destination: bool) -> Option<VectorTransform> {
    let source = read_rect(reader)?;
    let destination = if has_destination {
        match reader.u8().ok()? {
            0 => None,
            1 => Some(read_rect(reader)?),
            _ => return None,
        }
    } else {
        None
    };
    let mut quad = [(0.0, 0.0); 4];
    for point in &mut quad {
        *point = (finite(reader)?, finite(reader)?);
    }
    let style = reader.u8().ok()?;
    let warp = if style == 0 {
        None
    } else {
        let style = WarpStyle::all().nth(usize::from(style - 1))?;
        let vertical = reader.u8().ok()? != 0;
        let (bend, h_distort, v_distort) = (finite(reader)?, finite(reader)?, finite(reader)?);
        let mut bounds = [0.0; 4];
        for value in &mut bounds {
            *value = finite(reader)?;
        }
        let count = reader.u32().ok()? as usize;
        let mesh = if count == 0 {
            None
        } else {
            let us = knots(reader, count)?;
            let count = reader.u32().ok()? as usize;
            let vs = knots(reader, count)?;
            let points = (0..(3 * us.len() - 2) * (3 * vs.len() - 2))
                .map(|_| Some([finite(reader)?, finite(reader)?]))
                .collect::<Option<_>>()?;
            Some(BezierMesh { us, vs, points })
        };
        Some(Warp {
            style,
            bend,
            h_distort,
            v_distort,
            vertical,
            bounds,
            mesh,
        })
    };
    Some(VectorTransform {
        source,
        destination,
        quad,
        warp,
    })
}

fn decode(data: &[u8]) -> Option<Vec<VectorTransform>> {
    let mut reader = Reader::new(data);
    let has_destination = match reader.u32().ok()? {
        1 => false,
        2 => true,
        _ => return None,
    };
    let count = reader.u32().ok()? as usize;
    if count > MAX_STEPS {
        return None;
    }
    (0..count)
        .map(|_| step(&mut reader, has_destination))
        .collect()
}

pub(crate) fn take_marker(layer: &mut LayerParts) -> Option<Vec<VectorTransform>> {
    let (index, transforms) =
        layer
            .extra_blocks
            .iter()
            .enumerate()
            .find_map(|(index, block)| {
                if block.key == KEY {
                    decode(&block.data).map(|transforms| (index, transforms))
                } else {
                    None
                }
            })?;
    layer.extra_blocks.remove(index);
    Some(transforms)
}
