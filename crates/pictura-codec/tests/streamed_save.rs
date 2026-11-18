//! The streamed PSD and raster saves (#1320): the file equals the in-memory
//! encode, and the writer's heap stays one band of rows however large the
//! document. The heap is measured with the counting allocator over each save,
//! after the document is built; one lock serialises the measurements.

use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::Mutex;

use pictura_codec::{SaveKind, SaveOptions};
use pictura_core::{Compression, Document};
use pictura_testkit::PeakAlloc;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc::system();
static MEASURE: Mutex<()> = Mutex::new(());

const MIB: usize = 1 << 20;
const SIDE: u32 = 4096;

fn noise(seed: u32, len: usize) -> Vec<u8> {
    let mut state = seed.wrapping_mul(2_654_435_761).wrapping_add(1);
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 24) as u8
        })
        .collect()
}

/// An RGB document of `side`² with three noise layers.
fn noise_layers(side: u32) -> Document {
    let n = (side * side) as usize * 4;
    let mut doc = Document::from_rgba("Layer 0", side, side, &noise(1, n));
    for seed in 2..4 {
        let top = Document::from_rgba("Layer", side, side, &noise(seed, n));
        doc.layers
            .push(top.layers.get(0).expect("a layer").clone())
            .expect("push");
    }
    doc
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "pictura-streamed-save-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// The heap `run` adds over what was live when it started.
fn heap_peak(run: impl FnOnce()) -> usize {
    let base = HEAP.current();
    HEAP.reset();
    run();
    HEAP.peak().saturating_sub(base)
}

/// Write `doc` as a PSD into `path` through a buffered file, as the save does.
fn write_file(doc: &Document, path: &std::path::Path) {
    let mut out = BufWriter::new(File::create(path).expect("create"));
    pictura_codec::write_psd_to(&mut out, doc, pictura_codec::writes_psb(doc), None)
        .expect("streamed write");
}

// [sps_identical]
#[test]
fn streamed_file_equals_in_memory_encode() {
    let dir = temp_dir("identical");
    let mut docs: Vec<(String, Document)> = Vec::new();
    let base = noise_layers(300);
    for (name, compression) in [
        ("rle", Compression::Rle),
        ("raw", Compression::Raw),
        ("zip", Compression::Zip),
        ("zipp", Compression::ZipPrediction),
    ] {
        let mut doc = base.clone();
        doc.layer_compression = compression;
        doc.composite_compression = compression;
        docs.push((name.into(), doc));
    }
    let mut psb = base.clone();
    psb.is_psb = true;
    docs.push(("psb".into(), psb));
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for name in ["gray.psd", "rgb16_layered.psd", "rgb32_layered.psd", "masked.psd"] {
        let bytes = std::fs::read(fixtures.join(name)).expect("fixture");
        docs.push((name.into(), pictura_codec::read_psd(&bytes).expect("reads")));
    }
    for (name, doc) in docs {
        let path = dir.join(format!("{name}.out"));
        write_file(&doc, &path);
        let expected = pictura_codec::write_psd(&doc).expect("in-memory encode");
        assert!(
            std::fs::read(&path).expect("written") == expected,
            "{name}: the file differs from the in-memory encode"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

fn bounded_psd_save(compression: Compression) -> (usize, u64) {
    let _lock = MEASURE.lock().unwrap_or_else(|e| e.into_inner());
    let mut doc = noise_layers(SIDE);
    doc.layer_compression = compression;
    doc.composite_compression = compression;
    let dir = temp_dir(&format!("{compression:?}"));
    let path = dir.join("big.psd");
    let peak = heap_peak(|| write_file(&doc, &path));
    let len = std::fs::metadata(&path).expect("written").len();
    let _ = std::fs::remove_dir_all(&dir);
    (peak, len)
}

// [sps_bounded]
#[test]
fn rle_save_heap_is_bounded() {
    let (peak, len) = bounded_psd_save(Compression::Rle);
    assert!(len > 100 * MIB as u64, "the file is {len} B");
    assert!(peak < 64 * MIB, "the RLE save's heap peak is {peak} B");
}

// [sps_zip_bounded]
#[test]
fn zip_save_heap_is_bounded() {
    for compression in [Compression::Zip, Compression::ZipPrediction] {
        let (peak, _) = bounded_psd_save(compression);
        assert!(
            peak < 64 * MIB,
            "the {compression:?} save's heap peak is {peak} B"
        );
    }
}

// [srs_bounded]
#[test]
fn raster_save_heap_is_bounded() {
    let _lock = MEASURE.lock().unwrap_or_else(|e| e.into_inner());
    let n = (SIDE * SIDE) as usize * 4;
    let mut doc = Document::from_rgba("Layer 0", SIDE, SIDE, &noise(9, n));
    let tiled = pictura_render::composite_tiled(&doc);
    doc.composite.replace(tiled).expect("valid composite");
    let dir = temp_dir("raster");
    let cases = [
        ("png", SaveKind::Png, SaveOptions::default()),
        (
            "tif",
            SaveKind::Tiff,
            SaveOptions {
                tiff_compression: 2,
                tiff_layer_compression: 0,
                ..SaveOptions::default()
            },
        ),
        (
            "bmp",
            SaveKind::Bmp,
            SaveOptions {
                bmp_depth: 24,
                ..SaveOptions::default()
            },
        ),
    ];
    for (ext, kind, options) in cases {
        let path = dir.join(format!("big.{ext}"));
        let peak = heap_peak(|| {
            pictura_codec::save_document(kind, &doc, &options, &path).expect("save");
        });
        assert!(path.exists());
        assert!(peak < 64 * MIB, "the {ext} save's heap peak is {peak} B");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// [sps_error_no_file]
#[test]
fn a_failed_raster_stream_leaves_the_target() {
    let dir = temp_dir("failed");
    let path = dir.join("keep.png");
    std::fs::write(&path, b"previous").expect("seed");
    let doc = noise_layers(300);
    let refuse = || false;
    let result = pictura_codec::save_document_with_jpeg_guarded(
        SaveKind::Png,
        &doc,
        &SaveOptions::default(),
        &path,
        None,
        Some(&refuse),
    );
    assert!(result.is_err());
    assert_eq!(std::fs::read(&path).expect("kept"), b"previous");
    let names: Vec<_> = std::fs::read_dir(&dir)
        .expect("dir")
        .map(|e| e.expect("entry").file_name())
        .collect();
    assert_eq!(names, vec![std::ffi::OsString::from("keep.png")]);
    let _ = std::fs::remove_dir_all(&dir);
}
