
#include "menu_registrar.h"

#include "commands.h"
#include <QtCore/QSizeF>
#include <QtCore/QRectF>
#include <QtCore/QPointF>

#include "menu_registrar_internal.h"

#include "document_registry.h"
#include "panel_host.h"
#include "tools.h"

#include "api/selection_api.h"
#include "duplicate_image_dialog.h"
#include "apply_image_dialog.h"
#include "canvas_size_dialog.h"
#include "memory_refusal.h"
#include "document_command.h"
#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "api/filters_bridge.h"

#include "image_size_dialog.h"


#include <QtWidgets/QInputDialog>

namespace pictura {

// Image > Crop, Trim, Duplicate, Image Size, and Canvas Size (Image > Mode is menu_image_mode.cpp).
// Lifted out of registerHandlers() to keep menu_registrar.cpp within its size
// budget; the handler bodies are unchanged.
void MenuRegistrar::wireImageMenu()
{
    wireImageModeMenu();
    wireVariablesMenu();

    commands_->setHandler(command_ids::ImageCrop, [this]() {
        if (tools_ && tools_->hasPendingCrop()) {
            if (CropCommitSession* session = tools_->cropCommitSession()) {
                if (session->commitCrop()) {
                    refresh();
                }
            }
            return;
        }
        PictureView* view = activeView();
        if (!view) {
            return;
        }
        const selection_api::SelectionBounds bounds = selection_api::bounds(*view);
        if (!bounds.valid) {
            return;
        }
        view->crop(bounds.x, bounds.y, bounds.width, bounds.height);
    });
    commands_->setEnabledProvider(command_ids::ImageCrop,
                                  [this]() { return activeView() && activeView()->has_document(); });

    commands_->setHandler(command_ids::ImageTrim, [this]() {
        PictureView* view = activeView();
        if (view) {
            trim_image(*view);
        }
    });
    commands_->setHandler(command_ids::ImageDuplicate, [this]() {
        PictureView* source = activeView();
        if (!source || !source->has_document()) {
            return;
        }
        const QString suggested = duplicate_name(*source);
        DuplicateImageDialog dialog(activeDocumentName(), suggested, dialogParent());
        if (runDialog(dialog, dialogParent()) != QDialog::Accepted) {
            return;
        }
        auto* target = new PictureView(dialogParent());
        if (!duplicate_into(*source, *target, dialog.copyName(), dialog.mergedOnly())) {
            delete target;
            return;
        }
        const int index = documents_->addDocument(target, QString());
        if (index >= 0) {
            documents_->docs()[index]->displayName = dialog.copyName();
            documents_->updateTabTitle(index);
        }
    });
    const auto imageEnabled = [this]() {
        return activeView() && activeView()->has_document();
    };
    for (const char* id : {command_ids::ImageTrim, command_ids::ImageDuplicate}) {
        commands_->setEnabledProvider(id, imageEnabled);
    }

    // Resampling keeps each window on the same relative point of the image, so
    // a shrunken image stays centred where the whole of it was.
    const auto applyImageSize = [this](PictureView* view, const QString& resample, int width,
                                       int height, double ppi, bool perCm) {
        const int index = documents_ ? documents_->indexOfView(view) : -1;
        const QSizeF before(view->document_width(), view->document_height());
        QList<QPair<QPointer<ImageView>, QPointF>> centres;
        if (index >= 0 && !before.isEmpty()) {
            documents_->forEachCanvas(*documents_->docs().at(index), [&centres, before](ImageView* canvas) {
                const QPointF centre = canvas->widgetToImage(QRectF(canvas->rect()).center());
                centres.append({canvas, QPointF(centre.x() / before.width(),
                                                centre.y() / before.height())});
            });
        }
        const auto apply = [view, &resample, width, height, ppi, perCm] {
            image_size_apply(*view, resample, width, height, ppi, perCm);
        };
        if (!runDocumentCommand(dialogParent(), *view, tr("Image Size"), apply)) {
            return false;
        }
        refresh();
        const QSizeF after(view->document_width(), view->document_height());
        for (const auto& [canvas, fraction] : centres) {
            if (canvas) {
                const QPointF target(fraction.x() * after.width(), fraction.y() * after.height());
                canvas->centerOnImageRect(QRectF(target - QPointF(0.5, 0.5), QSizeF(1.0, 1.0)));
            }
        }
        return true;
    };

    // Apply Image blends an open document of the same size into the active
    // layer or mask; Indexed and Bitmap documents have no blendable channels.
    const QString applyImage = QString::fromLatin1(command_ids::ImageApplyImage);
    commands_->setImplemented(applyImage, true);
    commands_->setEnabledProvider(applyImage, [this]() {
        PictureView* view = activeView();
        if (!view || !filter_target_ready(*view) || filter_target_smart(*view)) {
            return false;
        }
        const QString mode = QString(channels_mode(*view));
        return mode != QLatin1String("indexed") && mode != QLatin1String("bitmap");
    });
    commands_->setHandler(applyImage, [this]() {
        PictureView* view = activeView();
        if (!view || !filter_target_ready(*view)) {
            if (view) {
                reportFilterRefusal(view);
            }
            return;
        }
        QList<ApplyImageDialog::Source> documents;
        for (int i = 0; i < documentCount(); ++i) {
            if (viewAt(i)) {
                documents.append({documentName(i), viewAt(i)});
            }
        }
        ApplyImageDialog dialog(view, imageView(), documents, dialogParent());
        if (runDialog(dialog, dialogParent()) == QDialog::Accepted) {
            refresh();
        }
    });

    const QString imageSize = QString::fromLatin1(command_ids::ImageImageSize);
    commands_->setImplemented(imageSize, true);
    commands_->setEnabledProvider(imageSize, imageEnabled);
    commands_->setHandler(imageSize, [this, imageSize, applyImageSize]() {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return;
        }
        ImageSizeDialog dialog(view, dialogParent());
        if (!answerImageSizeDialog(dialog, dialogParent())) {
            return;
        }
        const bool resamples = !dialog.resampleKind().isEmpty()
            && (dialog.resultWidth() != view->document_width()
                || dialog.resultHeight() != view->document_height());
        if (resamples
            && !imageSizeFitsBudget(*view, dialog.resultWidth(), dialog.resultHeight())) {
            showImageSizeBudgetWarning(dialogParent(), dialog.resultWidth(), dialog.resultHeight());
            return;
        }
        if (!applyImageSize(view, dialog.resampleKind(), dialog.resultWidth(),
                            dialog.resultHeight(), dialog.resultPpi(), dialog.resultPerCm())) {
            return;
        }
        CommandParams command;
        command.values.insert(QStringLiteral("width"), dialog.resultWidth());
        command.values.insert(QStringLiteral("height"), dialog.resultHeight());
        command.values.insert(QStringLiteral("resolution"), dialog.resultPpi());
        command.values.insert(QStringLiteral("perCm"), dialog.resultPerCm());
        command.values.insert(QStringLiteral("resample"), dialog.resampleKind());
        commands_->recordCommand(imageSize, command);
    });
    commands_->setExecutor(imageSize, [this, applyImageSize](const CommandParams& params) {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return false;
        }
        const bool constrain =
            params.values.value(QStringLiteral("constrainProportions")).toBool(true);
        int width = params.values.value(QStringLiteral("width")).toInt();
        int height = params.values.value(QStringLiteral("height")).toInt();
        if (constrain && width > 0 && height <= 0) {
            height = qRound(double(view->document_height()) * width / view->document_width());
        } else if (constrain && height > 0 && width <= 0) {
            width = qRound(double(view->document_width()) * height / view->document_height());
        }
        double ppi = params.values.value(QStringLiteral("resolution")).toDouble(-1.0);
        bool perCm = params.values.value(QStringLiteral("perCm")).toBool();
        if (ppi <= 0.0) {
            const QString label = image_resolution(*view);
            ppi = label.section(QLatin1Char(' '), 0, 0).toDouble();
            perCm = label.contains(QStringLiteral("pixels/cm"));
            if (perCm) {
                ppi *= 2.54;
            }
        }
        const QString kind = params.values.value(QStringLiteral("resample")).toString();
        const bool resamples = !kind.isEmpty()
            && (width != view->document_width() || height != view->document_height());
        if (resamples && !imageSizeFitsBudget(*view, width, height)) {
            return false;
        }
        return applyImageSize(view, kind, width, height, ppi, perCm);
    });

    const QString canvasSize = QString::fromLatin1(command_ids::ImageCanvasSize);
    commands_->setImplemented(canvasSize, true);
    commands_->setEnabledProvider(canvasSize, imageEnabled);
    commands_->setHandler(canvasSize, [this, canvasSize]() {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return;
        }
        CanvasSizeDialog dialog(view, tools_->foreground(), tools_->background(), dialogParent());
        if (runDialog(dialog, dialogParent()) == QDialog::Accepted) {
            const uint fill = dialog.extensionColor().rgb() & 0xffffffu;
            if (!runDocumentCommand(dialogParent(), *view, tr("Canvas Size"), [&] {
                    canvas_size_apply(*view, dialog.anchorName(), dialog.resultWidth(),
                                      dialog.resultHeight(), fill);
                })) {
                reportCommitRefusal(dialogParent(), tr("resize the canvas"));
                return;
            }
            CommandParams command;
            command.values.insert(QStringLiteral("width"), dialog.resultWidth());
            command.values.insert(QStringLiteral("height"), dialog.resultHeight());
            command.values.insert(QStringLiteral("anchor"), dialog.anchorName());
            command.values.insert(QStringLiteral("color"), static_cast<double>(fill));
            commands_->recordCommand(canvasSize, command);
        }
    });
    commands_->setExecutor(canvasSize, [this](const CommandParams& params) {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return false;
        }
        const bool applied =
            canvas_size_apply(*view,
                              params.values.value(QStringLiteral("anchor"))
                                  .toString(QStringLiteral("center")),
                              params.values.value(QStringLiteral("width")).toInt(),
                              params.values.value(QStringLiteral("height")).toInt(),
                              uint32_t(params.values.value(QStringLiteral("color")).toDouble()));
        if (applied) {
            refresh();
        }
        return applied;
    });

    commands_->setHandler(command_ids::ImageRotateArbitrary, [this]() {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return;
        }
        bool ok = false;
        const double angle = QInputDialog::getDouble(dialogParent(), tr("Rotate Image"),
                                                     tr("Angle (degrees):"), 0.0, -359.99, 359.99, 2,
                                                     &ok);
        if (!ok || angle == 0.0) {
            return;
        }
        // A large document's rotation runs as a job behind a progress window (#1294).
        if (runDocumentCommand(dialogParent(), *view, tr("Rotate Canvas"),
                               [&] { rotate_document_arbitrary(*view, angle); })) {
            CommandParams command;
            command.values.insert(QStringLiteral("angle"), angle);
            commands_->recordCommand(command_ids::ImageRotateArbitrary, command);
        } else {
            reportCommitRefusal(dialogParent(), tr("rotate the canvas"));
        }
    });
    commands_->setEnabledProvider(command_ids::ImageRotateArbitrary, imageEnabled);
    commands_->setExecutor(command_ids::ImageRotateArbitrary,
                           [this](const CommandParams& params) {
                               PictureView* view = activeView();
                               const double angle =
                                   params.values.value(QStringLiteral("angle")).toDouble(0.0);
                               if (!view || !view->has_document() || angle == 0.0) {
                                   return false;
                               }
                               const bool applied = rotate_document_arbitrary(*view, angle);
                               if (applied) {
                                   refresh();
                               }
                               return applied;
                           });
}

} // namespace pictura
