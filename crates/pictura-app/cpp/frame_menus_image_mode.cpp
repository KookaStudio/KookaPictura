#include "frame_includes.h"

#include <QtCore/QScopeGuard>

#include "bitmap_mode_dialog.h"
#include "indexed_color_dialog.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"
#include "pictura_app/src/cxxqt_object/hdr_conversion.cxxqt.h"

namespace pictura {

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
            if (runDialog(dialog, this) == QDialog::Accepted) {
                convert_to_bitmap(*view, dialog.method());
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
            convert_to_indexed(*view, dialog.palette(), dialog.colors(), dialog.dither(),
                               dialog.amount());
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
    if (layered && mode != QLatin1String("rgb")) {
        QMessageBox box(QMessageBox::Warning, tr("Kooka Pictura"),
                        tr("Changing modes can affect the appearance of layers. Flatten image "
                           "before mode change?"),
                        QMessageBox::Cancel, this);
        QPushButton* flatten = box.addButton(tr("Flatten"), QMessageBox::AcceptRole);
        QPushButton* keep = box.addButton(tr("Don't Flatten"), QMessageBox::DestructiveRole);
        box.setDefaultButton(flatten);
        runDialog(box, this);
        if (box.clickedButton() == flatten) {
            view->flatten_image();
        } else if (box.clickedButton() != keep) {
            return;
        }
    }
    convert_image_mode(*view, mode);
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
        if (HdrConversionDialog::get(this, &spec)) {
            convert_depth_method(*view, bits, int(spec.method), spec.exposure_ev, spec.gamma,
                                 spec.radius, spec.strength, spec.local_gamma,
                                 spec.local_exposure, spec.detail, spec.shadow, spec.highlight,
                                 spec.vibrance, spec.saturation);
        }
        return;
    }
    convert_image_depth(*view, bits);
}

} // namespace pictura
