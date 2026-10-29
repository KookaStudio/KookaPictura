use pictura_adjust::{apply, apply_native, Adjustment, VibranceParams};
use pictura_core::{PixelBuffer, Samples};

fn params(vibrance: i16, saturation: i16) -> Adjustment {
    Adjustment::Vibrance(VibranceParams {
        vibrance,
        saturation,
    })
}

#[test]
fn vibrance_neutral_pixel_stays_neutral() {
    let mut buf = PixelBuffer {
        width: 1,
        height: 1,
        channels: 3,
        data: vec![128u8; 3].into(),
    };
    apply(&params(100, 0), &mut buf).unwrap();
    assert_eq!(&buf.data[..], &[128, 128, 128]);

    let mut store = Samples::U16(vec![32768; 3].into());
    apply_native(&params(100, 0), &mut store, 1, 1, 3).unwrap();
    assert_eq!(store, Samples::U16(vec![32768; 3].into()));
}
