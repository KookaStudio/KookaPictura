# Proposal

## Why

Issue [#1345](https://github.com/KookaStudio/KookaPictura/issues/1345) identifies independently writable document source metadata and composite fields that permit contradictory states. Consumers reconstruct the same decisions differently, making native-sample selection, rendering, and save behavior harder to maintain.

## What Changes

- **BREAKING** Rust interface change: replace `Document`'s public `source_mode`, `source_depth`, `source_planes`, `source_palette`, `source_icc`, and `document_icc` fields with one `SourceState` value whose invariants are private; replace public `composite` and `merged_composite_present` with one `CompositeState` value.
- Expose borrowed data access, checked construction, and coordinated state transitions rather than independent setters for correlated fields.
- Centralize effective mode/depth, retained-depth, native-plane authority, stored-composite presence, and composite-shape queries. Preserve the distinctions between these queries.
- Migrate core, codec, render, history, app, examples, and tests in the same change, including the surviving app-side stored-composite checks.
- Preserve working pixels, native samples, profile bytes, palettes, copy-on-write sharing, PSD/PSB encoding decisions, compression, and golden baselines.

## Capabilities

### New Capabilities

- `document/document-source-state`: private source/composite invariants, coordinated transitions, consumer predicates, and compatibility checks.

### Modified Capabilities

- `document/document-model`: express the existing document clone/storage-sharing contract through encapsulated source and composite values instead of removed fields.
- `color/color-profile-assignment`: expose original and working profile bytes through the state interface instead of a public `document_icc` optional, preserving profile semantics.

## Impact

The main seam is in `pictura-core`; codec read/normalize/profile/write paths and render mode/depth/composite operations construct and transition the state. App presentation, transform backdrops, snapshots, recovery, export, and memory accounting consume it. Existing field-based struct literals and fixtures need interface migration; fixture bytes and expected oracle results do not change. No new dependencies, user-facing feature, file-format change, rendering algorithm, or build-system change is proposed.
