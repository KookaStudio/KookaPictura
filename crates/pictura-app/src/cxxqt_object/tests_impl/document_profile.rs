use super::*;

/// The bridge QObject is C++-constructed (cxx-qt 0.10 exposes no Rust
/// constructor), so this replays the public `convert_profile(1)` + `undo()`
/// sequence against the same codec and `History` and checks the restored bytes.
#[test]
fn convert_profile_is_byte_reversible_through_history() {
    let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.composite
        .replace_data(vec![200, 10, 100, 20, 50, 30].into())
        .expect("valid composite plane");
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("base", 2, 1, (200, 100, 50))]);
    let original = doc.clone();
    let mut history = History::default();
    history.capture(
        Snapshot {
            quick_mask: None,
            doc: doc.clone(),
            selection: None,
        },
        "Open",
    );
    assert!(pictura_codec::convert_document(
        &mut doc,
        &pictura_codec::Profile::adobe_rgb()
    ));
    let adobe = Some(pictura_codec::Profile::adobe_rgb().to_icc().into());
    assert_eq!(doc.source.working_profile(), &adobe);
    assert_ne!(
        doc.composite, original.composite,
        "convert changes the composite"
    );
    assert_ne!(
        doc.layers, original.layers,
        "convert changes the layer pixels"
    );
    history.capture(
        Snapshot {
            quick_mask: None,
            doc: doc.clone(),
            selection: None,
        },
        "Convert to Profile",
    );

    assert_eq!(history.depth(), 1, "exactly one undo step");
    let restored = history.undo().expect("one undo step");
    assert_eq!(restored.doc.composite, original.composite);
    assert_eq!(restored.doc.layers, original.layers);
    assert_eq!(
        restored.doc.source.working_profile(),
        original.source.working_profile()
    );
    assert!(restored.doc.source.working_profile().is_none());
}
/// Bridge-equivalent byte-level check for `assign_profile(1)` + `undo()`:
/// assign leaves every pixel byte untouched, then undo restores the working
/// profile and the image-resource section exactly.
#[test]
fn assign_profile_retags_and_is_reversible_through_history() {
    let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.composite
        .replace_data(vec![200, 10, 100, 20, 50, 30].into())
        .expect("valid composite plane");
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("base", 2, 1, (200, 100, 50))]);
    let original = doc.clone();
    let mut history = History::default();
    history.capture(
        Snapshot {
            quick_mask: None,
            doc: doc.clone(),
            selection: None,
        },
        "Open",
    );
    pictura_codec::assign_document_profile(&mut doc, &pictura_codec::Profile::adobe_rgb());
    let adobe = Some(pictura_codec::Profile::adobe_rgb().to_icc().into());
    assert_eq!(
        doc.composite, original.composite,
        "assign leaves the composite bytes untouched"
    );
    assert_eq!(
        doc.layers, original.layers,
        "assign leaves the layer bytes untouched"
    );
    assert_eq!(doc.source.working_profile(), &adobe);
    assert_ne!(
        doc.image_resources, original.image_resources,
        "assign writes resource 1039"
    );
    history.capture(
        Snapshot {
            quick_mask: None,
            doc: doc.clone(),
            selection: None,
        },
        "Assign Profile",
    );

    assert_eq!(history.depth(), 1, "exactly one undo step");
    let restored = history.undo().expect("one undo step");
    assert_eq!(restored.doc.composite, original.composite);
    assert_eq!(restored.doc.layers, original.layers);
    assert_eq!(
        restored.doc.source.working_profile(),
        original.source.working_profile()
    );
    assert_eq!(restored.doc.image_resources, original.image_resources);
}

/// The first frame `open` builds (and `File > Revert`, which reuses `open`) must
/// show the working-space conversion, not the raw embedded-space composite: an
/// Adobe RGB document displays through `buffer_to_srgb`, so its first frame
/// differs from the untagged one.
#[test]
fn open_first_frame_converts_the_document_profile() {
    let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.composite
        .replace_data(vec![200, 10, 100, 20, 50, 30].into())
        .expect("valid composite plane");
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("base", 2, 1, (200, 100, 50))]);
    doc.source
        .set_working_profile(Some(pictura_codec::Profile::adobe_rgb().to_icc().into()));

    let rendered = current_buffer(&doc, false);
    let converted = pictura_codec::buffer_to_srgb(&doc, &rendered);
    assert_ne!(
        *converted, rendered,
        "an Adobe RGB document must be converted for display"
    );
    let first_frame = buffer_to_image(&converted);
    let raw_frame = buffer_to_image(&rendered);
    assert_ne!(
        first_frame.pixel_color(0, 0),
        raw_frame.pixel_color(0, 0),
        "the displayed first frame must not be the embedded-space pixels"
    );
}
