# Proposal

## Why

Issue #1448: with the Move tool and `Show Transform Controls`, a handle drag
commits on release. The options bar therefore returns to the Move page and the
user never sees the Free Transform page with Confirm/Reject, unlike CS6, where
the resized object stays pending with that menu.

## What Changes

- A Move controls handle gesture that changes the target becomes a **pending**
  Free Transform session on release instead of committing immediately.
- The options bar shows the full transform page with Confirm/Reject while the
  transform is pending; the idle box and an in-progress gesture keep the Move
  page.
- The pending transform locks document edits and asks Apply / Cancel / Don't
  Apply before a tool switch or layer change, exactly like a modal Free
  Transform.
- Further handle gestures and inside drags refine the same pending session;
  Confirm commits everything as one `"Free Transform"` state.
- An identity gesture (press and release without a change) records nothing and
  returns to the idle controls box.
- The pending session shows the reference point; the idle controls box still
  does not.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/move-transform-controls`: handle gestures no longer commit per drag;
  a changed gesture becomes a pending transform with Confirm/Reject, and idle
  presses remain Move drags.
- `document/free-transform`: a pending Move controls transform now locks
  document edits like a modal transform; only an in-progress gesture does not.
- `ui/transform-options-bar`: the transform page appears once a Move controls
  gesture becomes pending, not while the idle box or the drag is up.

## Impact

- `crates/pictura-app/cpp/tool_transform.cpp`: release promotes a changed
  controls gesture to a pending session; identity cancels.
- `crates/pictura-app/cpp/tools.cpp` / `tools.h`: the release path and the
  pending-state helpers.
- `crates/pictura-app/src/cxxqt_object/impl_transform/session/readouts.rs`:
  a `transform_is_identity` readout so release can distinguish a real change
  from a click.
- Tests: `tst_transform_controls`, `tst_transform_lock`, `tst_transform_overlay`,
  `tst_linked_layers`.
- No new dependencies, document fields, or file formats.
