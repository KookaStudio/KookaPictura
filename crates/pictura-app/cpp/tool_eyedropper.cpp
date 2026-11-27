#include "tool_handler.h"
#include <QtCore/QCoreApplication>

#include "image_view.h"

#include "api/paint_bridge.h"

#include <QtCore/QObject>
#include <QtCore/QPair>
#include <QtCore/QtGlobal>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtWidgets/QMenu>

#include <memory>

namespace pictura {

namespace {

class EyedropperToolHandler : public ToolHandler {
public:
    explicit EyedropperToolHandler(std::shared_ptr<EyedropperGroup> options)
        : options_(std::move(options))
    {
        addOptions(options_.get());
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        dragging_ = false;
        if (mods.testFlag(Qt::ShiftModifier)) {
            PictureView* v = ctx.view();
            const int x = qRound(imagePos.x());
            const int y = qRound(imagePos.y());
            if (v && v->has_document() && x >= 0 && y >= 0
                && x < v->document_width() && y < v->document_height()) {
                if (add_marker(*v, 0, x, y) < 0) {
                    ctx.refused(QObject::tr("Color Sampler: all four samplers are placed."));
                } else {
                    ctx.refreshAnnotations();
                }
            }
            return true;
        }
        background_ = mods.testFlag(Qt::AltModifier);
        dragging_ = true;
        // A fresh press always samples, even at the pixel of the last gesture.
        lastSample_ = QPoint(-1, -1);
        sample(ctx, imagePos);
        return true;
    }

    // A held drag samples continuously, as CS6's Eyedropper does ("press the
    // mouse button over the image, drag to the target, release").
    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (dragging_) {
            sample(ctx, imagePos);
        }
    }

    void onRelease(ToolServices&, const QPointF&, Qt::KeyboardModifiers) override
    {
        dragging_ = false;
    }

    void onDeactivate(ToolServices&) override { dragging_ = false; }
    void onSessionCancelled(ToolServices&) override { dragging_ = false; }

    // The Eyedropper canvas menu: the documented Sample Size ladder and the
    // Sample source list, both writing through the same options group the
    // options bar reads.
    bool hasCanvasContextMenu() const override { return true; }
    QMenu* buildContextMenu(ToolServices& ctx, const QPointF& imagePos) override
    {
        (void)imagePos;
        ImageView* canvas = ctx.canvas();
        if (!canvas) {
            return nullptr;
        }
        auto* menu = new QMenu(canvas);
        const EyedropperOptions current = options_->get();

        QMenu* sizeMenu = menu->addMenu(QObject::tr("Sample Size"));
        static const int kSizes[] = {1, 3, 5, 11, 31, 51, 101};
        for (int size : kSizes) {
            const QString label = size == 1
                ? QCoreApplication::translate("PicturaUi", "Point Sample")
                : QCoreApplication::translate("PicturaUi", "%1 by %1 Average").arg(size);
            QAction* action = sizeMenu->addAction(label);
            action->setCheckable(true);
            action->setChecked(size == current.sampleSize);
            QObject::connect(action, &QAction::triggered, menu, [this, size]() {
                EyedropperOptions o = options_->get();
                o.sampleSize = size;
                options_->set(o);
            });
        }

        QMenu* sampleMenu = menu->addMenu(QObject::tr("Sample"));
        static const QPair<int, const char*> kScopes[] = {
            {0, QT_TRANSLATE_NOOP("PicturaUi", "Current Layer")},
            {1, QT_TRANSLATE_NOOP("PicturaUi", "Current & Below")},
            {2, QT_TRANSLATE_NOOP("PicturaUi", "All Layers")},
            {3, QT_TRANSLATE_NOOP("PicturaUi", "All Layers No Adjustments")},
            {4, QT_TRANSLATE_NOOP("PicturaUi", "Current & Below No Adjustments")}};
        for (const auto& scope : kScopes) {
            QAction* action = sampleMenu->addAction(QCoreApplication::translate("PicturaUi", scope.second));
            action->setCheckable(true);
            action->setChecked(scope.first == current.scope);
            QObject::connect(action, &QAction::triggered, menu, [this, id = scope.first]() {
                EyedropperOptions o = options_->get();
                o.scope = id;
                options_->set(o);
            });
        }
        return menu;
    }

private:
    void sample(ToolServices& ctx, const QPointF& imagePos)
    {
        PictureView* v = ctx.view();
        if (!v) {
            return;
        }
        // Skip a move that lands on the pixel last sampled, so a jitter within
        // one pixel does not recomposite. ponytail: a scope-aware sample still
        // runs the compositor per distinct pixel; reading the maintained
        // composite for every scope is a later optimisation.
        const QPoint point(qRound(imagePos.x()), qRound(imagePos.y()));
        if (point == lastSample_) {
            return;
        }
        lastSample_ = point;
        const EyedropperOptions options = options_->get();
        const quint32 argb = sample_argb_scoped(*v, point.x(), point.y(), options.sampleSize,
                                                options.scope);
        if (argb != 0) {
            if (background_) {
                ctx.sampledBackground(QColor::fromRgb(argb));
            } else {
                ctx.sampledForeground(QColor::fromRgb(argb));
            }
        }
    }

    std::shared_ptr<EyedropperGroup> options_;
    bool dragging_ = false;
    bool background_ = false;
    QPoint lastSample_{-1, -1};
};

} // namespace

std::unique_ptr<ToolHandler> makeEyedropperToolHandler(std::shared_ptr<EyedropperGroup> options)
{
    return std::make_unique<EyedropperToolHandler>(std::move(options));
}

} // namespace pictura
