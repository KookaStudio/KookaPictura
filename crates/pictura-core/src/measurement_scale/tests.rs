use super::*;

#[test]
fn measurement_scale_converts_length_and_area() {
    let scale = MeasurementScale::new(50, 1.0, "µm").unwrap();
    assert_eq!(scale.logical_distance(100.0), 2.0);
    assert!((scale.logical_area(2500.0) - 1.0).abs() < 1e-12);
    let default = MeasurementScale::default();
    assert_eq!(default.logical_distance(100.0), 100.0);
    assert_eq!(default.logical_area(2500.0), 2500.0);
    assert_eq!(default.units(), "px");
}

#[test]
fn measurement_scale_rejects_invalid_values() {
    for (pixels, logical, units) in [
        (0, 1.0, "mm"),
        (-5, 1.0, "mm"),
        (50, 0.0, "mm"),
        (50, -2.0, "mm"),
        (50, f64::NAN, "mm"),
        (50, f64::INFINITY, "mm"),
        (i32::MAX, f64::from_bits(1), "mm"),
        (50, 1.0, " \t"),
    ] {
        assert!(MeasurementScale::new(pixels, logical, units).is_none());
    }
}
