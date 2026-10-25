use pictura_core::PixelBuffer;

use super::*;

/// A `w`×`h` gradient (red across, green down, blue 128), opaque when
/// `planes` is 4.
fn layer(w: u32, h: u32, planes: u8) -> PixelBuffer {
    let mut buf = PixelBuffer::new(w, h, planes);
    let n = buf.pixel_count();
    for y in 0..h as usize {
        for x in 0..w as usize {
            let i = y * w as usize + x;
            buf.data[i] = (x * 255 / w as usize) as u8;
            buf.data[n + i] = (y * 255 / h as usize) as u8;
            buf.data[2 * n + i] = 128;
            if planes == 4 {
                buf.data[3 * n + i] = 255;
            }
        }
    }
    buf
}

/// A flat grey layer.
fn grey(w: u32, h: u32, planes: u8) -> PixelBuffer {
    let mut buf = PixelBuffer::new(w, h, planes);
    let n = buf.pixel_count();
    buf.data[..3 * n].fill(128);
    if planes == 4 {
        buf.data[3 * n..].fill(255);
    }
    buf
}

fn px(buf: &PixelBuffer, x: usize, y: usize) -> Vec<u8> {
    let n = buf.pixel_count();
    let i = y * buf.width as usize + x;
    (0..buf.channels as usize)
        .map(|c| buf.data[c * n + i])
        .collect()
}

fn corrected(mut buf: PixelBuffer, p: &LensCorrectionParams) -> PixelBuffer {
    lens_correction(&mut buf, p).unwrap();
    buf
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

#[test]
fn default_params_are_bit_identical() {
    let src = layer(40, 30, 4);
    let mut p = LensCorrectionParams {
        vignette_midpoint: 10.0,
        angle: 360.0,
        edge: LensEdge::Black,
        auto_scale: false,
        ..Default::default()
    };
    assert_eq!(corrected(src.clone(), &p).data, src.data);
    p.profile = Some(LensModel::NONE);
    assert_eq!(corrected(src.clone(), &p).data, src.data);
}

#[test]
fn geometry_free_settings_keep_every_sample_position() {
    // Vignette alone resamples at the pixel centres, so a zero gain region
    // (the centre) is untouched.
    let src = layer(40, 30, 3);
    let p = LensCorrectionParams {
        vignette_amount: -50.0,
        vignette_midpoint: 100.0,
        ..Default::default()
    };
    let out = corrected(src.clone(), &p);
    assert_eq!(px(&out, 20, 15), px(&src, 20, 15));
}

#[test]
fn distortion_keeps_centre_and_moves_edges_monotonically() {
    let (w, h) = (200, 100);
    let centre = [100.0, 50.0];
    let map = |d: f64| {
        let p = LensCorrectionParams {
            distortion: d,
            ..Default::default()
        };
        LensMap::new(&p, w, h)
    };
    for d in [-100.0, -30.0, 30.0, 100.0] {
        assert!(dist(map(d).source(centre).unwrap(), centre) < 1e-9);
    }
    let edge = [200.0, 50.0];
    let mut last = f64::INFINITY;
    for d in [-100.0, -50.0, 0.0, 50.0, 100.0] {
        let r = dist(map(d).source(edge).unwrap(), centre);
        assert!(r < last, "Remove Distortion {d} moved the edge to {r}");
        last = r;
    }
    // Along a ray the source radius still grows at the slider's end.
    let m = map(100.0);
    let mut prev = 0.0;
    for i in 1..=50 {
        let r = dist(
            m.source([100.0 + 2.0 * i as f64, 50.0 + i as f64]).unwrap(),
            centre,
        );
        assert!(r > prev);
        prev = r;
    }
}

#[test]
fn fringe_scales_only_its_channel() {
    let src = layer(64, 48, 4);
    let n = src.pixel_count();
    for (c, fringe) in [[100.0, 0.0, 0.0], [0.0, -100.0, 0.0], [0.0, 0.0, 100.0]]
        .into_iter()
        .enumerate()
    {
        let out = corrected(
            layer(64, 48, 4),
            &LensCorrectionParams {
                fringe,
                edge: LensEdge::Extension,
                ..Default::default()
            },
        );
        for k in 0..4 {
            let same = out.data[k * n..(k + 1) * n] == src.data[k * n..(k + 1) * n];
            // The blue plane is flat, so scaling it changes nothing.
            assert_eq!(same, k != c || c == 2, "fringe {c} channel {k}");
        }
    }
    let map = LensMap::new(
        &LensCorrectionParams {
            fringe: [100.0, 0.0, -100.0],
            ..Default::default()
        },
        64,
        48,
    );
    let s = [60.0, 40.0];
    assert!(dist(map.channel(s, 0), [32.0, 24.0]) > dist(s, [32.0, 24.0]));
    assert_eq!(map.channel(s, 1), s);
    assert!(dist(map.channel(s, 2), [32.0, 24.0]) < dist(s, [32.0, 24.0]));
}

#[test]
fn vignette_darkens_and_lightens_corners() {
    let vignette = |amount: f64, midpoint: f64| {
        corrected(
            grey(60, 40, 3),
            &LensCorrectionParams {
                vignette_amount: amount,
                vignette_midpoint: midpoint,
                ..Default::default()
            },
        )
    };
    let dark = vignette(-100.0, 50.0);
    assert_eq!(px(&dark, 30, 20), vec![128; 3]);
    assert!(px(&dark, 0, 0)[0] < 40);
    let light = vignette(100.0, 50.0);
    assert!(px(&light, 0, 0)[0] > 200);
    // A lower midpoint reaches further in.
    let wide = vignette(-100.0, 0.0);
    let narrow = vignette(-100.0, 100.0);
    assert!(px(&wide, 15, 10)[0] < px(&narrow, 15, 10)[0]);
}

#[test]
fn vertical_perspective_narrows_the_top_and_mirrors() {
    let (w, h) = (200u32, 100u32);
    let map = |v: f64| {
        LensMap::new(
            &LensCorrectionParams {
                vertical: v,
                ..Default::default()
            },
            w,
            h,
        )
    };
    let up = map(60.0);
    assert!(dist(up.source([100.0, 50.0]).unwrap(), [100.0, 50.0]) < 1e-6);
    // The top is narrower: its corner shows past the layer's left edge,
    // the bottom's inside it.
    assert!(up.source([0.5, 0.5]).unwrap()[0] < 0.0);
    assert!(up.source([0.5, 99.5]).unwrap()[0] > 0.0);
    // The opposite value is the same correction flipped top to bottom.
    let down = map(-60.0);
    for q in [[10.0, 10.0], [150.0, 30.0], [80.0, 95.0]] {
        let a = up.source(q).unwrap();
        let b = down.source([q[0], 100.0 - q[1]]).unwrap();
        assert!((a[0] - b[0]).abs() < 1e-6 && (a[1] - (100.0 - b[1])).abs() < 1e-6);
    }
}

#[test]
fn horizontal_perspective_narrows_the_right_side() {
    let map = LensMap::new(
        &LensCorrectionParams {
            horizontal: 60.0,
            ..Default::default()
        },
        200,
        100,
    );
    assert!(map.source([199.5, 0.5]).unwrap()[1] < 0.0);
    assert!(map.source([0.5, 0.5]).unwrap()[1] > 0.0);
}

#[test]
fn vertical_perspective_converges_verticals() {
    // A layer whose verticals lean in toward the top (shot looking up) is
    // undone by a negative Vertical Perspective: the output's vertical
    // lines map to source lines that converge upward.
    let map = LensMap::new(
        &LensCorrectionParams {
            vertical: -50.0,
            ..Default::default()
        },
        200,
        200,
    );
    let top = map.source([20.0, 10.0]).unwrap();
    let bottom = map.source([20.0, 190.0]).unwrap();
    assert!(top[0] > bottom[0], "{top:?} {bottom:?}");
}

#[test]
fn angle_rotates_clockwise_about_the_centre() {
    let map = LensMap::new(
        &LensCorrectionParams {
            angle: 90.0,
            ..Default::default()
        },
        100,
        100,
    );
    assert!(dist(map.source([50.0, 50.0]).unwrap(), [50.0, 50.0]) < 1e-9);
    // Clockwise: the source's top-left corner shows at the top right.
    assert!(dist(map.source([99.5, 0.5]).unwrap(), [0.5, 0.5]) < 1e-9);
}

#[test]
fn straighten_levels_a_line() {
    let (a, b) = ([20.0, 40.0], [180.0, 56.0]);
    let dev = straighten_angle(a, b);
    assert!((dev - (16.0f64 / 160.0).atan().to_degrees()).abs() < 1e-9);
    assert!(
        (straighten_angle([0.0, 0.0], [1.0, -100.0]) - (0.01f64).atan().to_degrees()).abs() < 1e-9
    );
    assert!((straighten_angle(b, a) - dev).abs() < 1e-9);
    let map = LensMap::new(
        &LensCorrectionParams {
            angle: -dev,
            ..Default::default()
        },
        200,
        100,
    );
    // A level output row through the centre shows the leaning line.
    let slope = (b[1] - a[1]) / (b[0] - a[0]);
    for x in [10.0, 60.0, 190.0] {
        let s = map.source([x, 50.0]).unwrap();
        assert!(((s[1] - 50.0) - slope * (s[0] - 100.0)).abs() < 1e-9);
    }
}

#[test]
fn scale_zooms_about_the_centre() {
    let map = LensMap::new(
        &LensCorrectionParams {
            scale: 2.0,
            ..Default::default()
        },
        100,
        60,
    );
    assert_eq!(map.source([90.0, 10.0]).unwrap(), [70.0, 20.0]);
}

fn shrunk(planes: u8, edge: LensEdge) -> PixelBuffer {
    corrected(
        layer(40, 40, planes),
        &LensCorrectionParams {
            scale: 0.5,
            edge,
            ..Default::default()
        },
    )
}

#[test]
fn edge_transparency_clears_or_whitens() {
    assert_eq!(px(&shrunk(4, LensEdge::Transparency), 0, 0), vec![0; 4]);
    assert_eq!(px(&shrunk(3, LensEdge::Transparency), 0, 0), vec![255; 3]);
    assert_eq!(px(&shrunk(4, LensEdge::Transparency), 20, 20)[3], 255);
}

#[test]
fn edge_colours_fill_opaque() {
    assert_eq!(px(&shrunk(4, LensEdge::Black), 0, 0), vec![0, 0, 0, 255]);
    assert_eq!(px(&shrunk(4, LensEdge::White), 0, 0), vec![255; 4]);
    assert_eq!(px(&shrunk(3, LensEdge::Black), 39, 39), vec![0; 3]);
}

#[test]
fn edge_extension_repeats_the_edge() {
    let src = layer(40, 40, 4);
    let out = shrunk(4, LensEdge::Extension);
    assert_eq!(px(&out, 0, 0), px(&src, 0, 0));
    assert_eq!(px(&out, 39, 39), px(&src, 39, 39));
}

fn pincushion(auto_scale: bool) -> LensCorrectionParams {
    LensCorrectionParams {
        profile: Some(LensModel {
            distortion: [0.12, 0.0, 0.0],
            ..LensModel::NONE
        }),
        auto_scale,
        ..Default::default()
    }
}

#[test]
fn auto_scale_leaves_no_blank() {
    let alpha = |buf: &PixelBuffer| {
        let n = buf.pixel_count();
        buf.data[3 * n..].iter().copied().min().unwrap()
    };
    assert_eq!(alpha(&corrected(layer(60, 40, 4), &pincushion(false))), 0);
    assert_eq!(alpha(&corrected(layer(60, 40, 4), &pincushion(true))), 255);
    // Auto Scale answers the profile only, not the Custom transform.
    let mut rotated = pincushion(true);
    rotated.angle = 30.0;
    assert_eq!(alpha(&corrected(layer(60, 40, 4), &rotated)), 0);
}

#[test]
fn preview_render_matches_commit() {
    let p = LensCorrectionParams {
        distortion: 40.0,
        fringe: [30.0, 0.0, -30.0],
        vignette_amount: -40.0,
        vertical: 20.0,
        angle: 12.0,
        ..Default::default()
    };
    let src = layer(48, 32, 4);
    let out = corrected(src.clone(), &p);
    let map = LensMap::new(&p, 48, 32);
    let mut view = vec![0u8; 48 * 32 * 4];
    LensSource::from_buffer(&src).render(Some(&map), [0.0; 2], 1.0, (48, 32), &mut view);
    for y in 0..32 {
        for x in 0..48 {
            let v = &view[(y * 48 + x) * 4..][..4];
            let o = px(&out, x, y);
            // The commit un-premultiplies; an opaque pixel may round apart
            // by one level.
            assert_eq!(v[3], o[3], "pixel {x},{y}");
            if o[3] == 255 {
                for c in 0..3 {
                    assert!(v[c].abs_diff(o[c]) <= 1, "pixel {x},{y}");
                }
            }
        }
    }
}

#[test]
fn deterministic() {
    let p = pincushion(true);
    let a = corrected(layer(50, 30, 4), &p);
    let b = corrected(layer(50, 30, 4), &p);
    assert_eq!(a.data, b.data);
}

#[test]
fn zero_scale_is_refused() {
    let mut buf = layer(4, 4, 3);
    let p = LensCorrectionParams {
        scale: 0.0,
        ..Default::default()
    };
    assert!(lens_correction(&mut buf, &p).is_err());
}

#[test]
fn builtin_profiles_parse() {
    let all = builtin_profiles();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].make, "ACME Cameras");
    assert!(all[0].coefficients.distortion[0] < 0.0);
}

#[test]
fn profile_parse_reports_the_line() {
    let text = "[profile]\nname = A\nmake = B\ndistortion = 1 2\n";
    assert_eq!(
        parse_profiles(text).unwrap_err(),
        "line 4: expected three numbers"
    );
    assert!(parse_profiles("name = A\n")
        .unwrap_err()
        .starts_with("line 1:"));
    assert!(parse_profiles("[profile]\nzoom = 1\n")
        .unwrap_err()
        .contains("zoom"));
    assert!(parse_profiles("[profile]\nname = A\n").is_err());
    let ok = parse_profiles("# c\n[profile]\nname = A # x\nmake = B\nca = 1 1 1\n").unwrap();
    assert_eq!(ok[0].name, "A");
    assert_eq!(ok[0].coefficients, LensModel::NONE);
}

#[test]
fn profiles_match_make_then_model_and_lens() {
    let text = "[profile]\nname = Z\nmake = Acme\nmodel = One\nlens = L1\n\
                [profile]\nname = Y\nmake = Acme\nmodel = Two\nlens = L2\n\
                [profile]\nname = X\nmake = Other\nmodel = One\nlens = L1\n";
    let all = parse_profiles(text).unwrap();
    assert_eq!(match_profiles(&all, "acme", "one", ""), vec![0]);
    assert_eq!(match_profiles(&all, "ACME", "", ""), vec![1, 0]);
    assert_eq!(match_profiles(&all, "Acme", "Two", "L1"), vec![0, 1]);
    assert!(match_profiles(&all, "", "One", "L1").is_empty());
    assert!(match_profiles(&all, "Nikon", "", "").is_empty());
    let acme = builtin_profiles();
    assert_eq!(
        match_profiles(acme, "ACME Cameras", "ACME One", ""),
        vec![0]
    );
}

#[test]
fn profile_composes_with_custom() {
    let profile = builtin_profiles()[0].coefficients;
    let vignette_only = LensModel {
        distortion: [0.0; 3],
        ca_scale: [1.0; 3],
        ..profile
    };
    // The profile's falloff is divided out: corners brighten.
    let out = corrected(
        grey(60, 40, 3),
        &LensCorrectionParams {
            profile: Some(vignette_only),
            ..Default::default()
        },
    );
    assert_eq!(px(&out, 30, 20), vec![128; 3]);
    assert!(px(&out, 0, 0)[0] > 150);
    // A custom darkening on top pulls the corner back down.
    let both = corrected(
        grey(60, 40, 3),
        &LensCorrectionParams {
            profile: Some(vignette_only),
            vignette_amount: -60.0,
            vignette_midpoint: 0.0,
            ..Default::default()
        },
    );
    assert!(px(&both, 0, 0)[0] < px(&out, 0, 0)[0]);
}
