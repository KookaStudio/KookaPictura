use super::*;
use pictura_core::{assert_no_materialization, Coverage, Plane};

#[test]
fn healing_reads_shared_coverage() {
    let (original, mut bytes) = patch_doc(PATCH_FIELD, (20, 20, 20, 20));
    bytes[20 * 140 + 25] = 128;
    for options in [
        PatchOptions {
            dx: 60,
            ..Default::default()
        },
        PatchOptions {
            dx: 60,
            destination: true,
            ..Default::default()
        },
        PatchOptions {
            dx: 60,
            content_aware: true,
            ..Default::default()
        },
        PatchOptions {
            dx: 60,
            transparent: true,
            ..Default::default()
        },
    ] {
        let flat: Plane<u8> = bytes.clone().into();
        let mut tiled = flat.clone();
        assert!(tiled.tile(140, 60));
        let (a, b) = (
            Coverage::for_canvas(&flat, 140, 60).unwrap(),
            Coverage::for_canvas(&tiled, 140, 60).unwrap(),
        );
        let (mut expected, mut actual) = (original.clone(), original.clone());
        let want = super::super::patch_layer(&mut expected, "0", &a, options);
        let got =
            assert_no_materialization(|| super::super::patch_layer(&mut actual, "0", &b, options));
        assert_eq!(got, want);
        assert_eq!(actual, expected);
    }
    let (original, bytes) = move_doc();
    let flat: Plane<u8> = bytes.into();
    let mut tiled = flat.clone();
    assert!(tiled.tile(96, 48));
    for extend in [false, true] {
        let (mut expected, mut actual) = (original.clone(), original.clone());
        let a = Coverage::for_canvas(&flat, 96, 48).unwrap();
        let b = Coverage::for_canvas(&tiled, 96, 48).unwrap();
        assert_eq!(
            super::super::move_layer(&mut expected, "0", &a, drag(40, extend)),
            assert_no_materialization(|| super::super::move_layer(
                &mut actual,
                "0",
                &b,
                drag(40, extend)
            ))
        );
        assert_eq!(actual, expected);
    }
}
