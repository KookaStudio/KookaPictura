use pictura_select::Selection;

#[test]
fn a_selection_clone_shares_and_a_write_forks() {
    let mut a = Selection::none(4, 4);
    a.data[0] = 255;
    let b = a.clone();
    assert!(
        a.data.shares(&b.data),
        "a clone must only bump the refcount"
    );
    a.data[0] = 0;
    assert!(!a.data.shares(&b.data), "a write must fork the shared mask");
    assert_eq!(a.data[0], 0);
    assert_eq!(b.data[0], 255, "the clone keeps the old bytes");
}
