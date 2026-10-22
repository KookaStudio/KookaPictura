//! Hostile documents (#844): crafted PSD/PSB and PDF files are refused with a
//! typed error before a large allocation, and a fixed-seed mutation pass over
//! valid files never panics or allocates past the budget it decodes under.
//!
//! A tracking global allocator records each thread's largest single request
//! and refuses anything over [`REFUSE`], so a regression fails here as an
//! allocation abort instead of exhausting the CI machine's memory.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::io::Write as _;

use pictura_codec::pdf::{
    editing_psd, import::PdfImages, write_pdf, EditingPsd, PdfEnv, PdfOptions,
};
use pictura_codec::{read_psd, read_psd_within, write_psd, ImageBudget, Policy};
use pictura_core::Document;

/// Larger requests are refused outright (an allocation-failure abort).
const REFUSE: usize = 1 << 30;

struct Tracking;

thread_local! {
    static PEAK: Cell<usize> = const { Cell::new(0) };
}

fn note(size: usize) {
    let _ = PEAK.try_with(|peak| peak.set(peak.get().max(size)));
}

// SAFETY: every call forwards to `System` with the caller's layout, except an
// over-limit request, which returns null as an allocator may.
unsafe impl GlobalAlloc for Tracking {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        if layout.size() > REFUSE {
            return std::ptr::null_mut();
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        if layout.size() > REFUSE {
            return std::ptr::null_mut();
        }
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note(new_size);
        if new_size > REFUSE {
            return std::ptr::null_mut();
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Tracking = Tracking;

/// `f`'s result and the largest single allocation it requested.
fn peak_of<T>(f: impl FnOnce() -> T) -> (T, usize) {
    PEAK.with(|peak| peak.set(0));
    let out = f();
    (out, PEAK.with(Cell::get))
}

const MIB: usize = 1 << 20;

// ---------------------------------------------------------------------------
// Hand-built PSD/PSB files
// ---------------------------------------------------------------------------

struct Header {
    psb: bool,
    channels: u16,
    width: u32,
    height: u32,
    depth: u16,
    mode: u16,
}

impl Header {
    fn rgb(width: u32, height: u32) -> Header {
        Header {
            psb: false,
            channels: 3,
            width,
            height,
            depth: 8,
            mode: 3,
        }
    }
}

/// One layer record: its rect (top, left, bottom, right), its channels (id,
/// then the stored data including the 2-byte compression word), an optional
/// user-mask rect, and an optional `lsct` section type.
#[derive(Default)]
struct LayerSpec {
    rect: [i32; 4],
    channels: Vec<(i16, Vec<u8>)>,
    mask: Option<[i32; 4]>,
    section: Option<u32>,
}

fn raw_channel(id: i16, pixels: usize) -> (i16, Vec<u8>) {
    let mut data = vec![0, 0];
    data.resize(2 + pixels, 7);
    (id, data)
}

/// A channel claiming RLE with `rows` scanlines of one 2-byte run each.
fn rle_channel(id: i16, rows: usize) -> (i16, Vec<u8>) {
    let mut data = vec![0, 1];
    for _ in 0..rows {
        data.extend_from_slice(&2u16.to_be_bytes());
    }
    for _ in 0..rows {
        data.extend_from_slice(&[0x81, 9]);
    }
    (id, data)
}

fn zip_channel(id: i16) -> (i16, Vec<u8>) {
    let mut data = vec![0, 2];
    data.extend_from_slice(&zlib(&[0u8; 4096]));
    (id, data)
}

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

fn rect_bytes(out: &mut Vec<u8>, rect: [i32; 4]) {
    for v in rect {
        out.extend_from_slice(&v.to_be_bytes());
    }
}

fn layer_record(spec: &LayerSpec) -> Vec<u8> {
    let mut out = Vec::new();
    rect_bytes(&mut out, spec.rect);
    out.extend_from_slice(&(spec.channels.len() as u16).to_be_bytes());
    for (id, data) in &spec.channels {
        out.extend_from_slice(&id.to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    }
    out.extend_from_slice(b"8BIMnorm");
    out.extend_from_slice(&[255, 0, 0, 0]);
    let mut extra = Vec::new();
    match spec.mask {
        Some(rect) => {
            extra.extend_from_slice(&20u32.to_be_bytes());
            rect_bytes(&mut extra, rect);
            extra.extend_from_slice(&[0, 0, 0, 0]);
        }
        None => extra.extend_from_slice(&0u32.to_be_bytes()),
    }
    extra.extend_from_slice(&0u32.to_be_bytes());
    extra.extend_from_slice(&[0, 0, 0, 0]);
    if let Some(section) = spec.section {
        extra.extend_from_slice(b"8BIMlsct");
        extra.extend_from_slice(&4u32.to_be_bytes());
        extra.extend_from_slice(&section.to_be_bytes());
    }
    out.extend_from_slice(&(extra.len() as u32).to_be_bytes());
    out.extend_from_slice(&extra);
    out
}

/// A PSD/PSB with `layers` and, when given, a composite image-data section
/// (compression word included).
fn psd(header: &Header, layers: &[LayerSpec], composite: Option<&[u8]>) -> Vec<u8> {
    let mut out = b"8BPS".to_vec();
    out.extend_from_slice(&(if header.psb { 2u16 } else { 1 }).to_be_bytes());
    out.extend_from_slice(&[0; 6]);
    out.extend_from_slice(&header.channels.to_be_bytes());
    out.extend_from_slice(&header.height.to_be_bytes());
    out.extend_from_slice(&header.width.to_be_bytes());
    out.extend_from_slice(&header.depth.to_be_bytes());
    out.extend_from_slice(&header.mode.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());

    let mut section = Vec::new();
    if !layers.is_empty() {
        let mut info = (layers.len() as i16).to_be_bytes().to_vec();
        for spec in layers {
            info.extend_from_slice(&layer_record(spec));
        }
        for spec in layers {
            for (_, data) in &spec.channels {
                info.extend_from_slice(data);
            }
        }
        if info.len() % 2 == 1 {
            info.push(0);
        }
        assert!(!header.psb, "layered PSB fixtures are not built here");
        section.extend_from_slice(&(info.len() as u32).to_be_bytes());
        section.extend_from_slice(&info);
        section.extend_from_slice(&0u32.to_be_bytes());
    }
    if header.psb {
        out.extend_from_slice(&(section.len() as u64).to_be_bytes());
    } else {
        out.extend_from_slice(&(section.len() as u32).to_be_bytes());
    }
    out.extend_from_slice(&section);
    if let Some(composite) = composite {
        out.extend_from_slice(composite);
    }
    out
}

fn refused_cheaply(bytes: &[u8]) {
    let (result, peak) = peak_of(|| read_psd(bytes));
    assert!(result.is_err(), "a hostile file was accepted");
    assert!(peak < 4 * MIB, "refusal still requested {peak} bytes");
}

#[test]
fn a_psb_header_past_the_budget_is_refused_before_allocating() {
    // 300000² RGB with a tiny ZIP composite requested a 270 GB plane (#844 H8).
    let header = Header {
        psb: true,
        ..Header::rgb(300_000, 300_000)
    };
    let mut composite = vec![0, 2];
    composite.extend_from_slice(&zlib(&[0u8; 64]));
    refused_cheaply(&psd(&header, &[], Some(&composite)));
}

#[test]
fn a_composite_over_the_document_total_is_refused() {
    // 56 channels of 30000² is 50 GB of planes, each within the per-plane
    // cap; the RLE count table alone was 3.4 MB of claimed rows (#844 M3).
    let header = Header {
        channels: 56,
        ..Header::rgb(30_000, 30_000)
    };
    refused_cheaply(&psd(&header, &[], Some(&[0, 1])));
}

fn huge_layer(channel: (i16, Vec<u8>)) -> LayerSpec {
    LayerSpec {
        rect: [0, 0, 2, i32::MAX],
        channels: vec![channel],
        ..LayerSpec::default()
    }
}

#[test]
fn a_huge_rle_layer_rect_is_refused() {
    // A 2^31-wide layer with a few bytes of RLE zero-filled 4 GB (#844 H1).
    refused_cheaply(&psd(
        &Header::rgb(4, 4),
        &[huge_layer(rle_channel(0, 2))],
        None,
    ));
}

#[test]
fn a_huge_zip_layer_rect_is_refused() {
    refused_cheaply(&psd(
        &Header::rgb(4, 4),
        &[huge_layer(zip_channel(0))],
        None,
    ));
}

#[test]
fn a_huge_mask_rect_is_refused() {
    let layer = LayerSpec {
        rect: [0, 0, 1, 1],
        channels: vec![raw_channel(0, 1), rle_channel(-2, 3)],
        mask: Some([0, 0, 3, i32::MAX]),
        ..LayerSpec::default()
    };
    refused_cheaply(&psd(&Header::rgb(4, 4), &[layer], None));
}

#[test]
fn a_huge_bitmap_layer_rect_is_refused() {
    // Depth 1 expanded every layer through `plane * 3` (#844 H1).
    let header = Header {
        channels: 1,
        depth: 1,
        mode: 0,
        ..Header::rgb(8, 8)
    };
    let layer = LayerSpec {
        rect: [0, 0, 2, 1 << 30],
        channels: vec![rle_channel(0, 2)],
        ..LayerSpec::default()
    };
    refused_cheaply(&psd(&header, &[layer], None));
}

#[test]
fn a_rect_spanning_the_whole_i32_range_is_refused() {
    let layer = LayerSpec {
        rect: [i32::MIN, i32::MIN, i32::MAX, i32::MAX],
        channels: vec![rle_channel(0, 2)],
        ..LayerSpec::default()
    };
    refused_cheaply(&psd(&Header::rgb(4, 4), &[layer], None));
}

fn raw_layers(count: usize, side: i32) -> Vec<LayerSpec> {
    (0..count)
        .map(|_| LayerSpec {
            rect: [0, 0, side, side],
            channels: (0..3)
                .map(|id| raw_channel(id, (side * side) as usize))
                .collect(),
            ..LayerSpec::default()
        })
        .collect()
}

#[test]
fn layer_planes_count_against_the_document_total() {
    // Each plane fits the per-allocation cap; together they do not (#844 H1).
    let budget = ImageBudget {
        max_dimension: 30_000,
        max_alloc_bytes: 64 * 1024,
    };
    let over = psd(&Header::rgb(4, 4), &raw_layers(8, 200), None);
    assert!(read_psd_within(&over, Policy::Convert, budget).is_err());
    let within = psd(&Header::rgb(4, 4), &raw_layers(2, 200), None);
    let doc = read_psd_within(&within, Policy::Convert, budget).expect("within budget");
    assert_eq!(doc.layers.len(), 2);
}

fn nested_groups(depth: usize) -> Vec<LayerSpec> {
    let mut layers: Vec<LayerSpec> = (0..depth)
        .map(|_| LayerSpec {
            section: Some(3),
            ..LayerSpec::default()
        })
        .collect();
    layers.extend(raw_layers(1, 1));
    layers.extend((0..depth).map(|_| LayerSpec {
        section: Some(1),
        ..LayerSpec::default()
    }));
    layers
}

#[test]
fn group_nesting_is_capped() {
    // 32767 nested groups overflowed the compositor's stack (#844 M9).
    let deep = psd(&Header::rgb(4, 4), &nested_groups(65), None);
    assert!(read_psd(&deep).is_err());
    let doc = read_psd(&psd(&Header::rgb(4, 4), &nested_groups(64), None)).expect("at the cap");
    let mut depth = 0;
    let mut layer = &doc.layers[0];
    while layer.is_group {
        depth += 1;
        layer = &layer.children[0];
    }
    assert_eq!(depth, 64);
}

#[test]
fn a_folder_without_a_divider_is_an_empty_group() {
    // Found by the mutation pass: an unmatched folder record popped the root
    // of the group stack and panicked.
    let folder = LayerSpec {
        section: Some(1),
        ..LayerSpec::default()
    };
    let mut layers = raw_layers(1, 1);
    layers.push(folder);
    let doc = read_psd(&psd(&Header::rgb(4, 4), &layers, None)).expect("reads");
    assert_eq!(doc.layers.len(), 2);
    assert!(doc.layers[1].is_group && doc.layers[1].children.is_empty());
}

#[test]
fn a_canvas_class_document_header_still_opens() {
    // The budget must not refuse the 16000²-class canvas: a layered RGB file
    // without a merged image reads (its 768 MB blank composite is zeroed
    // lazily, under this file's 1 GiB refusal line).
    let header = Header::rgb(16_000, 16_000);
    let layers = raw_layers(1, 1);
    assert!(read_psd(&psd(&header, &layers, None)).is_ok());
}

// ---------------------------------------------------------------------------
// Seeded mutation smoke test
// ---------------------------------------------------------------------------

/// xorshift64*: a fixed, dependency-free stream for reproducible mutations.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// Values that sit on parser boundaries: zero, sign bits, and the extremes.
const INTERESTING: [u32; 7] = [
    0,
    1,
    0x7fff_ffff,
    0x8000_0000,
    0xffff_ffff,
    0x4000_0000,
    0xffff,
];

/// One to eight random edits of `seed`: byte flips, boundary words, and
/// truncation.
fn mutate(seed: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut out = seed.to_vec();
    for _ in 0..1 + rng.below(8) {
        if out.is_empty() {
            break;
        }
        let at = rng.below(out.len());
        match rng.below(4) {
            0 => out[at] ^= 1 << rng.below(8),
            1 => out[at] = rng.next() as u8,
            2 => {
                let word = INTERESTING[rng.below(INTERESTING.len())].to_be_bytes();
                let end = (at + 4).min(out.len());
                out[at..end].copy_from_slice(&word[..end - at]);
            }
            _ => out.truncate(at.max(1)),
        }
    }
    out
}

/// Run `parse` over `iterations` mutations of each seed, failing with the
/// reproducing seed and iteration on a panic or an allocation over `limit`.
fn fuzz(name: &str, seeds: &[Vec<u8>], iterations: usize, limit: usize, parse: impl Fn(&[u8])) {
    let mut rng = Rng(0x5eed_0844_cafe_f00d);
    for (s, seed) in seeds.iter().enumerate() {
        for i in 0..iterations {
            let input = mutate(seed, &mut rng);
            let (result, peak) = peak_of(|| {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse(&input)))
            });
            assert!(result.is_ok(), "{name}: seed {s} iteration {i} panicked");
            assert!(
                peak <= limit,
                "{name}: seed {s} iteration {i} allocated {peak} bytes"
            );
        }
    }
}

fn psd_seeds() -> Vec<Vec<u8>> {
    let rgba: Vec<u8> = (0..6 * 5 * 4).map(|i| (i * 37) as u8).collect();
    let doc = Document::from_rgba("Layer 0", 6, 5, &rgba);
    vec![
        write_psd(&doc).expect("seed writes"),
        include_bytes!("fixtures/group.psd").to_vec(),
        include_bytes!("fixtures/masked.psd").to_vec(),
        include_bytes!("fixtures/bitmap.psd").to_vec(),
        include_bytes!("fixtures/rgb16_layered.psd").to_vec(),
        psd(&Header::rgb(4, 4), &nested_groups(3), None),
    ]
}

#[test]
fn mutated_psds_never_panic_or_overallocate() {
    let budget = ImageBudget {
        max_dimension: 30_000,
        max_alloc_bytes: MIB as u64,
    };
    fuzz("psd", &psd_seeds(), 3000, 64 * MIB, |bytes| {
        let _ = read_psd_within(bytes, Policy::Convert, budget);
    });
}

fn pdf_seed(password: &str) -> Vec<u8> {
    let rgba: Vec<u8> = (0..8 * 8 * 4).map(|i| (i * 11) as u8).collect();
    let doc = Document::from_rgba("Layer 0", 8, 8, &rgba);
    let mut options = PdfOptions::default();
    options.security.open_password = password.to_string();
    let env = PdfEnv {
        jpeg: &|_| Some(b"\xff\xd8FAKE".to_vec()),
        now: 0,
        entropy: [3; 32],
        title: "seed".into(),
        live_text: None,
    };
    write_pdf(&doc, &doc.composite, &options, &env).expect("seed writes")
}

#[test]
fn mutated_pdfs_never_panic_or_overallocate() {
    let seeds = vec![pdf_seed(""), pdf_seed("pw")];
    assert!(matches!(editing_psd(&seeds[1], "pw"), EditingPsd::Psd(_)));
    let budget = ImageBudget {
        max_dimension: 30_000,
        max_alloc_bytes: MIB as u64,
    };
    fuzz("pdf", &seeds, 1500, 64 * MIB, |bytes| {
        if let EditingPsd::Psd(psd) = editing_psd(bytes, "pw") {
            let _ = read_psd_within(&psd, Policy::Convert, budget);
        }
        let _ = PdfImages::read(bytes);
    });
}
