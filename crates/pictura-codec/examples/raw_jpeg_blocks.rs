//! Prints a raw file's camera RGB and its camera-rendered JPEG averaged over the
//! same grid of blocks, the input `scripts/fit-camera-jpeg-matrices.py` fits a
//! colour matrix from (#1542).
//!
//! ```bash
//! cargo run --release -p pictura-codec --example raw_jpeg_blocks -- RAW JPEG COLS ROWS
//! ```
//!
//! Both images are mapped onto the raw file's default crop. The first line is
//! `neutral R G B` (the as-shot neutral, or `none`); then one line per block:
//! `col row cam_r cam_g cam_b jpeg_r jpeg_g jpeg_b clipped`, where camera RGB
//! is 0–1 over black–white, JPEG RGB is linear sRGB 0–1 and `clipped` is the
//! share of raw photosites at the white level.

use pictura_codec::{read_raw, ImageBudget};

fn srgb_to_linear(v: u8) -> f64 {
    let c = v as f64 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, raw_path, jpeg_path, cols, rows] = args.as_slice() else {
        eprintln!("usage: raw_jpeg_blocks RAW JPEG COLS ROWS");
        std::process::exit(2);
    };
    let (cols, rows): (usize, usize) = (cols.parse().unwrap(), rows.parse().unwrap());
    let raw = read_raw(&std::fs::read(raw_path).unwrap(), &ImageBudget::default())
        .unwrap_or_else(|e| panic!("{raw_path}: {e}"));
    let jpeg = image::load_from_memory_with_format(
        &std::fs::read(jpeg_path).unwrap(),
        image::ImageFormat::Jpeg,
    )
    .unwrap()
    .to_rgb8();

    match raw.color.as_shot_neutral {
        Some([r, g, b]) => println!("neutral {r} {g} {b}"),
        None => println!("neutral none"),
    }

    let crop = raw.default_crop;
    let cfa = &raw.cfa;
    let mut cam = vec![([0f64; 3], [0u64; 3], 0u64); cols * rows];
    let (x0, y0) = (crop.x as usize, crop.y as usize);
    let (cw, ch) = (crop.width as usize, crop.height as usize);
    for y in y0..(y0 + ch).min(raw.height) {
        let by = (y - y0) * rows / ch;
        for x in x0..(x0 + cw).min(raw.width) {
            let bx = (x - x0) * cols / cw;
            let v = raw.samples[y * raw.width + x];
            let c = cfa.colors[(y % cfa.height) * cfa.width + x % cfa.width] as usize;
            let block = &mut cam[by * cols + bx];
            block.0[c] += v as f64 / 65535.0;
            block.1[c] += 1;
            if v == u16::MAX {
                block.2 += 1;
            }
        }
    }

    let (jw, jh) = (jpeg.width() as usize, jpeg.height() as usize);
    let mut out = vec![([0f64; 3], 0u64); cols * rows];
    for (x, y, px) in jpeg.enumerate_pixels() {
        let (x, y) = (x as usize, y as usize);
        let block = &mut out[(y * rows / jh) * cols + x * cols / jw];
        for (sum, v) in block.0.iter_mut().zip(px.0) {
            *sum += srgb_to_linear(v);
        }
        block.1 += 1;
    }

    for row in 0..rows {
        for col in 0..cols {
            let (sum, count, clipped) = cam[row * cols + col];
            let (jsum, n) = out[row * cols + col];
            if count.contains(&0) || n == 0 {
                continue;
            }
            let c: [f64; 3] = std::array::from_fn(|i| sum[i] / count[i] as f64);
            let j = jsum.map(|v| v / n as f64);
            let total: u64 = count.iter().sum();
            println!(
                "{col} {row} {:.6} {:.6} {:.6} {:.6} {:.6} {:.6} {:.6}",
                c[0],
                c[1],
                c[2],
                j[0],
                j[1],
                j[2],
                clipped as f64 / total as f64
            );
        }
    }
}
