# Design: layer-style-authoring

## D1 — The descriptor is the model

The renderer decodes effects from `lfx2` on every composite and the codec writes
`extra_blocks` verbatim, so authoring rewrites the descriptor and adds no typed
style model. A field table (`layer_style_fields.rs`) names each control's dialog
key, descriptor key, value kind (flag, unit float with range, colour, blend
mode, enum choice, gradient stop) and default. Reads of an absent effect or key
return the default, so the dialog shows CS6's values before anything exists.
A new effect object is written complete (all table keys, `showInDialog`, a
linear contour where the effect has one) so other readers see a full object.

## D2 — Live edits, one state

The dialog writes each control through `layer_style_set`, which recomposites
without recording. OK calls `layer_style_commit` (one "Layer Style" state, only
when something changed); Cancel calls `layer_style_cancel`, which re-applies
the current history state. No per-dialog snapshot is kept in the bridge.

## D3 — On means enabled and present

The list checkbox sets `enab` and `present` together, as CS6's checkbox adds or
removes the effect. Unchecking keeps the object (its settings come back when
rechecked); Clear Layer Style removes `lfx2` and `lrFX`.

## D4 — Blend modes are indices into `LAYER_MODES`

The dialog's combo order is `BlendMode::LAYER_MODES` (shared with the Layers
panel through `blend_modes.h`). The encoder writes Photoshop's `BlnM` values:
char codes for the classic modes, string ids for the CS3+ modes; the decoder
accepts both those and the legacy short codes.

## D5 — Scope ceilings

Not authored here: contours, texture, gradient editor (two stops only), stroke
gradient/pattern fill, Blend If, channel restrictions, Global Light, Create Layers, style
presets, and styles on groups (the compositor draws none). A layer with only a
legacy `lrFX` starts a fresh `lfx2` on its first edit.

## D6 — Built-in patterns live in the document

A Pattern Overlay names a pattern by id; the renderer and Photoshop look it up
in the document's `Patt` block. Picking a built-in pattern therefore writes it
into that block (id `kooka-builtin-pattern-<n>`), so the effect renders now and
after a save, in Kooka or elsewhere. The built-in set is `pictura-paint`'s, the
one the Pattern Stamp uses.
