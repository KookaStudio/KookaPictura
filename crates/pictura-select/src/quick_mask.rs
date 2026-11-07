//! Display interpretation of selection coverage. Protection is never stored.

/// Convert canonical selected coverage to or from the grayscale editing view.
pub fn working_value(coverage: u8, selected_areas: bool) -> u8 {
    if selected_areas {
        255 - coverage
    } else {
        coverage
    }
}

/// Blend the display tint over one premultiplied display sample.
#[inline]
pub fn overlay(base: [u8; 4], coverage: u8, selected: bool, color: u32, opacity: u8) -> [u8; 4] {
    let amount =
        (255 - working_value(coverage, selected)) as f32 / 255.0 * opacity.min(100) as f32 / 100.0;
    let tint = [(color >> 16) as u8, (color >> 8) as u8, color as u8];
    let mut out = base;
    for k in 0..3 {
        out[k] = (base[k] as f32 * (1.0 - amount) + tint[k] as f32 * amount).round() as u8;
    }
    out[3] = (base[3] as f32 * (1.0 - amount) + 255.0 * amount).round() as u8;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Selection;

    #[test]
    fn quick_mask_coverage_is_lossless() {
        let selection = Selection {
            width: 256,
            height: 1,
            data: (0..=255).collect::<Vec<u8>>().into(),
        };
        for selected in [false, true] {
            for c in 0..=255 {
                assert_eq!(working_value(working_value(c, selected), selected), c);
            }
        }
        let channel = selection.to_channel(1);
        assert_eq!(
            Selection::from_channel(&channel, 256, 1).unwrap(),
            selection
        );
    }

    #[test]
    fn quick_mask_overlay_uses_protection() {
        assert_eq!(
            overlay([255, 255, 255, 255], 255, false, 0xffff0000, 50),
            [255, 255, 255, 255]
        );
        assert_eq!(
            overlay([255, 255, 255, 255], 0, false, 0xffff0000, 50),
            [255, 128, 128, 255]
        );
        assert_eq!(
            overlay([255, 255, 255, 255], 255, true, 0xffff0000, 100),
            [255, 0, 0, 255]
        );
        assert_eq!(
            overlay([12, 34, 56, 255], 0, false, 0xff000000, 0),
            [12, 34, 56, 255]
        );
    }
}
