//! Full-document mask profile (issue #971). Run with:
//!
//! ```text
//! cargo test -p pictura-select --test profile -- --ignored --nocapture
//! ```

use std::time::{Duration, Instant};

use pictura_core::PixelBuffer;
use pictura_select::{similar, Selection};

const W: u32 = 6000;
const H: u32 = 4000;

fn image() -> PixelBuffer {
    let n = W as usize * H as usize;
    let mut img = PixelBuffer::new(W, H, 3);
    for y in 0..H {
        for x in 0..W {
            let p = y as usize * W as usize + x as usize;
            img.data[p] = (x % 256) as u8;
            img.data[n + p] = (y % 256) as u8;
            img.data[2 * n + p] = ((x * 7 + y * 13) % 256) as u8;
        }
    }
    img
}

fn block(sel: &mut Selection, x0: u32, y0: u32, x1: u32, y1: u32) {
    for y in y0..y1 {
        for x in x0..x1 {
            sel.data[(y * W + x) as usize] = 255;
        }
    }
}

fn report(label: &str, elapsed: Duration) {
    println!("{label:<34} {:>8.1} ms", elapsed.as_secs_f64() * 1000.0);
}

#[test]
#[ignore = "6000x4000 mask profile; run with --ignored --nocapture"]
fn mask_ops_profile() {
    let img = image();

    let mut few = Selection::none(W, H);
    block(&mut few, 0, 0, 10, 1);
    let start = Instant::now();
    let out = similar(&few, &img, 32).expect("similar");
    report("Similar, 10 reference colours", start.elapsed());
    assert!(out.data.contains(&255));

    let mut many = Selection::none(W, H);
    block(&mut many, 0, 0, 100, 100);
    let start = Instant::now();
    let out = similar(&many, &img, 32).expect("similar");
    report("Similar, 10 000 reference colours", start.elapsed());
    assert!(out.data.contains(&255));

    let mut mask = Selection::none(W, H);
    block(&mut mask, 2000, 1500, 4000, 2500);

    let start = Instant::now();
    let smoothed = mask.smooth(10);
    report("Smooth, radius 10", start.elapsed());
    assert!(smoothed.data.contains(&255));

    let start = Instant::now();
    let smoothed = mask.smooth(100);
    report("Smooth, radius 100", start.elapsed());
    assert!(smoothed.data.contains(&255));

    let start = Instant::now();
    let expanded = mask.expand(1);
    report("Expand, radius 1", start.elapsed());
    assert!(expanded.data.contains(&255));

    let start = Instant::now();
    let expanded = mask.expand(100);
    report("Expand, radius 100", start.elapsed());
    assert!(expanded.data.contains(&255));
}
