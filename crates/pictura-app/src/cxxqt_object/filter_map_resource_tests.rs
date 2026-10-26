use super::*;

#[test]
fn resource_backed_rows_resolve_through_the_app_seam() {
    for kind in [
        "lens-blur",
        "displace",
        "liquify",
        "adaptive-wide-angle",
        "vanishing-point",
    ] {
        assert!(
            filter_from_kind_params(kind, &[]).is_some(),
            "{kind} must resolve its default resource"
        );
    }

    let mut stale = super::descriptor::descriptor("liquify")
        .expect("liquify row")
        .defaults();
    stale[0] = 999_999.0;
    assert!(
        filter_from_kind_params("liquify", &stale).is_none(),
        "a stale mesh id must be refused"
    );
}

#[test]
fn whole_layer_comes_from_the_catalogue() {
    for spec in super::descriptor::all() {
        assert_eq!(
            crate::cxxqt_object::filter_tools::filter_preview_needs_whole_layer(spec.id),
            spec.whole_layer,
            "{}",
            spec.id
        );
    }
    assert!(!crate::cxxqt_object::filter_tools::filter_preview_needs_whole_layer("hdr-toning"));
}
