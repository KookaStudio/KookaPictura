# Design

## Context

- `picker_store()` holds the working CMYK profile as ICC bytes in a
  `OnceLock<RwLock<Vec<u8>>>`. `set_color_settings` replaces the bytes, and
  `working_profile()` re-parses them on every service call.
- `pictura_codec::{rgb,cmyk}_to_cmyk_with_profile` call
  `pictura_color::convert_space`, which builds a fresh `lcms2::Transform` each
  time. For one pixel the build dominates the cost.
- `ByteTransform` is reusable but same-format only (grey/RGB/RGBA in and out),
  so it cannot carry RGB→CMYK.
- lcms2 transforms are `Sync` only with `DisallowCache` (`Flags::NO_CACHE`).
  The one-pixel cache does not change output
  (`byte_transform_cache_flag_does_not_change_output`).

## Decisions

### D1: `SpaceTransform` in `pictura-color`

A struct holding a `Transform<u8, u8, GlobalContext, DisallowCache>` plus its
in/out channel counts. `new(src, dst, intent, bpc)` picks the pixel format from
each profile's colour space, as `convert_space` does today. `apply(data, out)`
converts interleaved pixels. `convert_space` keeps its validation and empty-input
behaviour and delegates the transform to `SpaceTransform`. Rejected: widening
`ByteTransform`, whose channel count means "same format with alpha copied".

### D2: Cache the picker pair, not the profile

`PICKER` becomes `OnceLock<RwLock<Option<PickerTransforms>>>`, where
`PickerTransforms` holds `to_cmyk` and `to_rgb`. It is built from the ICC bytes
with `picker_options()`, the same intent and BPC as before. If the bytes do not
parse, it falls back to the bundled Coated profile, as `working_profile` did. If
the transform build fails, the slot is `None` and every service returns its
existing fallback (`[0,0,0,100]`, `u32::MAX`, in gamut). `set_color_settings`
calls `install_picker_profile(&icc)`.

One pixel has the same interleaved and planar layout, so the services convert
`[u8; 3]` / `[u8; 4]` arrays directly with no planar round trip.

A `#[cfg(test)]` build counter lets a unit test prove that repeated calls do not
rebuild the pair. nextest runs each test in its own process, so the counter
sees no other test.

## Risks / Trade-offs

- The pair stays alive for the process lifetime: two small lcms2 transforms. The
  lifetime is the same as the stored ICC bytes it replaces.
