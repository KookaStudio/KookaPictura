# Tasks

## 1. Codec
- [x] 1.1 Split `ChannelIndex::band` into `band_buffer`, `fit_packed` and `decode_band`; decode straight into the `Arc`
- [x] 1.2 `MergedPreview::bands` allocates on the reading thread, decodes side by side, keeps packed buffers per read
- [x] 1.3 Test `mpb_caller`; `mpb_pixels` is the existing preview test

## 2. Close
- [x] 2.1 Measure M12 (`mpb_m12`) against master and the allocator alternatives, record in STATE.md
- [x] 2.2 Archive the change
