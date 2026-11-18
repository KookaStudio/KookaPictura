use super::*;

#[test]
fn converts_planar_rgb_to_rgba() {
    let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.composite
        .replace_data(vec![10, 20, 30, 40, 50, 60].into())
        .expect("valid composite plane");
    let image = document_to_image(&doc, false);
    assert_eq!(image.width(), 2);
    assert_eq!(image.height(), 1);
    assert_eq!(image.pixel_color(0, 0).red(), 10);
    assert_eq!(image.pixel_color(0, 0).green(), 30);
    assert_eq!(image.pixel_color(0, 0).blue(), 50);
    assert_eq!(image.pixel_color(0, 0).alpha(), 255);
}

#[test]
fn buffer_to_rgba_bytes_matches_image_pixels() {
    let gray = PixelBuffer {
        width: 1,
        height: 1,
        channels: 1,
        data: vec![42].into(),
    };
    assert_eq!(buffer_to_rgba_bytes(&gray), vec![42, 42, 42, 255]);

    let gray_alpha = PixelBuffer {
        width: 1,
        height: 1,
        channels: 2,
        data: vec![10, 200].into(),
    };
    assert_eq!(buffer_to_rgba_bytes(&gray_alpha), vec![10, 10, 10, 200]);

    let rgb = PixelBuffer {
        width: 1,
        height: 1,
        channels: 3,
        data: vec![1, 2, 3].into(),
    };
    assert_eq!(buffer_to_rgba_bytes(&rgb), vec![1, 2, 3, 255]);

    let rgba = PixelBuffer {
        width: 1,
        height: 1,
        channels: 4,
        data: vec![4, 5, 6, 7].into(),
    };
    assert_eq!(buffer_to_rgba_bytes(&rgba), vec![4, 5, 6, 7]);

    let image = buffer_to_image(&rgba);
    assert_eq!(image.pixel_color(0, 0).red(), 4);
    assert_eq!(image.pixel_color(0, 0).green(), 5);
    assert_eq!(image.pixel_color(0, 0).blue(), 6);
    assert_eq!(image.pixel_color(0, 0).alpha(), 7);
}
