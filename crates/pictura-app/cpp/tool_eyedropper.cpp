#include "tool_handler.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/eyedropper_sample.cxxqt.h"

#include <QtCore/QtGlobal>
#include <QtGui/QColor>

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

private:
    std::shared_ptr<EyedropperGroup> options_;
};

} // namespace

std::unique_ptr<ToolHandler> makeEyedropperToolHandler(std::shared_ptr<EyedropperGroup> options)
{
    return std::make_unique<EyedropperToolHandler>(std::move(options));
}

} // namespace pictura
