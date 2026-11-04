//! A Marquee commit on a 16507×16196 document, stage by stage: the mask build,
//! the history record, the bounds the Info panel reads per pointer move, and
//! the outline over the whole mask and within the bounds (#1150).

use super::super::document_session::DocumentSession;
use super::super::helpers::feathered;
use super::PictureViewRust;
use pictura_core::{BitDepth, ColorMode, Document};
use pictura_select::Selection;
use std::time::Instant;

/// `PICTURA_PROFILE_DIVISOR` shrinks both sides for a machine without the
/// memory the full size takes; the costs measured here scale with area.
fn size() -> (u32, u32) {
    let divisor = std::env::var("PICTURA_PROFILE_DIVISOR")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1)
        .max(1);
    (16507 / divisor, 16196 / divisor)
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

#[test]
#[ignore = "16507x16196 Marquee commit profile; run explicitly with --ignored --nocapture"]
fn marquee_selection_profile() {
    let (w, h) = size();
    let mut rust = PictureViewRust {
        session: DocumentSession::with_document(Document::new(
            w,
            h,
            ColorMode::Rgb,
            BitDepth::Eight,
        )),
        ..Default::default()
    };
    let r = &mut rust;
    r.session.history.capture_live(
        r.session.doc.as_mut().expect("document"),
        &None,
        &r.layer_target,
        "Open",
    );
    // The profile-tools box: 20 %..70 % across, 20 %..65 % down.
    let (x, y) = ((w / 5) as i32, (h / 5) as i32);
    let (bw, bh) = ((w / 2) as i32, (h * 9 / 20) as i32);

    for (name, elliptical) in [("rect", false), ("ellipse", true)] {
        let t = Instant::now();
        let shape = if elliptical {
            Selection::ellipse(w, h, x, y, bw, bh)
        } else {
            Selection::rect(w, h, x, y, bw, bh)
        };
        r.session.set_selection(Some(feathered(shape, 0.0)));
        let build = ms(t);

        let t = Instant::now();
        r.session.history.capture_live(
            r.session.doc.as_mut().expect("document"),
            &r.session.selection,
            &r.layer_target,
            "Rectangular Marquee",
        );
        let record = ms(t);

        let t = Instant::now();
        let bounds = r.selection_mask_bounds().expect("bounds");
        let scan = ms(t);
        let t = Instant::now();
        for _ in 0..100 {
            assert_eq!(r.selection_mask_bounds(), Some(bounds));
        }
        let cached = ms(t) / 100.0;
        let t = Instant::now();
        let count = r.selection_pixel_count();
        let count_ms = ms(t);

        let sel = r.session.selection.as_ref().expect("selection");
        let t = Instant::now();
        let whole = pictura_select::contour(sel, 128);
        let whole_ms = ms(t);
        let t = Instant::now();
        let within = pictura_select::contour_within(sel, 128, bounds);
        let within_ms = ms(t);
        assert_eq!(whole.len(), within.len());

        let t = Instant::now();
        let moved = sel.translate(37, 21);
        let translate = ms(t);
        let t = Instant::now();
        let moved_loops = pictura_select::contour(&moved, 128).len();
        let moved_contour = ms(t);
        println!(
            "marquee_selection {name} {w}x{h}: build={build:.2}ms record={record:.2}ms \
             bounds={scan:.2}ms cached={cached:.4}ms count={count_ms:.2}ms ({count} px) \
             contour whole={whole_ms:.2}ms within={within_ms:.2}ms ({} loops) \
             move_translate={translate:.2}ms move_contour={moved_contour:.2}ms ({moved_loops} loops)",
            within.len(),
        );
    }
}
