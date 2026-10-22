//! The per-variant engine facts the filter crate exposes.

mod common;

use std::collections::BTreeSet;

use pictura_core::PixelBuffer;
use pictura_filters::{apply, Filter, FilterEngine, FilterKind};

use common::instances;

#[test]
fn filter_reports_its_own_facts() {
    let gaussian = Filter::GaussianBlur { radius: 16.0 };
    assert!(gaussian.moves_alpha());
    assert_eq!(gaussian.preview_apron(), 16);
    assert_eq!(gaussian.engine(), FilterEngine::Native);
    assert_eq!(gaussian.kind(), FilterKind::GaussianBlur);

    let solarize = Filter::Solarize;
    assert!(!solarize.moves_alpha());
    assert_eq!(solarize.preview_apron(), 128);
    assert_eq!(solarize.engine(), FilterEngine::Native);
}

#[test]
fn generated_variant_list_names_every_variant_once() {
    let mut names = BTreeSet::new();
    for &kind in FilterKind::ALL {
        assert!(
            names.insert(kind.name()),
            "duplicate variant {}",
            kind.name()
        );
        let engine = kind.engine();
        assert!(
            matches!(engine, FilterEngine::Native | FilterEngine::Ported),
            "{} has no engine",
            kind.name()
        );
    }
    assert_eq!(names.len(), FilterKind::ALL.len());
}

#[test]
fn each_variant_is_applied_by_the_engine_its_property_names() {
    let instances = instances::all();
    let mut kinds = BTreeSet::new();
    for filter in instances {
        assert!(
            kinds.insert(filter.kind().name()),
            "duplicate instance for {}",
            filter.kind().name()
        );
        let mut buf = PixelBuffer::new(24, 24, 4);
        buf.data.fill(128);
        apply(&filter, &mut buf).unwrap_or_else(|error| {
            panic!(
                "{} was refused by the {} engine: {error}",
                filter.kind().name(),
                match filter.engine() {
                    FilterEngine::Native => "native",
                    FilterEngine::Ported => "ported",
                }
            )
        });
    }
    assert_eq!(
        kinds.len(),
        FilterKind::ALL.len(),
        "every variant needs an instance"
    );
}
