use pictura_filters::Filter;

use super::plan::plan;
use super::resources::{chunks, run_chunked};

#[test]
fn chunks_cover_the_count_once_in_order() {
    for (count, cost, budget) in [(0, 1, 64), (1, 1, 64), (1000, 4, 1024), (1000, 1, 1 << 40)] {
        let ranges: Vec<_> = chunks(count, cost, budget).collect();
        let mut next = 0;
        for &(base, end) in &ranges {
            assert_eq!(base, next);
            assert!(end > base && end - base >= 64.min(count));
            next = end;
        }
        assert_eq!(next, count, "count {count} cost {cost} budget {budget}");
    }
    assert_eq!(chunks(1000, 4, 1024).count(), 4);
    // A cost past the budget still advances one workgroup at a time.
    assert_eq!(chunks(200, 1 << 30, 1024).count(), 4);
}

/// Chunked submissions must reproduce the single-submission bytes, including
/// the read-modify-write combine pass, which would double-apply on overlap.
#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn chunked_dispatch_matches_one_submission() {
    crate::gpu::require_gpu();
    let (device, queue) = crate::gpu::shared_device().expect("a validated device");
    let (w, h) = (53u32, 41u32);
    let n = (w * h) as usize;
    let input: Vec<u8> = (0..3 * n).map(|i| ((i * 37) ^ (i / 7)) as u8).collect();
    for filter in [
        Filter::GaussianBlur { radius: 4.0 },
        Filter::UnsharpMask {
            amount: 150.0,
            radius: 2.0,
            threshold: 0,
        },
        Filter::Median { radius: 2 },
        Filter::OilPaint {
            stylization: 4.0,
            cleanliness: 5.0,
            scale: 0.0,
            bristle_detail: 4.0,
            angular_direction: 135.0,
            shine: 2.0,
        },
    ] {
        let plan = plan(&filter).expect("accelerated filter");
        let whole = run_chunked(device, queue, &plan, &input, w, h, u64::MAX).expect("gpu run");
        let split = run_chunked(device, queue, &plan, &input, w, h, 1).expect("gpu run");
        assert_eq!(whole, split, "{filter:?}");
    }
}
