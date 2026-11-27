#include "tool_handler.h"

#include "image_view.h"

#include "api/paint_bridge.h"

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

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
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
        moved_ = false;
        armStart_.reset();
        if (const auto line = rulerOf(v)) {
            start_ = *line;
            const auto arm = ruler_protractor(*v);
            if (arm.size() == 4) {
                const QPointF vertex(arm[0], arm[1]);
                savedArmAtStart_ = QLineF(vertex, start_.p1()).length() < QLineF(vertex, start_.p2()).length();
                armStart_ = QPointF(arm[2], arm[3]) - vertex;
            }
            // The nearer end wins, so both ends of a short line stay grabbable.
            const double toA = QLineF(imagePos, start_.p1()).length();
            const double toB = QLineF(imagePos, start_.p2()).length();
            if (qMin(toA, toB) <= reach) {
                if (mods.testFlag(Qt::AltModifier)) {
                    armAtStart_ = toA < toB;
                    armConstrain_ = mods.testFlag(Qt::ShiftModifier);
                    grab_ = Grab::Arm;
                } else {
                    grab_ = toA < toB ? Grab::A : Grab::B;
                }
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

    bool onDoubleClick(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        const auto line = rulerOf(ctx.view());
        const double reach = 8.0 / qMax(ctx.canvas() ? ctx.canvas()->zoom() : 1.0, 1e-6);
        if (!line || distanceToSegment(*line, imagePos) > reach) return false;
        start_ = *line;
        armAtStart_ = QLineF(imagePos, start_.p1()).length() < QLineF(imagePos, start_.p2()).length();
        grab_ = Grab::Arm;
        armConstrain_ = mods.testFlag(Qt::ShiftModifier);
        ctx_ = &ctx;
        return true;
    }

    void onModifiers(ToolServices&, Qt::KeyboardModifiers mods) override
    {
        if (grab_ == Grab::Arm) armConstrain_ = mods.testFlag(Qt::ShiftModifier);
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (grab_ == Grab::None || !v) {
            return;
        }
        if (!fresh_ && grab_ != Grab::Arm && !moved_ && imagePos == press_) return;
        moved_ = moved_ || imagePos != press_;
        const bool shift = mods.testFlag(Qt::ShiftModifier) || (grab_ == Grab::Arm && armConstrain_);
        if (grab_ == Grab::Arm) {
            set_ruler_protractor(*v, armAtStart_, imagePos.x(), imagePos.y(), shift);
        } else if (grab_ == Grab::Line) {
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
        if (grab_ != Grab::Arm && !fresh_ && armStart_) {
            if (const auto line = rulerOf(v)) {
                const QPointF vertex = savedArmAtStart_ ? line->p1() : line->p2();
                const QPointF end = vertex + *armStart_;
                set_ruler_protractor(*v, savedArmAtStart_, end.x(), end.y(), false);
            }
        }
        show(ctx);
    }

    void onRelease(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        if (grab_ == Grab::None) {
            return;
        }
        if (fresh_ || grab_ == Grab::Arm || moved_ || imagePos != press_) {
            onMove(ctx, imagePos, mods);
        }
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
    enum class Grab { None, A, B, Line, Arm };

    static void show(ToolServices& ctx)
    {
        if (ImageView* canvas = ctx.canvas()) {
            if (const auto line = rulerOf(ctx.view())) {
                canvas->setRulerLine(*line);
                const auto arm = ruler_protractor(*ctx.view());
                if (arm.size() == 4) canvas->setRulerProtractor(QLineF(arm[0], arm[1], arm[2], arm[3]));
            } else {
                canvas->clearRulerLine();
            }
        }
        ctx.notifyRulerChanged();
    }

    Grab grab_ = Grab::None;
    bool fresh_ = false;
    bool moved_ = false;
    bool armAtStart_ = false;
    bool armConstrain_ = false;
    bool savedArmAtStart_ = false;
    std::optional<QPointF> armStart_;
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
