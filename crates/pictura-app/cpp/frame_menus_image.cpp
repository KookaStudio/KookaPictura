#include "frame_includes.h"

#include "canvas_size_dialog.h"
#include "image_size_dialog.h"

#include "pictura_app/src/cxxqt_object/image_adjust/image_ops.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_size.cxxqt.h"

#include <QtWidgets/QInputDialog>

namespace pictura {

// Image > Crop, Trim, Duplicate, Image Size, and Canvas Size (Image > Mode is frame_menus_image_mode.cpp).
// Lifted out of registerHandlers() to keep frame_menus.cpp within its size
// budget; the handler bodies are unchanged.
void PicturaMainWindow::wireImageMenu()
{
    wireImageModeMenu();

    registry_->setHandler(command_ids::ImageCrop, [this]() {
        if (tools_ && tools_->hasPendingCrop()) {
            commitCrop();
            return;
        }
        PictureView* view = activeView();
        if (!view) {
            return;
        }
        const QString bounds = view->selection_bounds();
        if (bounds.isEmpty()) {
            return;
        }
        const QStringList parts = bounds.split(QLatin1Char(' '), Qt::SkipEmptyParts);
        if (parts.size() != 4) {
            return;
        }
        if (view->crop(parts.at(0).toInt(), parts.at(1).toInt(), parts.at(2).toInt(),
                       parts.at(3).toInt())) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::ImageCrop,
                                  [this]() { return activeView() && activeView()->has_document(); });

    registry_->setHandler(command_ids::ImageTrim, [this]() {
        PictureView* view = activeView();
        if (view && trim_image(*view)) {
            refresh();
        }
    });
    registry_->setHandler(command_ids::ImageDuplicate, [this]() {
        PictureView* source = activeView();
        if (!source || !source->has_document()) {
            return;
        }
        const QString suggested = duplicate_name(*source);
        DuplicateImageDialog dialog(activeDocumentName(), suggested, this);
        if (runDialog(dialog, this) != QDialog::Accepted) {
            return;
        }
        auto* target = new PictureView(this);
        if (!duplicate_into(*source, *target, dialog.copyName(), dialog.mergedOnly())) {
            delete target;
            return;
        }
        const int index = addDocument(target, QString());
        if (index >= 0) {
            docs_[index].displayName = dialog.copyName();
            updateTabTitle(index);
        }
    });
    const auto imageEnabled = [this]() {
        return activeView() && activeView()->has_document();
    };
    for (const char* id : {command_ids::ImageTrim, command_ids::ImageDuplicate}) {
        registry_->setEnabledProvider(id, imageEnabled);
    }

    const QString imageSize =
        commandIdForPath({QStringLiteral("Image"), QStringLiteral("Image Size…")});
    registry_->setImplemented(imageSize, true);
    registry_->setEnabledProvider(imageSize, imageEnabled);
    registry_->setHandler(imageSize, [this]() {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return;
        }
        ImageSizeDialog dialog(view, this);
        if (runDialog(dialog, this) == QDialog::Accepted
            && image_size_apply(*view, dialog.resampleKind(), dialog.resultWidth(),
                                dialog.resultHeight(), dialog.resultPpi(), dialog.resultPerCm())) {
            refresh();
        }
    });

    const QString canvasSize =
        commandIdForPath({QStringLiteral("Image"), QStringLiteral("Canvas Size…")});
    registry_->setImplemented(canvasSize, true);
    registry_->setEnabledProvider(canvasSize, imageEnabled);
    registry_->setHandler(canvasSize, [this]() {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return;
        }
        CanvasSizeDialog dialog(view, tools_->foreground(), tools_->background(), this);
        if (runDialog(dialog, this) == QDialog::Accepted
            && canvas_size_apply(*view, dialog.anchorName(), dialog.resultWidth(),
                                 dialog.resultHeight(), dialog.extensionColor().rgb() & 0xffffffu)) {
            refresh();
        }
    });

    registry_->setHandler(command_ids::ImageRotateArbitrary, [this]() {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return;
        }
        bool ok = false;
        const double angle = QInputDialog::getDouble(this, tr("Rotate Image"),
                                                     tr("Angle (degrees):"), 0.0, -359.99, 359.99, 2,
                                                     &ok);
        if (!ok || angle == 0.0) {
            return;
        }
        if (rotate_document_arbitrary(*view, angle)) {
            refresh();
        }
    });
    registry_->setEnabledProvider(command_ids::ImageRotateArbitrary, imageEnabled);
}

} // namespace pictura
