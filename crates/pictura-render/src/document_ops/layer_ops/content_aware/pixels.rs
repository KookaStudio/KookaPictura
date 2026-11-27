use super::seams::buffer;
use pictura_core::{Channel, Plane, Samples};

pub(super) fn u8_values(plane: &Plane<u8>, width: usize, height: usize) -> Option<Vec<f32>> {
    let len = width.checked_mul(height)?;
    if plane.len() != len { return None; }
    let mut bytes = buffer(len, 0u8)?;
    plane.read_rect_into(0..width, 0..height, width, &mut bytes);
    let mut out = buffer(len, 0.0f32)?;
    for (to, from) in out.iter_mut().zip(bytes) { *to = f32::from(from) / 255.0; }
    Some(out)
}

pub(super) fn native_values(samples: &Samples, width: usize, height: usize) -> Option<Vec<f32>> {
    let len = width.checked_mul(height)?;
    if samples.len() != len { return None; }
    match samples {
        Samples::U8(plane) => u8_values(plane, width, height),
        Samples::U16(plane) => {
            let mut src = buffer(len, 0u16)?;
            plane.read_rect_into(0..width, 0..height, width, &mut src);
            let mut out = buffer(len, 0.0f32)?;
            for (to, from) in out.iter_mut().zip(src) { *to = f32::from(from) / 65535.0; }
            Some(out)
        }
        Samples::F32(plane) => {
            let mut out = buffer(len, 0.0f32)?;
            plane.read_rect_into(0..width, 0..height, width, &mut out);
            Some(out)
        }
    }
}

fn blend(carved: &[f32], ordinary: &[f32], carved_alpha: Option<&[f32]>, ordinary_alpha: Option<&[f32]>, color: bool, amount: f32) -> Option<Vec<f32>> {
    if carved.len() != ordinary.len() { return None; }
    let mut output = buffer(carved.len(), 0.0f32)?;
    for i in 0..output.len() {
        let a = carved_alpha.map_or(1.0, |p| p[i]);
        let b = ordinary_alpha.map_or(1.0, |p| p[i]);
        output[i] = if color {
            let alpha = a * amount + b * (1.0 - amount);
            if alpha > 0.0 { (carved[i] * a * amount + ordinary[i] * b * (1.0 - amount)) / alpha } else { 0.0 }
        } else {
            carved[i] * amount + ordinary[i] * (1.0 - amount)
        };
    }
    Some(output)
}

pub(super) fn channel(id: i16, carved: &[f32], ordinary: &Channel, alpha: Option<(&[f32], &[f32])>, width: usize, height: usize, amount: f32) -> Option<Channel> {
    let ordinary = u8_values(&ordinary.data, width, height)?;
    let values = blend(carved, &ordinary, alpha.map(|a| a.0), alpha.map(|a| a.1), id >= 0, amount)?;
    let mut output = buffer(values.len(), 0u8)?;
    for (dst, value) in output.iter_mut().zip(values) { *dst = (value * 255.0).round().clamp(0.0, 255.0) as u8; }
    Some(Channel { id, data: output.into() })
}

pub(super) fn native(carved: &[f32], ordinary: &Samples, alpha: Option<(&[f32], &[f32])>, color: bool, width: usize, height: usize, amount: f32) -> Option<Samples> {
    let values = blend(carved, &native_values(ordinary, width, height)?, alpha.map(|a| a.0), alpha.map(|a| a.1), color, amount)?;
    Some(match ordinary {
        Samples::U8(_) => {
            let mut out = buffer(values.len(), 0u8)?;
            for (to, v) in out.iter_mut().zip(values) { *to = (v * 255.0).round().clamp(0.0, 255.0) as u8; }
            Samples::U8(out.into())
        }
        Samples::U16(_) => {
            let mut out = buffer(values.len(), 0u16)?;
            for (to, v) in out.iter_mut().zip(values) { *to = (v * 65535.0).round().clamp(0.0, 65535.0) as u16; }
            Samples::U16(out.into())
        }
        Samples::F32(_) => Samples::F32(values.into()),
    })
}

pub(super) fn widen(channel: &Channel, depth: pictura_core::BitDepth, width: usize, height: usize) -> Option<Samples> {
    let values = u8_values(&channel.data, width, height)?;
    Some(match depth {
        pictura_core::BitDepth::Sixteen => {
            let mut out = buffer(values.len(), 0u16)?;
            for (to, value) in out.iter_mut().zip(values) { *to = (value * 65535.0).round() as u16; }
            Samples::U16(out.into())
        }
        pictura_core::BitDepth::ThirtyTwo => Samples::F32(values.into()),
        _ => return None,
    })
}
