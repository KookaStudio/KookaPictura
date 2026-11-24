# Proposal

## Why

One Free Transform gesture's preview state has three owners, and its teardown
code has drifted into three different field lists. Cancelling a transform walks
`cancel_transform_session`, which clears 12 fields; the C++ cancel walks
`clear_transform_preview`, which clears 19 preview fields plus the mask-move
baseline. The seven preview fields only the second list clears include
`move_above` and `move_drag_active`, so a cancelled Move controls gesture leaves
a stale above-layer image on the canvas and leaves `move_drag_active` true,
which is the flag a landing Move composite reads to decide whether the drag
preview still owns the canvas. A third list, `clear_document_scoped_state`,
clears 18 of the 19 preview fields and misses `move_base_origin`, and a failed
begin clears none of them.

The same session's 522 lines of Move-tool preview code sit in
`impl_transform/session.rs`, which is at exactly the 1200-line hard cap, so the
next transform change has nowhere to land.

## What Changes

- Add one teardown for the 20 preview and Move fields on the plain-Rust state
  and route the existing clear paths through it, so a cancelled, failed, or
  committed transform always leaves the same field set clear. This replaces the
  three hand-maintained field lists with one.
- Clear the overlay on the failed-begin branch, so a refused begin cannot leave
  the outgoing session's preview fields set. The C++ controller also drops its
  overlay when that failure leaves no live Rust session.
- Clear the Move controls flag when a begin takes an outgoing session, installs
  a new session, or commits, and in both cancel paths. A begin refused before
  the outgoing take keeps the live controls session and its flag.
- Move the Move-tool preview code out of `impl_transform/session.rs` into a
  sibling module: a pure move with no behaviour change, which brings the file
  back under the cap. `clear_transform_preview` stays behind as a wrapper.
- Replace the nine literal `TransformSession { … }` constructions with one
  constructor, so a field added to the session cannot silently miss a test
  fixture.
- Derive the C++ controls-session flag from the idle-box state plus the Rust
  session flag, and delete `transformControls_` and its thirteen assignments, so
  the field has one owner.

No edit rule changes: the begin, gesture, commit, cancel, and history contracts,
the pixel output, the existing signal set, and the self-test tokens stay as they
are. No golden image changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/document-session`: the requirement "The live edit session is one
  owned value" gains the preview-state teardown rule (one clear path for every
  way a transform ends, including a failed begin, a deferred landing, and a
  document replacement), the rule that a failed or cancelled transform leaves no
  preview or Move field set, and the rule that the C++ controller derives its
  controls-session flag instead of storing a copy.

## Impact

- `crates/pictura-app/src/cxxqt_object/state.rs` — the preview teardown lives on
  `PictureViewRust`; the 20 preview and Move fields stay where they are.
- `crates/pictura-app/src/cxxqt_object/impl_transform/session.rs` —
  `clear_transform_preview` becomes a wrapper; the failed-begin branch clears the
  overlay; loses the Move-preview code (~522 lines).
- `crates/pictura-app/src/cxxqt_object/document_session/session.rs` —
  `cancel_transform_session` and `clear_document_scoped_state` call the one
  teardown, and both cancel paths clear the controls flag.
- `crates/pictura-app/src/cxxqt_object/impl_transform/session/move_preview.rs` —
  new module holding the moved code.
- `crates/pictura-app/src/cxxqt_object/document_session/transform.rs` — gains
  `TransformSession::new`.
- Seven Rust test modules and the two production literal sites call the
  constructor.
- `crates/pictura-app/cpp/tools.h`, `tool_transform.cpp` — `transformControls_`
  deleted; `transformControlsSession()` becomes a definition in
  `tool_transform.cpp` reading the idle-box state plus the bridge call. A failed
  replacement begin resets the C++ overlay when no Rust session remains.
- No dependency, format, or persisted-state change. Rollback is a commit revert.
