#pragma once

namespace pictura {
class PicturaMainWindow;

// Runs the selection-tool drag checks: tsc_ellipse (242) commits an ellipse
// through the Elliptical Marquee drag path and proves the corners are excluded;
// tsc_feather (243) applies a tool-time Feather and observes a partial-coverage
// edge; tsc_polygon (245) commits a clicked triangle through the Polygonal
// Lasso press path;
// tsc_polygon_cancel (246) proves Escape discards an in-progress path without a
// selection or history change; tsc_polygon_feather (247) softens a polygon edge;
// tsc_wand_contiguous (248) selects only the connected uniform region;
// tsc_wand_global (249) includes a disconnected same-colour patch; tsc_wand_add
// (250) unions the wand result with an existing selection. The Select menu
// checks continue: tsc_select_inverse (251) complements coverage;
// tsc_select_reselect (252) restores a deselected selection; tsc_select_modify
// (253) applies each Modify op; tsc_select_grow (254) follows connectivity;
// tsc_select_similar (255) reaches disconnected patches; tsc_save_load (256)
// round-trips a channel exactly; tsc_select_refusal (257) proves refusals record
// nothing; tsc_layer_select_commands (258) selects/clears panel rows.
// tsc_selection_ants (259) mirrors a committed marquee contour onto the canvas;
// tsc_selection_ants_clear (260) proves Deselect clears it; and
// tsc_selection_edges_toggle (261) hides/shows the edges without losing data;
// tsc_marquee_preview (262) proves the live rubber band is shown during the drag
// and replaced by the committed contour on release; tsc_ellipse_preview (263)
// proves the elliptical rubber band is a multi-point ellipse, not a rectangle;
// tsc_cursor_modifiers (264) maps Shift/Alt to the marquee add/remove cursors;
// tsc_polygon_preview_open (265) keeps the Polygonal Lasso rubber band an open
// polyline until it commits; tsc_lasso_hotspot (266) pins the lasso cursors'
// (2,2) arrow-tip hotspot and a non-null rendered pixmap;
// tsc_move_selection (267) drags from inside a selection and shifts the
// committed outline by the drag delta with one "Move Selection" history state;
// tsc_move_selection_noop (268) records nothing for a zero-delta press;
// tsc_translate_clip (269) clips a translated selection at the document edge;
// tsc_quick_modes (270) maps Shift/Alt/both to Add/Subtract/Intersect and keeps
// New when nothing is selected; tsc_marquee_geometry (271) squares the drag
// under Shift and does not mirror under Alt; tsc_view_preview_hooks (272) pins mouse
// tracking, the solid/open preview flags, and the drag size hint round-trip;
// tsc_polygon_cursor_band (273) shows the clicked vertices plus the live cursor
// in an open solid rubber band; tsc_content_move (274) cuts and translates the
// selected pixels in one state; tsc_content_duplicate (275) Alt-copies them to
// a new layer; tsc_quick_mode_drag (276) drives an Add combine through the real
// marquee drag path.
// Returns 0 when all pass, otherwise the self-test failure code.
int runToolsSelectionChecks(PicturaMainWindow& frame);
} // namespace pictura
