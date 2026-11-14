use super::*;
use crate::composite_blend::{
    set_legacy_blend, set_legacy_source, BlendCtx, BlendOverride, Kernel,
};
use crate::composite_rows::{
    covered_runs, kernel_runs, reset_kernel_runs, serial_test_lock, set_force_general, FORCE_SERIAL,
};
use crate::tests::composite::{type_layer, type_style};
use crate::tests::smart_object::{embedded, green_proxy, payload_of, smart_layer};
use pictura_core::{
    BitDepth, BlendIf, BlendMode, Channel, ColorMode, Document, Knockout, Layer, LayerMask,
    Samples, SourceChannels, VectorFillRule, VectorMask, VectorSubpath,
};
use std::sync::atomic::Ordering;

struct KernelTest {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl KernelTest {
    fn new() -> Self {
        let lock = serial_test_lock();
        set_force_general(false);
        FORCE_SERIAL.store(false, Ordering::Relaxed);
        set_legacy_blend(false);
        set_legacy_source(false);
        reset_kernel_runs();
        crate::composite_blend::reset_mask_samples();
        Self { _lock: lock }
    }
}

impl Drop for KernelTest {
    fn drop(&mut self) {
        set_force_general(false);
        FORCE_SERIAL.store(false, Ordering::Relaxed);
        set_legacy_blend(false);
        set_legacy_source(false);
        reset_kernel_runs();
    }
}

fn without_alpha(mut layer: Layer) -> Layer {
    layer.channels.retain(|channel| channel.id != -1);
    layer
}

fn blend_if() -> BlendIf {
    BlendIf {
        composite_source: (1000, 60000),
        composite_dest: (2000, 64000),
        channel_ranges: vec![((0, 65535), (1000, 60000))],
    }
}

fn plain_mask(w: u32, h: u32, value: u8) -> LayerMask {
    LayerMask {
        rect: full(w, h),
        default_color: 0,
        data: Some(vec![value; (w * h) as usize].into()),
        ..Default::default()
    }
}

fn vector_mask(w: i32, h: i32, invert: bool) -> VectorMask {
    let p = |x: i32, y: i32| [x * 256, y * 256];
    VectorMask {
        subpaths: vec![VectorSubpath {
            closed: true,
            operation: 1,
            fill_rule: VectorFillRule::EvenOdd,
            points: vec![p(0, 0), p(w / 2, 0), p(w / 2, h), p(0, h)],
        }],
        invert,
        ..Default::default()
    }
}

fn native_mask_layer(w: u32, h: u32) -> Layer {
    let mut layer = solid("top", full(w, h), (255, 0, 0), 255, BlendMode::Normal, 255);
    layer.mask = Some(plain_mask(w, h, 128));
    let samples = (0..(w * h)).map(|i| i as u16 * 1000).collect::<Vec<_>>();
    layer.source_channels = Some(SourceChannels::new(
        BitDepth::Sixteen,
        layer.rect,
        vec![(-2, Samples::U16(samples.into()))],
    ));
    layer.vector_mask = Some(vector_mask(w as i32, h as i32, false));
    layer
}

fn native_layer(w: u32, h: u32) -> Layer {
    let mut layer = solid(
        "native",
        full(w, h),
        (100, 140, 200),
        255,
        BlendMode::Normal,
        255,
    );
    let plane = |v: u16| Samples::U16(vec![v; (w * h) as usize].into());
    layer.source_channels = Some(SourceChannels::new(
        BitDepth::Sixteen,
        layer.rect,
        vec![
            (0, plane(30000)),
            (1, plane(40000)),
            (2, plane(50000)),
            (-1, plane(65535)),
        ],
    ));
    layer
}

fn type_proxy(w: u32, h: u32) -> Layer {
    let n = (w * h) as usize;
    let mut layer = type_layer(Some(type_style()));
    layer.rect = full(w, h);
    layer.channels = vec![
        Channel {
            id: 0,
            data: vec![10; n].into(),
        },
        Channel {
            id: 1,
            data: vec![20; n].into(),
        },
        Channel {
            id: 2,
            data: vec![30; n].into(),
        },
        Channel {
            id: -1,
            data: vec![255; n].into(),
        },
    ];
    layer
}

fn masked_with(layer: Layer) -> Document {
    doc(
        8,
        8,
        vec![
            solid(
                "base",
                full(8, 8),
                (10, 20, 30),
                255,
                BlendMode::Normal,
                255,
            ),
            layer,
        ],
    )
}

fn masked_doc(mask: LayerMask) -> Document {
    let mut top = solid("top", full(8, 8), (255, 0, 0), 255, BlendMode::Normal, 255);
    top.mask = Some(mask);
    masked_with(top)
}

#[test]
fn unit_table_matches_division() {
    for value in 0..=255u8 {
        assert_eq!(
            crate::composite::unit_u8(value).to_bits(),
            (value as f32 / 255.0).to_bits(),
            "sample {value}"
        );
    }
}

#[test]
fn type_proxy_with_text_gamma_takes_the_general_kernel() {
    let layer = type_proxy(4, 4);
    let d = doc(4, 4, Vec::new());
    let ctx = BlendCtx::new(&layer, &d, full(4, 4));
    assert!(
        crate::text_gamma::text_gamma().is_some(),
        "the default text gamma is on"
    );
    assert!(
        ctx.kernel(false, true).is_none(),
        "the text-gamma curve keeps a type proxy on the general kernel"
    );
}

#[test]
fn common_layer_uses_the_row_span_kernel() {
    let _guard = KernelTest::new();
    let d = doc(
        4,
        4,
        vec![
            solid(
                "base",
                full(4, 4),
                (10, 20, 30),
                255,
                BlendMode::Normal,
                255,
            ),
            solid(
                "top",
                full(4, 4),
                (200, 100, 50),
                255,
                BlendMode::Normal,
                255,
            ),
        ],
    );
    reset_kernel_runs();
    let fast = crate::composite_rgba(&d);
    assert!(kernel_runs(Kernel::Alpha) > 0, "the alpha shape ran");
    assert_eq!(kernel_runs(Kernel::Opaque), 0);
    assert_eq!(kernel_runs(Kernel::Masked), 0);
    set_force_general(true);
    let forced = crate::composite_rgba(&d);
    set_force_general(false);
    assert_eq!(fast.data, forced.data);
}

#[test]
fn advanced_attributes_keep_the_general_kernel() {
    let _guard = KernelTest::new();

    let mut blend_if_layer = solid(
        "top",
        full(4, 4),
        (200, 100, 50),
        255,
        BlendMode::Normal,
        255,
    );
    blend_if_layer.blend_if = Some(blend_if());

    let mut native = doc(4, 4, vec![native_layer(4, 4)]);
    native.source_depth = Some(BitDepth::Sixteen);

    let multiply = doc(
        4,
        4,
        vec![solid(
            "top",
            full(4, 4),
            (200, 100, 50),
            255,
            BlendMode::Multiply,
            255,
        )],
    );

    let mut knockout_top = solid(
        "top",
        full(4, 4),
        (200, 100, 50),
        255,
        BlendMode::Normal,
        255,
    );
    knockout_top.knockout = Knockout::Deep;
    let knockout = doc(
        4,
        4,
        vec![
            solid(
                "base",
                full(4, 4),
                (10, 20, 30),
                255,
                BlendMode::Multiply,
                255,
            ),
            knockout_top,
        ],
    );

    let cases = [
        doc(4, 4, vec![blend_if_layer]),
        // The default text gamma gives the type layer a curve, so its proxy
        // takes the general kernel.
        doc(4, 4, vec![type_proxy(4, 4)]),
        native,
        multiply,
        knockout,
    ];
    for d in &cases {
        reset_kernel_runs();
        let fast = crate::composite_rgba(d);
        set_force_general(true);
        let forced = crate::composite_rgba(d);
        set_force_general(false);
        assert_eq!(fast.data, forced.data);
        assert_eq!(
            kernel_runs(Kernel::Masked) + kernel_runs(Kernel::Alpha) + kernel_runs(Kernel::Opaque),
            0,
            "an advanced layer selected a row-span kernel"
        );
    }
    assert!(
        covered_runs() > 0,
        "the knockout cover forced a general pass"
    );
}

#[test]
fn kernel_selection_follows_synthetic_overrides() {
    let _guard = KernelTest::new();
    let clip = full(4, 4);
    let mut base = solid("base", clip, (200, 100, 50), 255, BlendMode::Multiply, 255);
    base.fill = 128;
    base.mask = Some(plain_mask(4, 4, 128));
    base.blend_if = Some(blend_if());
    let d = doc(4, 4, Vec::new());
    let ctx = BlendCtx::new(&base, &d, clip);
    assert!(ctx.kernel(false, true).is_none());
    let over = BlendOverride {
        mode: Some(BlendMode::Normal),
        fill: Some(255),
        no_mask: true,
        no_blend_if: true,
        ..Default::default()
    };
    let effective = ctx.with_override(&over);
    assert_eq!(effective.kernel(false, true), Some(Kernel::Alpha));

    let mut clipped = solid("clip", clip, (10, 20, 30), 255, BlendMode::Normal, 255);
    clipped.clipping = true;
    let clip_doc = doc(4, 4, vec![base, clipped]);
    reset_kernel_runs();
    let fast = crate::composite_rgba(&clip_doc);
    set_force_general(true);
    let forced = crate::composite_rgba(&clip_doc);
    set_force_general(false);
    assert_eq!(fast.data, forced.data);
    assert!(
        kernel_runs(Kernel::Alpha) + kernel_runs(Kernel::Opaque) + kernel_runs(Kernel::Masked) > 0,
        "the clipping group's effective state selected a kernel"
    );

    let mut top = solid("top", clip, (200, 100, 50), 255, BlendMode::Normal, 255);
    top.knockout = Knockout::Deep;
    let knockout = doc(
        4,
        4,
        vec![
            solid("base", clip, (10, 20, 30), 255, BlendMode::Multiply, 255),
            top,
        ],
    );
    reset_kernel_runs();
    let fast = crate::composite_rgba(&knockout);
    set_force_general(true);
    let forced = crate::composite_rgba(&knockout);
    set_force_general(false);
    assert_eq!(fast.data, forced.data);
    assert!(covered_runs() > 0, "the cover canvas forced a general pass");
}

#[test]
fn opaque_kernel_matches_the_general_path() {
    let _guard = KernelTest::new();
    let transparent = doc(
        4,
        4,
        vec![without_alpha(solid(
            "top",
            full(4, 4),
            (200, 100, 50),
            255,
            BlendMode::Normal,
            255,
        ))],
    );
    reset_kernel_runs();
    let fast = crate::composite_rgba(&transparent);
    assert!(kernel_runs(Kernel::Opaque) > 0, "the opaque shape ran");
    set_force_general(true);
    let forced = crate::composite_rgba(&transparent);
    set_force_general(false);
    assert_eq!(fast.data, forced.data);

    let overlay = doc(
        4,
        4,
        vec![
            solid(
                "base",
                full(4, 4),
                (10, 20, 30),
                128,
                BlendMode::Normal,
                255,
            ),
            without_alpha(solid(
                "top",
                full(4, 4),
                (200, 100, 50),
                255,
                BlendMode::Normal,
                255,
            )),
        ],
    );
    reset_kernel_runs();
    let fast = crate::composite_rgba(&overlay);
    assert!(kernel_runs(Kernel::Opaque) > 0, "the opaque shape ran");
    set_force_general(true);
    let forced = crate::composite_rgba(&overlay);
    set_force_general(false);
    assert_eq!(fast.data, forced.data);
}

#[test]
fn kernel_masks_match_the_general_path() {
    let _guard = KernelTest::new();

    let mut plain = plain_mask(8, 8, 128);
    plain.density = 128;
    let mut feathered = plain_mask(8, 8, 128);
    feathered.set_feather(1.5);

    let mut vector = solid("top", full(8, 8), (0, 0, 255), 255, BlendMode::Normal, 255);
    vector.vector_mask = Some(vector_mask(8, 8, false));
    let mut inverted = solid("top", full(8, 8), (0, 0, 255), 255, BlendMode::Normal, 255);
    inverted.vector_mask = Some(vector_mask(8, 8, true));

    let mut native_mask = doc(
        8,
        8,
        vec![
            solid(
                "base",
                full(8, 8),
                (10, 20, 30),
                255,
                BlendMode::Normal,
                255,
            ),
            native_mask_layer(8, 8),
        ],
    );
    native_mask.source_depth = Some(BitDepth::Sixteen);

    let mut gray = masked_doc(plain_mask(8, 8, 128));
    gray.mode = ColorMode::Grayscale;

    let cases = [
        masked_doc(plain_mask(8, 8, 128)),
        masked_doc(plain),
        masked_doc(feathered),
        masked_with(vector),
        masked_with(inverted),
        native_mask,
        gray,
    ];
    for d in &cases {
        reset_kernel_runs();
        let fast = crate::composite_rgba(d);
        assert!(kernel_runs(Kernel::Masked) > 0, "the masked shape ran");
        set_force_general(true);
        let forced = crate::composite_rgba(d);
        set_force_general(false);
        assert_eq!(fast.data, forced.data);
    }

    for content in ["opacity", "fill"] {
        let mut top = solid("top", full(8, 8), (255, 0, 0), 255, BlendMode::Normal, 255);
        top.mask = Some(plain_mask(8, 8, 128));
        if content == "opacity" {
            top.opacity = 0;
        } else {
            top.fill = 0;
        }
        let d = doc(
            8,
            8,
            vec![
                solid(
                    "base",
                    full(8, 8),
                    (10, 20, 30),
                    255,
                    BlendMode::Normal,
                    255,
                ),
                top,
            ],
        );
        FORCE_SERIAL.store(true, Ordering::Relaxed);
        reset_kernel_runs();
        crate::composite_blend::reset_mask_samples();
        let fast = crate::composite_rgba(&d);
        let samples = crate::composite_blend::mask_samples();
        FORCE_SERIAL.store(false, Ordering::Relaxed);
        assert_eq!(samples, 0, "zero {content} must not sample the mask");
        assert!(kernel_runs(Kernel::Masked) > 0, "the masked shape ran");
        set_force_general(true);
        let forced = crate::composite_rgba(&d);
        set_force_general(false);
        assert_eq!(fast.data, forced.data);
    }

    let d = masked_doc(plain_mask(8, 8, 128));
    FORCE_SERIAL.store(true, Ordering::Relaxed);
    reset_kernel_runs();
    crate::composite_blend::reset_mask_samples();
    let fast = crate::composite_rgba(&d);
    let samples = crate::composite_blend::mask_samples();
    FORCE_SERIAL.store(false, Ordering::Relaxed);
    assert!(samples > 0, "the masked kernel samples the mask");
    assert!(kernel_runs(Kernel::Masked) > 0, "the masked shape ran");
    set_force_general(true);
    let forced = crate::composite_rgba(&d);
    set_force_general(false);
    assert_eq!(fast.data, forced.data);
}

#[test]
fn kernel_matrix_matches_the_general_path() {
    let _guard = KernelTest::new();
    let mut knockout_top = solid(
        "top",
        full(4, 4),
        (200, 100, 50),
        255,
        BlendMode::Normal,
        255,
    );
    knockout_top.knockout = Knockout::Deep;
    let mut clipped = solid(
        "clip",
        full(4, 4),
        (10, 20, 30),
        255,
        BlendMode::Normal,
        255,
    );
    clipped.clipping = true;
    let mut opacity = solid(
        "top",
        full(4, 4),
        (200, 100, 50),
        255,
        BlendMode::Normal,
        255,
    );
    opacity.fill = 64;
    opacity.opacity = 200;

    let mut feathered = plain_mask(8, 8, 128);
    feathered.set_feather(1.5);
    let mut vector = solid("top", full(8, 8), (0, 0, 255), 255, BlendMode::Normal, 255);
    vector.vector_mask = Some(vector_mask(8, 8, false));
    let mut native_mask = doc(
        8,
        8,
        vec![
            solid(
                "base",
                full(8, 8),
                (10, 20, 30),
                255,
                BlendMode::Normal,
                255,
            ),
            native_mask_layer(8, 8),
        ],
    );
    native_mask.source_depth = Some(BitDepth::Sixteen);
    let mut gray = masked_doc(plain_mask(8, 8, 128));
    gray.mode = ColorMode::Grayscale;

    let cases = [
        doc(
            4,
            4,
            vec![
                solid(
                    "base",
                    full(4, 4),
                    (10, 20, 30),
                    255,
                    BlendMode::Normal,
                    255,
                ),
                solid(
                    "top",
                    full(4, 4),
                    (200, 100, 50),
                    255,
                    BlendMode::Normal,
                    255,
                ),
            ],
        ),
        doc(
            4,
            4,
            vec![without_alpha(solid(
                "top",
                full(4, 4),
                (200, 100, 50),
                255,
                BlendMode::Normal,
                255,
            ))],
        ),
        masked_doc(plain_mask(8, 8, 128)),
        masked_doc(feathered),
        masked_with(vector),
        native_mask,
        gray,
        doc(4, 4, vec![opacity]),
        doc(
            4,
            4,
            vec![
                solid(
                    "base",
                    full(4, 4),
                    (10, 20, 30),
                    255,
                    BlendMode::Normal,
                    255,
                ),
                clipped,
            ],
        ),
        doc(
            4,
            4,
            vec![
                solid(
                    "base",
                    full(4, 4),
                    (10, 20, 30),
                    255,
                    BlendMode::Multiply,
                    255,
                ),
                knockout_top,
            ],
        ),
        doc(
            4,
            4,
            vec![smart_layer(
                "smart",
                full(4, 4),
                embedded(payload_of([255, 0, 0])),
                green_proxy(),
            )],
        ),
        doc(4, 4, vec![type_proxy(4, 4)]),
    ];
    reset_kernel_runs();
    for d in &cases {
        let fast = crate::composite_rgba(d);
        set_force_general(true);
        let forced = crate::composite_rgba(d);
        set_force_general(false);
        assert_eq!(fast.data, forced.data);
    }
    assert!(kernel_runs(Kernel::Alpha) > 0);
    assert!(kernel_runs(Kernel::Opaque) > 0);
    assert!(kernel_runs(Kernel::Masked) > 0);
    assert!(covered_runs() > 0);
}

#[test]
fn kernel_switches_compose_with_legacy_and_serial() {
    let _guard = KernelTest::new();
    let d = doc(
        8,
        8,
        vec![
            solid(
                "base",
                full(8, 8),
                (10, 20, 30),
                255,
                BlendMode::Normal,
                255,
            ),
            solid(
                "top",
                full(8, 8),
                (200, 100, 50),
                255,
                BlendMode::Normal,
                255,
            ),
        ],
    );
    let kernels =
        || kernel_runs(Kernel::Masked) + kernel_runs(Kernel::Alpha) + kernel_runs(Kernel::Opaque);

    reset_kernel_runs();
    set_legacy_blend(true);
    let _ = crate::composite_rgba(&d);
    set_legacy_blend(false);
    assert_eq!(kernels(), 0, "legacy_blend must disable selection");

    reset_kernel_runs();
    set_legacy_source(true);
    let _ = crate::composite_rgba(&d);
    set_legacy_source(false);
    assert_eq!(kernels(), 0, "legacy_source must disable selection");

    reset_kernel_runs();
    FORCE_SERIAL.store(true, Ordering::Relaxed);
    let serial = crate::composite_rgba(&d);
    assert!(
        kernel_runs(Kernel::Alpha) > 0,
        "the fast kernel runs serially"
    );
    set_force_general(true);
    let serial_general = crate::composite_rgba(&d);
    set_force_general(false);
    FORCE_SERIAL.store(false, Ordering::Relaxed);
    assert_eq!(serial.data, serial_general.data);
    let parallel = crate::composite_rgba(&d);
    assert_eq!(parallel.data, serial.data);
}

#[test]
fn kernel_parity_across_banded_region_and_serial() {
    let _guard = KernelTest::new();
    let (w, h) = (8u32, 160u32);
    let mut masked = solid(
        "masked",
        rect(20, 3, 140, 8),
        (0, 0, 255),
        255,
        BlendMode::Normal,
        255,
    );
    masked.mask = Some(plain_mask(w, h, 128));
    let d = doc(
        w,
        h,
        vec![
            solid(
                "base",
                full(w, h),
                (10, 20, 30),
                255,
                BlendMode::Normal,
                255,
            ),
            without_alpha(solid(
                "opaque",
                rect(4, 1, 120, 6),
                (200, 100, 50),
                255,
                BlendMode::Normal,
                255,
            )),
            masked,
            solid(
                "top",
                rect(10, 2, 150, 7),
                (200, 100, 50),
                200,
                BlendMode::Normal,
                255,
            ),
        ],
    );
    let whole = crate::composite::composite_whole(&d);
    reset_kernel_runs();
    let banded = crate::composite_region::composite_banded_for_test(&d, 0, 0, w, h, true);
    assert_eq!(banded.data, whole.data);
    let serial_bands = crate::composite_region::composite_banded_for_test(&d, 0, 0, w, h, false);
    assert_eq!(serial_bands.data, whole.data);

    FORCE_SERIAL.store(true, Ordering::Relaxed);
    let serial = crate::composite::composite_whole(&d);
    FORCE_SERIAL.store(false, Ordering::Relaxed);
    assert_eq!(serial.data, whole.data);

    let (x0, y0, rw, rh) = (2u32, 3u32, 4u32, 150u32);
    let part = crate::composite::composite_rgba_region(&d, x0, y0, rw, rh);
    for yy in 0..rh {
        for xx in 0..rw {
            assert_eq!(
                px(&part, xx, yy),
                px(&whole, x0 + xx, y0 + yy),
                "at {xx},{yy}"
            );
        }
    }
}

#[test]
fn profile_cases_use_the_intended_shapes() {
    let _guard = KernelTest::new();
    let opaque = doc(
        8,
        8,
        vec![
            solid(
                "base",
                full(8, 8),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            without_alpha(solid(
                "top",
                full(8, 8),
                (200, 100, 50),
                255,
                BlendMode::Normal,
                255,
            )),
        ],
    );
    reset_kernel_runs();
    let _ = crate::composite_rgba(&opaque);
    assert!(
        kernel_runs(Kernel::Opaque) > 0,
        "an alpha-less Normal layer uses the opaque shape"
    );

    let mut masked = solid(
        "masked",
        full(8, 8),
        (0, 0, 255),
        255,
        BlendMode::Normal,
        255,
    );
    masked.mask = Some(plain_mask(8, 8, 128));
    let masked = doc(
        8,
        8,
        vec![
            solid(
                "base",
                full(8, 8),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            masked,
        ],
    );
    reset_kernel_runs();
    let _ = crate::composite_rgba(&masked);
    assert!(
        kernel_runs(Kernel::Masked) > 0,
        "a plain-raster-mask Normal layer uses the masked shape"
    );
}
