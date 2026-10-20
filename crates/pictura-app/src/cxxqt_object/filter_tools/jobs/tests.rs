use super::*;
use crate::cxxqt_object::impl_filters::apply_filter_active;
use crate::cxxqt_object::tests::pixel_layer;
use pictura_core::{BitDepth, ColorMode};

/// A 96×80 layer of diagonal stripes, so every filter here changes it.
fn striped() -> PictureViewRust {
    let mut doc = Document::new(96, 80, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel_layer("px", 96, 80, (0, 0, 0));
    for (c, ch) in layer.channels.iter_mut().filter(|c| c.id >= 0).enumerate() {
        ch.data = (0..96 * 80)
            .map(|i| {
                if (i % 96 + i / 96 + c) % 7 < 3 {
                    30u8
                } else {
                    220
                }
            })
            .collect::<Vec<_>>()
            .into();
    }
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    let rendered = current_buffer(&doc, false);
    store_composite(&mut doc, &rendered);
    let mut rust = PictureViewRust {
        doc: Some(doc),
        active_layer: Some("0".to_string()),
        gpu_compute: false,
        ..Default::default()
    };
    let r = &mut rust;
    r.history
        .capture_live(r.doc.as_mut().expect("document"), &None, "Open");
    rust
}

fn red(rust: &PictureViewRust) -> Vec<u8> {
    rust.doc.as_ref().unwrap().layers[0].channels[0]
        .data
        .to_vec()
}

fn blur() -> Filter {
    Filter::GaussianBlur { radius: 3.0 }
}

fn whole() -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: 80,
        right: 96,
    }
}

/// Land a started commit, as the bridge does once the worker is done.
fn finish(rust: &mut PictureViewRust) -> Option<Landed> {
    commit_finish(rust).ok()
}

#[test]
fn a_background_commit_matches_the_synchronous_one_and_records_its_label() {
    let mut sync = striped();
    apply_filter_active(&mut sync, "gaussian-blur", &[3.0], true).expect("sync commit");

    let mut rust = striped();
    let before = red(&rust);
    assert!(commit_start(
        &mut rust,
        vec![blur()],
        "Filter: Gaussian Blur".into(),
        None
    ));
    assert_eq!(red(&rust), before, "the live document waits for finish");
    let refresh = finish(&mut rust).expect("committed");
    assert!(matches!(refresh, Landed::Region { .. }));
    assert_eq!(red(&rust), red(&sync));
    assert_eq!(rust.history.count(), 2);
    assert_eq!(rust.history.label(1), "Filter: Gaussian Blur");
    // The composite the commit patched is the one a recomposite stores.
    let doc = rust.doc.as_ref().unwrap();
    let mut recomposited = doc.clone();
    store_composite(&mut recomposited, &current_buffer(doc, false));
    assert_eq!(doc.composite.data, recomposited.composite.data);
}

#[test]
fn a_commit_reuses_a_finished_exact_preview() {
    let mut rust = striped();
    assert!(exact_preview_start(&mut rust, vec![blur()], whole(), 0.5));
    while exact_preview_busy(&mut rust) {
        std::thread::yield_now();
    }
    let picture = exact_preview_poll(&mut rust).expect("exact picture");
    assert_eq!((picture.width, picture.height), (48, 40));
    assert!(exact_preview_poll(&mut rust).is_none(), "handed over once");
    // Swap the layer the preview filtered for a marker: a reused preview
    // commits the marker, a fresh filter would not.
    let marker = {
        let preview = rust.filter_jobs.preview.as_mut().unwrap();
        let (_, exact) = preview.done.as_mut().unwrap();
        exact.layer.channels[0].data.as_mut_slice().fill(7);
        exact.layer.clone()
    };
    assert!(commit_start(&mut rust, vec![blur()], "Filter".into(), None));
    finish(&mut rust).expect("committed");
    assert_eq!(red(&rust), marker.channels[0].data.to_vec());
}

#[test]
fn a_newer_exact_preview_supersedes_an_older_one() {
    let mut rust = striped();
    assert!(exact_preview_start(&mut rust, vec![blur()], whole(), 0.5));
    let sharper = Filter::GaussianBlur { radius: 1.0 };
    assert!(exact_preview_start(
        &mut rust,
        vec![sharper.clone()],
        whole(),
        0.5
    ));
    while exact_preview_busy(&mut rust) {
        std::thread::yield_now();
    }
    exact_preview_poll(&mut rust).expect("the latest picture");
    let preview = rust.filter_jobs.preview.as_ref().unwrap();
    assert_eq!(preview.done.as_ref().unwrap().0.filters, vec![sharper]);
}

#[test]
fn a_refused_commit_keeps_the_document_and_history() {
    let mut rust = striped();
    rust.doc.as_mut().unwrap().layers[0].lock =
        pictura_core::LockFlags::default().with(pictura_core::LockFlags::PIXELS, true);
    let before = red(&rust);
    assert!(!commit_start(
        &mut rust,
        vec![blur()],
        "Filter".into(),
        None
    ));
    assert!(rust.filter_error.is_some());
    assert!(rust.filter_jobs.commit.is_none());
    assert_eq!(red(&rust), before);
    assert_eq!(rust.history.count(), 1);
}

#[test]
fn the_section_render_matches_a_section_preview_and_leaves_the_document() {
    let section = PsdRect {
        top: 20,
        left: 30,
        bottom: 50,
        right: 70,
    };
    let mut previewed = striped();
    apply_filter_active_region_section(&mut previewed, section);
    let expected = previewed.doc.as_ref().unwrap().layers[0].channels[0]
        .data
        .to_vec();

    let mut rust = striped();
    let before = red(&rust);
    let shown = section_buffer(&mut rust, &blur(), section).expect("section");
    assert_eq!(red(&rust), before, "the document is untouched");
    assert_eq!((shown.width, shown.height), (40, 30));
    for row in 0..30usize {
        for col in 0..40usize {
            let doc_i = (20 + row) * 96 + 30 + col;
            assert_eq!(shown.data[row * 40 + col], expected[doc_i], "({col},{row})");
        }
    }
}

fn apply_filter_active_region_section(rust: &mut PictureViewRust, section: PsdRect) {
    crate::cxxqt_object::impl_filters::apply_filter_active_region(
        rust,
        "gaussian-blur",
        &[3.0],
        false,
        Some(section),
    )
    .expect("section preview");
}
