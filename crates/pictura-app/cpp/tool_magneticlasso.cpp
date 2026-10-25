#include "tool_handler.h"

#include "api/document_api.h"
#include "image_view.h"
#include "selection_geometry.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/magnetic.cxxqt.h"

#include <QtCore/QElapsedTimer>
#include <QtGui/QPolygonF>
#include <QtWidgets/QApplication>

#include <algorithm>
#include <cmath>
#include <memory>

namespace pictura {

namespace {

// Magnetic Lasso: the first click sets a fastening point, then a live wire
// follows the pointer along the strongest edge within Width. A click fastens
// the wire; Frequency fastens it automatically as it lengthens. Clicking the
// first point, double-clicking, or Enter closes the outline; Delete removes the
// last fastening point; Escape discards the outline and leaves the selection
// untouched. The closed outline commits through the shared lasso path.
//
// ponytail: CS6's Alt-drag (temporary Lasso) / Alt-click (temporary Polygonal
// Lasso), the Caps Lock width ring, and Stylus Pressure are not modelled.
class MagneticLassoToolHandler : public ToolHandler {
public:
    explicit MagneticLassoToolHandler(std::shared_ptr<SelectionOptions> selection)
        : selection_(std::move(selection))
    {
        addOptions(selection_.get());
    }

    void onActivate(ToolServices& ctx) override { ctx_ = &ctx; }

    void onDeactivate(ToolServices& ctx) override
    {
        if (active_) {
            cancel(ctx);
        }
        ctx_ = nullptr;
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (!v) {
            return true;
        }
        ctx_ = &ctx;
        const QPoint p = imagePos.toPoint();
        if (!active_) {
            begin(ctx, *v, p, mods);
            return true;
        }
        const bool closeClick = path_.size() >= 2 && near(ctx, p, path_.first());
        const bool doubleClick = clock_.isValid()
            && clock_.elapsed() <= QApplication::doubleClickInterval()
            && near(ctx, p, lastPress_);
        if (closeClick || doubleClick) {
            close(ctx);
            return true;
        }
        fasten(p);
        cursor_ = lastPress_ = p;
        clock_.restart();
        showPreview(ctx);
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!active_) {
            return;
        }
        cursor_ = imagePos.toPoint();
        updateWire(ctx);
        autoFasten(ctx);
        showPreview(ctx);
    }

    bool onFinishPath() override
    {
        if (!active_ || !ctx_) {
            return false;
        }
        close(*ctx_);
        return true;
    }

    bool onEscape() override
    {
        if (!active_ || !ctx_) {
            return false;
        }
        cancel(*ctx_);
        return true;
    }

    bool onDeletePoint() override
    {
        if (!active_ || !ctx_) {
            return false;
        }
        if (fastening_.size() <= 1) {
            cancel(*ctx_);
            return true;
        }
        fastening_.removeLast();
        path_.resize(fastening_.last() + 1);
        updateWire(*ctx_);
        showPreview(*ctx_);
        return true;
    }

    bool gestureInProgress() const override { return active_; }

    bool hasCanvasContextMenu() const override { return true; }
    QMenu* buildContextMenu(ToolServices& ctx, const QPointF& imagePos) override
    {
        return selectionCanvasMenu(ctx, imagePos);
    }

private:
    void begin(ToolServices& ctx, PictureView& v, const QPoint& p, Qt::KeyboardModifiers mods)
    {
        if (document_api::documentDepthBits(v) == 32) {
            ctx.refused(QStringLiteral(
                "The Magnetic Lasso is not available for 32-bit documents."));
            return;
        }
        ctx.setDragMode(ctx.resolveSelectionMode(mods, v.has_selection()));
        if (!magnetic_begin(v, selection_->magneticContrast())) {
            return;
        }
        if (!v.begin_lasso(selectionModeString(ctx.dragMode()))) {
            magnetic_end(v);
            return;
        }
        active_ = true;
        path_ = {p};
        preview_.clear();
        fastening_ = {0};
        cursor_ = lastPress_ = p;
        clock_.restart();
        showPreview(ctx);
    }

    // Lay the live wire down up to `p` and fasten there.
    void fasten(const QPoint& p)
    {
        path_ += preview_;
        preview_.clear();
        if (path_.last() != p) {
            path_ << p;
        }
        fastening_ << int(path_.size()) - 1;
    }

    void updateWire(ToolServices& ctx)
    {
        preview_.clear();
        PictureView* v = ctx.view();
        if (!v || path_.isEmpty()) {
            return;
        }
        const QPoint from = path_.last();
        const ::rust::Vec<std::int32_t> flat = magnetic_trace(
            *v, from.x(), from.y(), cursor_.x(), cursor_.y(), selection_->magneticWidth());
        // The wire starts on the last fastening point, which the path holds.
        for (std::size_t i = 2; i + 1 < flat.size(); i += 2) {
            preview_ << QPoint(flat[i], flat[i + 1]);
        }
    }

    // Only a pointer move fastens automatically, so Delete can peel an
    // automatic point back without it reappearing at once. Frequency 0 never
    // fastens; 100 fastens every ~8 px of wire, 1 every ~107 px.
    void autoFasten(ToolServices& ctx)
    {
        const int frequency = selection_->magneticFrequency();
        if (frequency > 0 && preview_.size() >= std::max(8, 108 - frequency)) {
            path_ += preview_;
            preview_.clear();
            fastening_ << int(path_.size()) - 1;
        }
    }

    void close(ToolServices& ctx)
    {
        PictureView* v = ctx.view();
        path_ += preview_;
        bool committed = false;
        if (v) {
            for (const QPoint& p : path_) {
                v->lasso_add_point(p.x(), p.y());
            }
            committed = path_.size() >= 3 && v->end_lasso(selection_->feather());
            if (!committed) {
                v->cancel_lasso();
            }
        }
        // An outline under three points encloses nothing: a click, which
        // clears the selection in New mode, as in CS6 and photorust.
        const bool click = !committed && v && path_.size() < 3
            && ctx.dragMode() == SelectionMode::New && v->has_selection();
        reset(ctx);
        if (click) {
            v->deselect();
        }
        if (committed || click) {
            ctx.emitSelectionCommitted();
        }
    }

    void cancel(ToolServices& ctx)
    {
        if (PictureView* v = ctx.view()) {
            v->cancel_lasso();
        }
        reset(ctx);
    }

    void reset(ToolServices& ctx)
    {
        if (PictureView* v = ctx.view()) {
            magnetic_end(*v);
        }
        active_ = false;
        path_.clear();
        preview_.clear();
        fastening_.clear();
        clock_.invalidate();
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearSelectionPreview();
        }
    }

    void showPreview(ToolServices& ctx)
    {
        ImageView* canvas = ctx.canvas();
        if (!canvas) {
            return;
        }
        QPolygonF outline;
        for (const QPoint& p : path_) {
            outline << QPointF(p);
        }
        for (const QPoint& p : preview_) {
            outline << QPointF(p);
        }
        if (outline.isEmpty() || outline.last() != QPointF(cursor_)) {
            outline << QPointF(cursor_);
        }
        // Closed: the straight connector back to the origin previews the edge
        // closing will add, as CS6 shows it.
        canvas->setSelectionShapePreview({outline});
        if (!path_.isEmpty()) {
            canvas->setSelectionPreviewOrigin(QPointF(path_.first()));
        }
    }

    // The close hit zone is fixed on screen, so it stays reachable at any zoom.
    static bool near(ToolServices& ctx, const QPoint& a, const QPoint& b)
    {
        const double zoom = ctx.canvas() ? std::max(ctx.canvas()->zoom(), 1e-6) : 1.0;
        return std::hypot(a.x() - b.x(), a.y() - b.y()) * zoom <= kPolygonCloseRadius;
    }

    bool active_ = false;
    QList<QPoint> path_;
    QList<QPoint> preview_;
    // Index into `path_` of each fastening point, first to last.
    QList<int> fastening_;
    QPoint cursor_;
    QPoint lastPress_;
    QElapsedTimer clock_;
    ToolServices* ctx_ = nullptr;
    std::shared_ptr<SelectionOptions> selection_;
};

} // namespace

std::unique_ptr<ToolHandler> makeMagneticLassoToolHandler(
    std::shared_ptr<SelectionOptions> selection)
{
    return std::make_unique<MagneticLassoToolHandler>(std::move(selection));
}

} // namespace pictura
