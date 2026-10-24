use pictura_core::PixelBuffer;

use super::canvas::Canvas;
use super::float::Texture;
use super::geometry::{P2, V3};
use super::heal::HealMode;
use super::space::FamilyMap;
use super::tests::{camera, line_miss, planes_with, square_on, tilted, torn, H, W};
use super::tools::PasteError;
use super::{vanishing_point, VpEdit, VpEditor, VpPatch, VpPlanes};

fn close(a: P2, b: P2, tol: f64) -> bool {
    (a.0 - b.0).abs() <= tol && (a.1 - b.1).abs() <= tol
}

/// A document-sized opaque layer whose pixel at `(x, y)` is `paint(x, y)`.
fn layer(paint: impl Fn(usize, usize) -> [u8; 4]) -> Canvas {
    let (w, h) = (W as usize, H as usize);
    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            rgba.extend_from_slice(&paint(x, y));
        }
    }
    Canvas::new(w, h, 0, 0, rgba).unwrap()
}

fn editor_on(planes: VpPlanes, canvas: Canvas) -> VpEditor {
    let mut e = VpEditor::new(planes);
    e.attach(canvas, None, false);
    e
}

fn solid(w: usize, h: usize, px: [u8; 4]) -> Texture {
    Texture::new(w, h, px.repeat(w * h)).unwrap()
}

fn canvas_of(e: &VpEditor) -> &Canvas {
    &e.retouch().unwrap().canvas
}

fn px(e: &VpEditor, x: i32, y: i32) -> [u8; 4] {
    canvas_of(e).get(x, y)
}

const BLACK: [u8; 4] = [0, 0, 0, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

#[test]
fn a_hinge_unfolds_into_the_flat_space() {
    let mut planes = planes_with(square_on());
    let child = torn(&mut planes, 0, 1);
    let map = FamilyMap::new(&planes, 0).unwrap();
    let (frame, cam) = (
        *planes.frame(child).unwrap(),
        *planes.camera(child).unwrap(),
    );
    let [s0, s1, t0, t1] = frame.rect;
    let (s, t) = ((s0 + s1) / 2.0, (t0 + t1) / 2.0);
    let p = cam.project(frame.point(s, t)).unwrap();
    let unit = planes.unit(0);
    // The square-on root measures 400 flat units across; the hinge is its
    // right edge, so the child unfolds to the right of it.
    let (_, flat) = map.locate(p, Some(child)).unwrap();
    assert!(close(flat, (400.0 + t / unit, s / unit), 1e-6), "{flat:?}");
    assert!(close(map.to_image(flat, Some(child)).unwrap(), p, 1e-6));
}

#[test]
fn paste_places_a_float_in_the_active_plane() {
    let mut e = editor_on(planes_with(square_on()), layer(|_, _| BLACK));
    e.paste(solid(40, 30, WHITE)).unwrap();
    let f = e.floating().unwrap();
    assert!(close(f.centre, (200.0, 200.0), 1e-9) && f.scale == (1.0, 1.0));
    let c = e.float_corners();
    for (got, want) in c.iter().zip([
        (380.0, 285.0),
        (420.0, 285.0),
        (420.0, 315.0),
        (380.0, 315.0),
    ]) {
        assert!(close(*got, want, 1e-6), "{got:?} vs {want:?}");
    }
    // A texture larger than the plane shrinks to 80 % of it.
    e.paste(solid(1000, 1000, WHITE)).unwrap();
    assert!((e.floating().unwrap().scale.0 - 0.32).abs() < 1e-9);

    let mut empty = editor_on(VpPlanes::new(W, H), layer(|_, _| BLACK));
    assert_eq!(empty.paste(solid(4, 4, WHITE)), Err(PasteError::NoPlane));
    assert!(!empty.has_float());
}

fn changed(e: &VpEditor, against: [u8; 4]) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            if px(e, x, y) != against {
                out.push((x, y));
            }
        }
    }
    out
}

#[test]
fn a_float_follows_the_planes_perspective() {
    let mut e = editor_on(planes_with(square_on()), layer(|_, _| BLACK));
    e.paste(solid(40, 30, WHITE)).unwrap();
    assert!(e.drop_float());
    let lit: f64 = changed(&e, BLACK)
        .iter()
        .map(|&(x, y)| px(&e, x, y)[0] as f64 / 255.0)
        .sum();
    assert!((lit - 1200.0).abs() / 1200.0 < 0.02, "covered {lit}");

    let (q, eu, ev) = tilted();
    let mut t = editor_on(planes_with(q), layer(|_, _| BLACK));
    t.paste(solid(40, 30, WHITE)).unwrap();
    let cam = camera();
    let vanish = |d: V3| {
        let k = cam.half * cam.f / d.z;
        (cam.centre.0 + d.x * k, cam.centre.1 + d.y * k)
    };
    let c = t.float_corners();
    assert!(line_miss(c[0], c[1], vanish(eu)) < 1e-3);
    assert!(line_miss(c[3], c[2], vanish(eu)) < 1e-3);
    assert!(line_miss(c[1], c[2], vanish(ev)) < 1e-3);
    assert!(line_miss(c[0], c[3], vanish(ev)) < 1e-3);
}

#[test]
fn a_dragged_float_wraps_across_a_hinge() {
    let mut planes = planes_with(square_on());
    torn(&mut planes, 0, 1);
    let mut e = editor_on(planes, layer(|_, _| BLACK));
    e.set_active(Some(0));
    e.paste(solid(100, 60, WHITE)).unwrap();
    // Grab the float's middle and carry it onto the hinge at x = 600.
    assert!(e.marquee_press((400.0, 300.0), false, false));
    for k in 1..=4 {
        e.marquee_drag((400.0 + 49.75 * k as f64, 300.0), false);
    }
    e.gesture_release();
    assert!(close(e.floating().unwrap().centre, (399.0, 200.0), 1e-6));
    assert!(e.drop_float());
    let hit = changed(&e, BLACK);
    assert!(
        hit.iter().any(|&(x, _)| x < 598),
        "nothing on the root plane"
    );
    assert!(
        hit.iter().any(|&(x, _)| x > 602),
        "nothing on the torn-off plane"
    );
    // The two halves meet at the hinge.
    assert!(px(&e, 597, 300)[0] > 200 && px(&e, 603, 300)[0] > 200);
}

#[test]
fn transform_scales_rotates_and_flips() {
    let mut tex = WHITE.repeat(40 * 30);
    tex[..4].copy_from_slice(&[255, 0, 0, 255]);
    let marked = Texture::new(40, 30, tex).unwrap();
    let mut e = editor_on(planes_with(square_on()), layer(|_, _| BLACK));
    e.paste(marked.clone()).unwrap();
    // Corner 2 sits at (420, 315): drag it out to twice the size.
    assert!(e.transform_press((420.0, 315.0), 4.0));
    e.transform_drag((440.0, 330.0), false);
    e.gesture_release();
    assert!(close(e.floating().unwrap().scale, (2.0, 2.0), 1e-6));
    e.drop_float();
    assert!((changed(&e, BLACK).len() as f64 - 4800.0).abs() < 4800.0 * 0.03);

    // Rotated a quarter turn, texel (0, 0) lands up and to the right.
    let mut e = editor_on(planes_with(square_on()), layer(|_, _| BLACK));
    e.paste(marked.clone()).unwrap();
    assert!(
        e.transform_press((428.0, 323.0), 4.0),
        "just past a corner rotates"
    );
    e.transform_drag((380.0, 330.0), false);
    e.gesture_release();
    assert!(e.floating().unwrap().angle.abs() > 0.1);
    let f = e.state.float.as_mut().unwrap();
    f.angle = std::f64::consts::FRAC_PI_2;
    e.drop_float();
    assert_eq!(px(&e, 414, 280), [255, 0, 0, 255]);

    // Flipped, it lands at the top right instead of the top left.
    let mut e = editor_on(planes_with(square_on()), layer(|_, _| BLACK));
    e.paste(marked).unwrap();
    assert!(e.mirror(true));
    e.drop_float();
    assert_eq!(px(&e, 419, 285), [255, 0, 0, 255]);
    assert_eq!(px(&e, 380, 285), WHITE);
}

#[test]
fn dropping_uses_opacity_and_feather() {
    let mut e = editor_on(planes_with(square_on()), layer(|_, _| BLACK));
    e.retouch_mut().unwrap().marquee.opacity = 0.5;
    e.paste(solid(60, 60, WHITE)).unwrap();
    e.drop_float();
    assert!((px(&e, 400, 300)[0] as i32 - 128).abs() <= 1);

    let mut e = editor_on(planes_with(square_on()), layer(|_, _| BLACK));
    e.retouch_mut().unwrap().marquee.feather = 8.0;
    e.paste(solid(60, 60, WHITE)).unwrap();
    e.drop_float();
    // The float spans x = 370..430: full inside, fading over its last 8 px.
    assert_eq!(px(&e, 400, 300)[0], 255);
    let edge = px(&e, 371, 300)[0];
    assert!(edge > 0 && edge < 100, "edge {edge}");
}

#[test]
fn heal_modes_blend_as_stated() {
    let grey = [128, 128, 128, 255];
    let drop_with = |heal: HealMode| {
        let mut e = editor_on(planes_with(square_on()), layer(|_, _| grey));
        e.retouch_mut().unwrap().marquee.heal = heal;
        e.paste(solid(40, 40, [200, 40, 40, 255])).unwrap();
        e.drop_float();
        px(&e, 400, 300)
    };
    assert_eq!(drop_with(HealMode::Off), [200, 40, 40, 255]);
    let lum = drop_with(HealMode::Luminance);
    let luma = 0.299 * lum[0] as f64 + 0.587 * lum[1] as f64 + 0.114 * lum[2] as f64;
    assert!(
        lum[0] > lum[1] + 100 && (luma - 128.0).abs() < 4.0,
        "{lum:?}"
    );
    let on = drop_with(HealMode::On);
    assert!(
        on[..3].iter().all(|&c| (c as i32 - 128).abs() <= 3),
        "{on:?}"
    );
}

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

fn halves() -> VpEditor {
    editor_on(
        planes_with(square_on()),
        layer(|x, _| if x < 400 { RED } else { BLUE }),
    )
}

fn draw_marquee(e: &mut VpEditor, a: P2, b: P2) {
    assert!(e.marquee_press(a, false, false));
    e.marquee_drag(b, false);
    e.gesture_release();
}

#[test]
fn the_marquee_copies_moves_and_fills() {
    let mut e = halves();
    draw_marquee(&mut e, (250.0, 200.0), (300.0, 250.0));
    let q = *e.marquee().unwrap();
    assert!(close(q.min(), (50.0, 100.0), 1e-6) && close(q.max(), (100.0, 150.0), 1e-6));
    // Alt-drag floats a copy; the source stays.
    assert!(e.marquee_press((275.0, 225.0), true, false));
    e.marquee_drag((475.0, 225.0), false);
    e.gesture_release();
    assert!(e.has_float() && !e.has_marquee());
    assert_eq!(px(&e, 275, 225), RED);
    e.drop_float();
    assert_eq!(px(&e, 475, 225), RED);

    // Destination mode moves only the marquee.
    let mut e = halves();
    draw_marquee(&mut e, (450.0, 350.0), (500.0, 400.0));
    let before = canvas_of(&e).rgba.clone();
    assert!(e.marquee_press((475.0, 375.0), false, false));
    e.marquee_drag((455.0, 375.0), false);
    e.gesture_release();
    assert!(close(e.marquee().unwrap().min(), (230.0, 250.0), 1e-6));
    assert!(!e.has_float() && canvas_of(&e).rgba == before);

    // Ctrl-drag fills the marquee from under the pointer (200 px left: red).
    assert!(e.marquee_press((455.0, 375.0), false, true));
    e.marquee_drag((255.0, 375.0), false);
    e.gesture_release();
    e.drop_float();
    assert_eq!(px(&e, 455, 375), RED);
    assert_eq!(px(&e, 520, 375), BLUE);
}

#[test]
fn a_double_click_selects_the_whole_plane() {
    let mut e = halves();
    assert!(e.select_plane((300.0, 300.0)));
    let q = *e.marquee().unwrap();
    assert!(close(q.min(), (0.0, 0.0), 1e-6) && close(q.max(), (400.0, 400.0), 1e-6));
    assert!(!e.select_plane((50.0, 50.0)));
}

/// A layer whose red channel counts x and green counts y.
fn ramp() -> Canvas {
    layer(|x, y| [(x % 256) as u8, (y % 256) as u8, 0, 255])
}

#[test]
fn the_stamp_clones_in_perspective() {
    let mut e = editor_on(planes_with(square_on()), ramp());
    e.retouch_mut().unwrap().stamp.diameter = 20.0;
    assert!(!e.stroke_press(false, (450.0, 300.0)), "no source yet");
    assert!(e.set_stamp_source((250.0, 300.0)));
    assert!(e.stroke_press(false, (450.0, 300.0)));
    e.gesture_release();
    for (x, y) in [(450, 300), (452, 298), (446, 305)] {
        assert_eq!(
            px(&e, x, y),
            [((x - 200) % 256) as u8, (y % 256) as u8, 0, 255]
        );
    }
    // Aligned: the next stroke keeps the 200 px offset.
    assert!(e.stroke_press(false, (450.0, 400.0)));
    e.gesture_release();
    assert_eq!(px(&e, 450, 400), [250, (400 % 256) as u8, 0, 255]);
    // Not aligned: it starts again from the source point.
    e.retouch_mut().unwrap().aligned = false;
    assert!(e.stroke_press(false, (450.0, 450.0)));
    e.gesture_release();
    assert_eq!(px(&e, 450, 450), [250, (300 % 256) as u8, 0, 255]);
}

fn extent(e: &VpEditor) -> (i32, i32) {
    let hit = changed(e, WHITE);
    let w = hit.iter().map(|p| p.0).max().unwrap() - hit.iter().map(|p| p.0).min().unwrap() + 1;
    let h = hit.iter().map(|p| p.1).max().unwrap() - hit.iter().map(|p| p.1).min().unwrap() + 1;
    (w, h)
}

#[test]
fn brush_dabs_foreshorten() {
    let dab = |q: [P2; 4], at: P2| {
        let mut e = editor_on(planes_with(q), layer(|_, _| WHITE));
        let r = e.retouch_mut().unwrap();
        r.brush.diameter = 40.0;
        r.colour = [255, 0, 0];
        assert!(e.stroke_press(true, at));
        e.gesture_release();
        extent(&e)
    };
    let (w, h) = dab(square_on(), (400.0, 300.0));
    assert!((w - 40).abs() <= 3 && (h - 40).abs() <= 3, "{w}×{h}");
    // A floor receding upward: dabs squash vertically.
    let floor = [
        (300.0, 250.0),
        (500.0, 250.0),
        (700.0, 550.0),
        (100.0, 550.0),
    ];
    let (w, h) = dab(floor, (400.0, 450.0));
    assert!((h as f64) < 0.9 * w as f64, "{w}×{h}");
}

#[test]
fn edits_stay_inside_planes() {
    let mut e = editor_on(planes_with(square_on()), layer(|_, _| WHITE));
    e.retouch_mut().unwrap().brush.diameter = 30.0;
    assert!(e.stroke_press(true, (500.0, 300.0)));
    for k in 1..=10 {
        e.stroke_drag((500.0 + 25.0 * k as f64, 300.0));
    }
    e.gesture_release();
    e.paste(solid(300, 300, BLACK)).unwrap();
    e.state.float.as_mut().unwrap().scale = (2.0, 2.0);
    e.drop_float();
    for (x, y) in changed(&e, WHITE) {
        assert!(
            (200..600).contains(&x) && (100..500).contains(&y),
            "({x}, {y}) changed"
        );
    }
}

#[test]
fn pixel_edits_undo_and_redo() {
    let mut e = editor_on(planes_with(square_on()), ramp());
    let original = canvas_of(&e).rgba.clone();
    assert!(e.stroke_press(true, (400.0, 300.0)));
    e.stroke_drag((450.0, 320.0));
    e.gesture_release();
    let painted = canvas_of(&e).rgba.clone();
    assert_ne!(painted, original);
    assert!(e.undo());
    assert!(canvas_of(&e).rgba == original);
    assert!(e.redo());
    assert!(canvas_of(&e).rgba == painted);
}

#[test]
fn the_commit_replays_the_sessions_pixels() {
    let mut e = editor_on(planes_with(square_on()), ramp());
    let original = canvas_of(&e).clone();
    e.retouch_mut().unwrap().colour = [10, 200, 30];
    assert!(e.stroke_press(true, (400.0, 300.0)));
    e.stroke_drag((480.0, 340.0));
    e.gesture_release();
    let (x, y, w, h, rgba) = e.changed_patch().unwrap();
    let mut edit = VpEdit {
        patch: Some(VpPatch {
            x,
            y,
            width: w,
            height: h,
            rgba: rgba.into(),
        }),
        lines: Vec::new(),
    };
    let planar = |c: &Canvas| {
        let n = c.width * c.height;
        let mut b = PixelBuffer::new(c.width as u32, c.height as u32, 4);
        let data = b.data.as_mut_slice();
        for i in 0..n {
            for ch in 0..4 {
                data[ch * n + i] = c.rgba[i * 4 + ch];
            }
        }
        b
    };
    let mut buf = planar(&original);
    vanishing_point(&mut buf, &edit).unwrap();
    assert!(buf == planar(canvas_of(&e)));
    // Grids draw over the replayed pixels.
    edit.lines = e.planes().rendered_grid(0.0, 0.0);
    let mut gridded = planar(&original);
    vanishing_point(&mut gridded, &edit).unwrap();
    let n = W as usize * H as usize;
    let at = |b: &PixelBuffer, x: usize, y: usize| {
        b.data[y * W as usize + x] as i32 + b.data[n + y * W as usize + x] as i32
    };
    assert!(at(&gridded, 299, 450) < at(&buf, 299, 450));
}

#[test]
fn the_eyedropper_picks_from_the_preview() {
    let (w, h) = (W as usize, H as usize);
    let clear = Canvas::new(w, h, 0, 0, vec![0; w * h * 4]).unwrap();
    let mut e = VpEditor::new(planes_with(square_on()));
    e.attach(clear, Some([0, 160, 0, 255].repeat(w * h)), false);
    e.retouch_mut().unwrap().colour = [220, 10, 10];
    assert!(e.stroke_press(true, (400.0, 300.0)));
    e.gesture_release();
    assert_eq!(e.sample((400.5, 300.5)), Some([220, 10, 10]));
    assert_eq!(e.sample((50.5, 50.5)), Some([0, 160, 0]));
}
