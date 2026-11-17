//! The document global light: `uglg` effects follow it, local effects do not.

use super::*;
use pictura_core::GlobalLight;

fn compose_with_light(shadowed: Layer, angle: i32) -> PixelBuffer {
    let backdrop = solid(
        "Backdrop",
        full(9, 9),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    let mut d = doc(9, 9, vec![backdrop, shadowed]);
    d.global_light = GlobalLight {
        angle,
        altitude: 30,
    };
    composite_rgba(&d)
}

fn shadowed(global: bool, distance: f64) -> Layer {
    spec_shadow(&ShadowSpec {
        global,
        distance,
        ..Default::default()
    })
}

#[test]
fn global_light_angle_orients_a_shadow() {
    let left = compose_with_light(shadowed(true, 4.0), 0);
    let down = compose_with_light(shadowed(true, 4.0), 90);
    assert_ne!(px(&left, 1, 4), [255, 255, 255, 255], "angle 0 casts left");
    assert_eq!(px(&down, 1, 4), [255, 255, 255, 255]);
    assert_ne!(px(&down, 4, 7), [255, 255, 255, 255], "angle 90 casts down");
    assert_eq!(px(&left, 4, 7), [255, 255, 255, 255]);
}

#[test]
fn global_light_ignored_when_local() {
    let zero = compose_with_light(shadowed(false, 4.0), 0);
    let ninety = compose_with_light(shadowed(false, 4.0), 90);
    assert_eq!(zero.data.as_ref(), ninety.data.as_ref());
}

#[test]
fn global_light_change_rebuilds_passes() {
    let backdrop = solid(
        "Backdrop",
        full(9, 9),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    let mut d = doc(9, 9, vec![backdrop, shadowed(true, 4.0)]);
    d.global_light = GlobalLight {
        angle: 0,
        altitude: 30,
    };
    crate::purge_source_cache();
    let first = composite_rgba(&d);
    d.global_light.angle = 90;
    let second = composite_rgba(&d);
    assert_ne!(first.data.as_ref(), second.data.as_ref());
}

#[test]
fn paint_reach_is_conservative_for_global_shadows() {
    let reach = crate::paint_reach(&shadowed(true, 10.0));
    assert!(reach >= (10.0 * std::f64::consts::SQRT_2).ceil() as i32);
}
