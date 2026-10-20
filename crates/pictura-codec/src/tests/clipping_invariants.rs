use pictura_core::{Document, LayerStack};

#[test]
fn malformed_clip_round_trips_unchanged() {
    let mut doc = Document::from_rgba("Background", 2, 2, &[255; 16]);
    doc.layers[0].background = true;
    doc.layers[0].clipping = true;

    let first = crate::write_psd(&doc).expect("write a clipped bottom");
    let read = crate::read_psd(&first).expect("read a clipped bottom");
    assert!(read.layers[0].clipping, "the clip survives the read");
    let second = crate::write_psd(&read).expect("write again");
    assert_eq!(first, second, "an untouched malformed file round-trips");
}

#[test]
fn clip_on_a_background_round_trips_unchanged() {
    let mut doc = Document::from_rgba("Background", 2, 2, &[255; 16]);
    doc.layers[0].background = true;
    let mut top = doc.layers[0].clone();
    top.name = "top".into();
    top.background = false;
    top.clipping = true;
    doc.layers = LayerStack::from_layers(vec![doc.layers[0].clone(), top]);

    let first = crate::write_psd(&doc).expect("write a background-based clip");
    let read = crate::read_psd(&first).expect("read a background-based clip");
    assert!(read.layers[0].background);
    assert!(read.layers[1].clipping);
    let second = crate::write_psd(&read).expect("write again");
    assert_eq!(first, second);
}
