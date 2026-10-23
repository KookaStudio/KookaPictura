//! The descriptor table's contract with the mapping.

use super::*;
use crate::cxxqt_object::filter_tools::filter_preview_needs_whole_layer;
use pictura_filters::{Filter, FrameOptions, Lighting, SharpenRemove, TonalFade};

#[test]
fn arity_is_the_descriptor_slot_count() {
    assert_eq!(arity("blur-gallery"), Some(62));
    assert_eq!(arity("custom"), Some(27));
    assert_eq!(arity("path-blur"), Some(27));
    assert_eq!(arity("lighting-effects"), Some(22));
    for descriptor in all() {
        let kind = descriptor.kind;
        let slots = descriptor.slots();
        let defaults = descriptor.defaults();
        assert_eq!(
            defaults.len(),
            slots,
            "{kind}'s defaults do not cover every slot"
        );
        assert!(
            super::super::filter_from_kind_params(kind, &defaults).is_some(),
            "{kind} must accept its descriptor defaults"
        );
        let mut too_many = defaults.clone();
        too_many.push(0.0);
        assert!(
            super::super::filter_from_kind_params(kind, &too_many).is_none(),
            "{kind} must refuse {} slots",
            slots + 1
        );
        if slots > 1 {
            let mut too_few = defaults;
            too_few.pop();
            assert!(
                super::super::filter_from_kind_params(kind, &too_few).is_none(),
                "{kind} must refuse {} slots",
                slots - 1
            );
        }
    }
}

#[test]
fn mapping_defaults_come_from_the_descriptor() {
    for descriptor in all() {
        let kind = descriptor.kind;
        let empty = super::super::filter_from_kind_params(kind, &[]);
        let built = super::super::filter_from_kind_params(kind, &descriptor.defaults());
        assert_eq!(
            empty, built,
            "{kind}'s empty mapping differs from its defaults"
        );
    }
}

#[test]
fn whole_layer_comes_from_the_descriptor() {
    for descriptor in all() {
        assert_eq!(
            filter_preview_needs_whole_layer(descriptor.kind),
            descriptor.whole_layer,
            "{}'s whole-layer flag disagrees with its preview",
            descriptor.kind
        );
    }
    assert!(
        descriptor("hdr-toning").is_none(),
        "hdr-toning is not a filter kind"
    );
    assert!(!filter_preview_needs_whole_layer("hdr-toning"));
}

#[test]
fn engine_defaults_agree_with_the_descriptor() {
    let cases = [
        (
            "lighting-effects",
            Filter::Lighting {
                lighting: Lighting::default(),
            },
        ),
        (
            "picture-frame",
            Filter::PictureFrame {
                options: FrameOptions::default(),
            },
        ),
        (
            "smart-sharpen",
            Filter::SmartSharpen {
                amount: 100.0,
                radius: 1.0,
                reduce_noise: 0.0,
                remove: SharpenRemove::GaussianBlur,
                angle: 0.0,
                more_accurate: false,
                shadow: TonalFade::default(),
                highlight: TonalFade::default(),
            },
        ),
    ];
    for (kind, expected) in cases {
        let descriptor = descriptor(kind).unwrap_or_else(|| panic!("{kind} has no descriptor"));
        assert_eq!(
            super::super::filter_from_kind_params(kind, &descriptor.defaults()),
            Some(expected),
            "{kind}'s descriptor defaults disagree with the engine"
        );
    }
}

#[test]
fn descriptor_covers_every_mapping_arm() {
    let descriptors: Vec<&FilterDescriptor> = all().collect();
    assert_eq!(descriptors.len(), 112, "the mapping has 112 kinds");
    let mut kinds: Vec<&str> = descriptors
        .iter()
        .map(|descriptor| descriptor.kind)
        .collect();
    kinds.sort_unstable();
    let before = kinds.len();
    kinds.dedup();
    assert_eq!(before, kinds.len(), "a kind is recorded more than once");
    for descriptor in descriptors {
        assert!(
            super::super::filter_from_kind_params(descriptor.kind, &descriptor.defaults())
                .is_some(),
            "{} has a descriptor but no mapping",
            descriptor.kind
        );
    }
    assert!(super::super::filter_from_kind_params("bogus", &[]).is_none());
}
