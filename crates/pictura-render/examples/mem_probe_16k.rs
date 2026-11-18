//! Peak-RSS probe for a 16000² document, to size the view-pyramid memory budget.
//! Run: cargo run --release -p pictura-render --example mem_probe_16k

use pictura_core::{BitDepth, Channel, ColorMode, Document};

fn status_kib(key: &str) -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            let kb: u64 = rest
                .trim()
                .trim_end_matches(" kB")
                .trim()
                .parse()
                .unwrap_or(0);
            return kb / 1024;
        }
    }
    0
}

fn touch(buf: &mut [u8]) {
    for b in buf.chunks_mut(4096) {
        b[0] = 1;
    }
}

fn report(label: &str) {
    println!(
        "{label:<52} VmHWM={:>5} MiB  VmRSS={:>5} MiB",
        status_kib("VmHWM:"),
        status_kib("VmRSS:")
    );
}

fn main() {
    let (w, h) = (16000u32, 16000u32);
    let n = w as usize * h as usize;
    println!("target {w}x{h} = {} Mpx", n / 1_000_000);
    report("baseline");

    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.composite
        .edit_data(|data| touch(data))
        .expect("touch preserves geometry");
    report(&format!(
        "after Document::new (composite {} ch)",
        doc.composite.channels
    ));

    let mut layer: Vec<Channel> = Vec::new();
    for id in [0i16, 1, 2, -1] {
        let mut d = vec![0u8; n];
        touch(&mut d);
        layer.push(Channel { id, data: d.into() });
    }
    report("after one full-canvas 4-channel pixel layer");

    let mut level0 = vec![0u8; n * 4];
    touch(&mut level0);
    report("after a stored premultiplied level-0 copy");

    let mut levels: Vec<Vec<u8>> = Vec::new();
    let mut levels_bytes = 0usize;
    let (mut lw, mut lh) = (w, h);
    loop {
        let (nw, nh) = (lw.div_ceil(2), lh.div_ceil(2));
        if nw.max(nh) < 256 || nw <= 1 || nh <= 1 {
            break;
        }
        let mut b = vec![0u8; nw as usize * nh as usize * 4];
        touch(&mut b);
        levels_bytes += b.len();
        levels.push(b);
        lw = nw;
        lh = nh;
    }
    report(&format!(
        "after {} halved levels ({} MiB, ~1/3 of level 0)",
        levels.len(),
        levels_bytes / 1024 / 1024
    ));

    std::hint::black_box((&doc, &layer, &level0, &levels));
}
