use pictura_core::PsdRect;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// The existing blend-pass timestamps travel with every fragment of one result.
pub struct FrameTileSample {
    generation: u64,
    nanos: u64,
    consumed: AtomicBool,
}

impl FrameTileSample {
    pub fn new(generation: u64, nanos: u64) -> Arc<Self> {
        Arc::new(Self {
            generation,
            nanos,
            consumed: AtomicBool::new(false),
        })
    }
}

/// Coverage is in level pixels, using half-open rectangles like the tile cache.
#[derive(Default)]
pub struct FrameTileSamples(Vec<(PsdRect, Arc<FrameTileSample>)>);

impl FrameTileSamples {
    pub fn overwrite(&mut self, rect: PsdRect, sample: Option<&Arc<FrameTileSample>>) {
        let mut kept = Vec::new();
        for (covered, old) in self.0.drain(..) {
            if old.consumed.load(Ordering::Relaxed) {
                continue;
            }
            let Some(cut) = covered.intersect(rect) else {
                kept.push((covered, old));
                continue;
            };
            let pieces = [
                PsdRect {
                    bottom: cut.top,
                    ..covered
                },
                PsdRect {
                    top: cut.bottom,
                    ..covered
                },
                PsdRect {
                    top: cut.top,
                    bottom: cut.bottom,
                    right: cut.left,
                    ..covered
                },
                PsdRect {
                    top: cut.top,
                    bottom: cut.bottom,
                    left: cut.right,
                    ..covered
                },
            ];
            for piece in pieces {
                if piece.width() > 0 && piece.height() > 0 {
                    kept.push((piece, Arc::clone(&old)));
                }
            }
        }
        if let Some(sample) = sample {
            kept.push((rect, Arc::clone(sample)));
        }
        self.0 = kept;
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn copy_clipped_from(&mut self, source: &Self, rect: PsdRect, scale: i32) {
        self.0
            .extend(source.0.iter().filter_map(|(covered, sample)| {
                if sample.consumed.load(Ordering::Relaxed) {
                    return None;
                }
                let scaled = PsdRect {
                    left: covered.left.saturating_mul(scale),
                    right: covered.right.saturating_mul(scale),
                    top: covered.top.saturating_mul(scale),
                    bottom: covered.bottom.saturating_mul(scale),
                };
                Some((scaled.intersect(rect)?, Arc::clone(sample)))
            }));
    }

    pub fn retain_generation(&mut self, generation: u64) {
        self.0.retain(|(_, sample)| {
            sample.generation == generation && !sample.consumed.load(Ordering::Relaxed)
        });
    }

    pub fn rects(&self) -> impl Iterator<Item = PsdRect> + '_ {
        self.0.iter().map(|&(rect, _)| rect)
    }

    pub fn take(&self, index: usize, generation: u64) -> Option<u64> {
        let (_, sample) = self.0.get(index)?;
        (sample.generation == generation && !sample.consumed.swap(true, Ordering::Relaxed))
            .then_some(sample.nanos)
    }
}
