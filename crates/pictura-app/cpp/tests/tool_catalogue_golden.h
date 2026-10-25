#pragma once

// The tool table as `kToolTable` and `toolHintEntries` held it before the
// catalogue moved into Rust (#274), captured as a golden. Each hint is
// `key|text|commandId`, hints joined by `;`. Test-only: production reads the
// engine catalogue.

#include <QtCore/Qt>

#include "tools.h"

namespace pictura::test {

struct ToolGolden {
    pictura::ToolId id;
    const char* name;
    const char* label;
    char shortcut;
    Qt::CursorShape cursor;
    const char* hint;
    int group;
    bool implemented;
    int hotspotX;
    int hotspotY;
    const char* hints;
};

// clang-format off
inline constexpr ToolGolden kToolGolden[] = {
    {pictura::ToolId::Move, "move", "Move", 'V', Qt::SizeAllCursor, "Move: drag to move the active layer", 1, true, 2, 2, "Arrows|Nudge 1 px|;Shift+Arrows|Nudge 10 px|;|Free transform|edit.freeTransform"},
    {pictura::ToolId::Marquee, "marquee", "Rectangular Marquee", 'M', Qt::CrossCursor, "Marquee: drag to select a rectangle", 2, true, 12, 12, "Shift|Add to selection|;Alt|Subtract from selection|"},
    {pictura::ToolId::EllipticalMarquee, "ellipticalmarquee", "Elliptical Marquee", 'M', Qt::CrossCursor, "Elliptical Marquee: drag to select an ellipse", 2, true, 12, 12, "Shift|Add to selection|;Alt|Subtract from selection|"},
    {pictura::ToolId::Lasso, "lasso", "Lasso", 'L', Qt::CrossCursor, "Lasso: drag around a region to select", 3, true, 2, 2, "Shift|Add to selection|;Alt|Subtract from selection|"},
    {pictura::ToolId::PolygonalLasso, "polygonallasso", "Polygonal Lasso", 'L', Qt::CrossCursor, "Polygonal Lasso: click vertices, close on the first vertex or double-click", 3, true, 2, 2, "Shift|Add to selection|;Alt|Subtract from selection|"},
    {pictura::ToolId::MagneticLasso, "magneticlasso", "Magnetic Lasso", 'L', Qt::CrossCursor, "Magnetic Lasso: click to start, trace an edge, close on the first point or double-click", 3, true, 2, 2, "Shift|Add to selection|;Alt|Subtract from selection|"},
    {pictura::ToolId::MagicWand, "magicwand", "Magic Wand", 'W', Qt::CrossCursor, "Magic Wand: click to select by colour, combining with the current selection", 4, true, 12, 12, "Shift|Add to selection|;Alt|Subtract from selection|"},
    {pictura::ToolId::QuickSelection, "quickselection", "Quick Selection", 'W', Qt::CrossCursor, "Quick Selection: drag to grow a selection", 4, true, 12, 12, "Shift|Add to selection|;Alt|Subtract from selection|"},
    {pictura::ToolId::Crop, "crop", "Crop", 'C', Qt::CrossCursor, "Crop: drag the handles or a new box, Enter or double-click to apply, Esc to reset", 5, true, 12, 12, ""},
    {pictura::ToolId::PerspectiveCrop, "perspectivecrop", "Perspective Crop", 'C', Qt::CrossCursor, "Perspective Crop: drag a box, pull its corners onto the subject, press Enter to commit", 5, true, 12, 12, ""},
    {pictura::ToolId::Slice, "slice", "Slice", 'C', Qt::CrossCursor, "Slice: drag to cut a user slice", 5, true, 12, 12, ""},
    {pictura::ToolId::SliceSelect, "sliceselect", "Slice Select", 'C', Qt::CrossCursor, "Slice Select: click a slice to select it, drag to move or resize, Delete removes it", 5, true, 12, 12, ""},
    {pictura::ToolId::Eyedropper, "eyedropper", "Eyedropper", 'I', Qt::CrossCursor, "Eyedropper: click to sample a colour", 6, true, 2, 22, ""},
    {pictura::ToolId::ColorSampler, "colorsampler", "Color Sampler", 'I', Qt::CrossCursor, "Color Sampler: click to place a sampler (up to four), drag to move, Alt-click to delete", 6, true, 2, 22, ""},
    {pictura::ToolId::Ruler, "ruler", "Ruler", 'I', Qt::CrossCursor, "Ruler: drag to measure, Shift snaps to 45°, drag an end to adjust", 6, true, 2, 22, ""},
    {pictura::ToolId::Note, "note", "Note", 'I', Qt::CrossCursor, "Note: click to add a note, click a note to open it, Alt-click to delete", 6, true, 2, 22, ""},
    {pictura::ToolId::Count, "count", "Count (Extended)", 'I', Qt::CrossCursor, "Count (Extended): click to add a numbered mark, drag to move, Alt-click to delete", 6, true, 12, 12, ""},
    {pictura::ToolId::SpotHealingBrush, "spothealingbrush", "Spot Healing Brush", 'J', Qt::CrossCursor, "Spot Healing Brush: click or drag over a defect to rebuild it from its surroundings", 7, true, 2, 22, ""},
    {pictura::ToolId::HealingBrush, "healingbrush", "Healing Brush", 'J', Qt::CrossCursor, "Healing Brush: Alt-click a source, then paint to transplant its texture", 7, true, 2, 22, ""},
    {pictura::ToolId::Patch, "patch", "Patch", 'J', Qt::CrossCursor, "Patch: drag to outline a region, then drag the outline to patch", 7, true, 2, 2, ""},
    {pictura::ToolId::ContentAwareMove, "contentawaremove", "Content-Aware Move", 'J', Qt::CrossCursor, "Content-Aware Move: drag to outline a region, then drag the outline to move it", 7, true, 2, 2, ""},
    {pictura::ToolId::RedEye, "redeye", "Red Eye", 'J', Qt::CrossCursor, "Red Eye: click a red pupil or drag a box over the eye", 7, true, 12, 12, ""},
    {pictura::ToolId::Brush, "brush", "Brush", 'B', Qt::CrossCursor, "Brush: drag to paint the foreground colour", 8, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::Pencil, "pencil", "Pencil", 'B', Qt::CrossCursor, "Pencil: drag to paint a hard aliased line", 8, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::ColorReplacement, "colorreplacement", "Color Replacement", 'B', Qt::CrossCursor, "Color Replacement: drag to repaint the sampled colour with the foreground colour", 8, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::MixerBrush, "mixerbrush", "Mixer Brush", 'B', Qt::CrossCursor, "Mixer Brush: drag to paint wet paint, Alt-click to load the brush from the image", 8, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::CloneStamp, "clonestamp", "Clone Stamp", 'S', Qt::CrossCursor, "Clone Stamp: Alt-click to set the source, then drag to paint it", 9, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::PatternStamp, "patternstamp", "Pattern Stamp", 'S', Qt::CrossCursor, "Pattern Stamp: drag to paint the chosen pattern", 9, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::HistoryBrush, "historybrush", "History Brush", 'Y', Qt::CrossCursor, "History Brush: drag to paint back the History panel's source state", 10, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::ArtHistoryBrush, "arthistorybrush", "Art History Brush", 'Y', Qt::CrossCursor, "Art History Brush: drag to paint stylized strokes from the History panel's source state", 10, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::Eraser, "eraser", "Eraser", 'E', Qt::CrossCursor, "Eraser: drag to erase, Alt-drag to erase to history", 11, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::BackgroundEraser, "backgrounderaser", "Background Eraser", 'E', Qt::CrossCursor, "Background Eraser: drag with the crosshair on the colour to erase to transparency", 11, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::MagicEraser, "magiceraser", "Magic Eraser", 'E', Qt::CrossCursor, "Magic Eraser: click to erase similar colours to transparency", 11, true, 2, 22, ""},
    {pictura::ToolId::Gradient, "gradient", "Gradient", 'G', Qt::CrossCursor, "Gradient: drag to draw the gradient, Shift to constrain the angle to 45°", 12, true, 2, 22, "Shift|Constrain to 45°|"},
    {pictura::ToolId::PaintBucket, "paintbucket", "Paint Bucket", 'G', Qt::CrossCursor, "Paint Bucket: click to fill similar colours with the foreground colour or a pattern", 12, true, 2, 22, ""},
    {pictura::ToolId::Blur, "blur", "Blur", '\0', Qt::CrossCursor, "Blur: drag to soften; going over a spot again softens it more", 13, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::Sharpen, "sharpen", "Sharpen", '\0', Qt::CrossCursor, "Sharpen: drag to crisp detail; going over a spot again sharpens it more", 13, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::Smudge, "smudge", "Smudge", '\0', Qt::CrossCursor, "Smudge: drag to push colour along the stroke", 13, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::Dodge, "dodge", "Dodge", 'O', Qt::CrossCursor, "Dodge: drag to lighten; going over a spot again lightens it more", 14, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::Burn, "burn", "Burn", 'O', Qt::CrossCursor, "Burn: drag to darken; going over a spot again darkens it more", 14, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::Sponge, "sponge", "Sponge", 'O', Qt::CrossCursor, "Sponge: drag to drain or lift colour", 14, true, 2, 22, "[|Decrease brush size|;]|Increase brush size|"},
    {pictura::ToolId::Pen, "pen", "Pen", 'P', Qt::CrossCursor, "Pen: click for a corner, drag for a curve, click the first point to close, Enter to end", 15, true, 2, 2, "Shift|Constrain to 45°|;Alt|Break the handle|;Enter|End the path|"},
    {pictura::ToolId::FreeformPen, "freeformpen", "Freeform Pen", 'P', Qt::CrossCursor, "Freeform Pen: drag to draw a path; end near the start to close it", 15, true, 2, 2, ""},
    {pictura::ToolId::AddAnchorPoint, "addanchorpoint", "Add Anchor Point", '\0', Qt::CrossCursor, "Add Anchor Point: click a path segment to add an anchor", 15, true, 2, 2, ""},
    {pictura::ToolId::DeleteAnchorPoint, "deleteanchorpoint", "Delete Anchor Point", '\0', Qt::CrossCursor, "Delete Anchor Point: click an anchor to remove it", 15, true, 2, 2, ""},
    {pictura::ToolId::ConvertPoint, "convertpoint", "Convert Point", '\0', Qt::CrossCursor, "Convert Point: drag an anchor to make it smooth, click it for a corner, drag a handle to break it", 15, true, 2, 2, ""},
    {pictura::ToolId::HorizontalType, "horizontaltype", "Horizontal Type", 'T', Qt::IBeamCursor, "Horizontal Type: click and type; Enter for a new line, Ctrl+Enter to commit, Esc to cancel", 16, true, 12, 12, "Ctrl+Enter|Commit|;Esc|Cancel|"},
    {pictura::ToolId::VerticalType, "verticaltype", "Vertical Type", 'T', Qt::IBeamCursor, "Vertical Type: click and type a column; Enter for a new column, Ctrl+Enter to commit", 16, true, 12, 12, "Ctrl+Enter|Commit|;Esc|Cancel|"},
    {pictura::ToolId::HorizontalTypeMask, "horizontaltypemask", "Horizontal Type Mask", 'T', Qt::IBeamCursor, "Horizontal Type Mask: click and type; Ctrl+Enter makes it a selection", 16, true, 12, 12, "Ctrl+Enter|Commit|;Esc|Cancel|"},
    {pictura::ToolId::VerticalTypeMask, "verticaltypemask", "Vertical Type Mask", 'T', Qt::IBeamCursor, "Vertical Type Mask: click and type a column; Ctrl+Enter makes it a selection", 16, true, 12, 12, "Ctrl+Enter|Commit|;Esc|Cancel|"},
    {pictura::ToolId::PathSelection, "pathselection", "Path Selection", 'A', Qt::ArrowCursor, "Path Selection: click a path component to select it, drag to move, Alt-drag to copy", 17, true, 4, 2, "Alt|Drag a copy|;Delete|Delete the component|"},
    {pictura::ToolId::DirectSelection, "directselection", "Direct Selection", 'A', Qt::ArrowCursor, "Direct Selection: drag an anchor or direction handle, Alt-click to select a component", 17, true, 4, 2, "Alt|Break the handle|;Alt-click|Select the component|"},
    {pictura::ToolId::Rectangle, "rectangle", "Rectangle", 'U', Qt::CrossCursor, "Rectangle: drag to draw a rectangle as a shape layer, path, or pixels", 18, true, 12, 12, "Shift|Constrain proportions|;Alt|Draw from the centre|"},
    {pictura::ToolId::RoundedRectangle, "roundedrectangle", "Rounded Rectangle", 'U', Qt::CrossCursor, "Rounded Rectangle: drag to draw a rectangle with rounded corners", 18, true, 12, 12, "Shift|Constrain proportions|;Alt|Draw from the centre|"},
    {pictura::ToolId::Ellipse, "ellipse", "Ellipse", 'U', Qt::CrossCursor, "Ellipse: drag to draw an ellipse", 18, true, 12, 12, "Shift|Constrain proportions|;Alt|Draw from the centre|"},
    {pictura::ToolId::Polygon, "polygon", "Polygon", 'U', Qt::CrossCursor, "Polygon: drag out from the centre to draw a regular polygon", 18, true, 12, 12, "Shift|Snap the angle to 15°|"},
    {pictura::ToolId::Line, "line", "Line", 'U', Qt::CrossCursor, "Line: drag to draw a line of the chosen weight, with optional arrowheads", 18, true, 12, 12, "Shift|Snap the angle to 45°|"},
    {pictura::ToolId::CustomShape, "customshape", "Custom Shape", 'U', Qt::CrossCursor, "Custom Shape: drag to draw the shape chosen in the picker", 18, true, 12, 12, "Shift|Defined proportions|;Alt|Draw from the centre|"},
    {pictura::ToolId::ObjectRotate, "objectrotate", "Object Rotate (Extended)", 'K', Qt::CrossCursor, "Object Rotate (Extended): not implemented yet", 19, false, 12, 12, ""},
    {pictura::ToolId::ObjectRoll, "objectroll", "Object Roll (Extended)", 'K', Qt::CrossCursor, "Object Roll (Extended): not implemented yet", 19, false, 12, 12, ""},
    {pictura::ToolId::ObjectPan, "objectpan", "Object Pan (Extended)", 'K', Qt::CrossCursor, "Object Pan (Extended): not implemented yet", 19, false, 12, 12, ""},
    {pictura::ToolId::ObjectSlide, "objectslide", "Object Slide (Extended)", 'K', Qt::CrossCursor, "Object Slide (Extended): not implemented yet", 19, false, 12, 12, ""},
    {pictura::ToolId::ObjectScale, "objectscale", "Object Scale (Extended)", 'K', Qt::CrossCursor, "Object Scale (Extended): not implemented yet", 19, false, 12, 12, ""},
    {pictura::ToolId::CameraRotate, "camerarotate", "Camera Rotate (Extended)", 'N', Qt::CrossCursor, "Camera Rotate (Extended): not implemented yet", 20, false, 12, 12, ""},
    {pictura::ToolId::CameraRoll, "cameraroll", "Camera Roll (Extended)", 'N', Qt::CrossCursor, "Camera Roll (Extended): not implemented yet", 20, false, 12, 12, ""},
    {pictura::ToolId::CameraPan, "camerapan", "Camera Pan (Extended)", 'N', Qt::CrossCursor, "Camera Pan (Extended): not implemented yet", 20, false, 12, 12, ""},
    {pictura::ToolId::CameraWalk, "camerawalk", "Camera Walk (Extended)", 'N', Qt::CrossCursor, "Camera Walk (Extended): not implemented yet", 20, false, 12, 12, ""},
    {pictura::ToolId::CameraZoom, "camerazoom", "Camera Zoom (Extended)", 'N', Qt::CrossCursor, "Camera Zoom (Extended): not implemented yet", 20, false, 12, 12, ""},
    {pictura::ToolId::Hand, "hand", "Hand", 'H', Qt::OpenHandCursor, "Hand: drag to pan the canvas", 21, true, 9, 2, ""},
    {pictura::ToolId::RotateView, "rotateview", "Rotate View", 'R', Qt::CrossCursor, "Rotate View: drag to turn the canvas; Esc resets the view", 22, true, 12, 12, "Shift|Snap to 15°|;Esc|Reset View|"},
    {pictura::ToolId::Zoom, "zoom", "Zoom", 'Z', Qt::CrossCursor, "Zoom: click to zoom in, Ctrl/Alt-click to zoom out", 23, true, 9, 2, ""},
};
// clang-format on

} // namespace pictura::test
