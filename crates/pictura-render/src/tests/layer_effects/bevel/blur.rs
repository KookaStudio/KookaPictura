use super::*;
use crate::layer_effects::blur_reference;

fn generated_doc(style: &[u8], technique: &[u8], soften: f64) -> Document {
    let (w, h) = (57, 43);
    let mut layer = solid(
        "generated bevel",
        rect(5, 8, 35, 46),
        (191, 73, 29),
        255,
        BlendMode::Normal,
        219,
    );
    let lw = layer.rect.width() as usize;
    let lh = layer.rect.height() as usize;
    let mask: Vec<u8> = (0..lw * lh)
        .map(|i| {
            let (x, y) = (i % lw, i / lw);
            if x < 3 + y / 4 || (x > 21 && y < 9) {
                0
            } else {
                (63 + (x * 13 + y * 7) % 193) as u8
            }
        })
        .collect();
    layer.mask = Some(LayerMask {
        rect: layer.rect,
        data: Some(mask.into()),
        ..Default::default()
    });
    layer.extra_blocks = vec![bevel_block(&BevelSpec {
        style: style.to_vec(),
        technique: technique.to_vec(),
        size: 17.5,
        soften,
        depth: 480.0,
        angle: 137.0,
        ..Default::default()
    })];
    doc(
        w,
        h,
        vec![
            solid(
                "backdrop",
                full(w, h),
                (37, 91, 163),
                255,
                BlendMode::Normal,
                255,
            ),
            layer,
        ],
    )
}

fn render(
    pool: &rayon::ThreadPool,
    d: &Document,
    serial: bool,
    expected_calls: usize,
) -> PixelBuffer {
    pool.install(|| {
        let _cache = crate::layer_source::cache_test_guard();
        assert!(!blur_reference::using_serial());
        crate::purge_source_cache();
        let (out, calls) = blur_reference::with_serial(serial, || composite_rgba(d));
        assert!(!blur_reference::using_serial(), "selector must reset");
        assert_eq!(
            calls,
            if serial {
                [expected_calls, 0]
            } else {
                [0, expected_calls]
            }
        );
        out
    })
}

fn compare_composites(with_reference: bool) {
    let _serial = crate::composite_rows::serial_test_lock();
    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let many = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    for style in [b"InrB", b"OtrB", b"Embs", b"PlEb"] {
        for technique in [b"SfBL", b"PrBL", b"Slmt"] {
            for soften in [0.0, 7.25] {
                let d = generated_doc(style, technique, soften);
                let calls = usize::from(technique != b"PrBL") + if soften > 0.0 { 2 } else { 0 };
                let single = render(&one, &d, false, calls);
                let parallel = render(&many, &d, false, calls);
                assert_eq!(
                    single, parallel,
                    "style={style:?} technique={technique:?} soften={soften}"
                );
                if with_reference {
                    let former = render(&one, &d, true, calls);
                    assert_eq!(
                        parallel, former,
                        "former blur: style={style:?} technique={technique:?} soften={soften}"
                    );
                }
            }
        }
    }
}

#[test]
fn bevel_blur_composites_match_former_serial() {
    compare_composites(true);
}

#[test]
fn bevel_blur_composites_match_across_worker_counts() {
    compare_composites(false);
    let reset = std::panic::catch_unwind(|| {
        blur_reference::with_serial(true, || panic!("test selector unwinding"));
    });
    assert!(reset.is_err());
    assert!(!blur_reference::using_serial());
}
