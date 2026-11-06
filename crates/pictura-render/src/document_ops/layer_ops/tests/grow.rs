//! Empty-rect layer creation and conversion (#1208).

use super::*;

#[test]
fn neutral_fill_layer_is_canvas_sized() {
    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
    let spec = NewLayerSpec {
        name: "Neutral".into(),
        blend: BlendMode::Multiply,
        neutral_fill: true,
        ..NewLayerSpec::default()
    };
    let path = add_layer_full(&mut doc, "", &spec);
    assert_eq!(path, "1");
    let added = &doc.layers[1];
    assert_eq!(added.rect, rect(4, 4));
    assert_eq!(added.channels[0].data.len(), 16);
    assert!(added.channels[0].data.iter().all(|&v| v == 255));
}

#[test]
fn background_from_an_empty_layer_fills_the_canvas() {
    let mut doc = doc_with(vec![pixel_layer("other", 4, 4, 10)]);
    let path = add_layer_in(&mut doc, "", "Empty");
    assert!(background_from_layer(&mut doc, &path));
    let background = &doc.layers[0];
    assert!(background.background);
    assert_eq!(background.rect, rect(4, 4), "a Background is canvas-sized");
    for channel in &background.channels {
        assert!(
            channel.data.iter().all(|&v| v == 255),
            "channel {} is opaque white",
            channel.id
        );
    }
}
