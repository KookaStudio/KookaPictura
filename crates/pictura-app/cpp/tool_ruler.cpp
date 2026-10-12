#include "tool_handler.h"

#include "image_view.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/annotations.cxxqt.h"

#include <QtCore/QLineF>

#include <memory>
#include <optional>

namespace pictura {

namespace {

std::optional<QLineF> rulerOf(PictureView* v)
{
    const ::rust::Vec<double> l = v ? ruler_line(*v) : ::rust::Vec<double>();
    if (l.size() != 4) {
        return std::nullopt;
    }
    return QLineF(l[0], l[1], l[2], l[3]);
}

double distanceToSegment(const QLineF& line, const QPointF& p)
{
    const QPointF d = line.p2() - line.p1();
    const double len2 = QPointF::dotProduct(d, d);
    const double t = len2 > 0.0 ? qBound(0.0, QPointF::dotProduct(p - line.p1(), d) / len2, 1.0) : 0.0;
    return QLineF(p, line.p1() + d * t).length();
}

// Ruler: drag to draw the measuring line (Shift snaps to 45°), drag an end to
// adjust it or the line to move it; Clear removes it. The line is view state,
// shown only while the tool is active, and never recorded in history.
// ponytail: no protractor (Alt-drag) or Straighten yet.
// Ported from photorust's CanvasView::annotationPress / annotationDrag.
class RulerToolHandler : public ToolHandler {
public:
    void onActivate(ToolServices& ctx) override
    {
        ctx_ = &ctx;
        show(ctx);
    }

    void onDeactivate(ToolServices& ctx) override
    {
        grab_ = Grab::None;
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearRulerLine();
        }
        ctx_ = nullptr;
    }

    void onDocumentRefreshed(ToolServices& ctx) override { show(ctx); }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        PictureView* v = ctx.view();
        ctx_ = &ctx;
        grab_ = Grab::None;
        if (!v || !v->has_document()) {
            return true;
        }
        const double zoom = ctx.canvas() ? ctx.canvas()->zoom() : 1.0;
        const double reach = 8.0 / qMax(zoom, 1e-6);
        press_ = imagePos;
        fresh_ = false;
        if (const auto line = rulerOf(v)) {
            start_ = *line;
            // The nearer end wins, so both ends of a short line stay grabbable.
            const double toA = QLineF(imagePos, start_.p1()).length();
            const double toB = QLineF(imagePos, start_.p2()).length();
            if (qMin(toA, toB) <= reach) {
                grab_ = toA < toB ? Grab::A : Grab::B;
            } else if (distanceToSegment(start_, imagePos) <= reach) {
                grab_ = Grab::Line;
            }
        }
        if (grab_ == Grab::None) {
            start_ = QLineF(imagePos, imagePos);
            grab_ = Grab::B;
            fresh_ = true;
            set_ruler(*v, imagePos.x(), imagePos.y(), imagePos.x(), imagePos.y(), false);
            show(ctx);
        }
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (grab_ == Grab::None || !v) {
            return;
        }
        const bool shift = mods.testFlag(Qt::ShiftModifier);
        if (grab_ == Grab::Line) {
            const QLineF moved = start_.translated(imagePos - press_);
            set_ruler(*v, moved.x1(), moved.y1(), moved.x2(), moved.y2(), false);
        } else if (grab_ == Grab::B) {
            set_ruler(*v, start_.x1(), start_.y1(), imagePos.x(), imagePos.y(), shift);
        } else {
            // Snap about the fixed end B, then restore the A-to-B order the
            // readout's X/Y depend on.
            set_ruler(*v, start_.x2(), start_.y2(), imagePos.x(), imagePos.y(), shift);
            const QLineF snapped = rulerOf(v).value_or(start_);
            set_ruler(*v, snapped.x2(), snapped.y2(), snapped.x1(), snapped.y1(), false);
        }
        show(ctx);
    }

    void onRelease(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        if (grab_ == Grab::None) {
            return;
        }
        onMove(ctx, imagePos, mods);
        PictureView* v = ctx.view();
        const auto line = rulerOf(v);
        // A click without a drag measures nothing; leave no zero-length line.
        if (fresh_ && v && line && line->p1() == line->p2()) {
            clear_ruler(*v);
            show(ctx);
        }
        grab_ = Grab::None;
    }

    bool clearAnnotations() override
    {
        PictureView* v = ctx_ ? ctx_->view() : nullptr;
        if (!v || !rulerOf(v)) {
            return false;
        }
        clear_ruler(*v);
        show(*ctx_);
        return true;
    }

private:
    enum class Grab { None, A, B, Line };

    static void show(ToolServices& ctx)
    {
        if (ImageView* canvas = ctx.canvas()) {
            if (const auto line = rulerOf(ctx.view())) {
                canvas->setRulerLine(*line);
            } else {
                canvas->clearRulerLine();
            }
        }
        ctx.notifyRulerChanged();
    }

    Grab grab_ = Grab::None;
    bool fresh_ = false;
    QLineF start_;
    QPointF press_;
    ToolServices* ctx_ = nullptr;
};

} // namespace

std::unique_ptr<ToolHandler> makeRulerToolHandler()
{
    return std::make_unique<RulerToolHandler>();
}

} // namespace pictura
