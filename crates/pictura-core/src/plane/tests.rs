use super::*;

fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i.wrapping_mul(37) % 251) as u8).collect()
}

#[test]
fn a_clone_shares_and_a_write_forks() {
    let mut a: Plane<u8> = vec![1, 2, 3, 4].into();
    let b = a.clone();
    assert!(a.shares(&b), "a clone must only bump the refcount");
    assert_eq!(a, b);

    a[0] = 9;
    assert!(!a.shares(&b), "a write must fork the plane");
    assert_eq!(a, vec![9, 2, 3, 4]);
    assert_eq!(b, vec![1, 2, 3, 4], "the sharer keeps the old bytes");
}

#[test]
fn an_ident_is_shared_by_copies_and_lost_by_any_change() {
    let mut a: Plane<u8> = vec![1, 2, 3].into();
    let id = a.ident();
    assert_ne!(id, 0);
    assert_eq!(a.ident(), id, "an ident is stable until a write");
    let b = a.clone();
    assert_eq!(b.ident(), id, "a clone keeps the ident");
    a.as_mut_slice()[0] = 9;
    assert_ne!(a.ident(), id, "a mutable access retires the ident");
    assert_eq!(b.ident(), id, "the untouched copy keeps it");
}

#[test]
fn a_stamp_is_shared_by_copies_and_lost_by_any_change() {
    let mut a: Plane<u8> = vec![1, 2, 3, 4].into();
    assert_eq!(a.stamp(), 0, "a new plane has no stamp");
    let stamp = fresh_stamp();
    a.set_stamp(stamp);
    let b = a.clone();
    assert_eq!(b.stamp(), stamp, "a clone keeps the stamp");
    assert_eq!(a, b);

    let changes: [fn(&mut Plane<u8>); 6] = [
        |p| p[0] = 9,
        |p| p.as_mut_slice()[1] = 9,
        |p| p.extend_from_slice(&[5]),
        |p| p.extend([5u8]),
        |p| p.truncate(1),
        |p| p.clear(),
    ];
    for change in changes {
        let mut c = b.clone();
        change(&mut c);
        assert_eq!(c.stamp(), 0, "every mutable access clears the stamp");
    }
    assert_ne!(fresh_stamp(), stamp);
}

#[test]
fn a_built_plane_holds_what_its_fill_wrote() {
    let p: Plane<u8> = Plane::build(4, |s| s.copy_from_slice(&[4, 3, 2, 1]));
    assert_eq!(p, vec![4, 3, 2, 1]);
    assert_eq!(p.stamp(), 0);
}

#[test]
fn a_unique_plane_writes_in_place() {
    let mut a: Plane<u8> = vec![1, 2, 3, 4].into();
    a.truncate(2);
    a.fill(7);
    assert_eq!(a, vec![7, 7]);
    assert_eq!(a.as_ref(), &[7u8, 7]);
}

#[test]
fn planes_compare_their_bytes_against_every_operand_the_tests_use() {
    let p: Plane<u8> = vec![1, 2, 3].into();
    assert_eq!(p, vec![1, 2, 3]);
    assert_eq!(p, [1u8, 2, 3][..]);
    assert_eq!(p, &[1u8, 2, 3][..]);
    assert_eq!(&p, &[1u8, 2, 3][..]);
    assert_eq!(p, p.clone());
    assert_ne!(p, vec![1, 2]);
}

#[test]
fn an_empty_plane_is_default_and_deref_coerces() {
    let p: Plane<u8> = Plane::default();
    assert_eq!(p.len(), 0);
    let s: &[u8] = &p;
    assert!(s.is_empty());
    let o: Option<Plane<u8>> = Some(vec![5u8].into());
    let d: Option<&[u8]> = o.as_deref();
    assert_eq!(d, Some(&[5u8][..]));
}

#[test]
fn tiling_keeps_every_byte_and_the_identity() {
    let bytes = pattern(300 * 520);
    let mut p: Plane<u8> = bytes.clone().into();
    p.set_stamp(7);
    let ident = p.ident();
    assert!(p.tile(300, 520));
    assert_eq!(p.geometry(), Some((300, 520, 1)));
    assert_eq!(p.tile_count(), 2 * 3);
    assert_eq!((p.stamp(), p.ident()), (7, ident), "tiling changes no byte");
    assert_eq!(p, bytes);
    assert!(
        p.tile(300, 520),
        "tiling again at the same geometry is a no-op"
    );

    let mut odd: Plane<u8> = pattern(10).into();
    assert!(!odd.tile(3, 3), "a partial channel stays flat");
    assert!(!odd.is_tiled());
    let mut empty: Plane<u8> = Plane::default();
    assert!(!empty.tile(4, 4));
}

#[test]
fn tiled_planes_keep_their_std_traits() {
    let bytes = pattern(260 * 270);
    let mut p: Plane<u8> = bytes.clone().into();
    p.tile(260, 270);
    assert_eq!(p, bytes.clone());
    assert_eq!(p.as_ref(), bytes.as_slice());
    assert!(format!("{p:?}").contains("Plane"));
    let seen: Vec<u8> = (&p).into_iter().copied().collect();
    assert_eq!(seen, bytes);
    let flat = Plane::from(bytes.clone());
    assert_eq!(flat, p);
    assert_eq!(p, flat);
    let mut other = p.clone();
    other.write_rect_from(259..260, 269..270, 260, &[0]);
    assert_ne!(p, other);
}

#[test]
fn materialization_counter_counts_tiled_views_only() {
    let mut flat: Plane<u8> = pattern(300 * 300).into();
    let before = materializations();
    let _ = flat.as_slice();
    assert_eq!(materializations(), before, "a flat plane is free");
    flat.tile(300, 300);
    let _ = flat.as_slice();
    assert!(materializations() > before, "a tiled plane materializes");
    assert!(flat.is_mirrored());
    let mark = materializations();
    let _ = flat.as_slice();
    assert_eq!(materializations(), mark, "the view is cached until a write");
    flat.write_rect_from(0..1, 0..1, 300, &[1]);
    assert!(!flat.is_mirrored(), "a write drops the view");
}

#[test]
fn a_whole_plane_write_flattens_a_tiled_plane() {
    let bytes = pattern(300 * 300);
    let mut p: Plane<u8> = bytes.clone().into();
    p.tile(300, 300);
    let holder = p.clone();
    p.as_mut_slice()[0] = 0x99;
    assert!(!p.is_tiled(), "a whole-plane write leaves a flat plane");
    assert_eq!(p[0], 0x99);
    assert_eq!(
        holder.sample(0),
        Some(bytes[0]),
        "the holder keeps its bytes"
    );
}
