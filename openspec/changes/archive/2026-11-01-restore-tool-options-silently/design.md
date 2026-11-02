# Design

## One restore, one patch

`applyToolOptionsJson` is called from two places with different contracts:
`frame_build.cpp` restores the session store before the options bars exist, and
`ControlServer::methodToolOptions` patches live options. They were one function,
so the merge semantics of the patch leaked into the restore contract.

The restore keeps the name and gains the persistence contract: a missing group
or field is the group's default. The patch is a new function with the control
server's contract: a group or field it leaves out keeps its current value. The
patch body is the previous `applyToolOptionsJson` body, so the runtime behaviour
of `tool_options` does not change.

## Silent apply

`ToolOption<T>` already has `update`, which stores without notifying. It gains
`defaults()` (a clamped `T{}`) and `reset()`. `PaintOptions` and
`SelectionOptions` are hand-written classes, so they gain an `apply(const Self&)`
that copies the data members with the same clamping as their setters and leaves
the `std::function` callbacks alone. Copying the callbacks would disconnect the
live options bar; the restore only replaces values.

`CropOptions` keeps its existing silent `apply` for the four session keys and
gains `applySettings` for the `crop` group. `setSettings` keeps notifying for
the patch.

## Defaults for missing fields

The restore reads a present group from `defaults()` instead of `group->get()`.
The `Reader` visitor already leaves an undefined or null field untouched, so a
missing field keeps the base value, which is now the default. A group that is
absent, or not an object, is reset to `defaults()`.

## Non-persisted mirrors

`ShapeOptions::activeWidth`/`activeHeight`, `TypeOptions::color`, and
`MixerOptions::reservoir` mirror the document and are not persisted. The restore
runs at startup before a document is installed, so their default values are
correct there; the patch leaves them untouched.
