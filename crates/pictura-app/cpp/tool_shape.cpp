// The shape tools (`pictura_core::shape`, bridged by `cxxqt_object/shapes.rs`):
// Rectangle, Rounded Rectangle, Ellipse, Polygon, Line, and Custom Shape. A
// drag previews the outline on the canvas and lands on release in the options
// bar's Mode: a new
// shape layer (Shape), a Work Path component (Path), or foreground pixels on
// the active layer (Pixels). Shift squares the box off (the Polygon snaps its
// turn to 15° and the Line its angle to 45°; a Custom Shape keeps its designed
// proportions); Alt grows it from the press point. The modifiers are read live,
// so pressing one mid-drag changes the preview. A click instead opens the
// tool's Create dialog and places the shape at the click (or centred on it);
// the Line has no dialog, so a click draws nothing.
// Outside Path mode the active shape layer's outline is drawn with its anchors,
// ready for Direct Selection, and its fill, stroke, and size are mirrored into
// the options bar, whose edits restyle or resize it. The geometry gear can
// constrain the proportions, fix the size, or draw from the centre. Ported from photorust's CanvasView shape drag
// (shapeOutlineFor / paintShapeOverlay / drawShape).
//
// ponytail: no gradient / pattern Fill or Stroke, dashed strokes, Proportional
// geometry, or path operations / alignment / arrangement.

#include "tool_handler.h"

#include "image_view.h"
#include "path_overlay.h"
#include "shape_dialogs.h"
#include "tools.h"

#include "api/layers_bridge.h"

#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QLineF>
#include <QtCore/QObject>
#include <QtCore/QVector>
#include <QtGui/QPainterPath>

#include <cmath>
#include <cstdint>
#include <limits>
#include <memory>

namespace pictura {

namespace {

int shapeKind(ToolId id)
{
    switch (id) {
    case ToolId::RoundedRectangle:
        return 1;
    case ToolId::Ellipse:
        return 2;
    case ToolId::Polygon:
        return 3;
    case ToolId::Line:
        return 4;
    case ToolId::CustomShape:
        return 5;
    default:
        return 0;
    }
}

::rust::Vec<std::uint8_t> bytesOf(const QString& text)
{
    ::rust::Vec<std::uint8_t> out;
    const QByteArray bytes = text.toUtf8();
    out.reserve(std::size_t(bytes.size()));
    for (const char value : bytes) {
        out.push_back(std::uint8_t(value));
    }
    return out;
}

QPainterPath pathFromCoords(const ::rust::Vec<double>& coords)
{
    QPainterPath path;
    const int length = int(coords.size());
    for (int a = 0; a < length;) {
        const bool marker = std::isnan(coords[a]);
        const bool closed = !marker || coords[a + 1] != 0.0;
        if (marker) {
            a += 6;
        }
        int end = a;
        while (end < length && !std::isnan(coords[end])) {
            end += 6;
        }
        if (end - a >= 12) {
            path.moveTo(coords[a], coords[a + 1]);
            for (int i = a; i + 6 < end || (closed && i < end); i += 6) {
                const int j = i + 6 < end ? i + 6 : a;
                path.cubicTo(QPointF(coords[i + 4], coords[i + 5]),
                             QPointF(coords[j + 2], coords[j + 3]),
                             QPointF(coords[j], coords[j + 1]));
            }
            if (closed) {
                path.closeSubpath();
            }
        }
        a = end;
    }
    return path;
}

::rust::Vec<double> encodePath(const QPainterPath& path)
{
    ::rust::Vec<double> out;
    int index = 0;
    while (index < path.elementCount()) {
        if (path.elementAt(index).type != QPainterPath::MoveToElement) {
            ++index;
            continue;
        }
        struct Point {
            QPointF anchor;
            QPointF in;
            QPointF out;
            bool hasIn = false;
            bool hasOut = false;
        };
        QVector<Point> points;
        const auto start = path.elementAt(index);
        points.push_back(Point{QPointF(start.x, start.y), {}, {}, false, false});
        ++index;
        while (index < path.elementCount()
               && path.elementAt(index).type != QPainterPath::MoveToElement) {
            const auto element = path.elementAt(index);
            if (element.type == QPainterPath::CurveToElement) {
                if (index + 2 >= path.elementCount()) {
                    break;
                }
                const auto control2 = path.elementAt(index + 1);
                const auto end = path.elementAt(index + 2);
                points.last().out = QPointF(element.x, element.y);
                points.last().hasOut = true;
                points.push_back(Point{QPointF(end.x, end.y),
                                       QPointF(control2.x, control2.y), {}, true, false});
                index += 3;
                continue;
            }
            if (element.type == QPainterPath::LineToElement) {
                points.push_back(Point{QPointF(element.x, element.y), {}, {}, false, false});
            }
            ++index;
        }
        const bool closed = points.size() > 2 && points.first().anchor == points.last().anchor;
        if (closed) {
            points.pop_back();
        }
        out.push_back(std::numeric_limits<double>::quiet_NaN());
        out.push_back(closed ? 1.0 : 0.0);
        out.push_back(0.0);
        out.push_back(0.0);
        out.push_back(0.0);
        out.push_back(0.0);
        for (const Point& point : points) {
            out.push_back(point.anchor.x());
            out.push_back(point.anchor.y());
            out.push_back(point.hasIn ? point.in.x() : point.anchor.x());
            out.push_back(point.hasIn ? point.in.y() : point.anchor.y());
            out.push_back(point.hasOut ? point.out.x() : point.anchor.x());
            out.push_back(point.hasOut ? point.out.y() : point.anchor.y());
        }
    }
    return out;
}

class ShapeToolHandler : public ToolHandler {
public:
    ShapeToolHandler(ToolId id, std::shared_ptr<ShapeGroup> shape)
        : id_(id)
        , kind_(shapeKind(id))
        , shape_(std::move(shape))
    {
        addOptions(shape_.get());
    }

    void onActivate(ToolServices& ctx) override
    {
        syncFromLayer(ctx);
        refreshOverlay(ctx, {});
    }

    void onDeactivate(ToolServices& ctx) override
    {
        ctx.setDragging(false);
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearPathOverlay();
        }
    }

    void onDocumentRefreshed(ToolServices& ctx) override
    {
        if (!ctx.dragging()) {
            syncFromLayer(ctx);
            refreshOverlay(ctx, {});
        }
    }

    void onOptionsChanged(ToolServices& ctx) override
    {
        if (!ctx.dragging()) {
            if (!syncing_) {
                applyToLayer(ctx);
            }
            refreshOverlay(ctx, {});
        }
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return true;
        }
        press_ = imagePos;
        ctx.setDragging(true);
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        if (ctx.dragging()) {
            refreshOverlay(ctx, outlinePath(dragSpec(ctx, imagePos, mods)));
        }
    }

    void onRelease(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        if (!ctx.dragging()) {
            return;
        }
        ctx.setDragging(false);
        refreshOverlay(ctx, {});
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return;
        }
        // A press that wobbles under two screen pixels is a click.
        const double zoom = ctx.canvas() ? qMax(ctx.canvas()->zoom(), 1e-6) : 1.0;
        if (fixedSize(ctx)) {
            land(ctx, *v, dragSpec(ctx, imagePos, mods));
        } else if (QLineF(press_, imagePos).length() * zoom < 2.0) {
            if (id_ != ToolId::Line) {
                createAtClick(ctx, *v);
            }
        } else {
            const ShapeSpec spec = dragSpec(ctx, imagePos, mods);
            const QRectF drawn = outlinePath(spec).boundingRect();
            if (land(ctx, *v, spec) && !drawn.isEmpty()) {
                values_.width = drawn.width();
                values_.height = drawn.height();
            }
        }
        // The new layer became active after the document refreshed.
        syncFromLayer(ctx);
        refreshOverlay(ctx, {});
    }

private:
    // The spec's options from the options bar; the geometry is the caller's.
    ShapeSpec baseSpec(ToolServices& ctx) const
    {
        const ShapeOptions o = shape_->get();
        ShapeSpec spec{};
        spec.kind = kind_;
        spec.r_tl = spec.r_tr = spec.r_br = spec.r_bl = o.radius;
        spec.sides = o.sides;
        spec.indent = 50.0;
        spec.weight = o.weight;
        spec.arrow_start = o.arrowStart;
        spec.arrow_end = o.arrowEnd;
        spec.arrow_width = o.arrowWidth;
        spec.arrow_length = o.arrowLength;
        spec.arrow_concavity = o.arrowConcavity;
        spec.custom = o.custom;
        if (kind_ == 5) {
            spec.custom_preset.reserve(size_t(o.customPreset.size()));
            for (const char value : o.customPreset) { spec.custom_preset.push_back(std::uint8_t(value)); }
        }
        spec.star = o.star;
        spec.indent = o.indent;
        spec.smooth_corners = o.smoothCorners;
        spec.smooth_indents = o.smoothIndents;
        spec.align_edges = o.alignEdges;
        spec.no_fill = !o.fillEnabled;
        spec.fill_kind = o.fillEnabled ? o.fillKind : 0;
        spec.fill_gradient = bytesOf(o.fillGradient);
        spec.fill_pattern = bytesOf(o.fillPattern);
        spec.stroke = o.strokeEnabled;
        spec.stroke_kind = o.strokeKind;
        spec.stroke_color = o.strokeColor.rgba();
        spec.stroke_gradient = bytesOf(o.strokeGradient);
        spec.stroke_pattern = bytesOf(o.strokePattern);
        spec.stroke_width = o.strokeWidth;
        spec.stroke_align = o.strokeAlign;
        spec.stroke_line = o.strokeLine;
        spec.path_operation = o.pathOperation;
        spec.snap_to_pixels = o.snapToPixels;
        spec.proportion = o.geometry == 3 && o.proportionHeight > 0.0
                              ? o.proportionWidth / o.proportionHeight
                              : 0.0;
        spec.fill_mode = o.paintMode;
        spec.fill_opacity = o.paintOpacity;
        spec.fill_antialias = o.paintAntialias;
        return spec;
    }

    // The tools whose geometry gear sizes a box (not the Polygon or Line).
    bool boxed() const { return id_ != ToolId::Polygon && id_ != ToolId::Line; }

    bool fixedSize(ToolServices& ctx) const
    {
        return boxed() && shape_->get().geometry == 2;
    }

    QColor fillColor(ToolServices& ctx) const
    {
        const QColor fill = shape_->get().fillColor;
        return fill.isValid() ? fill : ctx.foreground();
    }

    ShapeSpec dragSpec(ToolServices& ctx, const QPointF& end, Qt::KeyboardModifiers mods) const
    {
        const ShapeOptions o = shape_->get();
        ShapeSpec spec = baseSpec(ctx);
        if (fixedSize(ctx)) {
            // Fixed Size: the box keeps its size and follows the pointer.
            spec.boxed = true;
            spec.x0 = end.x() - (o.fromCenter ? o.fixedWidth / 2.0 : 0.0);
            spec.y0 = end.y() - (o.fromCenter ? o.fixedHeight / 2.0 : 0.0);
            spec.x1 = spec.x0 + o.fixedWidth;
            spec.y1 = spec.y0 + o.fixedHeight;
            return spec;
        }
        spec.x0 = press_.x();
        spec.y0 = press_.y();
        spec.x1 = end.x();
        spec.y1 = end.y();
        spec.shift = mods.testFlag(Qt::ShiftModifier) || (boxed() && o.geometry == 1);
        spec.alt = mods.testFlag(Qt::AltModifier) || (boxed() && o.fromCenter);
        return spec;
    }

    static QPainterPath outlinePath(const ShapeSpec& spec)
    {
        const ::rust::Vec<double> k = shape_outline(spec);
        QPainterPath path;
        const int length = int(k.size());
        for (int a = 0; a < length;) {
            const bool marker = std::isnan(k[a]);
            const bool closed = !marker || k[a + 1] != 0.0;
            if (marker) { a += 6; }
            int end = a;
            while (end < length && !std::isnan(k[end])) { end += 6; }
            if (end - a >= 12) {
                path.moveTo(k[a], k[a + 1]);
                for (int i = a; i + 6 < end || (closed && i < end); i += 6) {
                    const int j = i + 6 < end ? i + 6 : a;
                    path.cubicTo(QPointF(k[i + 4], k[i + 5]), QPointF(k[j + 2], k[j + 3]),
                                 QPointF(k[j], k[j + 1]));
                }
                if (closed) { path.closeSubpath(); }
            }
            a = end;
        }
        return path;
    }

    // The Create dialog, seeded with the last shape's size and the options
    // bar's radius and sides the first time; the shape's top-left corner (or
    // centre) goes at the click.
    void createAtClick(ToolServices& ctx, PictureView& v)
    {
        const ShapeOptions o = shape_->get();
        if (!dialogSeeded_) {
            values_.radii.fill(o.radius);
            values_.sides = o.sides;
            dialogSeeded_ = true;
        }
        if (!execCreateShapeDialog(id_, values_, ctx.canvas())) {
            return;
        }
        ShapeSpec spec = baseSpec(ctx);
        spec.boxed = true;
        spec.x0 = press_.x() - (values_.fromCenter ? values_.width / 2.0 : 0.0);
        spec.y0 = press_.y() - (values_.fromCenter ? values_.height / 2.0 : 0.0);
        spec.x1 = spec.x0 + values_.width;
        spec.y1 = spec.y0 + values_.height;
        spec.r_tl = values_.radii[0];
        spec.r_tr = values_.radii[1];
        spec.r_br = values_.radii[2];
        spec.r_bl = values_.radii[3];
        spec.sides = values_.sides;
        spec.star = values_.star;
        spec.indent = values_.indent;
        spec.smooth_corners = values_.smoothCorners;
        spec.smooth_indents = values_.smoothIndents;
        land(ctx, v, spec);
    }

    // Combine the new outline with the target through Qt's path booleans.
    // False means "no target and Combine", so the caller creates a new layer.
    bool applyPathOperation(PictureView& v, const ShapeSpec& spec, int op)
    {
        const bool pathMode = shape_->get().mode == 1;
        const ::rust::Vec<double> existing =
            pathMode ? path_target_outline(v) : shape_active_outline(v);
        if (existing.empty() && !pathMode && op == 1) {
            return false;
        }
        const QPainterPath before = existing.empty() ? QPainterPath() : pathFromCoords(existing);
        const QPainterPath drawn = outlinePath(spec);
        QPainterPath result;
        switch (op) {
        case 1:
            result = before.isEmpty() ? drawn : before.united(drawn);
            break;
        case 2:
            result = before.subtracted(drawn);
            break;
        case 3:
            result = before.intersected(drawn);
            break;
        case 4:
            result = before.subtracted(drawn).united(drawn.subtracted(before));
            break;
        default:
            return false;
        }
        const ::rust::Vec<double> coords = encodePath(result);
        if (coords.empty()) {
            return true;
        }
        const ::rust::Slice<const double> flat(coords.data(), coords.size());
        return pathMode ? path_replace_target_outline(v, flat)
                        : shape_replace_active_outline(v, flat);
    }

    bool land(ToolServices& ctx, PictureView& v, const ShapeSpec& spec)
    {
        const std::uint32_t color = fillColor(ctx).rgba();
        const int operation = shape_->get().pathOperation;
        if (operation != 0 && shape_->get().mode != 2) {
            if (applyPathOperation(v, spec, operation)) {
                syncFromLayer(ctx);
                return true;
            }
            if (operation != 1) {
                syncFromLayer(ctx);
                return false;
            }
        }
        switch (shape_->get().mode) {
        case 1:
            return shape_add_path(v, spec);
        case 2:
            if (!shape_fill_pixels(v, spec, color)) {
                reportRefusal(ctx, v);
                return false;
            }
            return true;
        default: {
            const QString created = shape_add_layer(v, spec, color);
            if (created.isEmpty()) {
                return false;
            }
            ctx.notifyLayerCreated(created);
            return true;
        }
        }
    }

    static void reportRefusal(ToolServices& ctx, PictureView& v)
    {
        if (activePixelLocked(&v)) {
            ctx.refused(QObject::tr("Could not fill: the layer's pixels are locked."));
        } else if (!v.active_layer_visible()) {
            ctx.refused(QObject::tr("Could not fill: the active layer is invisible."));
        } else if (v.active_layer_path().isEmpty()) {
            ctx.refused(QObject::tr("Could not fill: select a single layer first."));
        }
    }

    // The active shape layer's size and full appearance into the options, so
    // the bar shows them; marked so the change is not applied straight back.
    void syncFromLayer(ToolServices& ctx)
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return;
        }
        path_set_layer_target(*v, shape_->get().mode != 1);
        const ::rust::Vec<double> info = shape_active(*v);
        ShapeOptions o = shape_->get();
        const ShapeOptions before = o;
        if (info.size() == 8) {
            o.activeWidth = info[0];
            o.activeHeight = info[1];
            const QJsonObject root =
                QJsonDocument::fromJson(shape_active_style(*v).toUtf8()).object();
            const QJsonObject fill = root.value(QStringLiteral("fill")).toObject();
            const int fillKind = fill.value(QStringLiteral("kind")).toInt(1);
            o.fillEnabled = fillKind != 0;
            o.fillKind = qMax(1, fillKind);
            if (fillKind == 1) {
                o.fillColor = QColor::fromRgba(QRgb(quint32(fill.value(QStringLiteral("color")).toDouble())));
            }
            o.fillGradient = fill.value(QStringLiteral("gradient")).toString();
            o.fillPattern = fill.value(QStringLiteral("pattern")).toString();
            const QJsonObject stroke = root.value(QStringLiteral("stroke")).toObject();
            const int strokeKind = stroke.value(QStringLiteral("kind")).toInt(1);
            o.strokeEnabled = strokeKind != 0;
            o.strokeKind = qMax(1, strokeKind);
            if (strokeKind == 1) {
                o.strokeColor = QColor::fromRgba(QRgb(quint32(stroke.value(QStringLiteral("color")).toDouble())));
            }
            o.strokeGradient = stroke.value(QStringLiteral("gradient")).toString();
            o.strokePattern = stroke.value(QStringLiteral("pattern")).toString();
            o.strokeWidth = stroke.value(QStringLiteral("width")).toDouble(3.0);
            o.strokeAlign = stroke.value(QStringLiteral("align")).toInt(1);
            o.strokeLine = stroke.value(QStringLiteral("line")).toInt(0);
        } else {
            o.activeWidth = o.activeHeight = 0.0;
        }
        if (o.activeWidth != before.activeWidth || o.activeHeight != before.activeHeight
            || o.fillEnabled != before.fillEnabled || o.fillKind != before.fillKind
            || o.fillColor != before.fillColor || o.fillGradient != before.fillGradient
            || o.fillPattern != before.fillPattern
            || o.strokeEnabled != before.strokeEnabled || o.strokeKind != before.strokeKind
            || o.strokeColor != before.strokeColor || o.strokeGradient != before.strokeGradient
            || o.strokePattern != before.strokePattern || o.strokeWidth != before.strokeWidth
            || o.strokeAlign != before.strokeAlign || o.strokeLine != before.strokeLine) {
            syncing_ = true;
            shape_->set(o);
            syncing_ = false;
        }
    }

    // An options-bar edit of the appearance or W / H restyles or resizes the
    // active shape layer; each differing aspect is one history state.
    void applyToLayer(ToolServices& ctx)
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return;
        }
        const ::rust::Vec<double> info = shape_active(*v);
        if (info.size() != 8) {
            return;
        }
        const ShapeOptions o = shape_->get();
        const QRgb fill = fillColor(ctx).rgba();
        const QByteArray fillGradient = o.fillGradient.toUtf8();
        const QByteArray fillPattern = o.fillPattern.toUtf8();
        shape_set_active_fill_style(*v, o.fillEnabled ? o.fillKind : 0, fill,
                                    ::rust::Str(fillGradient.constData(), size_t(fillGradient.size())),
                                    ::rust::Str(fillPattern.constData(), size_t(fillPattern.size())));
        const QByteArray strokeGradient = o.strokeGradient.toUtf8();
        const QByteArray strokePattern = o.strokePattern.toUtf8();
        shape_set_active_stroke_style(
            *v, o.strokeEnabled ? o.strokeKind : 0, o.strokeColor.rgba(),
            ::rust::Str(strokeGradient.constData(), size_t(strokeGradient.size())),
            ::rust::Str(strokePattern.constData(), size_t(strokePattern.size())),
            o.strokeWidth, o.strokeAlign, o.strokeLine);
        if (o.activeWidth > 0.0 && o.activeHeight > 0.0
            && (qAbs(o.activeWidth - info[0]) > 1e-3 || qAbs(o.activeHeight - info[1]) > 1e-3)) {
            shape_resize_active(*v, o.activeWidth, o.activeHeight);
        }
    }

    // The live outline over either the Work Path it joins (Path mode) or the
    // active shape layer's outline with its anchors.
    void refreshOverlay(ToolServices& ctx, const QPainterPath& outline)
    {
        ImageView* canvas = ctx.canvas();
        if (!canvas) {
            return;
        }
        PictureView* v = ctx.view();
        ImageView::PathOverlay overlay;
        if (v && v->has_document()) {
            const bool pathMode = shape_->get().mode == 1;
            path_set_layer_target(*v, !pathMode);
            if (pathMode) {
                overlay = workPathOverlay(*v, -2);
            } else if (path_target_is_layer(*v)) {
                overlay = workPathOverlay(*v, -1);
                overlay.handles.clear();
            }
        }
        overlay.preview = outline;
        canvas->setPathOverlay(overlay);
    }

    const ToolId id_;
    const int kind_;
    QPointF press_;
    CreateShapeValues values_;
    bool dialogSeeded_ = false;
    bool syncing_ = false;
    std::shared_ptr<ShapeGroup> shape_;
};

} // namespace

std::unique_ptr<ToolHandler> makeShapeToolHandler(ToolId id, std::shared_ptr<ShapeGroup> shape)
{
    return std::make_unique<ShapeToolHandler>(id, std::move(shape));
}

} // namespace pictura
