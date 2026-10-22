// Type > Convert To Point Text / Convert To Paragraph Text and Type > Warp
// Text…, over the Layers panel's current layer.

#include "frame_includes.h"

#include "api/type_api.h"
#include "warp_text_dialog.h"

#include <QtCore/QPointer>
#include <QtWidgets/QMessageBox>

namespace pictura {

namespace {

QString warpRefusalText(type_api::WarpRefusal refusal)
{
    const QString lead
        = QObject::tr("Could not complete the Warp Text command because the type ");
    switch (refusal) {
    case type_api::WarpRefusal::OnPath:
        return lead + QObject::tr("is on a path.");
    case type_api::WarpRefusal::FauxBold:
        return lead + QObject::tr("layer uses a faux bold style.");
    case type_api::WarpRefusal::NoOutlines:
        return lead + QObject::tr("layer uses a font that has no outline data.");
    default:
        return lead + QObject::tr("layer cannot be warped.");
    }
}

} // namespace

void PicturaMainWindow::wireTypeMenu()
{
    // The current type layer when it is paragraph type (`paragraph`) or point
    // type, else empty.
    const auto convertible = [this](bool paragraph) -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
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
            if (view && !path.isEmpty()) {
                type_api::convert(*view, path, toParagraph);
            }
        });
        registry_->setEnabledProvider(id, [convertible, toParagraph]() {
            return !convertible(!toParagraph).isEmpty();
        });
    }
    wireWarpText();
}

// The dialog is window-modal but not blocking: each change previews on the
// canvas, OK commits one "Warp Text" state, and Cancel puts the layer back.
void PicturaMainWindow::wireWarpText()
{
    const auto target = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        return view && !path.isEmpty() && type_api::isType(*view, path) ? path : QString();
    };
    const QString id
        = commandIdForPath({QStringLiteral("Type"), QStringLiteral("Warp Text…")});
    registry_->setImplemented(id, true);
    registry_->setEnabledProvider(id, [target]() { return !target().isEmpty(); });
    registry_->setHandler(id, [this, target]() {
        PictureView* view = activeView();
        const QString path = target();
        if (!view || path.isEmpty()) {
            return;
        }
        const type_api::WarpRefusal refusal = type_api::warpRefusal(*view, path);
        if (refusal != type_api::WarpRefusal::None) {
            auto* box = new QMessageBox(QMessageBox::Warning, tr("Warp Text"),
                                        warpRefusalText(refusal), QMessageBox::Ok, this);
            box->setObjectName(QStringLiteral("warpTextRefusal"));
            box->setAttribute(Qt::WA_DeleteOnClose);
            box->open();
            return;
        }
        auto* dialog = new WarpTextDialog(type_api::layerWarp(*view, path), this);
        dialog->setAttribute(Qt::WA_DeleteOnClose);
        const QPointer<PictureView> guard(view);
        connect(dialog, &WarpTextDialog::changed, this, [this, dialog, guard, path]() {
            if (guard && type_api::previewWarp(*guard, path, dialog->setting())) {
                refresh();
            }
        });
        connect(dialog, &QDialog::accepted, this, [this, dialog, guard, path]() {
            if (guard && type_api::commitWarp(*guard, path, dialog->setting())) {
                refresh();
            }
        });
        connect(dialog, &QDialog::rejected, this, [this, guard]() {
            if (guard && type_api::cancelWarp(*guard)) {
                refresh();
            }
        });
        dialog->open();
    });
}

} // namespace pictura
