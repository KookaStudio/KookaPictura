# Design: merged-preview-band-buffers (#1423)

## 1. Choices measured

M12 at 17000², `profile-tools.py --scenarios M12 --peak`, three runs each on
one host (peak MiB / first paint s):

| Build | Peak | First paint |
|---|---|---|
| master (with #1321) | 723–730 | 2.10–2.31 |
| `mallopt(M_MMAP_THRESHOLD, 4 MiB)` | 600 | 4.06–4.37 |
| `mallopt(M_MMAP_THRESHOLD, 16 MiB)` | 652–660 | 3.73–3.81 |
| `MALLOC_ARENA_MAX=2` | 638–679 | 3.14–3.74 |
| serial band decode | 629–637 | 2.97–3.14 |
| this change | 662–663 | 1.99–2.06 |

A fixed threshold maps every strip chunk afresh, and the kernel faults and
zeroes about 4 GiB of pages over the open; transparent huge pages are
`madvise`-only by default. The arena cap makes the workers contend. The
serial decode located the retained bytes: band buffers allocated on the
workers. The ~31 MiB strip chunks are allocated and freed on the tile job's
own thread and reuse its arena.

## 2. Mechanism

`ChannelIndex::band` splits into `band_buffer` (a zeroed `Arc<[u8]>` the size
of the band), `fit_packed` (grows a caller's packed buffer to the band's
packed bytes) and `decode_band` (fills both). `MergedPreview::bands` looks each
plane up in the band cache, prepares the missing ones' buffers on the calling
thread, decodes them side by side with `try_for_each`, and caches them there.
A worker then allocates nothing band-sized, so nothing band-sized lands in its
arena. `band` itself (backed layer tiles) keeps its behaviour, minus the copy.

## 3. Verification

An integration test with its own counting allocator asserts that a whole-image
`region` allocates no block of 64 KiB or more on a rayon worker, raw and RLE
(19.9 MB before this change). The existing `merged_preview_matches_the_merged_image`
keeps the pixels. M12 is measured by hand.
