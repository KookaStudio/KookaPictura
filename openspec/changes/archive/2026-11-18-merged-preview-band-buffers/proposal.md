# Proposal: merged-preview band buffers belong to the reading thread (#1423)

## Why

#1321 made a lazily opened PSD's merged preview decode a band's three colour
planes side by side on the rayon workers. Each worker allocated its band's
output (and copied it once more into an `Arc`) and its packed read buffer, and
glibc keeps a block freed into a worker's arena there, one arena per worker.
M12 at 17000² peaked at 723–730 MiB against 647 MiB before #1321; the extra
was arena heap, not live data. Decoding the bands serially on the reading
thread brought the peak back (629–637 MiB) but cost 0.8 s of first paint.

An allocator-wide fix was measured first and dropped (design §1): a fixed
mmap threshold or `MALLOC_ARENA_MAX=2` lowered the peak but made first paint
3.1–4.4 s against 2.2 s.

## What changes

- `MergedPreview::region` allocates every band buffer and packed read buffer
  on the reading thread and the workers only fill them; the packed buffers are
  kept from band to band within one read.
- A band decodes straight into its `Arc<[u8]>`, without the copy from a `Vec`.

No pixel changes; the merged preview still matches the merged image
(`codec/lazy-psd-open`).

## Capabilities

- New: `codec/merged-preview-band-buffers`.
