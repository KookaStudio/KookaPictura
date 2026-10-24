# Proposal

## Why

`Filter ▸ Video ▸ NTSC Colors` is a disabled stub in `crates/pictura-app/cpp/command_tree.cpp`. There is no engine kernel, so the menu entry has no descriptor, no mapping kind and no way to be applied. The behavioural contract already exists in `docs/06-filters/filters-overview.md`: the filter clamps the image's gamut into the NTSC video-safe colour space, so oversaturated colours do not bleed on an NTSC television.

Issue #345 asks for the kernel, the parameterless kind and the menu row, with an OpenSpec requirement and tests. It is part of the remaining-filter audit on #167.

## What Changes

- `pictura-color` gains a public `ntsc` module: sRGB↔YIQ conversion and the gamut clamp. The clamp reduces the `(I, Q)` chroma vector to a magnitude of at most `0.5` while preserving its direction, so luma and hue stay put and only saturation is reduced. Adobe's kernel is closed, so this is recorded as an approximation.
- `pictura-filters` gains `Filter::NtscColors`, a thin `video::ntsc_colors` kernel that runs the colour helper over the planar 8-bit buffer, and the variant/engine/property registrations the generated tables need.
- `pictura-app` gains the `ntsc-colors` descriptor row (`Filter/Video/NTSC Colors`, no controls, shared preview flags) and the mapping arm. The existing menu leaf becomes enabled and applies immediately — CS6 has no NTSC Colors dialog.
- The filter engine stays planar 8-bit, like every other filter. The docs' 16- and 32-bpc availability is an engine-wide gap, not part of this change.

## Capabilities

### New Capabilities

- `imaging/video-filters`: the NTSC Colors gamut-clamp contract and its parameterless command. `De-Interlace` (#344) will extend the same capability when it lands.

### Modified Capabilities

None. `imaging/filter-app-ui` already states the generic rule that every kind with a descriptor is enabled and that a parameterless entry applies directly, so no requirement there changes.

## Impact

- **Engine.** `crates/pictura-filters/src/video.rs` (new), `filter/types.rs`, `filter/variants.rs`, `filter/props.rs`, `filter/apply.rs`, `lib.rs`.
- **Colour.** `crates/pictura-color/src/ntsc.rs` (new), `lib.rs`.
- **Dependency.** `pictura-filters` now depends on `pictura-color`. This is a deliberate direction from the issue and the color-model move (#269): the RGB↔YIQ math belongs with the colour crate. There is no cycle (`pictura-color` depends only on `lcms2`), but the filter crate's build now also builds `lcms2`.
- **App.** `filter_map/descriptor/video.rs` (new) and `descriptor/mod.rs`, `filter_map/mapping.rs`, the descriptor and mapping tests, and a Qt Test case in `tst_filter_menu.cpp`.
- **Tests.** Rust unit tests in both crates, the existing exhaustive-kind tests (`instances`, `kernel_digests`, `oracle/mapping`), the existing app tests, and one new Qt Test case.
- **Docs.** `ROADMAP.md` ticks NTSC Colors and leaves De-Interlace (#344) open. No `docs/` change is needed: the behavioural contract already covers the filter.
