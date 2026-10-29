use super::*;
use crate::PsdRect;

fn layer(name: &str, clipping: bool) -> Layer {
    Layer {
        name: name.into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 1,
            right: 1,
        },
        clipping,
        ..Default::default()
    }
}

fn background(name: &str) -> Layer {
    Layer {
        background: true,
        ..layer(name, false)
    }
}

fn stack(layers: Vec<Layer>) -> LayerStack {
    LayerStack::from_layers(layers)
}

#[test]
fn valid_tree_passes() {
    let mut group = layer("group", false);
    group.is_group = true;
    group.children = stack(vec![layer("inner", false), layer("clipped", true)]);
    let tree = stack(vec![
        background("Background"),
        layer("base", false),
        layer("clipped", true),
        group,
    ]);
    assert_eq!(tree.well_formed(), Ok(()));
}

#[test]
fn clipped_bottom_is_reported() {
    let tree = stack(vec![layer("clipped", true)]);
    assert_eq!(
        tree.well_formed(),
        Err(StackViolation::ClippedBottom(vec![0]))
    );
    let mut group = layer("group", false);
    group.is_group = true;
    group.children = stack(vec![layer("clipped", true)]);
    let tree = stack(vec![layer("base", false), group]);
    assert_eq!(
        tree.well_formed(),
        Err(StackViolation::ClippedBottom(vec![1, 0]))
    );
}

#[test]
fn illegal_clip_base_is_reported() {
    let tree = stack(vec![background("Background"), layer("clipped", true)]);
    assert_eq!(
        tree.well_formed(),
        Err(StackViolation::ClippedWithoutBase(vec![1]))
    );
}

#[test]
fn misplaced_or_duplicate_background_is_reported() {
    let tree = stack(vec![layer("a", false), background("Background")]);
    assert_eq!(
        tree.well_formed(),
        Err(StackViolation::BackgroundNotBottom(vec![1]))
    );

    let mut group = background("group");
    group.is_group = true;
    assert_eq!(
        stack(vec![group]).well_formed(),
        Err(StackViolation::BackgroundIsGroup(vec![0]))
    );

    let tree = stack(vec![background("one"), background("two")]);
    assert_eq!(
        tree.well_formed(),
        Err(StackViolation::MultipleBackgrounds(vec![1]))
    );
}

#[test]
fn insert_clears_a_clip_without_a_base_and_refuses_the_floor() {
    let mut tree = stack(vec![background("Background")]);
    assert!(tree.insert(&[], 1, layer("clipped", true)).is_ok());
    let removed = tree.remove(&[1]).expect("inserted");
    assert!(!removed.clipping);

    let below = layer("below", false);
    assert_eq!(
        tree.insert(&[], 0, below).err(),
        Some(StackError::BackgroundFloor)
    );
    let mut second = layer("second", false);
    second.background = true;
    assert_eq!(
        tree.insert(&[], 1, second).err(),
        Some(StackError::IllegalBackgroundNode)
    );
}

#[test]
fn remove_releases_a_run_above_a_departing_base() {
    let mut tree = stack(vec![
        layer("base", false),
        layer("b", true),
        layer("c", true),
    ]);
    assert!(tree.remove(&[0]).is_some());
    assert_eq!(tree.len(), 2);
    assert!(!tree[0].clipping && !tree[1].clipping);
}

#[test]
fn move_releases_the_source_run_and_stays_well_formed() {
    let mut tree = stack(vec![
        layer("A", false),
        layer("B", true),
        layer("C", true),
        layer("D", false),
    ]);
    assert!(tree.move_node(&[0], &[], 4).is_ok());
    assert_eq!(
        tree.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
        vec!["B", "C", "D", "A"]
    );
    assert_eq!(tree.well_formed(), Ok(()));
}

#[test]
fn group_keeps_a_base_run_and_releases_a_removed_base_run() {
    let mut tree = stack(vec![layer("A", false), layer("B", true), layer("C", true)]);
    let mut group = layer("G", false);
    group.is_group = true;
    assert!(tree.group(&[], &[0, 1, 2], group).is_ok());
    assert_eq!(tree.len(), 1);
    assert!(tree[0].children[1].clipping);
    assert_eq!(tree.well_formed(), Ok(()));

    let mut tree = stack(vec![layer("A", false), layer("B", true)]);
    let mut group = layer("G", false);
    group.is_group = true;
    assert!(tree.group(&[], &[0], group).is_ok());
    assert!(tree[1].clipping, "the group becomes the base");
    assert_eq!(tree.well_formed(), Ok(()));
}

#[test]
fn retain_tree_releases_a_run_above_a_removed_base() {
    let mut tree = stack(vec![
        layer("base", false),
        layer("adjustment", false),
        layer("kept", true),
    ]);
    let removed = tree.retain_tree(&[], &mut |layer| layer.name != "adjustment");
    assert_eq!(removed, 1);
    assert_eq!(tree.len(), 2);
    assert!(!tree[1].clipping);
    assert_eq!(tree.well_formed(), Ok(()));
}

#[test]
fn set_clipping_refuses_illegal_positions() {
    let mut tree = stack(vec![background("Background"), layer("a", false)]);
    assert!(!tree.set_clipping(&[0], true));
    assert!(!tree.set_clipping(&[1], true));
    let mut tree = stack(vec![layer("a", false), layer("b", true)]);
    assert!(tree.set_clipping(&[1], false));
    assert!(!tree.set_clipping(&[1], false));
    assert!(tree.set_clipping(&[1], true));
}

#[test]
fn promote_background_moves_to_root_and_refuses_a_second() {
    let mut tree = stack(vec![layer("a", false), layer("b", false)]);
    assert!(tree.promote_background(&[1]).is_ok());
    assert!(tree[0].background);
    assert_eq!(tree.well_formed(), Ok(()));
    assert_eq!(
        tree.promote_background(&[1]),
        Err(StackError::SecondBackground)
    );
}

#[test]
fn from_layers_accepts_a_malformed_tree_unchanged() {
    let before = vec![layer("clipped", true)];
    let tree = LayerStack::from_layers(before.clone());
    assert_eq!(tree.as_slice(), before.as_slice());
    assert!(tree.well_formed().is_err());
}

#[test]
fn malformed_background_is_not_repaired() {
    let mut group = background("group");
    group.is_group = true;
    let cases = [
        (
            stack(vec![layer("a", false), background("Background")]),
            StackViolation::BackgroundNotBottom(vec![1]),
        ),
        (
            stack(vec![group]),
            StackViolation::BackgroundIsGroup(vec![0]),
        ),
        (
            stack(vec![background("one"), background("two")]),
            StackViolation::MultipleBackgrounds(vec![1]),
        ),
    ];
    for (mut tree, expected) in cases {
        let _ = tree.insert(&[], tree.len(), layer("top", false));
        assert_eq!(tree.well_formed(), Err(expected));
    }
}

#[test]
fn property_structural_primitives_keep_the_tree_well_formed() {
    let mut seed = 0x1234_5678_9abc_def0u64;
    let mut next = || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        seed
    };
    for _ in 0..200 {
        let mut tree = stack(vec![
            background("Background"),
            layer("base", false),
            layer("clipped", true),
        ]);
        for _ in 0..20 {
            let len = tree.len();
            match next() % 8 {
                0 => {
                    let _ = tree.insert(&[], len, layer("added", next() % 2 == 0));
                }
                1 if len > 1 => {
                    let _ = tree.remove(&[(next() as usize) % len]);
                }
                2 if len > 1 => {
                    let source = 1 + (next() as usize) % (len - 1);
                    let _ = tree.move_node(&[source], &[], len);
                }
                3 if len > 1 => {
                    let _ = tree.reverse(&[], 0, len);
                }
                4 => {
                    let index = if len == 0 { 0 } else { (next() as usize) % len };
                    let _ = tree.set_clipping(&[index], next() % 2 == 0);
                }
                5 if len > 0 => {
                    let index = (next() as usize) % len;
                    let _ = tree.group(&[], &[index], {
                        let mut group = layer("group", false);
                        group.is_group = true;
                        group
                    });
                }
                6 => {
                    let _ = tree.truncate(&[], len.saturating_sub(1));
                }
                _ => {
                    let _ = tree.retain_tree(&[], &mut |layer| layer.name != "clipped");
                }
            }
            assert_eq!(
                tree.well_formed(),
                Ok(()),
                "primitive left an invalid tree: {:?}",
                tree
            );
        }
    }

    // A malformed tree must not panic when an edit runs; the edit is
    // allowed to leave the untouched invalid container as loaded.
    let mut malformed = stack(vec![layer("clipped", true)]);
    let _ = malformed.insert(&[], 1, layer("top", false));
    let _ = malformed.remove(&[1]);
}

#[test]
fn first_edit_normalizes_only_the_touched_container() {
    let mut bad = layer("bad", false);
    bad.is_group = true;
    bad.children = stack(vec![layer("clipped", true)]);
    let mut good = layer("good", false);
    good.is_group = true;
    good.children = stack(vec![layer("clipped", true)]);
    let mut tree = stack(vec![good, bad]);
    assert!(tree.insert(&[0], 1, layer("top", false)).is_ok());
    assert_eq!(
        tree.well_formed(),
        Err(StackViolation::ClippedBottom(vec![1, 0]))
    );
}
#[test]
fn splice_maps_the_anchor_parent_after_a_removal() {
    let mut g0 = layer("G0", false);
    g0.is_group = true;
    let mut x = layer("X", false);
    x.is_group = true;
    let mut g1 = layer("G1", false);
    g1.is_group = true;
    let mut tree = stack(vec![g0, x, g1]);
    let at = tree
        .splice(&[vec![0]], &[1], 0, vec![layer("merged", false)])
        .expect("splice");
    assert_eq!(at, vec![0, 0]);
    assert_eq!(tree.len(), 2);
    assert_eq!(tree[0].name, "X");
    assert_eq!(tree[0].children[0].name, "merged");
    assert_eq!(tree.well_formed(), Ok(()));
}

#[test]
fn splice_refusal_leaves_the_tree_unchanged() {
    let mut tree = stack(vec![background("Background"), layer("A", false)]);
    let before = tree.clone();
    assert_eq!(
        tree.splice(&[vec![1]], &[], 0, vec![]),
        Err(StackError::BackgroundFloor)
    );
    assert_eq!(tree, before);
    assert_eq!(
        tree.splice(&[], &[], 9, vec![]),
        Err(StackError::NoSuchPath)
    );
    assert_eq!(tree, before);
}

#[test]
fn insert_refuses_a_second_nested_background() {
    let mut first = layer("first", false);
    first.is_group = true;
    first.children = stack(vec![background("first background")]);
    let mut tree = stack(vec![first]);
    let mut second = layer("second", false);
    second.is_group = true;
    second.children = stack(vec![background("second background")]);
    let before = tree.clone();
    assert_eq!(
        tree.insert(&[], 1, second),
        Err(StackError::SecondBackground)
    );
    assert_eq!(tree, before);
}

#[test]
fn retain_tree_noop_does_not_normalize_untouched_containers() {
    let mut bad = layer("bad", false);
    bad.is_group = true;
    bad.children = stack(vec![layer("clipped", true)]);
    let mut tree = stack(vec![layer("good", false), bad]);
    let before = tree.clone();
    assert_eq!(tree.retain_tree(&[], &mut |_| true), 0);
    assert_eq!(tree, before);
    assert!(tree[1].children[0].clipping);
}

#[test]
fn retain_tree_releases_every_member_of_a_run() {
    let mut tree = stack(vec![
        layer("base", false),
        layer("b", true),
        layer("c", true),
    ]);
    assert_eq!(tree.retain_tree(&[], &mut |layer| layer.name != "base"), 1);
    assert_eq!(tree.len(), 2);
    assert!(!tree[0].clipping && !tree[1].clipping);
    assert_eq!(tree.well_formed(), Ok(()));
}

#[test]
fn move_node_can_leave_a_source_below_the_background() {
    let mut group = layer("G", false);
    group.is_group = true;
    let mut tree = stack(vec![layer("below", false), background("Background"), group]);
    assert!(tree.move_node(&[0], &[2], 0).is_ok());
    assert_eq!(tree.len(), 2);
    assert!(tree[0].background);
    assert_eq!(tree[1].children[0].name, "below");
    assert_eq!(tree.well_formed(), Ok(()));
}

#[test]
fn replace_allows_a_background_only_at_the_container_bottom() {
    let mut group = layer("G", false);
    group.is_group = true;
    group.children = stack(vec![background("bg")]);
    let mut tree = stack(vec![group, layer("top", false)]);
    assert!(tree.replace(&[0], vec![background("bg")]).is_ok());
    assert_eq!(tree.well_formed(), Ok(()));

    let mut group = layer("G", false);
    group.is_group = true;
    group.children = stack(vec![background("bg")]);
    let mut tree = stack(vec![layer("a", false), group]);
    let before = tree.clone();
    assert_eq!(
        tree.replace(&[1], vec![background("bg")]),
        Err(StackError::IllegalBackgroundNode)
    );
    assert_eq!(tree, before);
}

#[test]
fn refusals_leave_the_tree_unchanged() {
    let mut tree = stack(vec![background("Background"), layer("A", false)]);
    let before = tree.clone();
    assert!(tree.insert(&[], 0, layer("x", false)).is_err());
    assert_eq!(tree, before);
    assert!(tree.splice(&[vec![1]], &[], 0, vec![]).is_err());
    assert_eq!(tree, before);
    assert!(tree.move_node(&[0], &[], 1).is_err());
    assert_eq!(tree, before);
    assert!(tree.promote_background(&[1]).is_err());
    assert_eq!(tree, before);
}

#[test]
fn a_clone_shares_and_a_structural_edit_forks() {
    let mut a = LayerStack::from_layers(vec![layer("A", false), layer("B", false)]);
    let b = a.clone();
    assert_eq!(
        a.as_slice().as_ptr(),
        b.as_slice().as_ptr(),
        "a clone must only bump the refcount"
    );
    a.push(layer("C", false)).unwrap();
    assert_ne!(
        a.as_slice().as_ptr(),
        b.as_slice().as_ptr(),
        "a structural edit must fork the tree"
    );
    assert_eq!(a.len(), 3);
    assert_eq!(b.len(), 2);
    assert_eq!(b[0].name, "A");
}
