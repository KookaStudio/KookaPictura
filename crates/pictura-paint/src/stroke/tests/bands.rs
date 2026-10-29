use super::*;

#[test]
fn a_stroke_forks_only_touched_bands() {
    let (w, h) = (8usize, 128usize);
    let mut layer = Layer {
        name: "px".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        },
        channels: vec![Channel {
            id: 0,
            data: vec![1u8; w * h].into(),
        }],
        ..Default::default()
    };
    layer.channels[0].data.fracture(w);
    let holder = layer.channels[0].data.clone();

    let ch: PlaneIndex = [Some(0), None, None, None];
    let mut scratch = vec![0u8; w * h];
    let mut applied = vec![0u8; w * h];
    let cuts = crate::stroke::raster::plane_slices(&mut layer, ch, w, h, 0, 64);
    let bands = crate::stroke::raster::split_bands(0, 64, w, &mut scratch, &mut applied, cuts);
    for mut band in bands {
        if let Some(plane) = band.planes[0].as_mut() {
            plane[0] = 9;
        }
    }

    assert!(
        !layer.channels[0].data.shares_band(0, &holder),
        "the touched band forked"
    );
    assert!(
        layer.channels[0].data.shares_band(1, &holder),
        "the untouched band stayed shared"
    );
    assert_eq!(holder.band(1).expect("band 1")[0], 1);
}
