#include "tool_handler.h"

#include "image_view.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/annotations.cxxqt.h"

#include <QtCore/QObject>

#include <memory>

namespace pictura {

namespace {

constexpr int kSampler = 0;
constexpr int kNote = 1;

// Color Sampler and Note: click to place a marker, drag one to move it (one
// state on release), Alt-click or drag it off the canvas to delete it. A note
// click also makes that note current, which opens it in the Notes panel.
// Ported from photorust's CanvasView::annotationPress / annotationDrag.
class MarkerToolHandler : public ToolHandler {
public:
    explicit MarkerToolHandler(int kind)
        : kind_(kind)
    {
    }

    void onActivate(ToolServices& ctx) override { ctx_ = &ctx; }

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
        const int hit = marker_near(*v, kind_, imagePos.x(), imagePos.y(), 8.0 / qMax(zoom, 1e-6));
        const bool alt = mods.testFlag(Qt::AltModifier);
        if (hit >= 0 && alt) {
            remove_marker(*v, kind_, hit);
            return true;
        }
        if (hit >= 0) {
            dragged_ = hit;
        } else if (!alt) {
            dragged_ = add_marker(*v, kind_, qRound(imagePos.x()), qRound(imagePos.y()));
            if (dragged_ < 0) {
                ctx.refused(QObject::tr("Color Sampler: all four samplers are placed."));
                return true;
            }
        }
        if (kind_ == kNote && dragged_ >= 0) {
            ctx.setCurrentNote(dragged_);
        }
        const ::rust::Vec<std::int32_t> p = dragged_ >= 0 ? marker_at(*v, kind_, dragged_)
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
        move_marker(*v, kind_, dragged_, qRound(imagePos.x()), qRound(imagePos.y()), false);
        ctx.refreshAnnotations();
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
        // CS6 deletes a marker dragged out of the image.
        if (end.x() < 0 || end.y() < 0 || end.x() >= v->document_width()
            || end.y() >= v->document_height()) {
            remove_marker(*v, kind_, index);
            return;
        }
        move_marker(*v, kind_, index, end.x(), end.y(), true);
    }

    bool clearAnnotations() override
    {
        PictureView* v = ctx_ ? ctx_->view() : nullptr;
        if (!v) {
            return false;
        }
        if (kind_ == kNote) {
            ctx_->setCurrentNote(-1);
        }
        return clear_markers(*v, kind_);
    }

private:
    const int kind_;
    int dragged_ = -1;
    QPoint start_;
    ToolServices* ctx_ = nullptr;
};

} // namespace

std::unique_ptr<ToolHandler> makeColorSamplerToolHandler()
{
    return std::make_unique<MarkerToolHandler>(kSampler);
}

std::unique_ptr<ToolHandler> makeNoteToolHandler()
{
    return std::make_unique<MarkerToolHandler>(kNote);
}

} // namespace pictura
