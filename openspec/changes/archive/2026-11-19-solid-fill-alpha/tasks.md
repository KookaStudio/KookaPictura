# Tasks

## 1. Tests

- [x] 1.1 Add `pictura-render::solid_fill_encoder_preserves_alpha`: the shared
  encoder yields the 4-byte tuple for alpha 40 and the version-16 descriptor for
  alpha 255, and both decode back.
- [x] 1.2 Add `pictura-render::created_solid_fill_layer_preserves_alpha`:
  `add_solid_fill` keeps `[10, 20, 30, 40]` through decode, composite, and
  save/reopen, while an opaque fill still takes the descriptor and composites
  opaque.

## 2. Encode seam

- [x] 2.1 Make the existing `encoded_fill` rule public as `encode_fill_content`
  and re-export it from `pictura-codec` and `pictura-render`.
- [x] 2.2 Route `add_solid_fill` through it instead of
  `encode_solid_color_fill` and delete the alpha `ponytail:` note.
- [x] 2.3 Route the app's `fill_layer` solid arm through the same encoder.

## 3. Verify

- [x] 3.1 Run fmt, clippy, the new tests, the test-plan check, and
  `openspec validate --strict`; archive the change.

## 4. Review follow-ups

- [x] 4.1 Cover alpha 0 in the encoder and created-layer tests.
- [x] 4.2 Scope the app's opacity requirement to its six-digit payload and
  record the external-reader limitation.
- [x] 4.3 Keep shape fills opaque while their model is RGB-only, reword the
  stale encoder doc, and update the fill-layer docs.
