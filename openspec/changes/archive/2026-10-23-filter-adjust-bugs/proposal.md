# Proposal: Filter and adjustment parity bugs

## Why

The audit found proxy previews using unscaled document-pixel parameters, preview kernels anchored to the crop, stale canvases after a refused preview, three adjustment formulas that break at neutral or native depth, and statistics computed outside the selection. The bundle builds on the `#826` exhaustive-filter-facts refactor.

## What changes

- #855 — the filter descriptor marks every document-pixel parameter explicitly; the Filter Gallery proxy, the gallery's catalog thumbnails, and the filter dialog's reduced copy scale those parameters by the copy's scale. The list includes the five controls whose labels carry no pixel suffix (Smart Blur Radius, Mosaic/Crystallize/Pointillize Cell Size, Color Halftone Max Radius), Mosaic Tiles Tile/Grout Width, and the other confirmed distance controls the design lists. Abstract levels such as Dry Brush/Fresco/Sponge Brush Size are not pixel distances and stay unflagged.
- #856 — Radial Blur, Clouds, Difference Clouds, and Fibers join the whole-layer preview set, so a cropped section preview no longer changes their geometry.
- #857 — a refused live preview restores the unfiltered layer and composite, clears the preview session, and leaves Cancel a no-op.
- #858 — Oil Paint Shine spans 0..10 and opens on 5.5; Offset opens on 0/0 and gains Repeat Edge Pixels (clamp-to-edge).
- #859 — Vibrance skips the vibrance term when the input saturation is zero, so neutral gray stays neutral; the documented `1 - S` falloff is unchanged.
- #860 — Auto Tone, Auto Contrast, and Auto Color compute their histogram, percentile bounds, and midtone means from the selected pixels when a selection is active.
- #862 — native-depth Exposure uses the linear domain for 32-bit stores, Posterize quantizes at the store's resolution, and Black & White and Channel Mixer stop rounding through an 8-bit grid while keeping u8 parity.

## Capabilities

- Modified: `imaging/filter-gallery-preview`
- Modified: `imaging/filter-dialog-preview`
- Modified: `imaging/filter-application`
- Modified: `imaging/filter-app-ui`
- Modified: `imaging/oil-paint-filter`
- Modified: `imaging/other-filters`
- Modified: `imaging/image-adjustments`
- Modified: `imaging/native-depth-adjustments`

## Impact

`crates/pictura-app/src/cxxqt_object/{filter_tools,filter_tools/jobs,filter_map,filter_map/mapping,filter_map/descriptor/*,impl_filters,image_adjust,image_replace_color,image_match_color}.rs`, `crates/pictura-app/cpp/{filter_gallery_dialog,filter_param_controls,filter_commands,filter_preview_dialog,lens_flare_dialog,lighting_effects_dialog,reduce_noise_dialog,picture_frame_dialog}`, `crates/pictura-app/cpp/tests/tst_lighting_effects_dialog.cpp`, `crates/pictura-filters/src/{oil_paint,other,filter/apply,filter/props,filter/types}.rs`, `crates/pictura-adjust/src/{color,native,auto,apply,lib}.rs`, `crates/pictura-render/src/filter.rs`.

## Issues

Closes #855, #856, #857, #858, #859, #860, #862.
