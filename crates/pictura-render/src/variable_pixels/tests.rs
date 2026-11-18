use super::*;
use pictura_core::variables::{Placement, ScaleMethod};

#[test]
fn variable_pixel_placement_respects_frame() {
    let frame = PsdRect {
        left: 10,
        top: 20,
        right: 18,
        bottom: 28,
    };
    for (method, expected) in [
        (ScaleMethod::Fit, (8, 4)),
        (ScaleMethod::Fill, (16, 8)),
        (ScaleMethod::AsIs, (4, 2)),
        (ScaleMethod::Conform, (8, 8)),
    ] {
        for horizontal in 0..=2 {
            for vertical in 0..=2 {
                let placement = Placement {
                    method,
                    horizontal,
                    vertical,
                    clip: false,
                };
                let (rect, _) = place_pixels(&[255; 32], 4, 2, frame, placement).unwrap();
                assert_eq!((rect.width(), rect.height()), expected);
                assert_eq!(
                    rect.left,
                    frame.left + (frame.width() - expected.0) * i32::from(horizontal) / 2
                );
                assert_eq!(
                    rect.top,
                    frame.top + (frame.height() - expected.1) * i32::from(vertical) / 2
                );
            }
        }
    }
    let (rect, bytes) = place_pixels(
        &[255; 32],
        4,
        2,
        frame,
        Placement {
            method: ScaleMethod::Fill,
            clip: true,
            ..Placement::default()
        },
    )
    .unwrap();
    assert_eq!(rect, frame);
    assert_eq!(bytes, vec![255; 8 * 8 * 4]);

    let (_, bytes) = place_pixels(
        &[0, 255, 0, 0, 255, 0, 0, 255],
        2,
        1,
        frame,
        Placement {
            method: ScaleMethod::Conform,
            ..Placement::default()
        },
    )
    .unwrap();
    let (pixels, remainder) = bytes.as_chunks::<4>();
    assert!(remainder.is_empty());
    assert!(pixels.iter().any(|p| (1..255).contains(&p[3])));
    assert!(pixels
        .iter()
        .filter(|p| p[3] != 0)
        .all(|p| p[0] == 255 && p[1] == 0));
}
