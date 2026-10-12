// Red Eye tool: drag a box over an eye (or click a pupil) and the red is
// taken out of it with the options bar's Pupil Size and Darken Amount
// (`red_eye` in `cxxqt_object/healing.rs`). Ported from photorust's
// CanvasView red-eye press/drag/release.

#include "tool_handler.h"

#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/healing.cxxqt.h"

#include <QtCore/QObject>
#include <QtCore/QRectF>
#include <QtGui/QPolygonF>

#include <cmath>
#include <memory>

namespace pictura {

namespace {

// A click still means "the eye is here": it gets a box this size around it.
constexpr double kClickBox = 24.0;

class RedEyeToolHandler : public ToolHandler {
public:
    explicit RedEyeToolHandler(std::shared_ptr<RedEyeGroup> redEye)
        : redEye_(std::move(redEye))
    {
        addOptions(redEye_.get());
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return true;
        }
        anchor_ = imagePos;
        ctx.setDragging(true);
        ctx.setDragCommitted(false);
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        if (ImageView* canvas = ctx.canvas()) {
            canvas->setSelectionPreview({QPolygonF(QRectF(anchor_, imagePos).normalized())});
        }
    }

    void onRelease(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        ctx.setDragging(false);
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearSelectionPreview();
        }
        PictureView* v = ctx.view();
        if (!v) {
            return;
        }
        QRectF box = QRectF(anchor_, imagePos).normalized();
        if (box.width() < 2.0 || box.height() < 2.0) {
            box = QRectF(box.center() - QPointF(kClickBox / 2, kClickBox / 2),
                         QSizeF(kClickBox, kClickBox));
        }
        if (red_eye(*v, int(std::floor(box.x())), int(std::floor(box.y())),
                    int(std::round(box.width())), int(std::round(box.height())),
                    redEye_->get().pupil, redEye_->get().darken)) {
            return;
        }
        if (activePixelLocked(v)) {
            ctx.refused(QObject::tr("Could not remove red eye: the layer's pixels are locked."));
        } else if (v->active_layer_path().isEmpty()) {
            ctx.refused(QObject::tr("Could not remove red eye: select a single layer first."));
        }
    }

    void onDeactivate(ToolServices& ctx) override
    {
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearSelectionPreview();
        }
    }

private:
    QPointF anchor_;
    std::shared_ptr<RedEyeGroup> redEye_;
};

} // namespace

std::unique_ptr<ToolHandler> makeRedEyeToolHandler(std::shared_ptr<PaintOptions> paint,
                                                   std::shared_ptr<RedEyeGroup> redEye)
{
    (void)paint;
    return std::make_unique<RedEyeToolHandler>(std::move(redEye));
}

} // namespace pictura
