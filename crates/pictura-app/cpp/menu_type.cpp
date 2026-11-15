// Type > Convert To Point Text / Convert To Paragraph Text and Type > Warp
// Text…, over the Layers panel's current layer; Type > Replace All Missing
// Fonts and the missing-font alert on open; applying Blend Text Colors Using
// Gamma.


#include "menu_registrar.h"

#include "commands.h"

#include "menu_registrar_internal.h"

#include "document_registry.h"
#include "panel_host.h"
#include "tools.h"

#include "api/type_api.h"
#include "type_fonts.h"
#include "warp_text_dialog.h"

#include <QtWidgets/QMessageBox>

#include <memory>

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

void MenuRegistrar::wireTypeMenu()
{
    // The current type layer when it is paragraph type (`paragraph`) or point
    // type, else empty.
    const auto convertible = [this](bool paragraph) -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !type_api::readLayer(*view, path).isType
            || type_api::isParagraph(*view, path) != paragraph) {
            return QString();
        }
        return path;
    };
    for (const bool toParagraph : {false, true}) {
        const QString id = QString::fromLatin1(toParagraph
                                               ? command_ids::TypeConvertToParagraphText
                                               : command_ids::TypeConvertToPointText);
        commands_->setImplemented(id, true);
        commands_->setHandler(id, [this, convertible, toParagraph]() {
            PictureView* view = activeView();
            const QString path = convertible(!toParagraph);
            if (view && !path.isEmpty()) {
                type_api::convert(*view, path, toParagraph);
            }
        });
        commands_->setEnabledProvider(id, [convertible, toParagraph]() {
            return !convertible(!toParagraph).isEmpty();
        });
    }
    wireWarpText();
    wireReplaceMissingFonts();
}

// The dialog is window-modal but not blocking: each change previews on the
// canvas, OK commits one "Warp Text" state, and Cancel puts the layer back.
void MenuRegistrar::wireWarpText()
{
    const auto target = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        return view && !path.isEmpty() && type_api::readLayer(*view, path).isType ? path : QString();
    };
    const QString id = QString::fromLatin1(command_ids::TypeWarpText);
    commands_->setImplemented(id, true);
    commands_->setEnabledProvider(id, [target]() { return !target().isEmpty(); });
    commands_->setHandler(id, [this, target]() {
        PictureView* view = activeView();
        const QString path = target();
        if (!view || path.isEmpty()) {
            return;
        }
        const type_api::WarpRefusal refusal = type_api::warpRefusal(*view, path);
        if (refusal != type_api::WarpRefusal::None) {
            auto* box = new QMessageBox(QMessageBox::Warning, tr("Warp Text"),
                                        warpRefusalText(refusal), QMessageBox::Ok, dialogParent());
            box->setObjectName(QStringLiteral("warpTextRefusal"));
            box->setAttribute(Qt::WA_DeleteOnClose);
            box->open();
            return;
        }
        auto* dialog = new WarpTextDialog(type_api::layerWarp(*view, path), dialogParent());
        dialog->setAttribute(Qt::WA_DeleteOnClose);
        // The dialog's connections own the session: closing the dialog
        // destroys them, and an unfinished session cancels.
        const auto session = std::make_shared<type_api::WarpSession>(*view, path);
        connect(dialog, &WarpTextDialog::changed, dialog, [this, dialog, session]() {
            if (session->preview(dialog->setting())) {
                refresh();
            }
        });
        connect(dialog, &QDialog::accepted, dialog, [this, dialog, session]() {
            if (session->commit(dialog->setting())) {
                refresh();
            }
        });
        connect(dialog, &QDialog::rejected, dialog, [this, session]() {
            if (session->cancel()) {
                refresh();
            }
        });
        dialog->open();
    });
}

// CS6 substitutes its default font; ours is the Type tool's default, which the
// engine bundles. One "Replace All Missing Fonts" state.
void MenuRegistrar::wireReplaceMissingFonts()
{
    const auto missing = [this]() -> QStringList {
        PictureView* view = activeView();
        return view ? missingFonts(type_api::documentFonts(*view)) : QStringList();
    };
    const QString id = QString::fromLatin1(command_ids::TypeReplaceAllMissingFonts);
    commands_->setImplemented(id, true);
    // Type is only re-set in 8-bit RGB, so elsewhere the command stays off and
    // the open-time alert says why.
    commands_->setEnabledProvider(id, [this, missing]() {
        PictureView* view = activeView();
        return view && type_api::layersEditable(*view) && !missing().isEmpty();
    });
    commands_->setHandler(id, [this, missing]() {
        PictureView* view = activeView();
        const QStringList names = missing();
        if (view && !names.isEmpty() && type_api::replaceFonts(*view, names) > 0) {
            refresh();
        }
    });
}

} // namespace pictura
