//! The Video submenu kinds' descriptor and slot decoding.

use super::*;
use pictura_filters::{DeinterlaceMethod, Filter, InterlaceField};

#[test]
fn ntsc_colors_is_a_parameterless_kind() {
    assert_eq!(
        filter_from_kind_params("ntsc-colors", &[]),
        Some(Filter::NtscColors)
    );
    assert!(filter_from_kind_params("ntsc-colors", &[0.0]).is_none());
}

#[test]
fn ntsc_colors_is_in_the_descriptor() {
    let descriptor = super::descriptor::descriptor("ntsc-colors").expect("ntsc-colors descriptor");
    assert_eq!(
        descriptor.path,
        &["Filter", "Video", "NTSC Colors"][..],
        "menu path"
    );
    assert!(descriptor.menu, "must be a menu row");
    assert_eq!(descriptor.label, "NTSC Colors");
    assert_eq!(descriptor.slots(), 0, "NTSC Colors has no controls");
}

#[test]
fn de_interlace_is_a_two_slot_kind() {
    assert_eq!(
        filter_from_kind_params("de-interlace", &[1.0, 1.0]),
        Some(Filter::DeInterlace {
            field: InterlaceField::Even,
            method: DeinterlaceMethod::Duplication,
        })
    );
    assert_eq!(
        filter_from_kind_params("de-interlace", &[]),
        Some(Filter::DeInterlace {
            field: InterlaceField::Odd,
            method: DeinterlaceMethod::Interpolation,
        })
    );
    assert!(filter_from_kind_params("de-interlace", &[0.0]).is_none());
}

#[test]
fn de_interlace_is_in_the_descriptor() {
    let descriptor =
        super::descriptor::descriptor("de-interlace").expect("de-interlace descriptor");
    assert_eq!(
        descriptor.path,
        &["Filter", "Video", "De-Interlace"][..],
        "menu path"
    );
    assert!(descriptor.menu, "must be a menu row");
    assert_eq!(descriptor.label, "De-Interlace");
    assert_eq!(descriptor.slots(), 2, "De-Interlace has two radio groups");
    assert!(descriptor.whole_layer, "row parity needs the whole layer");
}
