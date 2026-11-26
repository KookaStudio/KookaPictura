
#include "menu_registrar.h"

#include "commands.h"
#include <QtWidgets/QPushButton>

#include "menu_registrar_internal.h"

#include "document_registry.h"
#include "panel_host.h"
#include "tools.h"

#include <QtCore/QScopeGuard>

#include <utility>

#include "bitmap_mode_dialog.h"
#include "document_command.h"
#include "hdr_conversion_dialog.h"
#include "indexed_color_dialog.h"
#include "api/document_bridge.h"
#include "api/filters_bridge.h"

namespace pictura {

namespace {

QString depthCommandId(int bits)
{
    switch (bits) {
    case 8:
        return QString::fromLatin1(command_ids::ImageMode8Bits);
    case 16:
        return QString::fromLatin1(command_ids::ImageMode16Bits);
    case 32:
        return QString::fromLatin1(command_ids::ImageMode32Bits);
    default:
        return QString();
    }
}

} // namespace

// Image > Mode. Each entry is checked for the document's mode / depth and
// enabled when the engine can convert to it (the IMG-004 matrix), the checked
// entry staying enabled as a no-op. Ported from photorust's Mode submenu.
void MenuRegistrar::wireImageModeMenu()
{
    const std::pair<const char*, const char*> modes[] = {
        {command_ids::ImageModeBitmap, "bitmap"}, {command_ids::ImageModeGrayscale, "grayscale"},
        {command_ids::ImageModeIndexed, "indexed"}, {command_ids::ImageModeRgb, "rgb"},
        {command_ids::ImageModeCmyk, "cmyk"},      {command_ids::ImageModeLab, "lab"},
        {command_ids::ImageModeMultichannel, "multichannel"},
    };
    for (const auto& [id, name] : modes) {
        const QString mode = QString::fromLatin1(name);
        commands_->setHandler(id, [this, mode]() { convertImageMode(mode); });
        commands_->setEnabledProvider(id, [this, mode]() {
            PictureView* view = activeView();
            return view && view->has_document()
                   && (image_mode(*view) == mode || image_mode_available(*view, mode));
        });
        commands_->setCheckedProvider(id, [this, mode]() {
            PictureView* view = activeView();
            return view && view->has_document() && image_mode(*view) == mode;
        });
        commands_->setExecutor(id, [this, mode](const CommandParams& params) {
            PictureView* view = activeView();
            if (!view || !view->has_document() || !image_mode_available(*view, mode)) {
                return false;
            }
            bool applied = false;
            if (mode == QLatin1String("bitmap")) {
                applied = convert_to_bitmap(
                    *view, params.values.value(QStringLiteral("method")).toInt());
            } else if (mode == QLatin1String("indexed")) {
                applied = convert_to_indexed(
                    *view, params.values.value(QStringLiteral("palette")).toInt(4),
                    params.values.value(QStringLiteral("colors")).toInt(256),
                    params.values.value(QStringLiteral("dither")).toInt(),
                    params.values.value(QStringLiteral("amount")).toInt());
            } else {
                if (mode != QLatin1String("multichannel")
                    && params.values.value(QStringLiteral("flatten")).toBool()
                    && view->layer_count() > 1) {
                    view->flatten_image();
                }
                applied = convert_image_mode(*view, mode);
            }
            if (applied) {
                refresh();
            }
            return applied;
        });
    }

    const std::pair<const char*, int> depths[] = {
        {command_ids::ImageMode8Bits, 8},
        {command_ids::ImageMode16Bits, 16},
        {command_ids::ImageMode32Bits, 32},
    };
    for (const auto& [id, bits] : depths) {
        commands_->setHandler(id, [this, bits]() { convertImageDepth(bits); });
        commands_->setEnabledProvider(id, [this, bits]() {
            PictureView* view = activeView();
            return view && view->has_document()
                   && (image_depth_bits(*view) == bits || image_depth_available(*view, bits));
        });
        commands_->setCheckedProvider(id, [this, bits]() {
            PictureView* view = activeView();
            return view && view->has_document() && image_depth_bits(*view) == bits;
        });
        commands_->setExecutor(id, [this, bits](const CommandParams& params) {
            PictureView* view = activeView();
            if (!view || !view->has_document() || !image_depth_available(*view, bits)) {
                return false;
            }
            bool applied = false;
            if (image_depth_bits(*view) == 32) {
                applied = view->convert_depth(
                    bits, params.values.value(QStringLiteral("exposure")).toDouble(0.0),
                    params.values.value(QStringLiteral("gamma")).toDouble(1.0));
            } else {
                applied = convert_image_depth(*view, bits);
            }
            if (applied) {
                refresh();
            }
            return applied;
        });
    }
}

void MenuRegistrar::convertImageMode(const QString& mode)
{
    PictureView* view = activeView();
    // A checkable action toggles itself; every exit re-syncs the check marks.
    const auto done = qScopeGuard([this]() { refresh(); });
    if (!view || image_mode(*view) == mode || !image_mode_available(*view, mode)) {
        return;
    }
    const bool layered = view->layer_count() > 1;

    // These modes require flattening in the conversion transaction.
    const QString title = tr("Convert Mode");
    if (mode == QLatin1String("bitmap") || mode == QLatin1String("indexed")
        || mode == QLatin1String("multichannel")) {
        if (layered) {
            QMessageBox box(QMessageBox::Question, tr("Kooka Pictura"), tr("Flatten layers?"),
                            QMessageBox::Ok | QMessageBox::Cancel, dialogParent());
            runDialog(box, dialogParent());
            if (box.clickedButton() != box.button(QMessageBox::Ok)) {
                return;
            }
        }
        if (mode == QLatin1String("multichannel")) {
            if (runDocumentCommand(dialogParent(), *view, title,
                                   [view, &mode] { convert_image_mode(*view, mode); })) {
                CommandParams command;
                command.values.insert(QStringLiteral("mode"), mode);
                commands_->recordCommand(command_ids::ImageModeMultichannel, command);
            }
            return;
        }
        if (mode == QLatin1String("bitmap")) {
            BitmapModeDialog dialog(dialogParent());
            if (runDialog(dialog, dialogParent()) == QDialog::Accepted
                && runDocumentCommand(dialogParent(), *view, title, [view, &dialog] {
                       convert_to_bitmap(*view, dialog.method());
                   })) {
                CommandParams command;
                command.values.insert(QStringLiteral("mode"), QStringLiteral("bitmap"));
                command.values.insert(QStringLiteral("method"), dialog.method());
                commands_->recordCommand(command_ids::ImageModeBitmap, command);
            }
            return;
        }
        IndexedColorDialog dialog(indexed_exact_available(*view), dialogParent());
        const auto preview = [view, &dialog]() {
            if (dialog.preview()) {
                preview_indexed(*view, dialog.palette(), dialog.colors(), dialog.dither(),
                                dialog.amount());
            } else {
                cancel_mode_preview(*view);
            }
        };
        connect(&dialog, &IndexedColorDialog::optionsChanged, &dialog, preview);
        preview();
        if (runDialog(dialog, dialogParent()) == QDialog::Accepted) {
            const auto convert = [view, &dialog] {
                convert_to_indexed(*view, dialog.palette(), dialog.colors(), dialog.dither(),
                                   dialog.amount());
            };
            if (runDocumentCommand(dialogParent(), *view, title, convert)) {
                CommandParams command;
                command.values.insert(QStringLiteral("mode"), QStringLiteral("indexed"));
                command.values.insert(QStringLiteral("palette"), dialog.palette());
                command.values.insert(QStringLiteral("colors"), dialog.colors());
                command.values.insert(QStringLiteral("dither"), dialog.dither());
                command.values.insert(QStringLiteral("amount"), dialog.amount());
                commands_->recordCommand(command_ids::ImageModeIndexed, command);
            }
        } else {
            cancel_mode_preview(*view);
        }
        return;
    }

    const QString from = image_mode(*view);
    if (mode == QLatin1String("grayscale") && from != QLatin1String("bitmap")) {
        QMessageBox box(QMessageBox::Information, tr("Kooka Pictura"),
                        tr("Discard color information?"), QMessageBox::Cancel, dialogParent());
        box.setInformativeText(
            tr("To control the conversion, use Image > Adjustments > Black & White."));
        QPushButton* discard = box.addButton(tr("Discard"), QMessageBox::AcceptRole);
        box.setDefaultButton(discard);
        runDialog(box, dialogParent());
        if (box.clickedButton() != discard) {
            return;
        }
    }
    bool flatten = false;
    if (layered && mode != QLatin1String("rgb")) {
        QMessageBox box(QMessageBox::Warning, tr("Kooka Pictura"),
                        tr("Changing modes can affect the appearance of layers. Flatten image "
                           "before mode change?"),
                        QMessageBox::Cancel, dialogParent());
        QPushButton* flattenButton = box.addButton(tr("Flatten"), QMessageBox::AcceptRole);
        QPushButton* keep = box.addButton(tr("Don't Flatten"), QMessageBox::DestructiveRole);
        box.setDefaultButton(flattenButton);
        runDialog(box, dialogParent());
        if (box.clickedButton() == flattenButton) {
            if (!runDocumentCommand(dialogParent(), *view, tr("Flatten Image"),
                                    [view] { view->flatten_image(); })) {
                return;
            }
            flatten = true;
        } else if (box.clickedButton() != keep) {
            return;
        }
    }
    if (runDocumentCommand(dialogParent(), *view, title,
                           [view, &mode] { convert_image_mode(*view, mode); })) {
        const char* id = mode == QLatin1String("grayscale") ? command_ids::ImageModeGrayscale
                        : mode == QLatin1String("rgb")       ? command_ids::ImageModeRgb
                        : mode == QLatin1String("cmyk")      ? command_ids::ImageModeCmyk
                        : mode == QLatin1String("lab")       ? command_ids::ImageModeLab
                                                             : nullptr;
        if (id) {
            CommandParams command;
            command.values.insert(QStringLiteral("mode"), mode);
            command.values.insert(QStringLiteral("flatten"), flatten);
            commands_->recordCommand(QLatin1String(id), command);
        }
    }
}

void MenuRegistrar::convertImageDepth(int bits)
{
    PictureView* view = activeView();
    const auto done = qScopeGuard([this]() { refresh(); });
    if (!view || image_depth_bits(*view) == bits || !image_depth_available(*view, bits)) {
        return;
    }
    // Leaving 32-bit compresses the HDR range through the tone map.
    if (image_depth_bits(*view) == 32) {
        HdrConversionSpec spec;
        const auto convert = [view, bits, &spec] {
            convert_depth_method(*view, bits, int(spec.method), spec.exposure_ev, spec.gamma,
                                 spec.radius, spec.strength, spec.local_gamma, spec.local_exposure,
                                 spec.detail, spec.shadow, spec.highlight, spec.vibrance,
                                 spec.saturation);
        };
        if (HdrConversionDialog::get(dialogParent(), &spec)
            && runDocumentCommand(dialogParent(), *view, tr("HDR Toning"), convert)) {
            CommandParams command;
            command.values.insert(QStringLiteral("depth"), bits);
            command.values.insert(QStringLiteral("exposure"), spec.exposure_ev);
            command.values.insert(QStringLiteral("gamma"), spec.gamma);
            commands_->recordCommand(depthCommandId(bits), command);
        }
        return;
    }
    if (runDocumentCommand(dialogParent(), *view, tr("Convert Mode"),
                           [view, bits] { convert_image_depth(*view, bits); })) {
        CommandParams command;
        command.values.insert(QStringLiteral("depth"), bits);
        commands_->recordCommand(depthCommandId(bits), command);
    }
}

} // namespace pictura
