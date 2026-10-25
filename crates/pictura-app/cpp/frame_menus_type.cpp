// Type > Convert To Point Text / Convert To Paragraph Text and Type > Warp
// Text…, over the Layers panel's current layer; Type > Replace All Missing
// Fonts and the missing-font alert on open; applying Blend Text Colors Using
// Gamma.

#include "frame_includes.h"

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

void PicturaMainWindow::wireTypeMenu()
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
    wireReplaceMissingFonts();
}

// The dialog is window-modal but not blocking: each change previews on the
// canvas, OK commits one "Warp Text" state, and Cancel puts the layer back.
void PicturaMainWindow::wireWarpText()
{
    const auto target = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        return view && !path.isEmpty() && type_api::readLayer(*view, path).isType ? path : QString();
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
void PicturaMainWindow::wireReplaceMissingFonts()
{
    const auto missing = [this]() -> QStringList {
        PictureView* view = activeView();
        return view ? missingFonts(type_api::documentFonts(*view)) : QStringList();
    };
    const QString id = commandIdForPath(
        {QStringLiteral("Type"), QStringLiteral("Replace All Missing Fonts")});
    registry_->setImplemented(id, true);
    // Type is only re-set in 8-bit RGB, so elsewhere the command stays off and
    // the open-time alert says why.
    registry_->setEnabledProvider(id, [this, missing]() {
        PictureView* view = activeView();
        return view && type_api::layersEditable(*view) && !missing().isEmpty();
    });
    registry_->setHandler(id, [this, missing]() {
        PictureView* view = activeView();
        const QStringList names = missing();
        if (view && !names.isEmpty() && type_api::replaceFonts(*view, names) > 0) {
            refresh();
        }
    });
}

void PicturaMainWindow::alertMissingFonts(PictureView* view)
{
    const QStringList missing = view ? missingFonts(type_api::documentFonts(*view)) : QStringList();
    if (missing.isEmpty()) {
        return;
    }
    const QString remedy = type_api::layersEditable(*view)
        ? tr("Type > Replace All Missing Fonts sets them in the default font.")
        : tr("Type can only be re-set in an 8-bit RGB document, so convert it to 8-bit RGB "
             "before replacing them.");
    auto* box = new QMessageBox(
        QMessageBox::Warning, tr("Missing Fonts"),
        tr("This document's type layers use fonts that are not installed. They keep their "
           "current pixels until edited. %1\n\n%2")
            .arg(remedy, missing.join(QLatin1Char('\n'))),
        QMessageBox::Ok, this);
    box->setObjectName(QStringLiteral("missingFontsAlert"));
    box->setAttribute(Qt::WA_DeleteOnClose);
    box->open();
}

void PicturaMainWindow::applyTextGamma(bool enabled, double gamma)
{
    const bool changed = enabled != textGammaEnabled_ || !qFuzzyCompare(gamma, textGamma_);
    textGammaEnabled_ = enabled;
    textGamma_ = gamma;
    type_api::setTextGamma(enabled, gamma);
    if (!changed) {
        return;
    }
    for (const DocEntry* entry : docs_) {
        if (entry->view) {
            type_api::refreshTextGamma(*entry->view);
        }
    }
}

} // namespace pictura
