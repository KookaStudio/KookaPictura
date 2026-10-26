#include "frame_includes.h"

#include "bmp_options_dialog.h"
#include "jpeg_options_dialog.h"
#include "png_options_dialog.h"
#include "tiff_options_dialog.h"

#include "pictura_app/src/cxxqt_object/save_format.cxxqt.h"

namespace pictura {

bool PicturaMainWindow::saveActiveAsWithOptions(const QString& path)
{
    PictureView* view = activeView();
    if (!view || !view->has_document() || !promptSaveFormatOptions(path)) {
        return false;
    }
    const QString suffix = QFileInfo(path).suffix().toLower();
    const bool native = suffix == QStringLiteral("psd") || suffix == QStringLiteral("psb");
    const bool offersProfile = native || suffix == QStringLiteral("jpg")
        || suffix == QStringLiteral("jpeg") || suffix == QStringLiteral("jpe")
        || suffix == QStringLiteral("tif") || suffix == QStringLiteral("tiff");
    if (offersProfile) {
        SaveOptionsDialog options(
            pictura::default_save_embed_profile(*view, native), true, this);
        if (runDialog(options, this) != QDialog::Accepted) {
            return false;
        }
        pictura::set_save_embed_profile(*view, options.embedProfile());
    }
    if (saveAsNeedsFeatureWarning(path, view)) {
        const auto choice = QMessageBox::warning(
            this, tr("Save As"),
            tr("The chosen format cannot hold all of the document's features. "
               "The flattened composite will be written."),
            QMessageBox::Save | QMessageBox::Cancel);
        if (choice != QMessageBox::Save) {
            return false;
        }
    }
    return saveActiveAs(path);
}

bool PicturaMainWindow::promptSaveFormatOptions(const QString& path)
{
    const SaveFormatKind kind = saveFormatKindForSuffix(QFileInfo(path).suffix());
    if (kind == SaveFormatKind::None) {
        return true;
    }
    PictureView* view = activeView();
    if (!view) {
        return true;
    }
    SaveFormatOptions options = saveFormatOptionsFromJson(saveFormatOptions_);
    if (!runSaveFormatDialog(kind, options)) {
        return false;
    }
    rememberSaveFormatOptions(kind, options);
    applySaveRasterOptions(*view, kind, options);
    saveSession();
    return true;
}

void PicturaMainWindow::setSaveFormatDialogForTest(
    std::function<bool(SaveFormatKind, SaveFormatOptions&)> hook)
{
    saveFormatDialogForTest_ = std::move(hook);
}

bool PicturaMainWindow::runSaveFormatDialog(SaveFormatKind kind, SaveFormatOptions& options)
{
    if (saveFormatDialogForTest_) {
        return saveFormatDialogForTest_(kind, options);
    }
    PictureView* view = activeView();
    switch (kind) {
    case SaveFormatKind::Jpeg: {
        JpegOptionsDialog dialog(options, view, foregroundColor(), backgroundColor(), this);
        if (runDialog(dialog, this) != QDialog::Accepted) {
            return false;
        }
        options.jpegQuality = dialog.quality();
        options.jpegFormat = dialog.format();
        options.jpegScans = dialog.scans();
        options.jpegMatte = dialog.matte();
        options.jpegPreview = dialog.preview();
        return true;
    }
    case SaveFormatKind::Png: {
        PngOptionsDialog dialog(options, this);
        if (runDialog(dialog, this) != QDialog::Accepted) {
            return false;
        }
        options.pngCompression = dialog.compression();
        options.pngInterlace = dialog.interlace();
        return true;
    }
    case SaveFormatKind::Bmp: {
        BmpOptionsDialog dialog(options, this);
        if (runDialog(dialog, this) != QDialog::Accepted) {
            return false;
        }
        options.bmpFormat = dialog.format();
        options.bmpDepth = dialog.depth();
        options.bmpRle = dialog.rle();
        options.bmpFlip = dialog.flip();
        return true;
    }
    case SaveFormatKind::Tiff: {
        TiffOptionsDialog dialog(options, this);
        if (runDialog(dialog, this) != QDialog::Accepted) {
            return false;
        }
        options.tiffCompression = dialog.compression();
        options.tiffJpegQuality = dialog.jpegQuality();
        options.tiffPixelOrder = dialog.pixelOrder();
        options.tiffByteOrder = dialog.byteOrder();
        options.tiffLayerCompression = dialog.layerCompression();
        options.tiffPyramid = dialog.pyramid();
        options.tiffTransparency = dialog.transparency();
        return true;
    }
    default:
        return true;
    }
}

void PicturaMainWindow::rememberSaveFormatOptions(SaveFormatKind kind,
                                                  const SaveFormatOptions& options)
{
    const QString key = saveFormatKey(kind);
    if (key.isEmpty()) {
        return;
    }
    saveFormatOptions_.insert(key, saveFormatOptionsToJson(options).value(key).toObject());
}

void PicturaMainWindow::applySaveRasterOptions(PictureView& view, SaveFormatKind kind,
                                               const SaveFormatOptions& options)
{
    switch (kind) {
    case SaveFormatKind::Jpeg:
        pictura::set_save_raster_options(view, jpegQualityPercent(options.jpegQuality), -1,
                                         options.jpegFormat != 0, options.jpegFormat == 2,
                                         options.jpegMatte);
        break;
    case SaveFormatKind::Png:
        pictura::set_save_raster_options(view, -1, options.pngCompression, false, false, -1);
        break;
    case SaveFormatKind::Bmp:
        pictura::set_save_raster_options(view, -1, -1, false, false, -1);
        break;
    case SaveFormatKind::Tiff:
        pictura::set_save_raster_options(
            view, options.tiffCompression == 5 ? jpegQualityPercent(options.tiffJpegQuality) : -1,
            options.tiffCompression, false, false, options.tiffTransparency ? -1 : 0xFFFFFF);
        break;
    default:
        break;
    }
}

} // namespace pictura
