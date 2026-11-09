use super::*;
use pictura_core::{BlendMode, Channel, Layer, Plane, PsdRect};

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        left,
        top,
        right,
        bottom,
    }
}

fn plane(values: &[u8]) -> Plane<u8> {
    Plane::build(values.len(), |data| data.copy_from_slice(values))
}

fn layer(rect: PsdRect, rgba: [&[u8]; 4]) -> Layer {
    let mut layer = Layer {
        rect,
        ..Default::default()
    };
    for (id, values) in [0i16, 1, 2, -1].into_iter().zip(rgba) {
        layer.channels.push(Channel {
            id,
            data: plane(values),
        });
    }
    layer
}

fn solid(rect: PsdRect, color: [u8; 3], alpha: u8) -> Layer {
    let n = (rect.width() * rect.height()).max(0) as usize;
    let r = vec![color[0]; n];
    let g = vec![color[1]; n];
    let b = vec![color[2]; n];
    let a = vec![alpha; n];
    layer(rect, [&r, &g, &b, &a])
}

fn channel(layer: &Layer, id: i16) -> &[u8] {
    layer
        .channels
        .iter()
        .find(|c| c.id == id)
        .expect("channel")
        .data
        .as_slice()
}

#[test]
fn fade_opacity_zero_restores_the_original() {
    let original = solid(rect(0, 0, 2, 1), [10, 20, 30], 200);
    let effected = solid(rect(0, 0, 2, 1), [200, 210, 220], 100);
    let out = fade_layer(
        &original,
        &effected,
        &Fade {
            opacity: 0,
            mode: BlendMode::Normal,
        },
    );
    assert_eq!(channel(&out, 0), channel(&original, 0));
    assert_eq!(channel(&out, 1), channel(&original, 1));
    assert_eq!(channel(&out, 2), channel(&original, 2));
    assert_eq!(channel(&out, -1), channel(&original, -1));
}

#[test]
fn fade_opacity_zero_is_transparent_outside_the_original_rect() {
    let mut original = solid(rect(0, 0, 1, 1), [10, 20, 30], 255);
    original.channels.retain(|c| c.id != -1);
    let effected = solid(rect(1, 0, 2, 1), [200, 210, 220], 255);
    let out = fade_layer(
        &original,
        &effected,
        &Fade {
            opacity: 0,
            mode: BlendMode::Normal,
        },
    );
    assert_eq!(channel(&out, -1), &[255, 0]);
    assert_eq!(channel(&out, 0), &[10, 0]);
}

#[test]
fn fade_opacity_one_hundred_normal_returns_the_effect() {
    let original = solid(rect(0, 0, 2, 1), [10, 20, 30], 200);
    let effected = solid(rect(0, 0, 2, 1), [200, 210, 220], 100);
    let out = fade_layer(
        &original,
        &effected,
        &Fade {
            opacity: 100,
            mode: BlendMode::Normal,
        },
    );
    assert_eq!(out, effected);
}

#[test]
fn fade_normal_mode_is_a_linear_interpolation() {
    let original = solid(rect(0, 0, 1, 1), [20, 40, 60], 255);
    let effected = solid(rect(0, 0, 1, 1), [220, 140, 100], 255);
    let out = fade_layer(
        &original,
        &effected,
        &Fade {
            opacity: 50,
            mode: BlendMode::Normal,
        },
    );
    assert_eq!(channel(&out, 0), &[120]);
    assert_eq!(channel(&out, 1), &[90]);
    assert_eq!(channel(&out, 2), &[80]);
}

#[test]
fn fade_multiply_uses_its_w3c_function() {
    let original = solid(rect(0, 0, 1, 1), [64, 64, 64], 255);
    let effected = solid(rect(0, 0, 1, 1), [192, 192, 192], 255);
    let out = fade_layer(
        &original,
        &effected,
        &Fade {
            opacity: 100,
            mode: BlendMode::Multiply,
        },
    );
    assert_eq!(channel(&out, 0), &[48]);
}

#[test]
fn fade_alpha_composites_source_over() {
    let original = solid(rect(0, 0, 1, 1), [0, 0, 0], 0);
    let effected = solid(rect(0, 0, 1, 1), [200, 100, 50], 255);
    let out = fade_layer(
        &original,
        &effected,
        &Fade {
            opacity: 50,
            mode: BlendMode::Normal,
        },
    );
    assert_eq!(channel(&out, -1), &[128]);
    assert_eq!(channel(&out, 0), &[200]);
    assert_eq!(channel(&out, 1), &[100]);
    assert_eq!(channel(&out, 2), &[50]);
}

#[test]
fn fade_keeps_the_effected_non_color_channels() {
    let mut original = solid(rect(0, 0, 1, 1), [10, 20, 30], 255);
    let mut effected = solid(rect(0, 0, 1, 1), [200, 210, 220], 255);
    original.channels.push(Channel {
        id: -2,
        data: plane(&[10]),
    });
    effected.channels.push(Channel {
        id: -2,
        data: plane(&[20]),
    });
    let out = fade_layer(
        &original,
        &effected,
        &Fade {
            opacity: 50,
            mode: BlendMode::Normal,
        },
    );
    assert_eq!(channel(&out, -2), &[20]);
}

#[test]
fn fade_covers_the_union_of_both_rectangles() {
    let original = solid(rect(0, 0, 1, 1), [255, 0, 0], 255);
    let effected = solid(rect(1, 0, 2, 1), [0, 0, 255], 255);
    let out = fade_layer(
        &original,
        &effected,
        &Fade {
            opacity: 50,
            mode: BlendMode::Normal,
        },
    );
    assert_eq!(out.rect, rect(0, 0, 2, 1));
    assert_eq!(channel(&out, 0), &[255, 0]);
    assert_eq!(channel(&out, 2), &[0, 255]);
    assert_eq!(channel(&out, -1), &[255, 128]);
}
