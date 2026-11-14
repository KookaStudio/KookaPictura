use super::*;
use crate::memory::{set_host_for_test, HostReading};
use crate::scratch::tests::{serial, temp_volume};
use crate::tiled_plane::TileSample;
use crate::{Plane, TILE};

const MIB: u64 = 1024 * 1024;

/// Global eviction state for one test: serial, a fresh scratch volume,
/// eviction off, the host unpinned; restored on drop.
struct Fixture {
    _serial: std::sync::MutexGuard<'static, ()>,
}

fn fixture(name: &str) -> Fixture {
    let serial = serial();
    crate::scratch::set_volumes(&[temp_volume(name)]);
    set_enabled(false);
    set_host_for_test(None);
    Fixture { _serial: serial }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        set_enabled(false);
        set_host_for_test(None);
        crate::scratch::clear_faults();
    }
}

/// A 1 GiB cgroup (ceiling 768 MiB) with `percent` of the ceiling in use.
fn pin_host_at(percent: u64) {
    let ceiling = 768 * MIB;
    set_host_for_test(Some(HostReading {
        cgroup_max: Some(1024 * MIB),
        cgroup_current: Some(ceiling * percent / 100),
        ..HostReading::default()
    }));
}

fn noise(seed: u64) -> impl FnMut() -> u64 {
    let mut s = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
    move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    }
}

/// A seeded `w × h × channels` u8 plane, tiled; even tile columns are runs
/// (compressible), odd ones noise (incompressible).
fn plane_u8(w: usize, h: usize, channels: usize, seed: u64) -> Plane<u8> {
    let mut next = noise(seed);
    let data: Vec<u8> = (0..w * h * channels)
        .map(|i| {
            let x = i % w;
            if (x / TILE).is_multiple_of(2) {
                (x / 32) as u8 ^ (i / (w * 16)) as u8
            } else {
                next() as u8
            }
        })
        .collect();
    let mut plane = Plane::from(data);
    assert!(plane.tile(w, h));
    plane
}

/// Pack and spill every body that can go: passes until nothing moves.
fn evict_all() -> PassReport {
    let mut total = PassReport::default();
    for _ in 0..6 {
        let r = run_pass(u64::MAX);
        total.freed += r.freed;
        total.packed += r.packed;
        total.spilled += r.spilled;
        total.written += r.written;
        if r.freed == 0 && r.packed == 0 && r.spilled == 0 && total.freed > 0 {
            break;
        }
    }
    total
}

fn dense_tiles<T: TileSample>(plane: &Plane<T>) -> Vec<usize> {
    (0..plane.tile_count())
        .filter(|&i| matches!(plane.tile_state(i), Some(crate::TileState::Dense(_))))
        .collect()
}

fn every<T: TileSample>(plane: &Plane<T>) -> Vec<T> {
    let (w, h, c) = plane.geometry().unwrap();
    let mut out = vec![T::default(); w * h * c];
    plane.read_rect_into(0..w, 0..h * c, w, &mut out);
    out
}

fn assert_all<T: TileSample>(plane: &Plane<T>, want: Residency) {
    for i in dense_tiles(plane) {
        assert_eq!(plane.tile_residency(i), Some(want), "tile {i}");
    }
}

fn check_identity<T: TileSample + std::fmt::Debug>(plane: &Plane<T>) {
    let original = every(plane);
    let bits = |v: &[T]| T::bytes(v).to_vec();
    let (w, _, _) = plane.geometry().unwrap();
    evict_all();
    assert_all(plane, Residency::Spilled);
    assert_eq!(bits(&every(plane)), bits(&original), "rectangle read");
    evict_all();
    for i in dense_tiles(plane) {
        let t = plane.tile_at(i).unwrap();
        for dy in 0..t.h {
            for dx in 0..t.w {
                let at = (t.y + dy) * w + t.x + dx;
                assert!(t.get(dx, dy).same(original[at]), "tile {i} at {dx},{dy}");
            }
        }
    }
    evict_all();
    let mut row = vec![T::default(); w];
    for y in (0..original.len() / w).step_by(97) {
        assert!(plane.read_row_into(y, w, &mut row));
        assert_eq!(bits(&row), bits(&original[y * w..(y + 1) * w]), "row {y}");
    }
    evict_all();
    for i in (0..original.len()).step_by(4099) {
        assert!(plane.sample(i).unwrap().same(original[i]), "sample {i}");
    }
}

#[test]
fn evict_restore_identity() {
    let _f = fixture("identity");
    check_identity(&plane_u8(613, 517, 2, 1));
    let mut next = noise(2);
    let mut p16 = Plane::from(
        (0..613 * 300)
            .map(|i| {
                if i % 613 < 256 {
                    (i / 7) as u16
                } else {
                    next() as u16
                }
            })
            .collect::<Vec<u16>>(),
    );
    assert!(p16.tile(613, 300));
    check_identity(&p16);
    let mut p32 = Plane::from(
        (0..300 * 613)
            .map(|i| match i % 3 {
                0 => -0.0,
                1 => f32::from_bits(next() as u32 & 0x7f7f_ffff),
                _ => (i / 300) as f32 * 0.25,
            })
            .collect::<Vec<f32>>(),
    );
    assert!(p32.tile(300, 613));
    check_identity(&p32);
}

#[test]
fn a_pinned_tile_is_not_evicted() {
    let _f = fixture("pinned");
    let plane = plane_u8(600, 300, 1, 3);
    let original = every(&plane);
    let i = dense_tiles(&plane)[0];
    let borrowed = plane.tile_at(i).unwrap();
    evict_all();
    assert_eq!(plane.tile_residency(i), Some(Residency::Resident));
    assert!(
        dense_tiles(&plane)
            .iter()
            .any(|&j| plane.tile_residency(j) != Some(Residency::Resident)),
        "the other tiles were evicted"
    );
    let row = borrowed.row(5).expect("dense");
    assert_eq!(row, &original[5 * 600..5 * 600 + borrowed.w]);
}

#[test]
fn a_write_after_restore_survives_eviction() {
    let _f = fixture("write-restore");
    let mut plane = plane_u8(512, 512, 1, 4);
    evict_all();
    assert_all(&plane, Residency::Spilled);
    for mut t in plane.tiles_mut(0..300, 0..300, 512) {
        for dy in 0..t.h {
            for v in t.row_mut(dy) {
                *v = v.wrapping_add(13);
            }
        }
    }
    plane.settle();
    let written = every(&plane);
    let report = evict_all();
    assert!(report.written > 0, "written tiles are not clean");
    assert_eq!(every(&plane), written);
}

#[test]
fn history_tiles_are_evicted_before_live_ones() {
    let _f = fixture("order");
    let live = plane_u8(1024, 512, 1, 5);
    let held: Vec<_> = {
        let old = plane_u8(1024, 512, 1, 6);
        dense_tiles(&old)
            .into_iter()
            .map(|i| old.history_tile(i).unwrap())
            .collect()
    };
    let report = run_pass(1);
    assert_eq!(report.packed + report.spilled, 1);
    assert_all(&live, Residency::Resident);
    assert_eq!(counts().packed + counts().spilled, 1, "a history body went");
    drop(held);
}

#[test]
fn eviction_starts_above_the_ceiling_and_stops_at_ninety() {
    let _f = fixture("thresholds");
    // 128 MiB of compressible dense tiles: more than the ~84 MiB a pass at
    // 101 % must free.
    let w = 8192;
    let data: Vec<u8> = (0..w * w * 2).map(|i| ((i % w) / 64) as u8).collect();
    let mut plane = Plane::from(data);
    assert!(plane.tile(w, w));
    pin_host_at(99);
    assert_eq!(step(), PassReport::default(), "nothing at 99 %");
    assert_eq!(counts().packed, 0);
    pin_host_at(101);
    let ceiling = 768 * MIB;
    let need = ceiling * 101 / 100 - ceiling / 10 * 9;
    let report = step();
    assert!(report.freed >= need, "freed {} of {need}", report.freed);
    assert!(
        dense_tiles(&plane)
            .iter()
            .any(|&i| plane.tile_residency(i) == Some(Residency::Resident)),
        "the pass stopped at 90 % instead of evicting everything"
    );
}

fn resident<T: TileSample>(plane: &Plane<T>, i: usize) -> bool {
    plane.tile_residency(i) == Some(Residency::Resident)
}

/// Single-body passes until `done`, at most 256; whether `done` held.
fn pass_until(done: impl Fn() -> bool) -> bool {
    for _ in 0..256 {
        if done() {
            return true;
        }
        run_pass(1);
    }
    done()
}

// [te_clock_hot] Single-body passes with cold filler to spare. The registry
// is sharded by thread, so the order a pass meets bodies in is not fixed.
#[test]
fn second_chance_and_hot_holds() {
    let _f = fixture("clock");
    let plane = plane_u8(1024, 256, 1, 7);
    let filler = plane_u8(4096, 256, 1, 8);
    let tiles = dense_tiles(&plane);
    let (a, b) = (tiles[0], tiles[1]);
    drop(plane.tile_at(a));
    let hold = hot();
    drop(plane.tile_at(b));
    run_pass(1);
    assert!(resident(&plane, a), "second chance");
    assert!(
        pass_until(|| !resident(&plane, a)),
        "a goes once its bit is spent"
    );
    assert!(resident(&plane, b), "hot while cold tiles remain");
    drop(hold);
    // Make the filler hot: b, now cold, must go before any of it.
    let warm = hot();
    let rest: Vec<usize> = dense_tiles(&filler)
        .into_iter()
        .filter(|&i| resident(&filler, i))
        .collect();
    for &i in &rest {
        drop(filler.tile_at(i));
    }
    assert!(
        pass_until(|| !resident(&plane, b)),
        "b goes once the hold drops"
    );
    assert!(
        rest.iter().all(|&i| resident(&filler, i)),
        "before the hot filler"
    );
    drop(warm);
}

// [te_hot_last]
#[test]
fn hot_tiles_are_evicted_last() {
    let _f = fixture("hot-last");
    let plane = plane_u8(1024, 256, 1, 10);
    let tiles = dense_tiles(&plane);
    let hold = hot();
    for &i in &tiles {
        drop(plane.tile_at(i));
    }
    let borrowed = plane.tile_at(tiles[3]).unwrap();
    run_pass(u64::MAX);
    for &i in &tiles[..3] {
        assert!(
            !resident(&plane, i),
            "hot tile {i} went when nothing colder was left"
        );
    }
    assert!(resident(&plane, tiles[3]), "a pinned tile stays");
    drop(borrowed);
    drop(hold);
}

// [te_hold_closed]
#[test]
fn a_closed_hold_keeps_only_what_it_saw() {
    let _f = fixture("hold-closed");
    let plane = plane_u8(1024, 256, 1, 11);
    let _filler = plane_u8(4096, 256, 1, 12);
    let tiles = dense_tiles(&plane);
    let (a, b) = (tiles[0], tiles[1]);
    let mut hold = hot();
    drop(plane.tile_at(a));
    hold.close();
    assert_eq!(hot_epoch(), None, "a closed hold stamps nothing");
    drop(plane.tile_at(b));
    assert!(
        pass_until(|| !resident(&plane, b)),
        "read after the close: cold"
    );
    assert!(
        resident(&plane, a),
        "seen while open: hot while cold tiles remain"
    );
    drop(hold);
}

/// A compressible 2048² cold plane, tiled.
fn cold_plane() -> Plane<u8> {
    let data: Vec<u8> = (0..2048 * 2048).map(|i| ((i % 2048) / 16) as u8).collect();
    let mut cold = Plane::from(data);
    assert!(cold.tile(2048, 2048));
    cold
}

/// Write `side`² new dense tiles, as a commit's output does, one at a time
/// on this thread: the pinned host reading is this thread's.
fn allocate(side: usize) -> Plane<u8> {
    let mut fresh = Plane::filled(side, side, 1, 0u8);
    fresh.tile(side, side);
    assert!(
        fresh.tile_count() > 0,
        "tiled, so each write allocates a body"
    );
    for i in 0..fresh.tile_count() {
        for mut t in fresh.tiles_mut_at(&[i]) {
            t.row_mut(0)[0] = 1;
        }
    }
    fresh.settle();
    fresh
}

fn evicted() -> u64 {
    counts().packed + counts().spilled
}

// [te_urgent_hard]
#[test]
fn a_hard_limit_evicts_at_the_ceiling() {
    let _f = fixture("hard-limit");
    let cold = cold_plane();
    set_enabled(true);
    // RAM only, no cgroup: 110 % of the ceiling is under the 125 % threshold.
    let total = 8192 * MIB;
    let reading = HostReading {
        total: Some(total),
        ..HostReading::default()
    };
    let ceiling = ceiling_of(&reading, governor().usage_percent()).expect("a ceiling");
    set_host_for_test(Some(HostReading {
        rss: Some(ceiling / 100 * 110),
        ..reading
    }));
    refresh_urgency();
    let before = evicted();
    drop(allocate(512));
    assert_eq!(evicted(), before, "no hard limit: 110 % does not evict");
    pin_host_at(101);
    refresh_urgency();
    drop(allocate(512));
    assert!(evicted() > before, "a hard limit evicts above the ceiling");
    assert!(dense_tiles(&cold).iter().any(|&i| !resident(&cold, i)));
}

// [te_alloc_estimate]
#[test]
fn an_allocation_burst_is_caught_between_passes() {
    let _f = fixture("estimate");
    let cold = cold_plane();
    set_enabled(true);
    pin_host_at(95);
    refresh_urgency();
    // The host grows to 101 % with no background step to see it.
    pin_host_at(101);
    let before = evicted();
    // 7168² is 49 MiB of new tiles: more than the 38 MiB left at 95 %.
    let fresh = allocate(7168);
    assert!(
        evicted() > before,
        "the allocating thread re-read and evicted"
    );
    assert!(dense_tiles(&cold).iter().any(|&i| !resident(&cold, i)));
    drop(fresh);
}

#[test]
fn urgent_pressure_evicts_on_the_allocating_thread() {
    let _f = fixture("urgent");
    let cold: Vec<u8> = (0..2048 * 2048).map(|i| ((i % 2048) / 16) as u8).collect();
    let mut cold = Plane::from(cold);
    assert!(cold.tile(2048, 2048));
    set_enabled(true);
    pin_host_at(130);
    refresh_urgency();
    assert_eq!(counts().packed, 0);
    let mut fresh = Plane::filled(512, 512, 1, 0u8);
    drop(fresh.tiles_mut(0..10, 0..10, 512));
    assert!(
        counts().packed + counts().spilled > 0,
        "the allocating thread evicted cold tiles"
    );
    assert!(dense_tiles(&cold)
        .iter()
        .any(|&i| cold.tile_residency(i) != Some(Residency::Resident)));
}

#[test]
fn a_pass_asks_the_display_to_drop_first() {
    let _f = fixture("display-drop");
    let before = display_drop_epoch();
    pin_host_at(110);
    step();
    assert!(display_drop_epoch() > before);
    pin_host_at(50);
    let idle = display_drop_epoch();
    step();
    assert_eq!(display_drop_epoch(), idle, "no pass under the ceiling");
}

#[test]
fn a_clean_tile_drops_without_a_write() {
    let _f = fixture("clean");
    let mut plane = plane_u8(512, 256, 1, 8);
    evict_all();
    assert_all(&plane, Residency::Spilled);
    let original = every(&plane);
    let again = evict_all();
    assert_eq!(again.written, 0, "clean tiles drop with no write");
    assert_all(&plane, Residency::Spilled);
    assert_eq!(every(&plane), original);
    for mut t in plane.tiles_mut(0..512, 0..1, 512) {
        t.row_mut(0)[0] ^= 1;
    }
    plane.settle();
    assert!(
        evict_all().written > 0,
        "a written tile writes a new extent"
    );
}

#[test]
fn reclaim_turns_a_refusal_into_a_grant() {
    let _f = fixture("reclaim");
    let data: Vec<u8> = (0..4096 * 4096 * 4)
        .map(|i| ((i % 4096) / 8) as u8)
        .collect();
    let mut plane = Plane::from(data);
    assert!(plane.tile(4096, 4096));
    // 760 of 768 MiB in use: 8 MiB of room for a 32 MiB reservation.
    set_host_for_test(Some(HostReading {
        cgroup_max: Some(1024 * MIB),
        cgroup_current: Some(760 * MIB),
        ..HostReading::default()
    }));
    assert!(super::super::governor().reserve(32 * MIB, "test").is_err());
    set_enabled(true);
    let granted = super::super::governor().reserve(32 * MIB, "test");
    assert!(granted.is_ok(), "eviction made room");
    assert!(counts().packed > 0);
    drop(plane);
}

#[test]
fn no_pressure_no_eviction() {
    let _f = fixture("idle");
    set_enabled(true);
    pin_host_at(50);
    refresh_urgency();
    let mut plane = plane_u8(1024, 1024, 2, 9);
    for mut t in plane.tiles_mut(0..1024, 0..512, 1024) {
        t.row_mut(0)[0] = 1;
    }
    plane.settle();
    assert_eq!(step(), PassReport::default());
    assert_eq!(counts().packed + counts().spilled, 0);
    assert_all(&plane, Residency::Resident);
}

// ---- Backed tiles (#1218) ----

/// The sample a backed test plane's source holds at `(x, y)`.
fn pattern(x: usize, y: usize) -> u8 {
    (x.wrapping_mul(7) ^ y.wrapping_mul(13)) as u8
}

/// A `w × h` plane backed by [`pattern`], and its count of decodes.
fn backed_plane(w: usize, h: usize) -> (Plane<u8>, Arc<std::sync::atomic::AtomicUsize>) {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    let decode: crate::TileDecode<u8> = Arc::new(move |r: crate::TileRect, out: &mut [u8]| {
        counter.fetch_add(1, Ordering::Relaxed);
        for dy in 0..r.h {
            for dx in 0..r.w {
                out[dy * r.w + dx] = pattern(r.x + dx, r.y + dy);
            }
        }
        Ok(())
    });
    (Plane::backed(w, h, decode), calls)
}

fn expected(w: usize, h: usize) -> Vec<u8> {
    (0..w * h).map(|i| pattern(i % w, i / w)).collect()
}

use std::sync::Arc;

// [sb_redecode]
#[test]
fn backed_tile_dropped_and_redecoded_identity() {
    let _f = fixture("backed-redecode");
    let (plane, calls) = backed_plane(600, 300);
    let tiles = plane.tile_count();
    assert_all(&plane, Residency::Backed);
    assert_eq!(
        calls.load(Ordering::Relaxed),
        0,
        "nothing decodes until read"
    );
    assert_eq!(every(&plane), expected(600, 300));
    assert_all(&plane, Residency::Resident);
    let report = evict_all();
    assert_eq!(report.written, 0, "a backed tile drops without a write");
    assert_all(&plane, Residency::Backed);
    assert_eq!(every(&plane), expected(600, 300));
    assert_eq!(calls.load(Ordering::Relaxed), 2 * tiles);
}

// [sb_detach]
#[test]
fn a_written_backed_tile_leaves_its_source() {
    let _f = fixture("backed-detach");
    let (mut plane, _) = backed_plane(600, 300);
    let mut want = expected(600, 300);
    for mut t in plane.tiles_mut(0..10, 0..1, 600) {
        t.row_mut(0)[0] = !t.row(0)[0];
    }
    want[0] = !want[0];
    plane.settle();
    evict_all();
    let first = plane.tile_residency(0).unwrap();
    assert!(
        matches!(first, Residency::Packed | Residency::Spilled),
        "{first:?}"
    );
    assert_eq!(plane.tile_residency(1), Some(Residency::Backed));
    assert_eq!(every(&plane), want);
}

// [sb_transient]
#[test]
fn a_transient_read_leaves_a_backed_tile_away() {
    let _f = fixture("backed-transient");
    let (plane, calls) = backed_plane(600, 300);
    let read = transient(|| every(&plane));
    assert_eq!(read, expected(600, 300));
    assert_all(&plane, Residency::Backed);
    assert_eq!(calls.load(Ordering::Relaxed), plane.tile_count());
    assert!(!is_transient(), "the scope ends with the call");
}

// [sb_priority]
#[test]
fn backed_tiles_are_evicted_first_without_writes() {
    let _f = fixture("backed-order");
    let (backed, _) = backed_plane(TILE, TILE);
    let live = plane_u8(TILE, TILE, 1, 11);
    // Read both, so each holds its clock bit and the first pass takes them.
    let _ = (every(&backed), every(&live));
    assert_eq!(backed.tile_residency(0), Some(Residency::Resident));
    let need = (TILE * TILE) as u64;
    let first = run_pass(need);
    assert_eq!(first.freed, 0, "both had a second chance");
    let second = run_pass(need);
    assert_eq!(second.written, 0);
    assert_eq!(backed.tile_residency(0), Some(Residency::Backed));
    assert_all(&live, Residency::Resident);
}
