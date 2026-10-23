#include "frame_includes.h"

#include <QtCore/QScopeGuard>

#include "bitmap_mode_dialog.h"
#include "indexed_color_dialog.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"
#include "pictura_app/src/cxxqt_object/hdr_conversion.cxxqt.h"

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
void PicturaMainWindow::wireImageModeMenu()
{
    const std::pair<const char*, const char*> modes[] = {
        {command_ids::ImageModeBitmap, "bitmap"}, {command_ids::ImageModeGrayscale, "grayscale"},
        {command_ids::ImageModeIndexed, "indexed"}, {command_ids::ImageModeRgb, "rgb"},
        {command_ids::ImageModeCmyk, "cmyk"},      {command_ids::ImageModeLab, "lab"},
    };
    for (const auto& [id, name] : modes) {
        const QString mode = QString::fromLatin1(name);
        registry_->setHandler(id, [this, mode]() { convertImageMode(mode); });
        registry_->setEnabledProvider(id, [this, mode]() {
            PictureView* view = activeView();
            return view && view->has_document()
                   && (image_mode(*view) == mode || image_mode_available(*view, mode));
        });
        registry_->setCheckedProvider(id, [this, mode]() {
            PictureView* view = activeView();
            return view && view->has_document() && image_mode(*view) == mode;
        });
        registry_->setExecutor(id, [this, mode](const CommandParams& params) {
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
                if (params.values.value(QStringLiteral("flatten")).toBool()
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
        registry_->setHandler(id, [this, bits]() { convertImageDepth(bits); });
        registry_->setEnabledProvider(id, [this, bits]() {
            PictureView* view = activeView();
            return view && view->has_document()
                   && (image_depth_bits(*view) == bits || image_depth_available(*view, bits));
        });
        registry_->setCheckedProvider(id, [this, bits]() {
            PictureView* view = activeView();
            return view && view->has_document() && image_depth_bits(*view) == bits;
        });
        registry_->setExecutor(id, [this, bits](const CommandParams& params) {
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

void PicturaMainWindow::convertImageMode(const QString& mode)
{
    PictureView* view = activeView();
    // A checkable action toggles itself; every exit re-syncs the check marks.
    const auto done = qScopeGuard([this]() { refresh(); });
    if (!view || image_mode(*view) == mode || !image_mode_available(*view, mode)) {
        return;
    }
    const bool layered = view->layer_count() > 1;

    // Bitmap and Indexed hold no layers: the conversion flattens.
    if (mode == QLatin1String("bitmap") || mode == QLatin1String("indexed")) {
        if (layered) {
            QMessageBox box(QMessageBox::Question, tr("Kooka Pictura"), tr("Flatten layers?"),
                            QMessageBox::Ok | QMessageBox::Cancel, this);
            runDialog(box, this);
            if (box.clickedButton() != box.button(QMessageBox::Ok)) {
                return;
            }
        }
        if (mode == QLatin1String("bitmap")) {
            BitmapModeDialog dialog(this);
            if (runDialog(dialog, this) == QDialog::Accepted
                && convert_to_bitmap(*view, dialog.method())) {
                CommandParams command;
                command.values.insert(QStringLiteral("mode"), QStringLiteral("bitmap"));
                command.values.insert(QStringLiteral("method"), dialog.method());
                registry_->recordCommand(command_ids::ImageModeBitmap, command);
            }
            return;
        }
        IndexedColorDialog dialog(indexed_exact_available(*view), this);
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
        if (runDialog(dialog, this) == QDialog::Accepted) {
            if (convert_to_indexed(*view, dialog.palette(), dialog.colors(), dialog.dither(),
                                   dialog.amount())) {
                CommandParams command;
                command.values.insert(QStringLiteral("mode"), QStringLiteral("indexed"));
                command.values.insert(QStringLiteral("palette"), dialog.palette());
                command.values.insert(QStringLiteral("colors"), dialog.colors());
                command.values.insert(QStringLiteral("dither"), dialog.dither());
                command.values.insert(QStringLiteral("amount"), dialog.amount());
                registry_->recordCommand(command_ids::ImageModeIndexed, command);
            }
        } else {
            cancel_mode_preview(*view);
        }
        return;
    }

    const QString from = image_mode(*view);
    if (mode == QLatin1String("grayscale") && from != QLatin1String("bitmap")) {
        QMessageBox box(QMessageBox::Information, tr("Kooka Pictura"),
                        tr("Discard color information?"), QMessageBox::Cancel, this);
        box.setInformativeText(
            tr("To control the conversion, use Image > Adjustments > Black & White."));
        QPushButton* discard = box.addButton(tr("Discard"), QMessageBox::AcceptRole);
        box.setDefaultButton(discard);
        runDialog(box, this);
        if (box.clickedButton() != discard) {
            return;
        }
    }
    bool flatten = false;
    if (layered && mode != QLatin1String("rgb")) {
        QMessageBox box(QMessageBox::Warning, tr("Kooka Pictura"),
                        tr("Changing modes can affect the appearance of layers. Flatten image "
                           "before mode change?"),
                        QMessageBox::Cancel, this);
        QPushButton* flattenButton = box.addButton(tr("Flatten"), QMessageBox::AcceptRole);
        QPushButton* keep = box.addButton(tr("Don't Flatten"), QMessageBox::DestructiveRole);
        box.setDefaultButton(flattenButton);
        runDialog(box, this);
        if (box.clickedButton() == flattenButton) {
            view->flatten_image();
            flatten = true;
        } else if (box.clickedButton() != keep) {
            return;
        }
    }
    if (convert_image_mode(*view, mode)) {
        const char* id = mode == QLatin1String("grayscale") ? command_ids::ImageModeGrayscale
                        : mode == QLatin1String("rgb")       ? command_ids::ImageModeRgb
                        : mode == QLatin1String("cmyk")      ? command_ids::ImageModeCmyk
                        : mode == QLatin1String("lab")       ? command_ids::ImageModeLab
                                                             : nullptr;
        if (id) {
            CommandParams command;
            command.values.insert(QStringLiteral("mode"), mode);
            command.values.insert(QStringLiteral("flatten"), flatten);
            registry_->recordCommand(QLatin1String(id), command);
        }
    }
}

void PicturaMainWindow::convertImageDepth(int bits)
{
    PictureView* view = activeView();
    const auto done = qScopeGuard([this]() { refresh(); });
    if (!view || image_depth_bits(*view) == bits || !image_depth_available(*view, bits)) {
        return;
    }
    // Leaving 32-bit compresses the HDR range through the tone map.
    if (image_depth_bits(*view) == 32) {
        HdrConversionSpec spec;
        if (HdrConversionDialog::get(this, &spec)
            && convert_depth_method(*view, bits, int(spec.method), spec.exposure_ev, spec.gamma,
                                    spec.radius, spec.strength, spec.local_gamma,
                                    spec.local_exposure, spec.detail, spec.shadow, spec.highlight,
                                    spec.vibrance, spec.saturation)) {
            CommandParams command;
            command.values.insert(QStringLiteral("depth"), bits);
            command.values.insert(QStringLiteral("exposure"), spec.exposure_ev);
            command.values.insert(QStringLiteral("gamma"), spec.gamma);
            registry_->recordCommand(depthCommandId(bits), command);
        }
        return;
    }
    if (convert_image_depth(*view, bits)) {
        CommandParams command;
        command.values.insert(QStringLiteral("depth"), bits);
        registry_->recordCommand(depthCommandId(bits), command);
    }
}

} // namespace pictura
