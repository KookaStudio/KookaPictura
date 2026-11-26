//! The Pictura Raw settings as Camera Raw `crs:` XMP (`codec/raw-settings-xmp`,
//! #401): every tab read and written, packets patched in place.

use pictura_codec::{pictura_raw_from_xmp, pictura_raw_to_xmp, raw_settings_key};
use pictura_core::PicturaRawSettings;

const RDF_OPEN: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">"#;
const RDF_CLOSE: &str = "</rdf:RDF></x:xmpmeta>";

fn packet(description: &str) -> String {
    format!("{RDF_OPEN}{description}{RDF_CLOSE}")
}

// raw_xmp_reads_tabs
#[test]
fn xmp_reads_controls_from_every_tab() {
    let text = packet(
        r#"<rdf:Description rdf:about="" xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
 crs:Exposure2012="+0.50" crs:Sharpness="40" crs:HueAdjustmentOrange="-12"
 crs:SplitToningBalance="+20" crs:PostCropVignetteAmount="-30"
 crs:CameraProfile="Camera Matching" crs:ConvertToGrayscale="True"
 crs:ProcessVersion="6.7" crs:WhiteBalance="Custom">
 <crs:ToneCurvePV2012><rdf:Seq><rdf:li>0, 0</rdf:li><rdf:li>128, 140</rdf:li><rdf:li>255, 255</rdf:li></rdf:Seq></crs:ToneCurvePV2012>
</rdf:Description>"#,
    );
    let s = pictura_raw_from_xmp(text.as_bytes());
    assert_eq!(s.basic.exposure, Some(0.5));
    assert_eq!(s.detail.sharpen_amount, Some(40));
    assert_eq!(s.hsl.hue[1], Some(-12));
    assert_eq!(s.split_toning.balance, Some(20));
    assert_eq!(s.effects.vignette_amount, Some(-30));
    assert_eq!(
        s.calibration.camera_profile.as_deref(),
        Some("Camera Matching")
    );
    assert_eq!(s.mode.grayscale, Some(true));
    assert_eq!(s.mode.process_version, Some(2012));
    assert_eq!(s.mode.white_balance, Some(*b"Cst "));
    assert_eq!(s.tone_curve.curve_rgb, vec![0, 0, 128, 140, 255, 255]);
}

// raw_xmp_reads_nothing
#[test]
fn xmp_without_crs_reads_default() {
    let text = packet(
        r#"<rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title><rdf:Alt><rdf:li xml:lang="x-default">Shot</rdf:li></rdf:Alt></dc:title></rdf:Description>"#,
    );
    assert_eq!(
        pictura_raw_from_xmp(text.as_bytes()),
        PicturaRawSettings::default()
    );
}

fn every_tab() -> PicturaRawSettings {
    let mut s = PicturaRawSettings::default();
    let b = &mut s.basic;
    b.temperature = Some(5600.0);
    b.tint = Some(-12.0);
    b.exposure = Some(0.35);
    b.contrast = Some(10.0);
    b.highlights = Some(-40.0);
    b.shadows = Some(30.0);
    b.whites = Some(5.0);
    b.blacks = Some(-5.0);
    b.clarity = Some(15.0);
    b.vibrance = Some(20.0);
    b.saturation = Some(-10.0);
    s.mode.white_balance = Some(*b"Cst ");
    s.mode.auto_wb_version = Some(1);
    s.mode.process_version = Some(2010);
    s.mode.grayscale = Some(true);
    let t = &mut s.tone_curve;
    t.parametric_shadows = Some(-10);
    t.parametric_darks = Some(5);
    t.parametric_lights = Some(8);
    t.parametric_highlights = Some(-3);
    t.shadow_split = Some(25);
    t.midtone_split = Some(50);
    t.highlight_split = Some(75);
    t.curve_rgb = vec![0, 0, 128, 140, 255, 255];
    t.curve_red = vec![0, 10, 255, 250];
    t.curve_green = vec![0, 0, 64, 70, 255, 255];
    t.curve_blue = vec![0, 5, 255, 255];
    let d = &mut s.detail;
    d.sharpen_amount = Some(40);
    d.sharpen_radius = Some(1.2);
    d.sharpen_detail = Some(25);
    d.sharpen_masking = Some(10);
    d.noise_luminance = Some(20);
    d.noise_luminance_detail = Some(50);
    d.noise_luminance_contrast = Some(0);
    d.noise_color = Some(25);
    d.noise_color_detail = Some(50);
    d.noise_color_smoothness = Some(60);
    for i in 0..8 {
        let v = i as i32 * 7 - 20;
        s.hsl.hue[i] = Some(v);
        s.hsl.saturation[i] = Some(v + 1);
        s.hsl.luminance[i] = Some(v + 2);
        s.hsl.grayscale_mix[i] = Some(v + 3);
    }
    let st = &mut s.split_toning;
    st.shadow_hue = Some(210);
    st.shadow_saturation = Some(30);
    st.highlight_hue = Some(45);
    st.highlight_saturation = Some(20);
    st.balance = Some(-15);
    let l = &mut s.lens;
    l.profile_enabled = Some(1);
    l.distortion = Some(-8);
    l.vignette_amount = Some(12);
    l.vignette_midpoint = Some(-5);
    l.perspective_vertical = Some(-20);
    l.perspective_horizontal = Some(4);
    l.perspective_scale = Some(95);
    l.perspective_aspect = Some(-3);
    l.perspective_upright = Some(2);
    l.rotate = Some(1.5);
    l.offset_x = Some(-2.5);
    l.offset_y = Some(3.0);
    l.defringe_purple_amount = Some(5);
    l.defringe_purple_hue_lo = Some(30);
    l.defringe_purple_hue_hi = Some(70);
    l.defringe_green_amount = Some(2);
    l.defringe_green_hue_lo = Some(40);
    l.defringe_green_hue_hi = Some(60);
    let e = &mut s.effects;
    e.grain_amount = Some(25);
    e.grain_size = Some(30);
    e.grain_frequency = Some(50);
    e.dehaze = Some(-10);
    e.vignette_amount = Some(-30);
    e.vignette_midpoint = Some(50);
    e.vignette_feather = Some(60);
    e.vignette_roundness = Some(-20);
    e.vignette_style = Some(1);
    e.vignette_highlights = Some(10);
    let c = &mut s.calibration;
    c.camera_profile = Some("R&D <test>".into());
    c.profile_digest = Some("ABCDEF0123".into());
    c.red_hue = Some(5);
    c.red_saturation = Some(-5);
    c.green_hue = Some(10);
    c.green_saturation = Some(-10);
    c.blue_hue = Some(15);
    c.blue_saturation = Some(-15);
    c.shadow_tint = Some(3);
    s
}

// raw_xmp_round_trip
#[test]
fn xmp_round_trips_every_tab() {
    let settings = every_tab();
    let written = pictura_raw_to_xmp(None, &settings).expect("fresh packet");
    let text = String::from_utf8(written.clone()).unwrap();
    assert!(text.contains(r#"crs:HasSettings="True""#), "{text}");
    assert!(text.contains(r#"crs:Exposure2012="+0.35""#), "{text}");
    assert!(text.contains("R&amp;D &lt;test&gt;"), "{text}");
    assert_eq!(pictura_raw_from_xmp(&written), settings);
    // A second write over its own output changes nothing.
    assert_eq!(pictura_raw_to_xmp(Some(&written), &settings), Some(written));
}

// raw_xmp_preserves_unknown
#[test]
fn xmp_rewrite_keeps_unknown_bytes() {
    let sidecar = packet(
        r#"
 <rdf:Description rdf:about=""
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
   crs:Exposure2012="+1.00"
   crs:HasSettings="True"
   crs:LensProfileName='Some &amp; Lens'>
   <dc:title><rdf:Alt><rdf:li xml:lang="x-default">Shot</rdf:li></rdf:Alt></dc:title>
   <crs:RetouchInfo><rdf:Seq><rdf:li>centerX = 0.5</rdf:li></rdf:Seq></crs:RetouchInfo>
 </rdf:Description>
"#,
    );
    let mut settings = PicturaRawSettings::default();
    settings.basic.exposure = Some(0.25);
    let written = pictura_raw_to_xmp(Some(sidecar.as_bytes()), &settings).expect("patched");
    assert_eq!(
        String::from_utf8(written).unwrap(),
        sidecar.replace("+1.00", "+0.25")
    );
}

// raw_xmp_removes_unset
#[test]
fn xmp_rewrite_removes_unset_and_inserts_curves() {
    let sidecar = packet(
        r#"<rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" crs:Contrast2012="+25"/>"#,
    );
    let mut settings = PicturaRawSettings::default();
    settings.tone_curve.curve_rgb = vec![0, 0, 255, 255];
    let written = pictura_raw_to_xmp(Some(sidecar.as_bytes()), &settings).expect("patched");
    let text = String::from_utf8(written.clone()).unwrap();
    assert!(!text.contains("Contrast2012"), "{text}");
    let open = text.find("<rdf:Description").unwrap();
    let curve = text.find("<crs:ToneCurvePV2012>").expect("curve element");
    let close = text.find("</rdf:Description>").expect("description opened");
    assert!(open < curve && curve < close, "{text}");
    let read = pictura_raw_from_xmp(&written);
    assert_eq!(read.tone_curve.curve_rgb, vec![0, 0, 255, 255]);
    assert_eq!(read.basic.contrast, None);
}

// raw_xmp_refuses_unpatchable
#[test]
fn xmp_without_rdf_is_refused() {
    let mut settings = PicturaRawSettings::default();
    settings.basic.exposure = Some(1.0);
    assert_eq!(pictura_raw_to_xmp(Some(b"not xmp"), &settings), None);
}

// raw_settings_key_sha256
#[test]
fn settings_key_is_sha256() {
    assert_eq!(
        raw_settings_key(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
