# Tasks

## Tests first

- [x] 1.1 Add the engine tests named in `test-plan.md`: the CMYK identity,
      blend, constant, and validation cases in `pictura-adjust`.
- [x] 1.2 Add the render tests: `mixr` CMYK round-trip, K-source slot, clamp,
      mode-aware page, default, compositor, destructive, native, and monochrome
      cases.
- [x] 1.3 Add the Qt Test cases `tst_image_adjustments::channelMixerMonochromeToggle`,
      `tst_image_adjustments::channelMixerCmykOutputs`,
      `tst_properties_panel::channelMixerMonochrome`, and
      `tst_properties_panel::channelMixerCmykGroups`.

## Engine

- [x] 2.1 Extend `ChannelMixerParams` with `ink: [[f64; 4]; 4]` and
      `ink_constant: [f64; 4]`; add `Default` with both identities.
- [x] 2.2 Add `pictura_adjust::apply_channel_mixer_ink` (unit ink, constant
      sign, clamp, validation) and export it.

## Codec and render

- [x] 3.1 Read `mixr` as four 10-byte rows; fill both the RGB rows and `ink`
      rows; keep the monochrome and RGB behaviour unchanged.
- [x] 3.2 Add `encode_channel_mixer_ink` (clamped, 44 bytes) and keep
      `encode_channel_mixer`'s RGB form.
- [x] 3.3 Add `pictura-render::channel_mixer_cmyk` with the profile-aware 8-bit
      path, the f32 GCR fallback, and the planar/canvas helpers.
- [x] 3.4 Add `pictura_codec::document_cmyk_profile` as the public accessor.
- [x] 3.5 Apply the ink path in `composite_adjustment` for non-monochrome
      Channel Mixer on a CMYK document, and keep the RGB path for monochrome.
- [x] 3.6 Add the `CmykMix` context to `apply_adjustment_region` and use it in
      the app's destructive path.
- [x] 3.7 Make `adjustment_editor`/`set_adjustment_param` mode-aware: CMYK
      groups and keys, the RGB `monochrome` check, and the toggle re-encode.
- [x] 3.8 Make `default_adjustment_block` mode-aware for the CMYK identity.

## App and UI

- [x] 4.1 Pass the document mode through `image_adjustment_default`,
      `image_adjustment_page`, `image_adjustment_set`, and
      `adjustment_layer`; expose `image_mode` to the dialog.
- [x] 4.2 Add the Monochrome checkbox to `ChannelMixerDialog` (RGB only),
      rebuild the output menu on toggle, and add the four CMYK source rows.
- [x] 4.3 Pass the mode through the Properties panel page and edit bridge.

## Docs and close-out

- [x] 5.1 Tick #571 in `ROADMAP.md`, close `ADJ-1` in
      `docs/dev/doc-code-parity-gaps.md`, and record the change in
      `docs/dev/STATE.md` (the CMYK `mixr` layout stays inferred).
- [x] 5.2 Run `openspec validate --all --strict`, the Rust tests, the Qt suites,
      and `scripts/verify-fast.sh`.
