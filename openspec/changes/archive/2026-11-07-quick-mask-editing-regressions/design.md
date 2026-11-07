# Design

`DocumentSession.selection` continues to represent the active pixel editing constraint. Quick Mask owns a separate optional coverage plane: None is implicit full coverage. Entry moves the original selection into that plane and clears the editing constraint; exit moves mask coverage back and discards the constraint. Mode changes advance selection and canvas revisions.

History snapshots and deltas carry optional Quick Mask coverage metadata in addition to the existing editing selection. The outer option distinguishes ordinary history entries from Quick Mask entries, including implicit-full coverage. Captures attach that metadata; restores interpret it according to the current mode. Undo outside Quick Mask exposes the captured coverage, while undo inside restores mask and editing constraint independently. Mode toggles do not add states.

Brush clipping is applied once to stroke coverage against its saved baseline. Fill and filter clipping uses the temporary selection, never the pre-entry mask. Painting keeps unchanged coverage, with banded sharing to avoid whole-plane copies on small strokes.

Live dabs increment the canvas revision and queue only their damage. Deferred flushes emit damage without compositing document pixels or refreshing the shell. Commits and undo invalidate presentation once. Overlay crops blend native premultiplied RGBA bytes from the pyramid without per-pixel Qt calls.

Verification uses real Qt canvas gestures, cached marching ants, QAction keyboard undo, and crop-cache presentation. A live control-server script captures the pre-fix selection, undo and pointer-latency symptoms without compiling before implementation is complete, as required by this repository.

Geometry operations retain their existing selection-reset policy through `clear_geometry_selection`, clearing both runtime planes. Their undo restores the old geometry and the mask separately from the editing constraint.
