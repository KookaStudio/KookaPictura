# Design

## Live hook

`ColorPickerDialog::getColor` gains a trailing `std::function<void(const QColor&)>
onLive`. The Adobe dialog forwards `ColorPicker::colorChanged` to it. The System
picker branch builds a `QColorDialog` itself so `currentColorChanged` can feed the
same callback. Canvas sampling and `setColor` do not emit `colorChanged`, so the
sampler path stays silent; a sampled color is previewed only once the user edits
again. ponytail: sampled colors skip the live preview until OK.

## History

Preview uses `TextSession::previewTextColor` (while text is being typed it sets
the shared `TypeOptions`; on a selected layer it calls `type_preview_layer`, which
recomposites and records nothing). Cancel calls it with the color the picker
opened with. OK takes the existing path (`setForeground` from the bar, `apply()`
from the Character panel), which records one `Edit Type Layer` state.

## Character panel

With no `TextSession` (another tool active) the panel previews through
`type_api::previewLayer` directly; with a new-text session it already previews
through `setNewTextCharacter`, so it reuses that path.
