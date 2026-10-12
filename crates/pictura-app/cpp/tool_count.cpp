// Count (Photoshop Extended): numbered marks grouped into named count groups.
// Click adds a mark to the active group, drag moves it, Alt-click or dragging
// off the canvas deletes it. The group state (name, visibility, colour, marker
// and label sizes) lives on Document::annotations and is edited from the Count
// options bar. Ported from photorust's Count/annotation handling onto the
// cxx-qt bridge in `cxxqt_object/annotations.rs`.

#include "tool_handler.h"

#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/annotations.cxxqt.h"

#include <QtCore/QPoint>

#include <memory>

namespace pictura {

namespace {

class CountToolHandler : public ToolHandler {
public:
    void onActivate(ToolServices& ctx) override
    {
        ctx_ = &ctx;
        ctx.notifyCountChanged();
    }

    void onDeactivate(ToolServices&) override
    {
        dragged_ = -1;
        ctx_ = nullptr;
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        ctx_ = &ctx;
        dragged_ = -1;
        if (!v || !v->has_document()) {
            return true;
        }
        const double zoom = ctx.canvas() ? ctx.canvas()->zoom() : 1.0;
        // Eight screen pixels, so the grab area keeps its size at any zoom.
        const int hit = count_mark_near(*v, imagePos.x(), imagePos.y(), 8.0 / qMax(zoom, 1e-6));
        const bool alt = mods.testFlag(Qt::AltModifier);
        if (hit >= 0 && alt) {
            count_remove_mark(*v, hit);
            ctx.notifyCountChanged();
            return true;
        }
        if (hit >= 0) {
            dragged_ = hit;
        } else if (!alt) {
            dragged_ = count_add_mark(*v, qRound(imagePos.x()), qRound(imagePos.y()));
            ctx.notifyCountChanged();
        }
        const ::rust::Vec<std::int32_t> p =
            dragged_ >= 0 ? count_group_mark_at(*v, count_active_group(*v), dragged_)
                          : ::rust::Vec<std::int32_t>();
        start_ = p.size() == 2 ? QPoint(p[0], p[1]) : QPoint();
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        PictureView* v = ctx.view();
        if (dragged_ < 0 || !v) {
            return;
        }
        count_move_mark(*v, dragged_, qRound(imagePos.x()), qRound(imagePos.y()), false);
    }

    void onRelease(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        PictureView* v = ctx.view();
        const int index = dragged_;
        dragged_ = -1;
        if (index < 0 || !v) {
            return;
        }
        const QPoint end(qRound(imagePos.x()), qRound(imagePos.y()));
        if (end == start_) {
            return;
        }
        // CS6 deletes a mark dragged out of the image.
        if (end.x() < 0 || end.y() < 0 || end.x() >= v->document_width()
            || end.y() >= v->document_height()) {
            count_remove_mark(*v, index);
            ctx.notifyCountChanged();
            return;
        }
        count_move_mark(*v, index, end.x(), end.y(), true);
        ctx.notifyCountChanged();
    }

    bool clearAnnotations() override
    {
        PictureView* v = ctx_ ? ctx_->view() : nullptr;
        if (!v) {
            return false;
        }
        const bool cleared = count_clear(*v);
        if (cleared) {
            ctx_->notifyCountChanged();
        }
        return cleared;
    }

private:
    int dragged_ = -1;
    QPoint start_;
    ToolServices* ctx_ = nullptr;
};

} // namespace

std::unique_ptr<ToolHandler> makeCountToolHandler()
{
    return std::make_unique<CountToolHandler>();
}

} // namespace pictura
