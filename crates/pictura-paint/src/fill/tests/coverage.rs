use super::*;
use pictura_core::{assert_no_materialization, Coverage, Plane};

fn document_pixel(doc: &Document, x: i32, y: i32) -> [u8; 4] {
    let rect = doc.layers[0].rect;
    pixel(doc, x - rect.left, y - rect.top)
}

#[test]
fn paint_edits_share_coverage_input() {
    let bounds = PsdRect {
        left: 2,
        top: 2,
        right: 6,
        bottom: 6,
    };
    for tiled in [false, true] {
        let mut plane: Plane<u8> = vec![128; 16].into();
        if tiled {
            assert!(plane.tile(4, 4));
        }
        let clip = Coverage::new(bounds, plane, 0).unwrap();
        let mut image = doc_at((8, 8), (1, 1), [0, 0, 0, 255], false);
        assert_no_materialization(|| {
            crate::bucket::fill_tiled(
                &mut image,
                "0",
                bounds,
                &Plane::filled(8, 8, 1, 255),
                crate::bucket::BucketPaint::Foreground(RED),
                PaintMode::Normal,
                1.0,
                Some(&clip),
                Alpha::Blend,
            )
        })
        .unwrap();
        assert_eq!(document_pixel(&image, 1, 1), [0, 0, 0, 255]);
        assert_eq!(document_pixel(&image, 4, 4)[0], 110);
        let gradient = crate::gradient::Gradient::new(vec![
            crate::gradient::GradientStop::new(0.0, RED),
            crate::gradient::GradientStop::new(
                1.0,
                Rgba {
                    r: 0,
                    g: 0,
                    b: 255,
                    a: 255,
                },
            ),
        ]);
        let _ = crate::gradient::draw(
            &mut image,
            "0",
            &gradient,
            &crate::gradient::GradientOptions::default(),
            (0.0, 0.0),
            (7.0, 0.0),
            Some(&clip),
            Some(bounds),
        );
        assert_eq!(document_pixel(&image, 1, 1), [0, 0, 0, 255]);
        let source = crate::bucket::BucketSource::layer(&image, "0").unwrap();
        let flooded =
            crate::bucket::flood(source, (8, 8), bounds, Some(&clip), (3, 3), 255, true).unwrap();
        assert_eq!(flooded.sample(0), Some(0));
        assert_eq!(flooded.sample(3 * 8 + 3), Some(255));
        let cfg = crate::StrokeConfig {
            diameter: 2,
            hardness: 100,
            aliased: true,
            color: RED,
            ..Default::default()
        };
        let mut stroke = crate::Stroke::begin_at(&image, "0", cfg).unwrap();
        stroke.set_coverage_clip(Some(clip.clone()));
        stroke.sample(
            &mut image,
            crate::StrokeSample {
                x: 2.0,
                y: 4.0,
                pressure: 1.0,
            },
        );
        let _ = stroke.finish();
        assert_eq!(document_pixel(&image, 1, 1), [0, 0, 0, 255]);
        assert_eq!(document_pixel(&image, 0, 0), [0; 4]);
        let mut retouch =
            crate::Stroke::begin_tone(&image, "0", cfg, crate::tone::ToneOptions::default())
                .unwrap();
        retouch.set_coverage_clip(Some(clip.clone()));
        retouch.sample(
            &mut image,
            crate::StrokeSample {
                x: 2.0,
                y: 4.0,
                pressure: 1.0,
            },
        );
        let _ = retouch.finish();
        assert_eq!(document_pixel(&image, 1, 1), [0, 0, 0, 255]);
        crate::eraser::magic_erase(&mut image, "0", &clip, 1.0, RED).unwrap();
        assert_eq!(document_pixel(&image, 4, 4)[3], 127);
        assert_eq!(document_pixel(&image, 1, 1)[3], 255);
        let local = clip.rebased(1, 1).unwrap();
        let mut mask = doc(8, 8, [255; 4]);
        mask.mode = ColorMode::Grayscale;
        mask.layers[0].channels.retain(|c| c.id == 0);
        crate::bucket::fill_tiled(
            &mut mask,
            "0",
            bounds,
            &Plane::filled(8, 8, 1, 255),
            crate::bucket::BucketPaint::Foreground(RED),
            PaintMode::Normal,
            1.0,
            Some(&local),
            Alpha::Keep,
        )
        .unwrap();
        assert_eq!(mask.layers[0].channels[0].data.sample(0), Some(255));
        assert!(mask.layers[0].channels[0].data.sample(3 * 8 + 3).unwrap() < 255);
        let mut growing = doc(8, 8, [0; 4]);
        growing.layers[0].rect = PsdRect::default();
        for channel in &mut growing.layers[0].channels {
            channel.data = Plane::default();
        }
        crate::bucket::fill_tiled(
            &mut growing,
            "0",
            bounds,
            &Plane::filled(8, 8, 1, 255),
            crate::bucket::BucketPaint::Foreground(RED),
            PaintMode::Normal,
            1.0,
            Some(&clip),
            Alpha::Blend,
        )
        .unwrap();
        assert!(growing.layers[0].rect.intersect(bounds).is_some());
    }
}

#[test]
fn feathered_sparse_paint_matches_flat() {
    let (w, h) = (800, 600);
    let mut edge = vec![0.0; w * h];
    for y in 130..470 {
        for x in 130..670 {
            edge[y * w + x] = 255.0;
        }
    }
    pictura_core::blur::approx_gaussian(&mut edge, w, h, 2.0);
    let bytes: Vec<u8> = edge
        .iter()
        .map(|v| v.round().clamp(0.0, 255.0) as u8)
        .collect();
    assert!(bytes.iter().any(|v| *v > 0 && *v < 255));
    let flat: Plane<u8> = bytes.into();
    let mut tiled = flat.clone();
    assert!(tiled.tile(w, h));
    let rect = PsdRect {
        left: 0,
        top: 0,
        right: w as i32,
        bottom: h as i32,
    };
    let full = Plane::filled(w, h, 1, 255);
    let (a, b) = (
        Coverage::new(rect, flat, 0).unwrap(),
        Coverage::new(rect, tiled, 0).unwrap(),
    );
    let mut expected = doc(w as i32, h as i32, [0, 0, 0, 255]);
    let mut actual = expected.clone();
    let run = |image: &mut Document, clip: &Coverage| {
        crate::bucket::fill_tiled(
            image,
            "0",
            rect,
            &full,
            crate::bucket::BucketPaint::Foreground(RED),
            PaintMode::Normal,
            1.0,
            Some(clip),
            Alpha::Blend,
        )
    };
    assert!(run(&mut expected, &a).is_some());
    assert!(assert_no_materialization(|| run(&mut actual, &b)).is_some());
    assert_eq!(actual, expected);
}
