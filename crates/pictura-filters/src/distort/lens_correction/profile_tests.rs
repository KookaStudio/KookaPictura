//! Profiles (#932): the lensfun reader, sub-profiles, matching, ids, and
//! the slot encoding. Every database here is synthetic: invented lenses and
//! coefficients, not lensfun's (CC BY-SA) data.

use super::*;

/// A zoom calibrated at 24 and 70 mm with vignetting at 50 mm, and a
/// second calibration on a 1.5 crop.
const ZOOM: &str = r#"<?xml version="1.0"?>
<lensdatabase version="1">
  <camera><maker>ACME Cameras</maker><model>ACME One</model><cropfactor>2</cropfactor></camera>
  <lens>
    <maker>ACME Cameras</maker>
    <model>ACME Cameras 24-70mm f/2.8 &amp; more</model>
    <model lang="de">ACME Cameras 24-70mm f/2,8</model>
    <mount>ACME</mount>
    <cropfactor>1</cropfactor>
    <calibration>
      <distortion model="ptlens" focal="24" a="0.01" b="-0.03" c="0.005"/>
      <distortion model="poly3" focal="70" k1="0.02"/>
      <tca model="linear" focal="24" kr="1.0003" kb="0.9997"/>
      <tca model="poly3" focal="70" vr="1.0001" vb="0.9999" cr="0.0002" cb="-0.0002" br="0.0001" bb="-0.0001"/>
      <vignetting model="pa" focal="50" aperture="2.8" distance="10" k1="-0.3" k2="0.1" k3="-0.02"/>
      <vignetting model="pa" focal="50" aperture="4" distance="10" k1="-0.2" k2="0.05" k3="0"/>
    </calibration>
    <calibration cropfactor="1.5" aspect-ratio="3:2">
      <distortion model="poly5" focal="50" k1="-0.02" k2="0.003"/>
    </calibration>
  </lens>
</lensdatabase>
"#;

#[test]
fn lensfun_xml_yields_profiles_and_subprofiles() {
    let all = parse_lensfun(ZOOM).unwrap();
    assert_eq!(all.len(), 2, "one profile per calibration");
    let p = &all[0];
    assert_eq!(p.name, "ACME Cameras 24-70mm f/2.8 & more (crop 1)");
    assert_eq!(p.make, "ACME Cameras");
    assert_eq!(p.model, "", "a lens profile suits any camera");
    assert_eq!(p.lens, "ACME Cameras 24-70mm f/2.8 & more");
    assert!(!p.raw);
    let labels: Vec<String> = p.subs.iter().map(SubProfile::label).collect();
    assert_eq!(
        labels,
        ["24 mm", "50 mm, f/2.8, 10 m", "50 mm, f/4, 10 m", "70 mm"]
    );
    assert!(p.subs.iter().all(|s| s.crop == Some(1.0)));
    // 50 mm is nearer 70 than 24: it takes 70 mm's poly3 and TCA.
    let at50 = p.subs[1].coefficients;
    assert_eq!(at50.distortion, p.subs[3].coefficients.distortion);
    assert_eq!(at50.ca, p.subs[3].coefficients.ca);
    assert_eq!(at50.vignette, [-0.3, 0.1, -0.02]);
    assert_eq!(p.subs[3].coefficients.vignette, [0.0; 3]);
    assert!(
        p.subs[0].coefficients.distortion[0] != 0.0,
        "ptlens has an r term"
    );
    let q = &all[1];
    assert_eq!(q.name, "ACME Cameras 24-70mm f/2.8 & more (crop 1.5)");
    assert_eq!(q.subs.len(), 1);
    assert_eq!(q.crop(), Some(1.5));
}

#[test]
fn lensfun_skips_what_it_cannot_model() {
    let text = r#"<lensdatabase>
      <lens><maker>A</maker><model>A Fish 8mm</model><type>fisheye</type><cropfactor>1</cropfactor>
        <calibration><distortion model="poly3" focal="8" k1="0.1"/></calibration></lens>
      <lens><maker>A</maker><model>A Bad 50mm</model><cropfactor>1</cropfactor>
        <calibration><distortion model="poly3" focal="50" k1="x"/></calibration></lens>
      <lens><maker>A</maker><model>A Acm only</model><cropfactor>1</cropfactor>
        <calibration><distortion model="acm" focal="35" k1="0.1"/></calibration></lens>
      <lens><maker>A</maker><model>A 35mm</model><type>rectilinear</type><cropfactor>1</cropfactor>
        <calibration>
          <distortion model="acm" focal="35" k1="0.1"/>
          <vignetting model="pa" focal="35" aperture="2" distance="1" k1="-0.1"/>
        </calibration></lens>
    </lensdatabase>"#;
    let all = parse_lensfun(text).unwrap();
    assert_eq!(all.len(), 1, "{all:?}");
    assert_eq!(all[0].name, "A 35mm");
    assert_eq!(all[0].subs.len(), 1);
    assert_eq!(all[0].subs[0].coefficients.distortion, [0.0; 6]);
    assert_eq!(all[0].subs[0].coefficients.vignette, [-0.1, 0.0, 0.0]);
    assert!(parse_lensfun("<lensdatabase><lens><maker>A</lens>").is_err());
}

/// The map's source radius (pixels) for an output point at `px` pixels
/// right of the centre of a 300×200 layer.
fn source_radius(model: LensModel, px: f64) -> f64 {
    let map = LensMap::new(
        &LensCorrectionParams {
            profile: Some(model),
            auto_scale: false,
            ..Default::default()
        },
        300,
        200,
    );
    let s = map.source([150.0 + px, 100.0]).unwrap();
    (s[0] - 150.0).hypot(s[1] - 100.0)
}

fn one(xml_calibration: &str) -> LensModel {
    let text = format!(
        "<lensdatabase><lens><maker>A</maker><model>A 50mm</model><cropfactor>1</cropfactor>\
         <aspect-ratio>3:2</aspect-ratio><calibration>{xml_calibration}</calibration></lens>\
         </lensdatabase>"
    );
    parse_lensfun(&text).unwrap()[0].subs[0].coefficients
}

#[test]
fn lensfun_coefficients_convert_to_the_half_diagonal() {
    // On a 300×200 layer (the calibration's 3:2) lensfun's distortion and
    // TCA radius 1 is 100 px, half the shorter side; vignetting's 1 is the
    // corner.
    let close = |a: f64, b: f64| (a - b).abs() < 1e-9 * b.abs().max(1.0);
    for px in [10.0, 60.0, 100.0, 150.0, 180.0] {
        let r = px / 100.0;
        let (k1, k2): (f64, f64) = (-0.02, 0.003);
        let poly5 = one(r#"<distortion model="poly5" focal="50" k1="-0.02" k2="0.003"/>"#);
        let expect = 100.0 * r * (1.0 + k1 * r * r + k2 * r.powi(4));
        assert!(close(source_radius(poly5, px), expect), "poly5 at {px}");

        let k1 = 0.02;
        let d = 1.0 - k1;
        let poly3 = one(r#"<distortion model="poly3" focal="50" k1="0.02"/>"#);
        let u = r / d;
        let expect = 100.0 * u * (1.0 - k1 + k1 * u * u);
        assert!(close(source_radius(poly3, px), expect), "poly3 at {px}");

        let (a, b, c) = (0.01, -0.03, 0.005);
        let d = 1.0 - a - b - c;
        let ptlens = one(r#"<distortion model="ptlens" focal="50" a="0.01" b="-0.03" c="0.005"/>"#);
        let u = r / d;
        let expect = 100.0 * u * (a * u.powi(3) + b * u * u + c * u + d);
        assert!(close(source_radius(ptlens, px), expect), "ptlens at {px}");
    }
    let tca = one(
        r#"<tca model="poly3" focal="50" vr="1.0001" vb="0.9999" cr="0.0002" cb="-0.0002" br="0.0001" bb="-0.0001"/>"#,
    );
    let map = LensMap::new(
        &LensCorrectionParams {
            profile: Some(tca),
            ..Default::default()
        },
        300,
        200,
    );
    let s: [f64; 2] = [230.0, 160.0];
    let rs = (s[0] - 150.0).hypot(s[1] - 100.0);
    let r = rs / 100.0;
    let radius = |p: [f64; 2]| (p[0] - 150.0).hypot(p[1] - 100.0);
    assert!(close(
        radius(map.channel(s, 0)),
        rs * (1.0001 + 0.0002 * r + 0.0001 * r * r)
    ));
    assert!(close(radius(map.channel(s, 1)), rs));
    assert!(close(
        radius(map.channel(s, 2)),
        rs * (0.9999 - 0.0002 * r - 0.0001 * r * r)
    ));
    let pa = one(
        r#"<vignetting model="pa" focal="50" aperture="2" distance="1" k1="-0.3" k2="0.1" k3="-0.02"/>"#,
    );
    let map = LensMap::new(
        &LensCorrectionParams {
            profile: Some(pa),
            ..Default::default()
        },
        300,
        200,
    );
    let r2 = (rs / 300f64.hypot(200.0) * 2.0).powi(2);
    let falloff = 1.0 - 0.3 * r2 + 0.1 * r2 * r2 - 0.02 * r2.powi(3);
    assert!(close(map.gain(s), 1.0 / falloff));
}

/// `1 + c1 r + … + c6 r⁶`.
fn distortion_gain(m: &LensModel, r: f64) -> f64 {
    1.0 + m
        .distortion
        .iter()
        .enumerate()
        .map(|(i, c)| c * r.powi(i as i32 + 1))
        .sum::<f64>()
}

#[test]
fn profile_rescales_to_the_camera_crop() {
    let sub = SubProfile {
        crop: Some(1.0),
        ..SubProfile::plain(LensModel {
            distortion: [0.01, -0.1, 0.02, 0.03, 0.0, -0.01],
            ca: [
                [1.001, 0.002, 0.003],
                [1.0, 0.0, 0.0],
                [0.999, -0.002, 0.001],
            ],
            vignette: [-0.3, 0.1, -0.02],
        })
    };
    assert_eq!(sub.resolve(Some(1.0)), sub.coefficients);
    assert_eq!(sub.resolve(None), sub.coefficients);
    assert_eq!(
        SubProfile::plain(sub.coefficients).resolve(Some(2.0)),
        sub.coefficients
    );
    // A crop-2 sensor's corner is the calibration's half-radius.
    let small = sub.resolve(Some(2.0));
    let (cal, cam) = (sub.coefficients, small);
    for r in [0.1, 0.5, 1.0] {
        let t = r * 0.5;
        assert!((distortion_gain(&cam, r) - distortion_gain(&cal, t)).abs() < 1e-12);
        let ca = |m: &LensModel, r: f64| m.ca[0][0] + m.ca[0][1] * r + m.ca[0][2] * r * r;
        assert!((ca(&cam, r) - ca(&cal, t)).abs() < 1e-12);
        let v = |m: &LensModel, r: f64| {
            let r2 = r * r;
            m.vignette[0] * r2 + m.vignette[1] * r2 * r2 + m.vignette[2] * r2.powi(3)
        };
        assert!((v(&cam, r) - v(&cal, t)).abs() < 1e-12);
    }
}

#[test]
fn profile_parse_reads_subprofiles_and_raw() {
    let text = "[profile]\nname = Z\nmake = Acme\nraw = yes\ndistortion = -0.1 0 0\n\
                [sub]\nfocal = 24\naperture = 2.8\n\
                [sub]\nfocal = 70\ndistortion = 0.05 0 0\nvignette = -0.2 0 0\n\
                [profile]\nname = Y\nmake = Acme\n";
    let all = parse_profiles(text).unwrap();
    let z = &all[0];
    assert!(z.raw);
    assert_eq!(z.subs.len(), 2);
    assert_eq!(z.subs[0].label(), "24 mm, f/2.8");
    assert_eq!(
        z.subs[0].coefficients.distortion,
        LensModel::brown([-0.1, 0.0, 0.0])
    );
    assert_eq!(z.subs[1].focal, Some(70.0));
    assert_eq!(
        z.subs[1].coefficients.distortion,
        LensModel::brown([0.05, 0.0, 0.0])
    );
    assert_eq!(z.subs[1].coefficients.vignette, [-0.2, 0.0, 0.0]);
    assert!(z.subs.iter().all(|s| s.crop.is_none()));
    let y = &all[1];
    assert!(!y.raw);
    assert_eq!(y.subs, vec![SubProfile::plain(LensModel::NONE)]);
    assert_eq!(y.subs[0].label(), "Default");

    assert_eq!(
        parse_profiles("[sub]\n").unwrap_err(),
        "line 1: expected [profile]"
    );
    assert_eq!(
        parse_profiles("[profile]\nname = A\nmake = B\nfocal = 50\n").unwrap_err(),
        "line 4: `focal` belongs in a [sub]"
    );
    assert_eq!(
        parse_profiles("[profile]\nname = A\nmake = B\n[sub]\nname = C\n").unwrap_err(),
        "line 5: `name` belongs before the first [sub]"
    );
    assert!(parse_profiles("[profile]\nname = A\nmake = B\nraw = maybe\n").is_err());
    assert!(parse_profiles("[profile]\nname = A\nmake = B\n[sub]\nfocal = -5\n").is_err());
}

fn at(focal: Option<f64>, aperture: Option<f64>, distance: Option<f64>) -> SubProfile {
    SubProfile {
        focal,
        aperture,
        distance,
        ..SubProfile::plain(LensModel::NONE)
    }
}

#[test]
fn subprofile_pick_is_closest_focal_then_aperture_then_distance() {
    let subs = [
        at(Some(24.0), Some(2.8), Some(1.0)),
        at(Some(50.0), Some(2.8), Some(1.0)),
        at(Some(50.0), Some(4.0), Some(1.0)),
        at(Some(50.0), Some(4.0), Some(10.0)),
        at(Some(70.0), None, None),
    ];
    assert_eq!(pick_subprofile(&subs, Some(50.0), Some(4.0), Some(8.0)), 3);
    assert_eq!(pick_subprofile(&subs, Some(52.0), Some(2.8), None), 1);
    assert_eq!(pick_subprofile(&subs, Some(50.0), Some(3.8), Some(1.0)), 2);
    assert_eq!(pick_subprofile(&subs, Some(70.0), Some(2.8), None), 4);
    assert_eq!(pick_subprofile(&subs, Some(30.0), None, None), 0);
    assert_eq!(pick_subprofile(&subs, None, None, None), 0);
    assert_eq!(pick_subprofile(&subs, None, Some(4.0), Some(10.0)), 3);
    // Within half a millimetre is a tie; the first is kept.
    let near = [at(Some(50.0), None, None), at(Some(50.2), None, None)];
    assert_eq!(pick_subprofile(&near, Some(50.3), None, None), 0);
    // A sub-profile without the shot's f-stop ranks after one with it.
    let partial = [at(Some(50.0), None, None), at(Some(50.0), Some(4.0), None)];
    assert_eq!(pick_subprofile(&partial, Some(50.0), Some(4.0), None), 1);
    assert_eq!(pick_subprofile(&[], Some(50.0), None, None), 0);
}

fn profile(name: &str, make: &str, model: &str, lens: &str, crop: Option<f64>) -> LensProfile {
    LensProfile {
        name: name.into(),
        make: make.into(),
        model: model.into(),
        lens: lens.into(),
        raw: false,
        subs: vec![SubProfile {
            crop,
            ..SubProfile::plain(LensModel::NONE)
        }],
    }
}

#[test]
fn lens_profiles_match_any_camera_and_sort_by_sensor() {
    let all = [
        profile("Sigma 30mm", "Sigma", "", "Sigma 30mm F1.4 DC", Some(1.5)),
        profile(
            "Canon EF 50mm",
            "Canon",
            "",
            "Canon EF 50mm f/1.8",
            Some(1.0),
        ),
        profile(
            "Canon EF 50mm (crop 1.6)",
            "Canon",
            "",
            "Canon EF 50mm f/1.8",
            Some(1.6),
        ),
        profile("7D kit", "Canon", "EOS 7D", "", None),
    ];
    let find = |make, model, lens, crop| match_profiles(&all, make, model, lens, crop, false);
    assert_eq!(
        find("Canon", "EOS 7D", "Sigma 30mm F1.4 DC", Some(1.6)),
        vec![0, 3]
    );
    assert_eq!(find("Nikon", "D7000", "sigma 30mm f1.4  dc", None), vec![0]);
    assert_eq!(find("Canon", "", "", Some(1.5)), vec![2, 1, 3]);
    assert_eq!(find("Canon", "", "", None), vec![1, 2, 3]);
    assert_eq!(
        find("Canon", "EOS 7D", "Canon EF 50mm f/1.8", Some(1.0)),
        vec![1, 2, 3]
    );
    assert!(find("Nikon", "D7000", "", None).is_empty());
    assert!(find("", "", "Sigma 30mm F1.4 DC", None).is_empty());
}

#[test]
fn prefer_raw_ranks_raw_profiles_first() {
    let mut raw = profile("B", "Acme", "One", "", Some(1.0));
    raw.raw = true;
    let all = [profile("A", "Acme", "One", "", Some(1.0)), raw];
    assert_eq!(
        match_profiles(&all, "Acme", "", "", None, false),
        vec![0, 1]
    );
    assert_eq!(match_profiles(&all, "Acme", "", "", None, true), vec![1, 0]);
    assert_eq!(
        match_profiles(&all, "Acme", "One", "", None, true),
        vec![1, 0]
    );
    // A better match still outranks a raw one.
    let mut lens_only = profile("C", "Acme", "", "Acme 50mm", None);
    lens_only.raw = true;
    let all = [profile("A", "Acme", "One", "", None), lens_only];
    assert_eq!(
        match_profiles(&all, "Acme", "One", "", None, true),
        vec![0, 1]
    );
}

#[test]
fn profile_ids_are_stable() {
    let mut all: Vec<LensProfile> = builtin_profiles().to_vec();
    all.extend(parse_lensfun(ZOOM).unwrap());
    all.extend(parse_profiles("[profile]\nname = U\nmake = Acme\n").unwrap());
    let ids: Vec<i32> = all.iter().map(LensProfile::id).collect();
    let reversed: Vec<i32> = all.iter().rev().map(LensProfile::id).collect();
    assert_eq!(ids, reversed.into_iter().rev().collect::<Vec<_>>());
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), ids.len());
    assert!(ids.iter().all(|&id| id >= 1 << 30), "{ids:?}");
    // The id hashes the fields, so a copy loaded again has the same one.
    assert_eq!(all[0].clone().id(), ids[0]);
}

/// The 36 slots with the profile `id`, the coefficients flag, and `model`;
/// only Geometric Distortion and Vignette checked.
fn slots(id: f64, coefficients: bool, model: &LensModel) -> Vec<f64> {
    let mut v = vec![
        1.0, 0.0, 1.0, 1.0, 1.0, id, 0.0, 0.0, 0.0, 0.0, 0.0, 50.0, 0.0, 0.0, 0.0, 100.0, 2.0,
    ];
    v.push(if coefficients { 1.0 } else { 0.0 });
    v.extend(model.to_slots());
    v
}

#[test]
fn slots_carry_the_profile_coefficients() {
    let model = LensModel {
        distortion: LensModel::brown([0.1, 0.0, 0.0]),
        ca: [[1.001, 0.0, 0.0], [1.0, 0.0, 0.0], [0.999, 0.0, 0.0]],
        vignette: [-0.2, 0.0, 0.0],
    };
    assert_eq!(LensModel::from_slots(&model.to_slots()), Some(model));
    assert_eq!(LensModel::from_slots(&[0.0; 17]), None);

    let p = LensCorrectionParams::from_slots(&slots(123_456_789.0, true, &model)).unwrap();
    assert_eq!(
        p.profile,
        Some(LensModel {
            ca: LensModel::NONE.ca,
            ..model
        })
    );
    let legacy = LensCorrectionParams::from_slots(&slots(1.0, false, &LensModel::NONE)).unwrap();
    let bundled = builtin_profiles()[0].subs[0].coefficients;
    assert_eq!(
        legacy.profile,
        Some(LensModel {
            ca: LensModel::NONE.ca,
            ..bundled
        })
    );
    assert!(LensCorrectionParams::from_slots(&slots(123.0, false, &model)).is_none());
    let none = LensCorrectionParams::from_slots(&slots(0.0, false, &model)).unwrap();
    assert_eq!(none.profile, None);
    assert!(LensCorrectionParams::from_slots(&slots(0.0, false, &model)[..16]).is_none());
}
