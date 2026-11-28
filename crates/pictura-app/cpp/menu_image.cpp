
#include "menu_registrar.h"

#include "commands.h"
#include "crop_grip.h"
#include "preferences.h"
#include <QtCore/QLocale>
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
#include "calculations_dialog.h"
#include "canvas_size_dialog.h"
#include "memory_refusal.h"
#include "document_command.h"
#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/filters_bridge.h"

#include "image_size_dialog.h"


#include <QtGui/QDoubleValidator>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>

#include <cmath>

namespace pictura {

void MenuRegistrar::straightenRuler()
{
    QPointer<PictureView> view(activeView());
    if (!view || !view->has_document()) return;
    const auto line = ruler_line(*view);
    if (line.size() != 4) return;
    const double degrees = straightenAngle(QPointF(line[0], line[1]), QPointF(line[2], line[3]));
    if (!std::isfinite(degrees) || std::abs(degrees) < 1e-9) return;
    const QString interpolation = loadPreferences().imageInterpolation;
    if (runDocumentCommand(dialogParent(), *view, tr("Rotate Canvas"),
                           [&] { rotate_document_arbitrary_with(*view, degrees, interpolation); })) {
        if (view) clear_ruler(*view);
        refresh();
        CommandParams command;
        command.values.insert(QStringLiteral("angle"), degrees);
        command.values.insert(QStringLiteral("interpolation"), interpolation);
        commands_->recordCommand(command_ids::ImageRotateArbitrary, command);
    } else {
        reportCommitRefusal(dialogParent(), tr("rotate the canvas"));
    }
}

// Image > Crop, Trim, Duplicate, Image Size, and Canvas Size (Image > Mode is menu_image_mode.cpp).
// Lifted out of registerHandlers() to keep menu_registrar.cpp within its size
// budget; the handler bodies are unchanged.
void MenuRegistrar::wireImageMenu()
{
    wireImageModeMenu();
    wireVariablesMenu();
    wireMeasurementScale();

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
    commands_->setHandler(command_ids::ImageRevealAll, [this]() {
        PictureView* view = activeView();
        if (view) {
            reveal_all(*view);
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
    for (const char* id : {command_ids::ImageTrim, command_ids::ImageRevealAll,
                           command_ids::ImageDuplicate}) {
        commands_->setEnabledProvider(id, imageEnabled);
    }

    // Resampling keeps each window on the same relative point of the image, so
    // a shrunken image stays centred where the whole of it was.
    const auto applyImageSize = [this](PictureView* view, const QString& resample, int width,
                                       int height, double ppi, bool perCm, bool scaleStyles) {
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
        const auto apply = [view, &resample, width, height, ppi, perCm, scaleStyles] {
            image_size_apply_with_styles(*view, resample, width, height, ppi, perCm, scaleStyles);
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

    // Calculations blends two individual channels into a new document, a new
    // channel, or the selection. The dialog only gathers settings; the handler
    // owns the write, so Cancel cannot touch the document.
    const QString calculations = QString::fromLatin1(command_ids::ImageCalculations);
    commands_->setImplemented(calculations, true);
    commands_->setEnabledProvider(calculations, [this]() {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return false;
        }
        const QString mode = QString(channels_mode(*view));
        return mode != QLatin1String("indexed") && mode != QLatin1String("bitmap");
    });
    commands_->setHandler(calculations, [this]() {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return;
        }
        QList<CalculationsDialog::Source> documents;
        for (int i = 0; i < documentCount(); ++i) {
            if (viewAt(i)) {
                documents.append({documentName(i), viewAt(i)});
            }
        }
        CalculationsDialog dialog(view, documents, dialogParent());
        if (runDialog(dialog, dialogParent()) != QDialog::Accepted) {
            return;
        }
        if (dialog.result() == CalculationsDialog::Result::NewDocument) {
            auto* target = new PictureView(dialogParent());
            if (!dialog.apply(target)) {
                delete target;
                return;
            }
            documents_->addDocument(target, QString());
        } else if (!dialog.apply(view)) {
            return;
        }
        refresh();
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
                            dialog.resultHeight(), dialog.resultPpi(), dialog.resultPerCm(),
                            dialog.scaleStylesForResize())) {
            return;
        }
        CommandParams command;
        command.values.insert(QStringLiteral("width"), dialog.resultWidth());
        command.values.insert(QStringLiteral("height"), dialog.resultHeight());
        command.values.insert(QStringLiteral("resolution"), dialog.resultPpi());
        command.values.insert(QStringLiteral("perCm"), dialog.resultPerCm());
        command.values.insert(QStringLiteral("resample"), dialog.resampleKind());
        command.values.insert(QStringLiteral("scaleStyles"), dialog.scaleStylesForResize());
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
        const bool scaleStyles = constrain
            && params.values.value(QStringLiteral("scaleStyles")).toBool();
        return applyImageSize(view, kind, width, height, ppi, perCm, scaleStyles);
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

    const auto recordedInterpolation = [](const CommandParams& params) {
        return params.values.value(QStringLiteral("interpolation"))
            .toString(params.values.contains(QStringLiteral("interpolation"))
                          ? QString() : QStringLiteral("bilinear"));
    };
    commands_->setContextHandler(command_ids::ImageRotateArbitrary,
                                 [this, recordedInterpolation](const CommandContext& context) {
        const QPointer<PictureView> view(activeView());
        if (!view || !view->has_document()) {
            return;
        }
        const bool replay = context.mode == CommandMode::Modal;
        const QString interpolation = replay ? recordedInterpolation(context.params)
                                             : loadPreferences().imageInterpolation;
        double prefill = 0.0;
        if (replay) {
            prefill = context.params.values.value(QStringLiteral("angle")).toDouble();
        } else {
            const auto ruler = ruler_line(*view);
            if (ruler.size() == 4) {
                prefill = straightenAngle(QPointF(ruler[0], ruler[1]), QPointF(ruler[2], ruler[3]));
            }
        }
        if (!std::isfinite(prefill) || std::abs(prefill) > 359.99) {
            return;
        }

        QDialog dialog(dialogParent());
        dialog.setObjectName(QStringLiteral("rotateCanvasDialog"));
        dialog.setWindowTitle(tr("Rotate Canvas"));
        auto* layout = new QFormLayout(&dialog);
        auto* angle = new QLineEdit(&dialog);
        angle->setObjectName(QStringLiteral("rotateCanvasAngle"));
        auto* validator = new QDoubleValidator(-359.99, 359.99, 2, angle);
        validator->setNotation(QDoubleValidator::StandardNotation);
        QLocale locale = dialog.locale();
        locale.setNumberOptions(locale.numberOptions() | QLocale::RejectGroupSeparator);
        validator->setLocale(locale);
        angle->setValidator(validator);
        angle->setText(locale.toString(std::abs(prefill), 'f', 2));
        layout->addRow(tr("Angle (degrees)"), angle);

        auto* direction = new QWidget(&dialog);
        auto* choices = new QHBoxLayout(direction);
        choices->setContentsMargins(0, 0, 0, 0);
        auto* clockwise = new QRadioButton(tr("°CW"), direction);
        auto* counterclockwise = new QRadioButton(tr("°CCW"), direction);
        clockwise->setObjectName(QStringLiteral("rotateCanvasClockwise"));
        counterclockwise->setObjectName(QStringLiteral("rotateCanvasCounterclockwise"));
        choices->addWidget(clockwise);
        choices->addWidget(counterclockwise);
        (prefill < 0.0 ? counterclockwise : clockwise)->setChecked(true);
        layout->addRow(tr("Direction"), direction);

        auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
        auto* ok = buttons->button(QDialogButtonBox::Ok);
        ok->setDefault(true);
        connect(angle, &QLineEdit::textChanged, ok,
                [angle, ok] { ok->setEnabled(angle->hasAcceptableInput()); });
        connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
        connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
        layout->addRow(buttons);
        angle->selectAll();
        angle->setFocus();

        if (runDialog(dialog, dialogParent()) != QDialog::Accepted || !view
            || !angle->hasAcceptableInput()) {
            return;
        }
        bool parsed = false;
        const double magnitude = locale.toDouble(angle->text(), &parsed);
        const double degrees = clockwise->isChecked() ? magnitude : -magnitude;
        if (!parsed || degrees == 0.0) {
            return;
        }
        if (runDocumentCommand(dialogParent(), *view, tr("Rotate Canvas"),
                               [&] { rotate_document_arbitrary_with(*view, degrees, interpolation); })) {
            if (!replay && view && ruler_line(*view).size() == 4) {
                clear_ruler(*view);
                refresh();
            }
            CommandParams command;
            command.values.insert(QStringLiteral("angle"), degrees);
            command.values.insert(QStringLiteral("interpolation"), interpolation);
            if (replay) {
                *context.outParams = command;
                *context.applied = true;
            } else {
                commands_->recordCommand(command_ids::ImageRotateArbitrary, command);
            }
        } else {
            reportCommitRefusal(dialogParent(), tr("rotate the canvas"));
        }
    });
    commands_->setEnabledProvider(command_ids::ImageRotateArbitrary, imageEnabled);
    commands_->setExecutor(command_ids::ImageRotateArbitrary,
                           [this, recordedInterpolation](const CommandParams& params) {
                               PictureView* view = activeView();
                               const double angle =
                                   params.values.value(QStringLiteral("angle")).toDouble(0.0);
                               if (!view || !view->has_document() || angle == 0.0) {
                                   return false;
                               }
                               const bool applied =
                                   rotate_document_arbitrary_with(*view, angle, recordedInterpolation(params));
                               if (applied) {
                                   refresh();
                               }
                               return applied;
                           });
}

} // namespace pictura
