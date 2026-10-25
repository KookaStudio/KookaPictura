#include "tool_handler.h"

#include "commands.h"
#include "image_view.h"
#include "selection_geometry.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QElapsedTimer>
#include <QtCore/QPointF>
#include <QtCore/QRect>
#include <QtCore/Qt>
#include <QtGui/QPainterPath>
#include <QtGui/QPolygonF>
#include <QtWidgets/QApplication>

#include <cmath>
#include <memory>

namespace pictura {

namespace {

QRect marqueeGeometry(const SelectionOptions& selection, const QPointF& a, const QPointF& b,
                      Qt::KeyboardModifiers mods, bool mirror)
{
    return marqueeDragRect(a, b, mods, mirror, selection.marqueeStyle(),
                           selection.fixedRatioWidth(), selection.fixedRatioHeight(),
                           selection.fixedSizeWidth(), selection.fixedSizeHeight());
}

// A click that encloses no area (no drag, or a lasso outline under three
// points) clears the selection in New mode, as in CS6; a stray click while
// adding, subtracting, or intersecting leaves it alone. Ported from photorust's
// CanvasView release / commitLasso.
bool clickDeselect(ToolServices& ctx, PictureView* v)
{
    if (!v || ctx.dragMode() != SelectionMode::New || !v->has_selection()) {
        return false;
    }
    v->deselect();
    ctx.emitSelectionCommitted();
    return true;
}

// Rectangular and Elliptical Marquee share the drag lifecycle; `elliptical_`
// selects the ellipse preview and the ellipse raster at release.
class MarqueeToolHandler : public ToolHandler {
public:
    MarqueeToolHandler(bool elliptical, std::shared_ptr<SelectionOptions> selection)
        : elliptical_(elliptical)
        , selection_(std::move(selection))
    {
        addOptions(selection_.get());
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (!v) {
            return true;
        }
        ctx.setDragMode(ctx.resolveSelectionMode(mods, v->has_selection()));
        dragMods_ = mods;
        const bool alt = mods.testFlag(Qt::AltModifier);
        altArmed_ = alt;
        altWasDown_ = alt;
        mirrorAlt_ = false;
        ctx.setDragging(true);
        ctx.setDragCommitted(false);
        anchor_ = last_ = imagePos;
        updateOverlay(ctx, imagePos);
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        if (!ctx.dragging()) {
            return;
        }
        trackAltEdge(ctx, mods);
        last_ = imagePos;
        updateOverlay(ctx, imagePos);
    }

    // A modifier edge with no pointer movement still arms subtract or mirror.
    void onModifiers(ToolServices& ctx, Qt::KeyboardModifiers mods) override
    {
        if (!ctx.dragging()) {
            return;
        }
        trackAltEdge(ctx, mods);
        updateOverlay(ctx, last_);
    }

    void onRelease(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        ctx.setDragging(false);
        PictureView* v = ctx.view();
        const QRect rect = marqueeGeometry(*selection_, anchor_, imagePos, dragMods_, mirrorAlt_);
        const bool shaped = v && rect.width() > 0 && rect.height() > 0;
        const bool committed = shaped
            && (elliptical_ ? v->select_ellipse(rect.x(), rect.y(), rect.width(), rect.height(),
                                                selectionModeString(ctx.dragMode()), selection_->feather())
                            : v->select_rect(rect.x(), rect.y(), rect.width(), rect.height(),
                                             selectionModeString(ctx.dragMode()), selection_->feather()));
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearSelectionPreview();
            canvas->clearDragSizeHint();
        }
        if (committed) {
            ctx.emitSelectionCommitted();
        } else if (!shaped) {
            clickDeselect(ctx, v);
        }
    }

    Qt::KeyboardModifiers dragMods() const override { return dragMods_; }

    bool marqueeMirror() const override { return mirrorAlt_; }

private:
    // Two-stage Alt: the first Alt (held at press, or first pressed during the
    // drag) subtracts; a later Alt edge during the same drag mirrors the
    // rectangle about the anchor.
    void trackAltEdge(ToolServices& ctx, Qt::KeyboardModifiers mods)
    {
        const bool altNow = mods.testFlag(Qt::AltModifier);
        if (altNow && !altWasDown_) {
            if (altArmed_) {
                mirrorAlt_ = true;
            } else {
                altArmed_ = true;
                PictureView* v = ctx.view();
                if (v && v->has_selection()) {
                    ctx.setDragMode(SelectionMode::Subtract);
                }
            }
        }
        altWasDown_ = altNow;
    }

    void updateOverlay(ToolServices& ctx, const QPointF& imagePos)
    {
        ImageView* canvas = ctx.canvas();
        if (!canvas) {
            return;
        }
        const QRect rect = marqueeGeometry(*selection_, anchor_, imagePos, dragMods_, mirrorAlt_);
        if (rect.width() <= 0 || rect.height() <= 0) {
            canvas->clearSelectionPreview();
            return;
        }
        if (elliptical_) {
            // Preview the actual ellipse, not its bounding rectangle.
            QPainterPath path;
            path.addEllipse(QRectF(rect));
            canvas->setSelectionShapePreview({path.toFillPolygon()});
        } else {
            canvas->setSelectionShapePreview({QPolygonF(QRectF(rect))});
        }
        canvas->setDragSizeHint(
            QStringLiteral("%1 x %2").arg(rect.width()).arg(rect.height()), imagePos);
    }

    bool hasCanvasContextMenu() const override { return true; }
    QMenu* buildContextMenu(ToolServices& ctx, const QPointF& imagePos) override
    {
        return selectionCanvasMenu(ctx, imagePos);
    }

    bool elliptical_;
    std::shared_ptr<SelectionOptions> selection_;
    Qt::KeyboardModifiers dragMods_ = Qt::NoModifier;
    bool altArmed_ = false;
    bool altWasDown_ = false;
    bool mirrorAlt_ = false;
    QPointF anchor_;
    QPointF last_;
};

// Freehand Lasso: the mask grows with each move until release.
class LassoToolHandler : public ToolHandler {
public:
    explicit LassoToolHandler(std::shared_ptr<SelectionOptions> selection)
        : selection_(std::move(selection))
    {
        addOptions(selection_.get());
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (!v) {
            return true;
        }
        ctx.setDragMode(ctx.resolveSelectionMode(mods, v->has_selection()));
        if (!v->begin_lasso(selectionModeString(ctx.dragMode()))) {
            return true;
        }
        ctx.setDragging(true);
        ctx.setDragCommitted(false);
        last_ = imagePos;
        lassoPolygon_.clear();
        lassoPolygon_ << imagePos;
        if (ImageView* canvas = ctx.canvas()) {
            canvas->setSelectionShapePreview({lassoPolygon_});
        }
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        PictureView* v = ctx.view();
        if (!v) {
            return;
        }
        v->lasso_add_point(qRound(imagePos.x()), qRound(imagePos.y()));
        lassoPolygon_ << imagePos;
        if (ImageView* canvas = ctx.canvas()) {
            canvas->setSelectionShapePreview({lassoPolygon_});
        }
    }

    void onRelease(ToolServices& ctx, const QPointF&, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        ctx.setDragging(false);
        PictureView* v = ctx.view();
        const bool click = lassoPolygon_.size() < 3;
        const bool committed = !click && v && v->end_lasso(selection_->feather());
        if (click && v) {
            v->cancel_lasso();
            clickDeselect(ctx, v);
        }
        lassoPolygon_.clear();
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearSelectionPreview();
        }
        if (committed) {
            ctx.emitSelectionCommitted();
        }
    }

    void onSessionCancelled(ToolServices& ctx) override
    {
        if (PictureView* v = ctx.view()) {
            v->cancel_lasso();
        }
        lassoPolygon_.clear();
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearSelectionPreview();
        }
    }

    bool hasCanvasContextMenu() const override { return true; }
    QMenu* buildContextMenu(ToolServices& ctx, const QPointF& imagePos) override
    {
        return selectionCanvasMenu(ctx, imagePos);
    }

private:
    QPointF last_;
    QPolygonF lassoPolygon_;
    std::shared_ptr<SelectionOptions> selection_;
};

// Polygonal Lasso: click to add vertices, close on the first vertex or a rapid
// second click near the last one, commit on Enter, discard on Escape.
class PolygonalLassoToolHandler : public ToolHandler {
public:
    explicit PolygonalLassoToolHandler(std::shared_ptr<SelectionOptions> selection)
        : selection_(std::move(selection))
    {
        addOptions(selection_.get());
    }

    void onActivate(ToolServices& ctx) override { ctx_ = &ctx; }

    void onDeactivate(ToolServices& ctx) override
    {
        if (polygonInProgress_) {
            cancel(ctx);
        }
        ctx_ = nullptr;
    }

    void onSessionCancelled(ToolServices& ctx) override
    {
        if (polygonInProgress_) {
            cancel(ctx);
        }
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (!v) {
            return true;
        }
        ctx_ = &ctx;
        const double firstDist = polygonPoints_.isEmpty()
            ? 1e9
            : std::hypot(imagePos.x() - polygonPoints_.first().x(),
                         imagePos.y() - polygonPoints_.first().y());
        const double lastDist = polygonClock_.isValid()
            ? std::hypot(imagePos.x() - lastPolygonPress_.x(),
                         imagePos.y() - lastPolygonPress_.y())
            : 1e9;
        const bool closeClick = polygonInProgress_ && firstDist <= kPolygonCloseRadius;
        const bool doubleClick = polygonInProgress_ && polygonClock_.isValid()
            && polygonClock_.elapsed() <= QApplication::doubleClickInterval()
            && lastDist <= kPolygonCloseRadius;
        if (closeClick || doubleClick) {
            close(ctx);
            return true;
        }
        if (!polygonInProgress_) {
            ctx.setDragMode(ctx.resolveSelectionMode(mods, v->has_selection()));
            if (!v->begin_lasso(selectionModeString(ctx.dragMode()))) {
                return true;
            }
            polygonInProgress_ = true;
            polygonPoints_.clear();
        }
        // ponytail: CS6's Shift 45-degree segment snap is deferred; a plain
        // click adds the vertex at the pointer.
        polygonPoints_ << imagePos;
        lastPolygonPress_ = imagePos;
        polygonClock_.restart();
        v->lasso_add_point(qRound(imagePos.x()), qRound(imagePos.y()));
        if (ImageView* canvas = ctx.canvas()) {
            canvas->setSelectionPreview({polygonPoints_}, false, /*solid=*/true);
        }
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (polygonInProgress_) {
            if (ImageView* canvas = ctx.canvas()) {
                QPolygonF preview = polygonPoints_;
                preview << imagePos;
                canvas->setSelectionPreview({preview}, false, /*solid=*/true);
            }
        }
    }

    bool onFinishPath() override
    {
        if (!polygonInProgress_) {
            return false;
        }
        if (ctx_) {
            close(*ctx_);
        }
        return true;
    }

    bool onEscape() override
    {
        if (!polygonInProgress_) {
            return false;
        }
        if (ctx_) {
            cancel(*ctx_);
        }
        return true;
    }

private:
    void close(ToolServices& ctx)
    {
        PictureView* v = ctx.view();
        const bool enough = polygonPoints_.size() >= 3;
        const bool committed = enough && v && v->end_lasso(selection_->feather());
        if (v && !committed) {
            v->cancel_lasso();
        }
        polygonInProgress_ = false;
        polygonPoints_.clear();
        polygonClock_.invalidate();
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearSelectionPreview();
        }
        if (committed) {
            ctx.emitSelectionCommitted();
        } else if (!enough) {
            clickDeselect(ctx, v);
        }
    }

    void cancel(ToolServices& ctx)
    {
        polygonInProgress_ = false;
        polygonPoints_.clear();
        polygonClock_.invalidate();
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearSelectionPreview();
        }
        if (PictureView* v = ctx.view()) {
            v->cancel_lasso();
        }
    }

    bool hasCanvasContextMenu() const override { return true; }
    QMenu* buildContextMenu(ToolServices& ctx, const QPointF& imagePos) override
    {
        return selectionCanvasMenu(ctx, imagePos);
    }

    QPolygonF polygonPoints_;
    QPointF lastPolygonPress_;
    QElapsedTimer polygonClock_;
    bool polygonInProgress_ = false;
    ToolServices* ctx_ = nullptr;
    std::shared_ptr<SelectionOptions> selection_;
};

} // namespace

QMenu* selectionCanvasMenu(ToolServices& ctx, const QPointF& imagePos)
{
    Q_UNUSED(imagePos);
    QWidget* parent = ctx.canvas();
    if (!parent) {
        return nullptr;
    }
    auto* menu = new QMenu(parent);
    const auto add = [menu, &ctx](const QString& label, const QString& id) {
        QAction* action = menu->addAction(label);
        ToolServices* services = &ctx;
        QObject::connect(action, &QAction::triggered, menu,
                         [services, id]() { services->requestCommand(id); });
    };
    add(QObject::tr("Deselect"), QString::fromLatin1(command_ids::SelectDeselect));
    add(QObject::tr("Inverse"), QString::fromLatin1(command_ids::SelectInverse));
    add(QObject::tr("Feather…"), QString::fromLatin1(command_ids::SelectModifyFeather));
    add(QObject::tr("Refine Edge…"),
        commandIdForPath({QStringLiteral("Select"), QStringLiteral("Refine Edge…")}));
    return menu;
}

std::unique_ptr<ToolHandler> makeMarqueeToolHandler(std::shared_ptr<SelectionOptions> selection)
{
    return std::make_unique<MarqueeToolHandler>(false, std::move(selection));
}

std::unique_ptr<ToolHandler> makeEllipticalMarqueeToolHandler(
    std::shared_ptr<SelectionOptions> selection)
{
    return std::make_unique<MarqueeToolHandler>(true, std::move(selection));
}

std::unique_ptr<ToolHandler> makeLassoToolHandler(std::shared_ptr<SelectionOptions> selection)
{
    return std::make_unique<LassoToolHandler>(std::move(selection));
}

std::unique_ptr<ToolHandler> makePolygonalLassoToolHandler(
    std::shared_ptr<SelectionOptions> selection)
{
    return std::make_unique<PolygonalLassoToolHandler>(std::move(selection));
}

} // namespace pictura
