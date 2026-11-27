
#include "menu_registrar.h"

#include "commands.h"
#include <QtWidgets/QPushButton>

#include "menu_registrar_internal.h"

#include "document_registry.h"
#include "panel_host.h"
#include "tools.h"

#include <QtCore/QScopeGuard>

#include <limits>
#include <utility>

#include "bitmap_mode_dialog.h"
#include "color_table_dialog.h"
#include "document_command.h"
#include "hdr_conversion_dialog.h"
#include "indexed_color_dialog.h"
#include "api/document_bridge.h"
#include "api/filters_bridge.h"

namespace pictura {

namespace {

bool applyBitmap(PictureView& view, const QJsonObject& values)
{
    const int method = values.value(QStringLiteral("method")).toInt();
    if (method == 3) {
        const auto unit = values.value(QStringLiteral("perCm"));
        if (!unit.isBool()) {
            return false;
        }
        const double missing = std::numeric_limits<double>::quiet_NaN();
        return convert_to_bitmap_halftone(view, values.value(QStringLiteral("frequency")).toDouble(missing),
                                          unit.toBool(), values.value(QStringLiteral("angle")).toDouble(missing),
                                          values.value(QStringLiteral("shape")).toInt(-1));
    }
    if (method == 4) {
        const auto encoded = values.value(QStringLiteral("patternNative"));
        const auto native = QByteArray::fromBase64(encoded.toString().toLatin1(),
                                                    QByteArray::AbortOnBase64DecodingErrors);
        if (!encoded.isUndefined() && (!encoded.isString() || native.isEmpty())) {
            return false;
        }
        return convert_to_bitmap_pattern(view, values.value(QStringLiteral("patternId")).toString(), native);
    }
    return convert_to_bitmap(view, method);
}

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
        if (mode == QLatin1String("bitmap")) {
            commands_->setContextHandler(id, [this, mode](const CommandContext& context) {
                convertImageMode(mode, &context);
            });
        } else {
            commands_->setHandler(id, [this, mode]() { convertImageMode(mode); });
        }
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
                applied = applyBitmap(*view, params.values);
            } else if (mode == QLatin1String("indexed")) {
                applied = convert_to_indexed_with_options(
                    *view, params.values.value(QStringLiteral("palette")).toInt(4),
                    params.values.value(QStringLiteral("colors")).toInt(256),
                    params.values.value(QStringLiteral("dither")).toInt(),
                    params.values.value(QStringLiteral("amount")).toInt(),
                    params.values.value(QStringLiteral("transparency")).toBool(),
                    QByteArray::fromHex(
                        params.values.value(QStringLiteral("forced_rgb")).toString().toLatin1()));
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

    commands_->setEnabledProvider(command_ids::ImageModeColorTable, [this] {
        const auto* view = activeView();
        return view && indexed_color_table_minimum(*view) > 0;
    });
    commands_->setHandler(command_ids::ImageModeColorTable, [this] {
        QPointer<PictureView> view = activeView();
        if (!view) {
            return;
        }
        const QByteArray table = indexed_color_table(*view);
        if (table.isEmpty()) {
            return;
        }
        ColorTableDialog dialog(table, indexed_color_table_minimum(*view), dialogParent());
        if (runDialog(dialog, dialogParent()) == QDialog::Accepted && view
            && dialog.table() != table) {
            runDocumentCommand(dialogParent(), *view, tr("Color Table"),
                               [view, &dialog] { apply_indexed_color_table(*view, dialog.table()); });
            refresh();
        }
    });

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

void MenuRegistrar::convertImageMode(const QString& mode, const CommandContext* context)
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
            const auto initial = context ? context->params.values : QJsonObject{};
            BitmapModeDialog dialog(*view, initial, dialogParent());
            if (runDialog(dialog, dialogParent()) != QDialog::Accepted) {
                return;
            }
            const CommandParams command{dialog.options()};
            if (!runDocumentCommand(dialogParent(), *view, title, [view, &command] {
                    applyBitmap(*view, command.values);
                })) {
                return;
            }
            if (context && context->mode == CommandMode::Modal) {
                *context->outParams = command;
                *context->applied = true;
            } else {
                commands_->recordCommand(command_ids::ImageModeBitmap, command);
            }
            return;
        }
        QByteArray lastForced;
        bool lastTransparency = true;
        const bool exact = indexed_exact_available_with_options(*view, lastTransparency, lastForced);
        IndexedColorDialog dialog(exact, true, dialogParent());
        const auto preview = [view, &dialog, &lastForced, &lastTransparency]() {
            const auto forced = dialog.forcedColors();
            const bool transparency = dialog.transparency();
            if (forced != lastForced || transparency != lastTransparency) {
                dialog.setExactAvailable(
                    indexed_exact_available_with_options(*view, transparency, forced));
                lastForced = forced;
                lastTransparency = transparency;
            }
            if (!dialog.preview()
                || !preview_indexed_with_options(*view, dialog.palette(), dialog.colors(),
                    dialog.dither(), dialog.amount(), transparency, forced)) {
                cancel_mode_preview(*view);
            }
        };
        connect(&dialog, &IndexedColorDialog::optionsChanged, &dialog, preview);
        preview();
        if (runDialog(dialog, dialogParent()) == QDialog::Accepted) {
            const auto convert = [view, &dialog] {
                convert_to_indexed_with_options(*view, dialog.palette(), dialog.colors(),
                    dialog.dither(), dialog.amount(), dialog.transparency(), dialog.forcedColors());
            };
            if (runDocumentCommand(dialogParent(), *view, title, convert)) {
                CommandParams command;
                command.values.insert(QStringLiteral("mode"), QStringLiteral("indexed"));
                command.values.insert(QStringLiteral("palette"), dialog.palette());
                command.values.insert(QStringLiteral("colors"), dialog.colors());
                command.values.insert(QStringLiteral("dither"), dialog.dither());
                command.values.insert(QStringLiteral("amount"), dialog.amount());
                command.values.insert(QStringLiteral("transparency"), dialog.transparency());
                command.values.insert(QStringLiteral("forced_rgb"),
                                      QString::fromLatin1(dialog.forcedColors().toHex()));
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
