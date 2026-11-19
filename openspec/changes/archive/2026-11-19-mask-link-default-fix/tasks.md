# Tasks

## 1. Tests

- [x] 1.1 Add `pictura-app::default_masks_are_linked` beside
  `white_default_mask_has_a_thumbnail`: both `default_layer_mask` branches leave
  `flags & 0x01` clear.

## 2. Fix

- [x] 2.1 Drop `flags: 1` from `default_layer_mask`'s reveal-all branch so the
  mask takes the linked default, with a comment naming `MASK_FLAG_UNLINKED`.

## 3. Verify

- [ ] 3.1 Run fmt, clippy, the new test, the existing mask suites, and
  `openspec validate --strict`; archive the change.
