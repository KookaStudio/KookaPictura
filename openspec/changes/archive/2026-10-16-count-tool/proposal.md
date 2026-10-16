# Proposal: count-tool

## Why

The Count (Extended) tool (issue #9) was catalogued but disabled and tracked as
out-of-scope. It counts objects in an image with numbered marks organised into
named count groups, each with its own visibility, colour, marker size, and
label size. photorust (perfecto25/photorust) ships it as an annotation
(`MarkerKind::Count`) that reads a document without changing its pixels. This
change ports it onto Kooka's annotation infrastructure with the full CS6
options bar.

## What Changes

- `pictura_core::annotations`: a `CountGroup` model (name, visible, colour,
  marker size 1–10, label size 8–72, its own numbered marks) on
  `Document::annotations`, with the active group and add/remove/rename/select,
  independent of color samplers and notes.
- `cxxqt_object/annotations.rs`: count group and mark bridge functions (state
  reads, group create/delete/rename/visibility/colour/size, mark
  add/move/delete/clear), one history state per committed mark or group edit.
- `tool_count.cpp`: a Count handler (click to add to the active group, drag to
  move, Alt-click or drag-off to delete).
- The Count options bar: the running total, the group dropdown, eye (visibility),
  folder (new group, with a name dialog), trash (delete group), Clear, the group
  colour swatch, and the Marker Size / Label Size fields.
- A full Photoshop-style `ColorPickerDialog` (ported from photorust's
  `shell/src/dialogs/ColorPickerDialog.{h,cpp}`): the colour field + hue ramp,
  new/current compare, HSB/RGB/Lab/CMYK/hex fields, and Only Web Colors. The
  Count colour swatch opens it.
- The canvas overlay draws each visible group's marks as numbered discs in the
  group's colour, marker size, and label size; the disc grows with the label so
  a larger Label Size never clips the number.
- C++ self-test `healing_tools` (code 536) covers the Count groups.

## Capabilities

### New Capabilities

- `tools/count-tool`: the Count (Extended) tool, its count groups, and marks.
- `ui/color-picker`: the full Photoshop-style color picker dialog.

## Impact

- `pictura-core` (`annotations.rs`), `pictura-app` bridge and C++ as above.
- No new dependency.

## Ceilings

`ponytail:` the Measurement Log, automatic counting, and PSD persistence are
not shipped; marks and groups are session document state undone through
history, like the other annotations. A group is never deleted to zero: the last
group is kept.

## Provenance

Ported from photorust's `core/src/annotation.rs`
(<https://github.com/perfecto25/photorust>).
