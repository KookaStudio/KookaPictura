# Test plan

This plan covers `merged-preview-band-buffers` (#1423): the new capability
`codec/merged-preview-band-buffers`.

**Fixture.** A 2000×1100 layered PSD (opaque noise Background, one small top
layer) written raw and RLE, opened lazily with the merged image skipped, so
`region` decodes five bands per plane. The integration test binary installs a
global allocator that counts the bytes of blocks of 64 KiB or more allocated
while `rayon::current_thread_index()` is set.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mpb_caller` | Open each fixture lazily; read the whole image through `MergedPreview::region` | No 64 KiB-or-larger block allocated on a rayon worker during the read (master: 19.9 MB) | `pictura-codec::band_buffers_are_allocated_on_the_reading_thread` |
| `mpb_pixels` | Read a region of the lazy preview, raw and RLE; compare with the eager merged image | Colour planes equal; alpha 255 inside, 0 outside | `pictura-codec::merged_preview_matches_the_merged_image` |
| `mpb_m12` | `profile-tools.py --scenarios M12 --peak --size 17000x17000`, three runs, master build vs this branch, same host | Peak within master's pre-#1321 647–665 MiB; first paint ≤ about 2.4 s (result: 662–663 MiB, 1.99–2.06 s against 723–730 MiB, 2.10–2.31 s) | manual: needs the 17000² fixture, a GUI build and minutes of an idle machine; not a CI-sized test |
