use super::*;

/// 8-layer 4000×4000 open: retained tracked-plane bytes across the live document
/// and the history after open and after one band write, plus the materializations
/// and time of the first capture. No pass/fail budget.
#[test]
#[ignore = "4000x4000 open profile; run explicitly with --ignored --nocapture"]
fn large_document_open_profile() {
    let (w, h) = (4000u32, 4000u32);
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(
        (0..8)
            .map(|i| pixel_layer(&format!("layer {i}"), w, h, (i as u8 * 24, 0, 0)))
            .collect(),
    );
    let mut history = crate::history::History::default();
    let target = LayerTarget::default();

    let open = std::time::Instant::now();
    history.capture_live(&mut doc, &None, &target, "Open");
    ms("large_document_open capture", open.elapsed());
    println!(
        "large_document_open unique={}B history_only={}B",
        history.retained_plane_bytes_with(&doc),
        history.retained_plane_bytes(),
    );

    let plane = &mut doc.layers[0].channels[0].data;
    plane.fracture(w as usize);
    plane.write_band(0).expect("a band")[0] ^= 1;
    pictura_core::reset_materializations();
    let paint = std::time::Instant::now();
    history.capture_live(&mut doc, &None, &target, "Paint");
    ms("large_document_open first_capture", paint.elapsed());
    println!(
        "large_document_open materializations={} unique={}B",
        pictura_core::materializations(),
        history.retained_plane_bytes_with(&doc),
    );
}
