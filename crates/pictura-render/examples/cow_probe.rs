//! Copy-on-write probe for #1012: clone and clipping-composite cost on a
//! native-depth document with a smart-object payload and a raw linked section.
//! Run: cargo run --release -p pictura-render --example cow_probe

use std::time::Instant;

use pictura_core::{
    BitDepth, Channel, ColorMode, Document, Layer, LayerStack, PsdRect, Samples, SmartObject,
    SourceChannels, SourcePlanes,
};
use pictura_render::composite_rgba;

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

fn report(label: &str) {
    println!(
        "{label:<48} VmRSS={:>5} MiB  VmHWM={:>5} MiB",
        status_kib("VmRSS:"),
        status_kib("VmHWM:")
    );
}

fn rect(w: u32, h: u32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: w as i32,
    }
}

fn pixel_layer(name: &str, w: u32, h: u32, clipping: bool) -> Layer {
    let plane = w as usize * h as usize;
    Layer {
        name: name.into(),
        rect: rect(w, h),
        clipping,
        channels: [0i16, 1, 2, -1]
            .into_iter()
            .map(|id| Channel {
                id,
                data: vec![128u8; plane].into(),
            })
            .collect(),
        ..Default::default()
    }
}

fn main() {
    let (w, h) = (2000u32, 2000u32);
    let plane = w as usize * h as usize;
    println!("document {w}x{h}, 16-bit native samples, 8 MiB payload, 8 MiB raw section");
    report("baseline");

    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Sixteen);
    doc.source_depth = Some(BitDepth::Sixteen);
    doc.source_planes = Some(SourcePlanes {
        depth: BitDepth::Sixteen,
        width: w,
        height: h,
        samples: Samples::U16(vec![1u16; plane * 3].into()),
    });
    doc.layer_section_extra = vec![0u8; 8 * 1024 * 1024].into();
    let mut layer = pixel_layer("smart", w, h, false);
    layer.source_channels = Some(SourceChannels::new(
        BitDepth::Sixteen,
        rect(w, h),
        vec![(0, Samples::U16(vec![1u16; plane].into()))],
    ));
    layer.smart_object = Some(SmartObject::default().with_payload(vec![0u8; 8 * 1024 * 1024]));
    doc.layers = LayerStack::from_layers(vec![layer]);
    report("after document");

    // Warm up.
    std::hint::black_box(doc.clone());
    let iterations = 200;
    let started = Instant::now();
    for _ in 0..iterations {
        let clone = doc.clone();
        std::hint::black_box(&clone);
    }
    let clone_us = started.elapsed().as_micros() as f64 / iterations as f64;
    println!("Document::clone: {clone_us:.1} us/iter ({iterations} iterations)");
    report("after 200 clones (retained: one doc)");

    // The compositor clones a clipping base once per 64-row band. Measure the
    // layer clone directly, including the embedded payload and the tree.
    let iterations = 200;
    let started = Instant::now();
    for _ in 0..iterations {
        let clone = doc.layers[0].clone();
        std::hint::black_box(&clone);
    }
    let layer_us = started.elapsed().as_micros() as f64 / iterations as f64;
    println!("Layer::clone (8 MiB payload + native store): {layer_us:.1} us/iter");

    let mut group = Layer {
        name: "group".into(),
        is_group: true,
        ..Default::default()
    };
    group.children = LayerStack::from_layers(
        (0..1000)
            .map(|i| Layer {
                name: format!("child {i}"),
                ..Default::default()
            })
            .collect(),
    );
    let iterations = 1000;
    let started = Instant::now();
    for _ in 0..iterations {
        let clone = group.clone();
        std::hint::black_box(&clone);
    }
    let group_us = started.elapsed().as_micros() as f64 / iterations as f64;
    println!("Layer::clone (1000-child group): {group_us:.1} us/iter");

    // Clipping group: a base and one clipped layer, composited in bands.
    let (cw, ch) = (1024u32, 1024u32);
    let mut clip = Document::new(cw, ch, ColorMode::Rgb, BitDepth::Eight);
    clip.layers = LayerStack::from_layers(vec![
        pixel_layer("base", cw, ch, false),
        pixel_layer("clipped", cw, ch, true),
    ]);
    std::hint::black_box(composite_rgba(&clip));
    let runs = 5;
    let started = Instant::now();
    for _ in 0..runs {
        std::hint::black_box(composite_rgba(&clip));
    }
    let composite_ms = started.elapsed().as_millis() as f64 / runs as f64;
    println!("composite_rgba (clipping group): {composite_ms:.1} ms/run ({runs} runs)");
    report("after clipping composites");
}
