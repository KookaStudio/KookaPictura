//! The working RGB preview of a flat Multichannel plate document.

use pictura_core::{Document, PixelBuffer, Plane};

use crate::image_resources::{display_info, ChannelDisplayInfo, SPOT_CHANNEL_MODE};

/// Complete missing display entries once on import, so channel deletion and
/// subsequent saves keep each plate's fallback identity and colour.
pub(crate) fn complete_plate_display(doc: &mut Document) {
    let mut display = display_info(doc);
    if display.len() >= doc.channels.len() {
        return;
    }
    let names = crate::alpha_names(doc);
    for index in display.len()..doc.channels.len() {
        let color: [u8; 3] = match names
            .get(index)
            .map(|name| name.to_ascii_lowercase())
            .as_deref()
        {
            Some("cyan") => [0, 255, 255],
            Some("magenta") => [255, 0, 255],
            Some("yellow") => [255, 255, 0],
            _ => [0, 0, 0],
        };
        display.push(ChannelDisplayInfo {
            color_space: 0,
            components: [color[0], color[1], color[2], 0].map(|v| u16::from(v) * 257),
            opacity: 100,
            mode: SPOT_CHANNEL_MODE,
        });
    }
    crate::set_display_info(doc, &display);
}

/// Preview the named spot plates over white. The stored channel samples and
/// DisplayInfo remain authoritative for output.
pub fn multichannel_preview(doc: &Document) -> PixelBuffer {
    let width = doc.width as usize;
    let height = doc.height as usize;
    let pixels = width * height;
    let display = display_info(doc);
    let inks: Vec<_> = doc
        .channels
        .iter()
        .zip(&display)
        .filter(|(channel, info)| info.mode == SPOT_CHANNEL_MODE && channel.data.len() == pixels)
        .map(|(channel, info)| {
            (
                &channel.data,
                preview_rgb(info),
                u32::from(info.opacity.min(100)),
            )
        })
        .collect();
    let mut preview = Plane::filled(width, height, 3, 255u8);
    let mut shown = [vec![255u8; width], vec![255u8; width], vec![255u8; width]];
    let mut samples = vec![0u8; width];
    for y in 0..height {
        for row in &mut shown {
            row.fill(255);
        }
        for (plane, color, opacity) in &inks {
            plane.read_into(y * width, &mut samples);
            for (row, color) in shown.iter_mut().zip(color) {
                let absorb = (255 - u32::from(*color)) * opacity;
                for (out, &sample) in row.iter_mut().zip(&samples) {
                    let transmittance =
                        255 - ((255 - u32::from(sample)) * absorb + 12_750) / 25_500;
                    *out = (u32::from(*out) * transmittance / 255) as u8;
                }
            }
        }
        for (channel, row) in shown.iter().enumerate() {
            let at = channel * height + y;
            preview.write_rect_from(0..width, at..at + 1, width, row);
        }
    }
    preview.settle();
    PixelBuffer {
        width: doc.width,
        height: doc.height,
        channels: 3,
        data: preview,
    }
}

fn preview_rgb(info: &ChannelDisplayInfo) -> [u8; 3] {
    let [a, b, c, d] = info.components.map(|component| (component >> 8) as u8);
    match info.color_space {
        0 => [a, b, c],
        2 => [a, b, c].map(|component| (u16::from(component) * u16::from(d) / 255) as u8),
        // ponytail: unfamiliar ink colour spaces preview as black.
        _ => [0; 3],
    }
}
