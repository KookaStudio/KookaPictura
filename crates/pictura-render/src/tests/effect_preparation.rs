use super::*;
use std::sync::Arc;
use std::time::Duration;

pub(crate) mod probe;
mod profile;
use probe::{count, layer_id, Event, Guard};

const W: u32 = 40;
const H: u32 = 224;

fn styled(index: u8) -> Layer {
    let mut layer = solid(
        "styled",
        rect(12, 5, 206, 34),
        (50 + index, 90, 180),
        210,
        BlendMode::Normal,
        190,
    );
    for (key, value) in [
        ("dropShadow.on", 1.0),
        ("dropShadow.size", 3.0),
        ("innerGlow.on", 1.0),
        ("innerGlow.size", 2.0),
        ("bevel.on", 1.0),
        ("bevel.size", 2.0),
    ] {
        crate::set_layer_style_value(&mut layer, key, value);
    }
    layer
}

fn fixture() -> Document {
    doc(
        W,
        H,
        vec![
            solid(
                "base",
                full(W, H),
                (210, 200, 190),
                255,
                BlendMode::Normal,
                255,
            ),
            styled(1),
            styled(2),
        ],
    )
}

fn render(doc: &Document, parallel: bool) -> PixelBuffer {
    crate::composite_region::composite_banded_for_test(doc, 0, 0, doc.width, doc.height, parallel)
}

fn pool(workers: usize) -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .unwrap()
}

fn assert_once(records: &[probe::Record], layer: &Layer) {
    assert_eq!(count(records, layer, |e| matches!(e, Event::Resolve(_))), 1);
    assert_eq!(count(records, layer, |e| matches!(e, Event::Lookup(_))), 1);
    assert_eq!(
        count(records, layer, |e| matches!(e, Event::BuildStart(_))),
        1
    );
    assert_eq!(count(records, layer, |e| matches!(e, Event::BuildEnd)), 1);
}

fn overlap(d: &Document, targets: [&Layer; 2]) {
    crate::purge_source_cache();
    let guard = Guard::new(d, targets.iter().map(|layer| layer_id(layer)).collect());
    let out = pool(2).install(|| render(d, true));
    let records = guard.probe.snapshot();
    for layer in targets {
        assert_once(&records, layer);
    }
    let starts: Vec<_> = records
        .iter()
        .filter(|r| matches!(r.event, Event::BuildStart(_)))
        .collect();
    assert_eq!(starts.len(), 2);
    assert_ne!(starts[0].worker, starts[1].worker);
    let first_end = records
        .iter()
        .position(|r| matches!(r.event, Event::BuildEnd))
        .unwrap();
    assert_eq!(
        records[..first_end]
            .iter()
            .filter(|r| matches!(r.event, Event::BuildStart(_)))
            .count(),
        2
    );
    probe::assert_replay_only(&records);
    drop(guard);
    assert_eq!(out.data, crate::composite::composite_whole(d).data);
}

#[test]
fn banded_effect_builds_overlap() {
    let _cache = crate::layer_source::cache_test_guard();
    let d = fixture();
    overlap(&d, [&d.layers[1], &d.layers[2]]);
}

#[test]
fn banded_nested_effect_builds_overlap() {
    let _cache = crate::layer_source::cache_test_guard();
    for separate_groups in [false, true] {
        let children = if separate_groups {
            vec![
                group("a", BlendMode::Normal, 200, None, vec![styled(1)]),
                group("b", BlendMode::PassThrough, 255, None, vec![styled(2)]),
            ]
        } else {
            vec![styled(1), styled(2)]
        };
        let d = doc(
            W,
            H,
            vec![group("outer", BlendMode::Normal, 255, None, children)],
        );
        let inner = &d.layers[0].children;
        let targets = if separate_groups {
            [&inner[0].children[0], &inner[1].children[0]]
        } else {
            [&inner[0], &inner[1]]
        };
        overlap(&d, targets);
    }
}

#[test]
fn banded_hidden_subtrees_build_no_effects() {
    let _cache = crate::layer_source::cache_test_guard();
    let mut hidden = styled(3);
    hidden.visible = false;
    let mut hidden_group = group("hidden", BlendMode::Normal, 255, None, vec![styled(4)]);
    hidden_group.visible = false;
    let mut d = fixture();
    d.layers.push(hidden).unwrap();
    d.layers.push(hidden_group).unwrap();
    crate::purge_source_cache();
    let guard = Guard::new(&d, vec![]);
    let out = pool(2).install(|| render(&d, true));
    let records = guard.probe.snapshot();
    for layer in [&d.layers[3], &d.layers[4], &d.layers[4].children[0]] {
        assert_eq!(count(&records, layer, |_| true), 0);
    }
    drop(guard);
    d.layers.pop();
    d.layers.pop();
    assert_eq!(out.data, render(&d, true).data);
}

fn bounded_render(d: Arc<Document>, workers: usize) -> PixelBuffer {
    let (send, recv) = std::sync::mpsc::channel();
    // Detach on a failure: joining a wedged worker would defeat the timeout.
    let worker = std::thread::spawn(move || {
        send.send(pool(workers).install(|| render(&d, true))).ok();
    });
    let out = recv
        .recv_timeout(Duration::from_secs(15))
        .expect("banded preparation completes");
    worker.join().unwrap();
    out
}

#[test]
fn banded_effect_preparation_one_worker() {
    let _cache = crate::layer_source::cache_test_guard();
    let d = Arc::new(fixture());
    crate::purge_source_cache();
    let out = bounded_render(Arc::clone(&d), 1);
    assert_eq!(out.data, crate::composite::composite_whole(&d).data);
}

#[test]
fn banded_multiple_effects_reuse_passes() {
    let _cache = crate::layer_source::cache_test_guard();
    let d = fixture();
    crate::purge_source_cache();
    let guard = Guard::new(&d, vec![]);
    let cold = pool(2).install(|| render(&d, true));
    let before = guard.probe.snapshot();
    for layer in d.layers.iter().skip(1) {
        assert_once(&before, layer);
    }
    let warm = render(&d, true);
    let after = guard.probe.snapshot();
    for layer in d.layers.iter().skip(1) {
        assert_eq!(count(&after, layer, |e| matches!(e, Event::Resolve(_))), 2);
        assert_eq!(count(&after, layer, |e| matches!(e, Event::Lookup(_))), 2);
        assert_eq!(
            count(&after, layer, |e| matches!(e, Event::BuildStart(_))),
            1
        );
    }
    assert_eq!(cold.data, warm.data);
}

#[test]
fn banded_effect_passes_survive_eviction() {
    let _cache = crate::layer_source::cache_test_guard();
    let _budget = crate::layer_source::TestBudget::new(1);
    let d = fixture();
    crate::purge_source_cache();
    let guard = Guard::new(&d, vec![]);
    let out = pool(2).install(|| render(&d, true));
    let records = guard.probe.snapshot();
    for layer in d.layers.iter().skip(1) {
        assert_once(&records, layer);
    }
    probe::assert_replay_only(&records);
    drop(guard);
    assert_eq!(out.data, crate::composite::composite_whole(&d).data);
}

#[test]
fn banded_parallel_effect_edit_invalidates() {
    let _cache = crate::layer_source::cache_test_guard();
    let mut d = fixture();
    crate::purge_source_cache();
    let _ = render(&d, true);
    crate::set_layer_style_value(&mut d.layers[1], "dropShadow.size", 7.0);
    let guard = Guard::new(&d, vec![]);
    let edited = render(&d, true);
    let records = guard.probe.snapshot();
    assert_once(&records, &d.layers[1]);
    assert_eq!(
        count(&records, &d.layers[2], |e| matches!(
            e,
            Event::BuildStart(_)
        )),
        0
    );
    drop(guard);
    crate::purge_source_cache();
    assert_eq!(edited.data, render(&d, true).data);
}

#[test]
fn banded_shared_effect_key_completes_exactly() {
    let _cache = crate::layer_source::cache_test_guard();
    let mut d = doc(W, H, vec![styled(1)]);
    // Plane identities are lazy; assign them before cloning so the clones
    // deliberately share a cache key, as already-rendered duplicates do.
    let key = crate::layer_source::key::content_key(&d.layers[0], &d);
    let shared = d.layers[0].clone();
    d.layers.push(shared).unwrap();
    assert_eq!(key, crate::layer_source::key::content_key(&d.layers[1], &d));
    let d = Arc::new(d);
    crate::purge_source_cache();
    let guard = Guard::new(&d, vec![]);
    let out = bounded_render(Arc::clone(&d), 2);
    let records = guard.probe.snapshot();
    let mut ready = Vec::new();
    for layer in &d.layers {
        assert_eq!(
            count(&records, layer, |e| matches!(e, Event::Resolve(_))),
            1
        );
        assert_eq!(count(&records, layer, |e| matches!(e, Event::Lookup(_))), 1);
        ready.push(
            records
                .iter()
                .find_map(|r| match r.event {
                    Event::Ready(ptr) if r.layer == layer_id(layer) => Some(ptr),
                    _ => None,
                })
                .unwrap(),
        );
    }
    assert_eq!(ready[0], ready[1]);
    let builds = records
        .iter()
        .filter(|r| matches!(r.event, Event::BuildStart(_)))
        .count();
    assert!((1..=2).contains(&builds));
    probe::assert_replay_only(&records);
    drop(guard);
    assert_eq!(out.data, crate::composite::composite_whole(&d).data);
}

#[test]
fn banded_shared_feathered_masks_complete_exactly() {
    let _counters = crate::composite_rows::serial_test_lock();
    let _cache = crate::layer_source::cache_test_guard();
    for workers in [1, 4] {
        for vector in [false, true] {
            for feather in [1.5, 8.0] {
                let mut plain = solid(
                    "plain",
                    full(W, H),
                    (60, 100, 180),
                    210,
                    BlendMode::Normal,
                    190,
                );
                if vector {
                    let mut mask = pictura_core::VectorMask {
                        subpaths: vec![pictura_core::VectorSubpath {
                            closed: true,
                            operation: 1,
                            points: vec![
                                [5 * 256, 12 * 256],
                                [34 * 256, 12 * 256],
                                [34 * 256, 210 * 256],
                                [5 * 256, 210 * 256],
                            ],
                            ..Default::default()
                        }],
                        ..Default::default()
                    };
                    mask.set_feather(feather);
                    plain.vector_mask = Some(mask);
                } else {
                    let mut mask = LayerMask {
                        rect: full(W, H),
                        data: Some(
                            (0..W * H)
                                .map(|i| if i % W < W / 2 { 0 } else { 255 })
                                .collect::<Vec<_>>()
                                .into(),
                        ),
                        ..Default::default()
                    };
                    mask.set_feather(feather);
                    mask.data.as_ref().unwrap().ident();
                    plain.mask = Some(mask);
                }
                // Fresh masks keep this first render cold; clones share the
                // same feather key without relying on any layer-style build.
                let d = Arc::new(doc(W, H, vec![plain; 6]));
                for layer in d.layers.iter().skip(1) {
                    if vector {
                        assert_eq!(
                            layer.vector_mask.as_ref().unwrap().ident(),
                            d.layers[0].vector_mask.as_ref().unwrap().ident()
                        );
                    } else {
                        assert_eq!(
                            layer.mask.as_ref().unwrap().data.as_ref().unwrap().ident(),
                            d.layers[0]
                                .mask
                                .as_ref()
                                .unwrap()
                                .data
                                .as_ref()
                                .unwrap()
                                .ident()
                        );
                    }
                }
                crate::purge_source_cache();
                let parallel = bounded_render(Arc::clone(&d), workers);
                assert_eq!(parallel.data, render(&d, false).data);
                assert_eq!(parallel.data, crate::composite::composite_whole(&d).data);
            }
        }
    }
}

#[test]
fn banded_parallel_effects_equal_serial_and_whole() {
    let _counters = crate::composite_rows::serial_test_lock();
    let _cache = crate::layer_source::cache_test_guard();
    let mut d = fixture();
    d.layers[1].mask = Some(LayerMask {
        rect: full(W, H),
        default_color: 255,
        data: Some(
            (0..W * H)
                .map(|i| if i % 7 == 0 { 0 } else { 210 })
                .collect::<Vec<_>>()
                .into(),
        ),
        ..Default::default()
    });
    d.layers[2].vector_mask = Some(pictura_core::VectorMask {
        subpaths: vec![pictura_core::VectorSubpath {
            closed: true,
            operation: 1,
            points: vec![
                [6 * 256, 20 * 256],
                [34 * 256, 20 * 256],
                [34 * 256, 190 * 256],
                [6 * 256, 190 * 256],
            ],
            ..Default::default()
        }],
        ..Default::default()
    });
    let mut clipped = styled(3);
    clipped.clipping = true;
    d.layers.push(clipped).unwrap();
    d.layers
        .push(group(
            "isolated",
            BlendMode::Multiply,
            150,
            None,
            vec![styled(4)],
        ))
        .unwrap();
    crate::purge_source_cache();
    let parallel = render(&d, true);
    crate::purge_source_cache();
    let serial = render(&d, false);
    let whole = crate::composite::composite_whole(&d);
    assert_eq!(parallel.data, serial.data);
    assert_eq!(parallel.data, whole.data);
    let (x, y, w, h) = (3, 9, 32, 190);
    crate::purge_source_cache();
    let part = crate::composite_region::composite_banded_for_test(&d, x, y, w, h, true);
    crate::purge_source_cache();
    let serial_part = crate::composite_region::composite_banded_for_test(&d, x, y, w, h, false);
    assert_eq!(part.data, serial_part.data);
    for py in 0..h {
        for px in 0..w {
            assert_eq!(super::px(&part, px, py), super::px(&whole, px + x, py + y));
        }
    }
}

#[test]
fn non_banded_plans_keep_sequential_resolution() {
    let _cache = crate::layer_source::cache_test_guard();
    let mut d = fixture();
    d.layers
        .push(group(
            "nested",
            BlendMode::Normal,
            255,
            None,
            vec![styled(3)],
        ))
        .unwrap();
    let workers = pool(2);
    for depth in [BitDepth::Eight, BitDepth::Sixteen] {
        d.depth = depth;
        crate::purge_source_cache();
        let guard = Guard::new(&d, vec![]);
        workers.install(|| {
            use crate::composite_plan::CompositePlan;
            let clip = full(W, H);
            let _whole = CompositePlan::build(&d, clip, None);
            let _prefix = CompositePlan::build_prefix(&d, clip, Some(clip), 2);
            let _suffix = CompositePlan::build_suffix(&d, clip, Some(clip), 1);
            let layers: Vec<_> = d.layers.iter().collect();
            let _selected = CompositePlan::build_for_layers(&layers, &d, clip, None);
        });
        let records = guard.probe.snapshot();
        let resolved: Vec<_> = records
            .iter()
            .filter(|r| matches!(r.event, Event::Resolve(_)))
            .collect();
        assert!(!resolved.is_empty());
        assert!(resolved
            .iter()
            .all(|r| matches!(r.event, Event::Resolve(false)) && r.worker == resolved[0].worker));
    }
    d.depth = BitDepth::Eight;
    crate::purge_source_cache();
    let guard = Guard::new(&d, vec![]);
    let clip = rect(60, 10, 64, 16);
    let _ = crate::composite_region::composite_rgba_region(&d, 10, 60, 6, 4);
    for layer in d.layers.iter().skip(1).take(2) {
        let records = guard.probe.snapshot();
        assert_eq!(
            count(
                &records,
                layer,
                |e| matches!(e, Event::Lookup(Some(r)) if *r == clip)
            ),
            1
        );
        assert_eq!(
            count(
                &records,
                layer,
                |e| matches!(e, Event::BuildStart(Some(r)) if *r == clip)
            ),
            1
        );
    }
    drop(guard);
    let guard = Guard::new(&d, vec![]);
    let _ = render(&d, true);
    for layer in d.layers.iter().skip(1).take(2) {
        assert_once(&guard.probe.snapshot(), layer);
    }
}
