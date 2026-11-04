//! The rectangle and ellipse builders against their per-pixel definitions,
//! clipped to the canvas, however the masks are allocated.

use pictura_select::Selection;

#[test]
fn rects_and_ellipses_match_their_per_pixel_shapes() {
    let (cw, ch) = (19u32, 13u32);
    let boxes = [
        (3, 2, 9, 7),
        (-4, -3, 10, 8),
        (12, 8, 11, 9),
        (-2, 4, 25, 3),
        (20, 14, 5, 5),
        (-9, -9, 4, 4),
        (5, 5, 0, 3),
        (5, 5, 3, -1),
        (0, 0, 19, 13),
        (6, 1, 1, 11),
    ];
    for (x, y, w, h) in boxes {
        let in_box = |px: i64, py: i64| {
            w > 0
                && h > 0
                && px >= x as i64
                && px < x as i64 + w as i64
                && py >= y as i64
                && py < y as i64 + h as i64
        };
        let (rx, ry) = (w as f64 / 2.0, h as f64 / 2.0);
        let in_ellipse = |px: i64, py: i64| {
            let dx = (px as f64 + 0.5 - (x as f64 + rx)) / rx;
            let dy = (py as f64 + 0.5 - (y as f64 + ry)) / ry;
            in_box(px, py) && dx * dx + dy * dy <= 1.0
        };
        let rect = Selection::rect(cw, ch, x, y, w, h);
        let ellipse = Selection::ellipse(cw, ch, x, y, w, h);
        for py in 0..ch as i64 {
            for px in 0..cw as i64 {
                let i = (py * cw as i64 + px) as usize;
                let want = |inside: bool| if inside { 255 } else { 0 };
                assert_eq!(
                    rect.data[i],
                    want(in_box(px, py)),
                    "rect {x},{y} {w}x{h} at {px},{py}"
                );
                assert_eq!(
                    ellipse.data[i],
                    want(in_ellipse(px, py)),
                    "ellipse {x},{y} {w}x{h} at {px},{py}"
                );
            }
        }
    }
    assert!(Selection::all(cw, ch).data.iter().all(|&v| v == 255));
    assert!(Selection::none(cw, ch).data.iter().all(|&v| v == 0));
}
