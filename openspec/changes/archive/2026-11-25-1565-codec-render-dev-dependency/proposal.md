# Proposal

## Why

`pictura-codec` dev-depends on `pictura-render`, which depends on the codec. The cycle makes codec test binaries link the renderer and its optional GPU stack. Issue #1565 removes that edge without reducing coverage or moving document operations into `pictura-core`.

## What Changes

- Classify every codec test use of render as output, encoding, or document mutation.
- Move renderer-output and render-mutation tests to `pictura-render`.
- Point codec encoding tests at the codec APIs that render re-exports.
- Replace simple codec fixture setup with codec-local test builders, including a core-built tiled composite for the streamed raster-save heap check.
- Remove `pictura-render` from the codec dev-dependencies and lockfile package entry.
- Preserve every existing test, assertion, and ignore reason. Add no dependencies.

## Capabilities

### New Capabilities

- `meta/crate-test-dependency-boundaries`: defines dependency direction and test ownership between the codec and renderer crates.

### Modified Capabilities

None.

## Impact

Affects `crates/pictura-codec/Cargo.toml`, `Cargo.lock`, codec tests and inline test modules, render integration tests, and the render feature-gate manifest checks. Public production APIs and document mutation ownership stay unchanged.
