// Type > Convert To Point Text / Convert To Paragraph Text, over the Layers
// panel's current layer.

#include "frame_includes.h"

#include "api/type_api.h"

namespace pictura {

void PicturaMainWindow::wireTypeMenu()
{
    // The current type layer when it is paragraph type (`paragraph`) or point
    // type, else empty.
    const auto convertible = [this](bool paragraph) -> QString {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || path.isEmpty() || !type_api::isType(*view, path)
            || type_api::isParagraph(*view, path) != paragraph) {
            return QString();
        }
        return path;
    };
    for (const bool toParagraph : {false, true}) {
        const QString id = commandIdForPath(
            {QStringLiteral("Type"), toParagraph ? QStringLiteral("Convert To Paragraph Text")
                                                 : QStringLiteral("Convert To Point Text")});
        registry_->setImplemented(id, true);
        registry_->setHandler(id, [this, convertible, toParagraph]() {
            PictureView* view = activeView();
            const QString path = convertible(!toParagraph);
            if (view && !path.isEmpty() && type_api::convert(*view, path, toParagraph)) {
                refresh();
            }
        });
        registry_->setEnabledProvider(id, [convertible, toParagraph]() {
            return !convertible(!toParagraph).isEmpty();
        });
    }
}

} // namespace pictura
