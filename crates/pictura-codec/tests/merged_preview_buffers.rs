//! A merged preview's band buffers belong to the reading thread (#1423): a
//! block a rayon worker allocates stays in that worker's glibc arena after it
//! is freed, so band-sized buffers allocated on the workers held a fit-to-screen
//! open's memory up after it was freed. Its own test binary, because the
//! counting allocator below is process-wide.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use pictura_codec::{read_psd_lazy, write_psd, ImageBudget, OpenOptions, SourcePin};
use pictura_core::{Compression, Document, PsdRect};

/// Blocks at least this large count as band-sized.
const LARGE: usize = 64 * 1024;

/// Counts the bytes of large blocks allocated on rayon worker threads.
struct WorkerAlloc {
    large_on_workers: AtomicUsize,
}

// SAFETY: every call forwards to `System` unchanged; the wrapper only counts.
unsafe impl GlobalAlloc for WorkerAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.count(layout);
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.count(layout);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

impl WorkerAlloc {
    fn count(&self, layout: Layout) {
        if layout.size() >= LARGE && rayon::current_thread_index().is_some() {
            self.large_on_workers
                .fetch_add(layout.size(), Ordering::Relaxed);
        }
    }
}

#[global_allocator]
static HEAP: WorkerAlloc = WorkerAlloc {
    large_on_workers: AtomicUsize::new(0),
};

const W: u32 = 2000;
const H: u32 = 1100;

fn noise(seed: u32, n: usize) -> Vec<u8> {
    (0..n as u32)
        .map(|i| (i.wrapping_add(seed).wrapping_mul(2_654_435_761) >> 13) as u8)
        .collect()
}

/// A layered RGB document whose Background is opaque noise, so its merged
/// image is skipped at a lazy open and read through the preview.
fn layered(compression: Compression) -> Document {
    let mut doc = Document::from_rgba("Background", W, H, &noise(1, (W * H * 4) as usize));
    let bottom = doc.layers.get_mut(0).expect("a layer");
    bottom.channels_mut().retain(|c| c.id != -1);
    bottom.background = true;
    let top = Document::from_rgba("Top", 64, 64, &noise(7, 64 * 64 * 4));
    doc.layers
        .push(top.layers.get(0).expect("a layer").clone())
        .expect("push");
    doc.layer_compression = compression;
    doc.composite_compression = compression;
    doc
}

// [mpb_caller]
#[test]
fn band_buffers_are_allocated_on_the_reading_thread() {
    for compression in [Compression::Rle, Compression::Raw] {
        let dir = std::env::temp_dir().join(format!(
            "pictura-preview-buffers-{compression:?}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("doc.psd");
        std::fs::write(&path, write_psd(&layered(compression)).expect("write")).unwrap();
        let pin = SourcePin::open(&path).expect("pin");
        let bytes = pin.map().expect("map");
        let options = OpenOptions {
            skip_layered_merged: true,
            source: Some(pin.clone()),
            ..OpenOptions::default()
        };
        let (_, preview) = read_psd_lazy(&bytes, &options, ImageBudget::default()).expect("open");
        let preview = preview.expect("a preview");

        let before = HEAP.large_on_workers.load(Ordering::Relaxed);
        // Every band of the image, each decoded for the first time.
        let region = preview.region(PsdRect {
            top: 0,
            left: 0,
            bottom: H as i32,
            right: W as i32,
        });
        let on_workers = HEAP.large_on_workers.load(Ordering::Relaxed) - before;
        assert_eq!(
            on_workers, 0,
            "{compression:?}: band-sized blocks allocated on workers"
        );
        assert_eq!((region.width, region.height), (W, H));
        std::fs::remove_dir_all(&dir).ok();
    }
}
