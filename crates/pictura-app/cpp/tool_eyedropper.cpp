#include "tool_handler.h"

#include "image_view.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/eyedropper_sample.cxxqt.h"

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

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        PictureView* v = ctx.view();
        if (!v) {
            return true;
        }
        const EyedropperOptions options = options_->get();
        const quint32 argb = sample_argb_scoped(*v, qRound(imagePos.x()), qRound(imagePos.y()),
                                                options.sampleSize, options.scope);
        if (argb != 0) {
            ctx.sampledForeground(QColor::fromRgb(argb));
        }
        return true;
    }

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
                ? QObject::tr("Point Sample")
                : QObject::tr("%1 by %1 Average").arg(size);
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
            {0, QT_TR_NOOP("Current Layer")},
            {1, QT_TR_NOOP("Current & Below")},
            {2, QT_TR_NOOP("All Layers")},
            {3, QT_TR_NOOP("All Layers No Adjustments")},
            {4, QT_TR_NOOP("Current & Below No Adjustments")}};
        for (const auto& scope : kScopes) {
            QAction* action = sampleMenu->addAction(QObject::tr(scope.second));
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
    std::shared_ptr<EyedropperGroup> options_;
};

} // namespace

std::unique_ptr<ToolHandler> makeEyedropperToolHandler(std::shared_ptr<EyedropperGroup> options)
{
    return std::make_unique<EyedropperToolHandler>(std::move(options));
}

} // namespace pictura
