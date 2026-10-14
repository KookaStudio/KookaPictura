#include "tools.h"

#include "commands.h"

#include <QtCore/QHash>

namespace pictura {

namespace {

const ToolInfo kToolTable[] = {
    {ToolId::Move, "move", "Move", QLatin1Char('V'), Qt::SizeAllCursor,
     "Move: drag to move the active layer", 1, true, 2, 2},
    {ToolId::Marquee, "marquee", "Rectangular Marquee", QLatin1Char('M'), Qt::CrossCursor,
     "Marquee: drag to select a rectangle", 2, true, 12, 12},
    {ToolId::EllipticalMarquee, "ellipticalmarquee", "Elliptical Marquee", QLatin1Char('M'),
     Qt::CrossCursor, "Elliptical Marquee: drag to select an ellipse", 2, true, 12, 12},
    {ToolId::Lasso, "lasso", "Lasso", QLatin1Char('L'), Qt::CrossCursor,
     "Lasso: drag around a region to select", 3, true, 2, 2},
    {ToolId::PolygonalLasso, "polygonallasso", "Polygonal Lasso", QLatin1Char('L'),
     Qt::CrossCursor, "Polygonal Lasso: click vertices, close on the first vertex or double-click",
     3, true, 2, 2},
    {ToolId::MagneticLasso, "magneticlasso", "Magnetic Lasso", QLatin1Char('L'),
     Qt::CrossCursor,
     "Magnetic Lasso: click to start, trace an edge, close on the first point or double-click",
     3, true, 2, 2},
    {ToolId::MagicWand, "magicwand", "Magic Wand", QLatin1Char('W'), Qt::CrossCursor,
     "Magic Wand: click to select by colour, combining with the current selection", 4, true, 12,
     12},
    {ToolId::QuickSelection, "quickselection", "Quick Selection", QLatin1Char('W'),
     Qt::CrossCursor, "Quick Selection: drag to grow a selection", 4, true, 12, 12},
    {ToolId::Crop, "crop", "Crop", QLatin1Char('C'), Qt::CrossCursor,
     "Crop: drag the handles or a new box, Enter or double-click to apply, Esc to reset", 5,
     true, 12, 12},
    {ToolId::PerspectiveCrop, "perspectivecrop", "Perspective Crop", QLatin1Char('C'),
     Qt::CrossCursor,
     "Perspective Crop: drag a box, pull its corners onto the subject, press Enter to commit",
     5, true, 12, 12},
    {ToolId::Slice, "slice", "Slice", QLatin1Char('C'), Qt::CrossCursor,
     "Slice: drag to cut a user slice", 5, true, 12, 12},
    {ToolId::SliceSelect, "sliceselect", "Slice Select", QLatin1Char('C'), Qt::CrossCursor,
     "Slice Select: click a slice to select it, drag to move or resize, Delete removes it", 5,
     true, 12, 12},
    {ToolId::Eyedropper, "eyedropper", "Eyedropper", QLatin1Char('I'), Qt::CrossCursor,
     "Eyedropper: click to sample a colour", 6, true, 2, 22},
    {ToolId::ColorSampler, "colorsampler", "Color Sampler", QLatin1Char('I'), Qt::CrossCursor,
     "Color Sampler: click to place a sampler (up to four), drag to move, Alt-click to delete", 6,
     true, 2, 22},
    {ToolId::Ruler, "ruler", "Ruler", QLatin1Char('I'), Qt::CrossCursor,
     "Ruler: drag to measure, Shift snaps to 45°, drag an end to adjust", 6, true, 2, 22},
    {ToolId::Note, "note", "Note", QLatin1Char('I'), Qt::CrossCursor,
     "Note: click to add a note, click a note to open it, Alt-click to delete", 6, true, 2,
     22},
    {ToolId::Count, "count", "Count (Extended)", QLatin1Char('I'), Qt::CrossCursor,
     "Count (Extended): click to add a numbered mark, drag to move, Alt-click to delete", 6,
     true, 12, 12},
    {ToolId::SpotHealingBrush, "spothealingbrush", "Spot Healing Brush", QLatin1Char('J'),
     Qt::CrossCursor,
     "Spot Healing Brush: click or drag over a defect to rebuild it from its surroundings", 7,
     true, 2, 22},
    {ToolId::HealingBrush, "healingbrush", "Healing Brush", QLatin1Char('J'), Qt::CrossCursor,
     "Healing Brush: Alt-click a source, then paint to transplant its texture", 7, true, 2,
     22},
    {ToolId::Patch, "patch", "Patch", QLatin1Char('J'), Qt::CrossCursor,
     "Patch: drag to outline a region, then drag the outline to patch", 7, true, 2, 2},
    {ToolId::ContentAwareMove, "contentawaremove", "Content-Aware Move", QLatin1Char('J'),
     Qt::CrossCursor,
     "Content-Aware Move: drag to outline a region, then drag the outline to move it", 7, true,
     2, 2},
    {ToolId::RedEye, "redeye", "Red Eye", QLatin1Char('J'), Qt::CrossCursor,
     "Red Eye: click a red pupil or drag a box over the eye", 7, true, 12, 12},
    {ToolId::Brush, "brush", "Brush", QLatin1Char('B'), Qt::CrossCursor,
     "Brush: drag to paint the foreground colour", 8, true, 2, 22},
    {ToolId::Pencil, "pencil", "Pencil", QLatin1Char('B'), Qt::CrossCursor,
     "Pencil: drag to paint a hard aliased line", 8, true, 2, 22},
    {ToolId::ColorReplacement, "colorreplacement", "Color Replacement", QLatin1Char('B'),
     Qt::CrossCursor,
     "Color Replacement: drag to repaint the sampled colour with the foreground colour", 8,
     true, 2, 22},
    {ToolId::MixerBrush, "mixerbrush", "Mixer Brush", QLatin1Char('B'), Qt::CrossCursor,
     "Mixer Brush: drag to paint wet paint, Alt-click to load the brush from the image", 8,
     true, 2, 22},
    {ToolId::CloneStamp, "clonestamp", "Clone Stamp", QLatin1Char('S'), Qt::CrossCursor,
     "Clone Stamp: Alt-click to set the source, then drag to paint it", 9, true, 2, 22},
    {ToolId::PatternStamp, "patternstamp", "Pattern Stamp", QLatin1Char('S'), Qt::CrossCursor,
     "Pattern Stamp: drag to paint the chosen pattern", 9, true, 2, 22},
    {ToolId::HistoryBrush, "historybrush", "History Brush", QLatin1Char('Y'), Qt::CrossCursor,
     "History Brush: drag to paint back the History panel's source state", 10, true, 2, 22},
    {ToolId::ArtHistoryBrush, "arthistorybrush", "Art History Brush", QLatin1Char('Y'),
     Qt::CrossCursor,
     "Art History Brush: drag to paint stylized strokes from the History panel's source state",
     10, true, 2, 22},
    {ToolId::Eraser, "eraser", "Eraser", QLatin1Char('E'), Qt::CrossCursor,
     "Eraser: drag to erase, Alt-drag to erase to history", 11, true, 2, 22},
    {ToolId::BackgroundEraser, "backgrounderaser", "Background Eraser", QLatin1Char('E'),
     Qt::CrossCursor,
     "Background Eraser: drag with the crosshair on the colour to erase to transparency", 11,
     true, 2, 22},
    {ToolId::MagicEraser, "magiceraser", "Magic Eraser", QLatin1Char('E'), Qt::CrossCursor,
     "Magic Eraser: click to erase similar colours to transparency", 11, true, 2, 22},
    {ToolId::Gradient, "gradient", "Gradient", QLatin1Char('G'), Qt::CrossCursor,
     "Gradient: drag to draw the gradient, Shift to constrain the angle to 45°", 12, true, 2, 22},
    {ToolId::PaintBucket, "paintbucket", "Paint Bucket", QLatin1Char('G'), Qt::CrossCursor,
     "Paint Bucket: click to fill similar colours with the foreground colour or a pattern", 12,
     true, 2, 22},
    {ToolId::Blur, "blur", "Blur", QChar(), Qt::CrossCursor,
     "Blur: drag to soften; going over a spot again softens it more", 13, true, 2, 22},
    {ToolId::Sharpen, "sharpen", "Sharpen", QChar(), Qt::CrossCursor,
     "Sharpen: drag to crisp detail; going over a spot again sharpens it more", 13, true, 2, 22},
    {ToolId::Smudge, "smudge", "Smudge", QChar(), Qt::CrossCursor,
     "Smudge: drag to push colour along the stroke", 13, true, 2, 22},
    {ToolId::Dodge, "dodge", "Dodge", QLatin1Char('O'), Qt::CrossCursor,
     "Dodge: drag to lighten; going over a spot again lightens it more", 14, true, 2, 22},
    {ToolId::Burn, "burn", "Burn", QLatin1Char('O'), Qt::CrossCursor,
     "Burn: drag to darken; going over a spot again darkens it more", 14, true, 2, 22},
    {ToolId::Sponge, "sponge", "Sponge", QLatin1Char('O'), Qt::CrossCursor,
     "Sponge: drag to drain or lift colour", 14, true, 2, 22},
    {ToolId::Pen, "pen", "Pen", QLatin1Char('P'), Qt::CrossCursor,
     "Pen: click for a corner, drag for a curve, click the first point to close, Enter to end",
     15, true, 2, 2},
    {ToolId::FreeformPen, "freeformpen", "Freeform Pen", QLatin1Char('P'), Qt::CrossCursor,
     "Freeform Pen: drag to draw a path; end near the start to close it", 15, true, 2, 2},
    {ToolId::AddAnchorPoint, "addanchorpoint", "Add Anchor Point", QChar(), Qt::CrossCursor,
     "Add Anchor Point: click a path segment to add an anchor", 15, true, 2, 2},
    {ToolId::DeleteAnchorPoint, "deleteanchorpoint", "Delete Anchor Point", QChar(),
     Qt::CrossCursor, "Delete Anchor Point: click an anchor to remove it", 15, true, 2, 2},
    {ToolId::ConvertPoint, "convertpoint", "Convert Point", QChar(), Qt::CrossCursor,
     "Convert Point: drag an anchor to make it smooth, click it for a corner, drag a handle to "
     "break it",
     15, true, 2, 2},
    {ToolId::HorizontalType, "horizontaltype", "Horizontal Type", QLatin1Char('T'),
     Qt::IBeamCursor,
     "Horizontal Type: click and type; Enter for a new line, Ctrl+Enter to commit, Esc to cancel",
     16, true, 12, 12},
    {ToolId::VerticalType, "verticaltype", "Vertical Type", QLatin1Char('T'),
     Qt::IBeamCursor,
     "Vertical Type: click and type a column; Enter for a new column, Ctrl+Enter to commit",
     16, true, 12, 12},
    {ToolId::HorizontalTypeMask, "horizontaltypemask", "Horizontal Type Mask", QLatin1Char('T'),
     Qt::IBeamCursor, "Horizontal Type Mask: click and type; Ctrl+Enter makes it a selection",
     16, true, 12, 12},
    {ToolId::VerticalTypeMask, "verticaltypemask", "Vertical Type Mask", QLatin1Char('T'),
     Qt::IBeamCursor, "Vertical Type Mask: click and type a column; Ctrl+Enter makes it a selection",
     16, true, 12, 12},
    {ToolId::PathSelection, "pathselection", "Path Selection", QLatin1Char('A'),
     Qt::ArrowCursor,
     "Path Selection: click a path component to select it, drag to move, Alt-drag to copy",
     17, true, 4, 2},
    {ToolId::DirectSelection, "directselection", "Direct Selection", QLatin1Char('A'),
     Qt::ArrowCursor,
     "Direct Selection: drag an anchor or direction handle, Alt-click to select a component",
     17, true, 4, 2},
    {ToolId::Rectangle, "rectangle", "Rectangle", QLatin1Char('U'), Qt::CrossCursor,
     "Rectangle: drag to draw a rectangle as a shape layer, path, or pixels", 18, true, 12, 12},
    {ToolId::RoundedRectangle, "roundedrectangle", "Rounded Rectangle", QLatin1Char('U'),
     Qt::CrossCursor,
     "Rounded Rectangle: drag to draw a rectangle with rounded corners", 18, true, 12, 12},
    {ToolId::Ellipse, "ellipse", "Ellipse", QLatin1Char('U'), Qt::CrossCursor,
     "Ellipse: drag to draw an ellipse", 18, true, 12, 12},
    {ToolId::Polygon, "polygon", "Polygon", QLatin1Char('U'), Qt::CrossCursor,
     "Polygon: drag out from the centre to draw a regular polygon", 18, true, 12, 12},
    {ToolId::Line, "line", "Line", QLatin1Char('U'), Qt::CrossCursor,
     "Line: drag to draw a line of the chosen weight, with optional arrowheads", 18, true, 12,
     12},
    {ToolId::CustomShape, "customshape", "Custom Shape", QLatin1Char('U'), Qt::CrossCursor,
     "Custom Shape: drag to draw the shape chosen in the picker", 18, true, 12, 12},
    {ToolId::ObjectRotate, "objectrotate", "Object Rotate (Extended)", QLatin1Char('K'),
     Qt::CrossCursor, "Object Rotate (Extended): not implemented yet", 19, false, 12, 12},
    {ToolId::ObjectRoll, "objectroll", "Object Roll (Extended)", QLatin1Char('K'),
     Qt::CrossCursor, "Object Roll (Extended): not implemented yet", 19, false, 12, 12},
    {ToolId::ObjectPan, "objectpan", "Object Pan (Extended)", QLatin1Char('K'),
     Qt::CrossCursor, "Object Pan (Extended): not implemented yet", 19, false, 12, 12},
    {ToolId::ObjectSlide, "objectslide", "Object Slide (Extended)", QLatin1Char('K'),
     Qt::CrossCursor, "Object Slide (Extended): not implemented yet", 19, false, 12, 12},
    {ToolId::ObjectScale, "objectscale", "Object Scale (Extended)", QLatin1Char('K'),
     Qt::CrossCursor, "Object Scale (Extended): not implemented yet", 19, false, 12, 12},
    {ToolId::CameraRotate, "camerarotate", "Camera Rotate (Extended)", QLatin1Char('N'),
     Qt::CrossCursor, "Camera Rotate (Extended): not implemented yet", 20, false, 12, 12},
    {ToolId::CameraRoll, "cameraroll", "Camera Roll (Extended)", QLatin1Char('N'),
     Qt::CrossCursor, "Camera Roll (Extended): not implemented yet", 20, false, 12, 12},
    {ToolId::CameraPan, "camerapan", "Camera Pan (Extended)", QLatin1Char('N'),
     Qt::CrossCursor, "Camera Pan (Extended): not implemented yet", 20, false, 12, 12},
    {ToolId::CameraWalk, "camerawalk", "Camera Walk (Extended)", QLatin1Char('N'),
     Qt::CrossCursor, "Camera Walk (Extended): not implemented yet", 20, false, 12, 12},
    {ToolId::CameraZoom, "camerazoom", "Camera Zoom (Extended)", QLatin1Char('N'),
     Qt::CrossCursor, "Camera Zoom (Extended): not implemented yet", 20, false, 12, 12},
    {ToolId::Hand, "hand", "Hand", QLatin1Char('H'), Qt::OpenHandCursor,
     "Hand: drag to pan the canvas", 21, true, 9, 2},
    {ToolId::RotateView, "rotateview", "Rotate View", QLatin1Char('R'), Qt::CrossCursor,
     "Rotate View: drag to turn the canvas; Esc resets the view", 22, true, 12, 12},
    {ToolId::Zoom, "zoom", "Zoom", QLatin1Char('Z'), Qt::CrossCursor,
     "Zoom: click to zoom in, Ctrl/Alt-click to zoom out", 23, true, 9, 2},
};
constexpr int kToolCount = int(sizeof(kToolTable) / sizeof(kToolTable[0]));
static_assert(kToolCount == 71, "tool table must cover every ToolId");

int toolIndex(ToolId id) { return static_cast<int>(id); }

QHash<int, QChar>& toolShortcutOverrideTable()
{
    static QHash<int, QChar> overrides;
    return overrides;
}

} // namespace

const ToolInfo& toolInfo(ToolId id)
{
    const int index = toolIndex(id);
    return (index >= 0 && index < kToolCount) ? kToolTable[index] : kToolTable[0];
}

bool toolImplemented(ToolId id) { return toolInfo(id).implemented; }

bool isBrushTool(ToolId id)
{
    return id == ToolId::Brush || id == ToolId::Pencil || id == ToolId::ColorReplacement
        || id == ToolId::MixerBrush || id == ToolId::CloneStamp || id == ToolId::PatternStamp
        || id == ToolId::HistoryBrush || id == ToolId::ArtHistoryBrush || id == ToolId::Eraser
        || id == ToolId::BackgroundEraser || id == ToolId::Blur || id == ToolId::Sharpen
        || id == ToolId::Smudge || id == ToolId::Dodge || id == ToolId::Burn
        || id == ToolId::Sponge;
}

QString toolIdName(ToolId id)
{
    const int index = toolIndex(id);
    return QString::fromLatin1(
        (index >= 0 && index < kToolCount) ? kToolTable[index].name : kToolTable[0].name);
}

QString toolCursorId(ToolId id, Qt::KeyboardModifiers mods)
{
    const QString base = QStringLiteral("tool.") + toolIdName(id);
    if (id != ToolId::Marquee && id != ToolId::EllipticalMarquee) {
        return base;
    }
    if (mods.testFlag(Qt::ShiftModifier)) {
        return base + QStringLiteral(".add");
    }
    if (mods.testFlag(Qt::AltModifier)) {
        return base + QStringLiteral(".remove");
    }
    return base;
}

QChar toolShortcut(ToolId id)
{
    return toolShortcutOverrideTable().value(toolIndex(id), toolInfo(id).shortcut);
}

void setToolShortcut(ToolId id, QChar key)
{
    if (key == toolInfo(id).shortcut) {
        toolShortcutOverrideTable().remove(toolIndex(id));
    } else {
        toolShortcutOverrideTable().insert(toolIndex(id), key);
    }
}

void resetToolShortcuts() { toolShortcutOverrideTable().clear(); }

QMap<ToolId, QChar> toolShortcutOverrides()
{
    QMap<ToolId, QChar> overrides;
    for (auto it = toolShortcutOverrideTable().cbegin(); it != toolShortcutOverrideTable().cend();
         ++it) {
        overrides.insert(ToolId(it.key()), it.value());
    }
    return overrides;
}

QList<QChar> toolShortcutKeys()
{
    QList<QChar> keys;
    for (ToolId id : allToolIds()) {
        const QChar key = toolShortcut(id);
        if (!key.isNull() && !keys.contains(key)) {
            keys << key;
        }
    }
    return keys;
}

int toolGroupForKey(QChar key)
{
    if (key.isNull()) {
        return 0;
    }
    const QChar upper = key.toUpper();
    for (ToolId id : allToolIds()) {
        const QChar shortcut = toolShortcut(id);
        if (!shortcut.isNull() && shortcut.toUpper() == upper) {
            return toolInfo(id).group;
        }
    }
    return 0;
}

QList<ToolHint> toolHintEntries(ToolId id)
{
    switch (id) {
    case ToolId::Marquee:
    case ToolId::EllipticalMarquee:
    case ToolId::Lasso:
    case ToolId::PolygonalLasso:
    case ToolId::MagneticLasso:
    case ToolId::QuickSelection:
    case ToolId::MagicWand:
        return {{QStringLiteral("Shift"), QStringLiteral("Add to selection")},
                {QStringLiteral("Alt"), QStringLiteral("Subtract from selection")}};
    case ToolId::Brush:
    case ToolId::Pencil:
    case ToolId::ColorReplacement:
    case ToolId::MixerBrush:
    case ToolId::CloneStamp:
    case ToolId::PatternStamp:
    case ToolId::HistoryBrush:
    case ToolId::ArtHistoryBrush:
    case ToolId::Eraser:
    case ToolId::BackgroundEraser:
    case ToolId::Blur:
    case ToolId::Sharpen:
    case ToolId::Smudge:
    case ToolId::Dodge:
    case ToolId::Burn:
    case ToolId::Sponge:
        return {{QStringLiteral("["), QStringLiteral("Decrease brush size")},
                {QStringLiteral("]"), QStringLiteral("Increase brush size")}};
    case ToolId::Gradient:
        return {{QStringLiteral("Shift"), QStringLiteral("Constrain to 45°")}};
    case ToolId::Pen:
        return {{QStringLiteral("Shift"), QStringLiteral("Constrain to 45°")},
                {QStringLiteral("Alt"), QStringLiteral("Break the handle")},
                {QStringLiteral("Enter"), QStringLiteral("End the path")}};
    case ToolId::PathSelection:
        return {{QStringLiteral("Alt"), QStringLiteral("Drag a copy")},
                {QStringLiteral("Delete"), QStringLiteral("Delete the component")}};
    case ToolId::DirectSelection:
        return {{QStringLiteral("Alt"), QStringLiteral("Break the handle")},
                {QStringLiteral("Alt-click"), QStringLiteral("Select the component")}};
    case ToolId::Rectangle:
    case ToolId::RoundedRectangle:
    case ToolId::Ellipse:
        return {{QStringLiteral("Shift"), QStringLiteral("Constrain proportions")},
                {QStringLiteral("Alt"), QStringLiteral("Draw from the centre")}};
    case ToolId::Polygon:
        return {{QStringLiteral("Shift"), QStringLiteral("Snap the angle to 15°")}};
    case ToolId::Line:
        return {{QStringLiteral("Shift"), QStringLiteral("Snap the angle to 45°")}};
    case ToolId::RotateView:
        return {{QStringLiteral("Shift"), QStringLiteral("Snap to 15°")},
                {QStringLiteral("Esc"), QStringLiteral("Reset View")}};
    case ToolId::CustomShape:
        return {{QStringLiteral("Shift"), QStringLiteral("Defined proportions")},
                {QStringLiteral("Alt"), QStringLiteral("Draw from the centre")}};
    case ToolId::HorizontalType:
    case ToolId::VerticalType:
    case ToolId::HorizontalTypeMask:
    case ToolId::VerticalTypeMask:
        return {{QStringLiteral("Ctrl+Enter"), QStringLiteral("Commit")},
                {QStringLiteral("Esc"), QStringLiteral("Cancel")}};
    case ToolId::Move:
        return {{QStringLiteral("Arrows"), QStringLiteral("Nudge 1 px")},
                {QStringLiteral("Shift+Arrows"), QStringLiteral("Nudge 10 px")},
                {QString(), QStringLiteral("Free transform"), command_ids::EditFreeTransform}};
    default:
        return {};
    }
}

const QList<ToolId>& allToolIds()
{
    static const QList<ToolId> ids = {
        ToolId::Move,             ToolId::Marquee,          ToolId::EllipticalMarquee,
        ToolId::Lasso,            ToolId::PolygonalLasso,   ToolId::MagneticLasso,
        ToolId::MagicWand,        ToolId::QuickSelection,   ToolId::Crop,
        ToolId::PerspectiveCrop,  ToolId::Slice,            ToolId::SliceSelect,
        ToolId::Eyedropper,       ToolId::ColorSampler,     ToolId::Ruler,
        ToolId::Note,             ToolId::Count,            ToolId::SpotHealingBrush,
        ToolId::HealingBrush,     ToolId::Patch,            ToolId::ContentAwareMove,
        ToolId::RedEye,           ToolId::Brush,            ToolId::Pencil,
        ToolId::ColorReplacement, ToolId::MixerBrush,       ToolId::CloneStamp,
        ToolId::PatternStamp,     ToolId::HistoryBrush,     ToolId::ArtHistoryBrush,
        ToolId::Eraser,           ToolId::BackgroundEraser, ToolId::MagicEraser,
        ToolId::Gradient,         ToolId::PaintBucket,      ToolId::Blur,
        ToolId::Sharpen,          ToolId::Smudge,           ToolId::Dodge,
        ToolId::Burn,             ToolId::Sponge,           ToolId::Pen,
        ToolId::FreeformPen,      ToolId::AddAnchorPoint,   ToolId::DeleteAnchorPoint,
        ToolId::ConvertPoint,     ToolId::HorizontalType,   ToolId::VerticalType,
        ToolId::HorizontalTypeMask, ToolId::VerticalTypeMask, ToolId::PathSelection,
        ToolId::DirectSelection,  ToolId::Rectangle,        ToolId::RoundedRectangle,
        ToolId::Ellipse,          ToolId::Polygon,          ToolId::Line,
        ToolId::CustomShape,      ToolId::ObjectRotate,     ToolId::ObjectRoll,
        ToolId::ObjectPan,        ToolId::ObjectSlide,      ToolId::ObjectScale,
        ToolId::CameraRotate,     ToolId::CameraRoll,       ToolId::CameraPan,
        ToolId::CameraWalk,       ToolId::CameraZoom,       ToolId::Hand,
        ToolId::RotateView,       ToolId::Zoom,
    };
    return ids;
}

const QList<ToolId>& implementedToolIds()
{
    static const QList<ToolId> ids = {
        ToolId::Move,   ToolId::Marquee, ToolId::EllipticalMarquee, ToolId::Lasso,
        ToolId::PolygonalLasso, ToolId::MagneticLasso, ToolId::MagicWand, ToolId::QuickSelection, ToolId::Crop,
        ToolId::PerspectiveCrop, ToolId::Slice, ToolId::SliceSelect,
        ToolId::Eyedropper, ToolId::ColorSampler, ToolId::Ruler, ToolId::Note,
        ToolId::Count, ToolId::SpotHealingBrush, ToolId::HealingBrush, ToolId::Patch,
        ToolId::ContentAwareMove, ToolId::RedEye,
        ToolId::Hand, ToolId::RotateView, ToolId::Zoom, ToolId::Brush, ToolId::Pencil, ToolId::ColorReplacement,
        ToolId::MixerBrush, ToolId::CloneStamp, ToolId::PatternStamp, ToolId::HistoryBrush,
        ToolId::ArtHistoryBrush, ToolId::Eraser, ToolId::BackgroundEraser, ToolId::MagicEraser,
        ToolId::Gradient, ToolId::PaintBucket, ToolId::Blur, ToolId::Sharpen, ToolId::Smudge,
        ToolId::Dodge, ToolId::Burn, ToolId::Sponge, ToolId::Pen, ToolId::FreeformPen,
        ToolId::AddAnchorPoint, ToolId::DeleteAnchorPoint, ToolId::ConvertPoint,
        ToolId::HorizontalType, ToolId::VerticalType, ToolId::HorizontalTypeMask,
        ToolId::VerticalTypeMask, ToolId::PathSelection, ToolId::DirectSelection,
        ToolId::Rectangle, ToolId::RoundedRectangle, ToolId::Ellipse, ToolId::Polygon,
        ToolId::Line, ToolId::CustomShape,
    };
    return ids;
}

QString selectionModeString(SelectionMode mode)
{
    switch (mode) {
    case SelectionMode::New:
        return QStringLiteral("new");
    case SelectionMode::Add:
        return QStringLiteral("add");
    case SelectionMode::Subtract:
        return QStringLiteral("subtract");
    case SelectionMode::Intersect:
        return QStringLiteral("intersect");
    }
    return QStringLiteral("new");
}

} // namespace pictura
