//! A rendered type raster as layer channels.

use pictura_core::{Channel, PixelBuffer, Plane};
use rayon::prelude::*;

/// `buffer`'s planes as layer channels, copying only the rows that ink: a
/// paragraph box can be document-sized around a few words.
pub(crate) fn type_channels(buffer: &PixelBuffer) -> Vec<Channel> {
    let plane = buffer.pixel_count();
    let width = (buffer.width as usize).max(1);
    let alpha = &buffer.data[3 * plane..];
    let inked = |row: &[u8]| row.iter().any(|&a| a != 0);
    let first = alpha.par_chunks(width).position_first(inked).unwrap_or(0);
    let last = alpha.par_chunks(width).position_last(inked).unwrap_or(0);
    let span = first * width..((last + 1) * width).min(plane);
    [0, 1, 2, -1]
        .into_iter()
        .enumerate()
        .map(|(i, id)| Channel {
            id,
            data: Plane::build(plane, |channel| {
                let source = &buffer.data[i * plane..(i + 1) * plane];
                channel[span.clone()].copy_from_slice(&source[span.clone()]);
            }),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_hold_every_inked_row() {
        let mut buffer = PixelBuffer::zeroed(4, 6, 4);
        let plane = buffer.pixel_count();
        for (i, value) in [(0, 40), (plane, 50), (2 * plane, 60), (3 * plane, 200)] {
            buffer.data[i + 2 * 4 + 1] = value;
            buffer.data[i + 4 * 4 + 3] = value;
        }
        let channels = type_channels(&buffer);
        assert_eq!(
            channels.iter().map(|c| c.id).collect::<Vec<_>>(),
            [0, 1, 2, -1]
        );
        for (i, channel) in channels.iter().enumerate() {
            assert_eq!(&*channel.data, &buffer.data[i * plane..(i + 1) * plane]);
        }
    }

    #[test]
    fn a_blank_buffer_has_blank_channels() {
        let channels = type_channels(&PixelBuffer::zeroed(3, 2, 4));
        assert!(channels.iter().all(|c| c.data.iter().all(|&v| v == 0)));
    }
}
