#pragma once

namespace pictura {
class PicturaMainWindow;

// Runs the Batch 1 layers-interaction checks:
//   lpr_drag_flags (299)      the model advertises drag and drop for a row
//   lpr_label_tint (300)      a color label fills the visibility column,
//                             None does not
//   lpr_ctrl_g (301)          the Layer > Group Layers action wraps the whole
//                             selection in one group in one undo step
//   lpr_rename_name_only (302) disabled edit triggers; a double-click inside the
//                             name opens the editor, one in the eye gutter does
//                             not
// The drag check asserts the model flags, the root cause of the dead pipeline;
// it does not synthesize a real drag because QDrag::exec would block outside a
// live platform drag loop. Returns 0 when all pass, otherwise the failure code.
int runLayersDragChecks(PicturaMainWindow& frame);
} // namespace pictura
