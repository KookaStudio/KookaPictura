// Save, Save As, Revert, document close prompting, and smart-object edit
// sessions. The frame forwards its entry points here.

#include "import_save_coordinator.h"

#include "api/document_api.h"
#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "api/proofing_bridge.h"

#include "bmp_options_dialog.h"
#include "busy_job.h"
#include "cineon_options_dialog.h"
#include "command_ids.h"
#include "dcs_options_dialog.h"
#include "dialogs.h"
#include "document_command.h"
#include "document_registry.h"
#include "eps_options_dialog.h"
#include "gif_options_dialog.h"
#include "indexed_color_dialog.h"
#include "jpeg_options_dialog.h"
#include "memory_refusal.h"
#include "png_options_dialog.h"
#include "raw_options_dialog.h"
#include "save_options_dialog.h"
#include "session.h"
#include "tga_options_dialog.h"
#include "tiff_options_dialog.h"

#include "api/engine_bridge.h"

#include <QtCore/QByteArray>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonDocument>
#include <QtCore/QTemporaryDir>
#include <QtCore/QUrl>
#include <QtGui/QDesktopServices>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {
namespace {

// Save may route through Save As; only the origin command records the step.
thread_local bool saveActiveOrigin = false;

struct SaveActiveOrigin {
    SaveActiveOrigin() { saveActiveOrigin = true; }
    ~SaveActiveOrigin() { saveActiveOrigin = false; }
};

} // namespace

bool saveActiveInProgress()
{
    return saveActiveOrigin;
}

bool ImportSaveCoordinator::saveActive(bool interactive)
{
    SaveActiveOrigin origin;
    const auto record = [this] {
        recordCommand(command_ids::FileSave,
                      QJsonObject{{QStringLiteral("in"), documents_.activeFilePath()}});
    };
    const int index = documents_.activeIndex();
    if (index < 0) {
        return false;
    }
    PictureView* view = documents_.viewAt(index);
    for (const SmartObjectEditSession& session : editSessions_) {
        if (session.editor != view) {
            continue;
        }
        const QString file = session.temp->filePath(session.filename);
        const QString name = QFileInfo(file).fileName();
        const bool saved = saveRunsOnWorker(file)
            ? runBusyJob(dialogParent_, name, tr("Saving %1…").arg(name),
                         [view, file] { return document_api::saveStart(*view, file); },
                         [view] { return document_api::saveRunning(*view); },
                         [view] { return document_api::saveFinish(*view); })
            : document_api::save(*view, file);
        if (!saved
            || !session.origin->commit_smart_object_edit(session.layerPath, file)) {
            return false;
        }
        if (hooks_.requestRefresh) {
            hooks_.requestRefresh();
        }
        documents_.updateTabTitle(index);
        if (hooks_.updateWindowTitle) {
            hooks_.updateWindowTitle();
        }
        // The editor tab has no file path of its own; the step would replay an
        // empty save target, so a smart-object commit records nothing here.
        return true;
    }
    QString path = documents_.documentPath(index);
    if (view && print_metadata_pending(*view) && !isNativeDocumentPath(path)) {
        if (!interactive) {
            return false;
        }
        const QStringList filters = saveFileFilters().mid(0, 2);
        QString selected = filters.first();
        QString target = getSaveFileName(dialogParent_, tr("Save Print Settings As"),
                                         documents_.activeFilePath(), filters, &selected);
        if (target.isEmpty()) {
            return false;
        }
        target = pathForSaveFilter(target, filters.indexOf(selected) == 1 ? 1 : 0, 0);
        if (!isNativeDocumentPath(target)) {
            return false;
        }
        const auto includeMerged = chooseMergedImage(target, *view);
        if (!includeMerged.has_value()) {
            return false;
        }
        const bool saved = saveActiveAs(target, includeMerged);
        if (saved) {
            record();
        }
        return saved;
    }
    if (!path.isEmpty() && isPdfPath(path)) {
        const bool saved = savePdfWithDialog(path);
        if (saved) {
            record();
        }
        return saved;
    }
    SaveFormatKind kind = lastSaveKinds_.value(view, SaveFormatKind::None);
    if (kind == SaveFormatKind::None) {
        kind = saveFormatKindForSuffix(QFileInfo(path).suffix());
    }
    const bool rememberedDcs = kind == SaveFormatKind::Dcs;
    if (path.isEmpty()
        || savePathNeedsFormatDialog(path, view, isNativeDocumentPath(path), rememberedDcs)) {
        const bool saved = saveAsWithDialog();
        if (saved) {
            record();
        }
        return saved;
    }
    if (view && kind != SaveFormatKind::None) {
        applySaveRasterOptions(*view, kind, saveFormatOptionsFromJson(saveFormatOptions_));
    }
    std::optional<bool> includeMerged;
    if (interactive && view && saveRunsOnWorker(path)) {
        includeMerged = chooseMergedImage(path, *view);
        if (!includeMerged.has_value()) {
            return false;
        }
    }
    const bool saved = saveActiveAs(path, includeMerged);
    if (saved) {
        record();
    }
    return saved;
}

bool ImportSaveCoordinator::saveActiveAs(const QString& path, std::optional<bool> includeMerged)
{
    const int index = documents_.activeIndex();
    PictureView* view = documents_.viewAt(index);
    if (!view || path.isEmpty()) {
        return false;
    }
    const QString name = QFileInfo(path).fileName();
    const bool saved = saveRunsOnWorker(path)
        ? runBusyJob(dialogParent_, name, tr("Saving %1…").arg(name),
                     [view, path, includeMerged] {
                         return includeMerged.has_value()
                             ? pictura::native_compatibility_save_start(*view, path, false, *includeMerged)
                             : document_api::saveStart(*view, path);
                     },
                     [view] { return document_api::saveRunning(*view); },
                     [view] { return document_api::saveFinish(*view); })
        : document_api::save(*view, path);
    if (!saved) {
        if (!reportMemoryRefusal(dialogParent_, tr("save the document")) && hooks_.showStatus) {
            hooks_.showStatus(tr("Could not save %1.").arg(name), 6000);
        }
        return false;
    }
    documents_.setDocumentPath(index, path);
    // A stale save leaves the document modified; keep the recovery snapshot so
    // an edit made during the save is still recoverable.
    if (!document_api::isDirty(*view)) {
        dropRecoverySnapshot(view);
    }
    documents_.updateTabTitle(index);
    rememberRecent(path);
    if (hooks_.updateWindowTitle) {
        hooks_.updateWindowTitle();
    }
    if (!saveActiveOrigin) {
        QJsonObject step{{QStringLiteral("as"), path.section(QLatin1Char('.'), -1).toUpper()},
                         {QStringLiteral("in"), path}};
        // The step carries the format's options so playback (and Batch) saves
        // with them rather than whatever is remembered at replay time.
        const SaveFormatKind kind =
            lastSaveKinds_.value(view, saveFormatKindForSuffix(QFileInfo(path).suffix()));
        const QString key = saveFormatKey(kind);
        if (!key.isEmpty()) {
            step.insert(QStringLiteral("options"),
                        saveFormatOptionsToJson(saveFormatOptionsFromJson(saveFormatOptions_))
                            .value(key));
        }
        recordCommand(command_ids::FileSaveAs, step);
    }
    return true;
}

std::optional<bool> ImportSaveCoordinator::chooseMergedImage(const QString& path,
                                                            const PictureView& view)
{
    if (!saveRunsOnWorker(path) || nativeSaveCompatibility_ == NativeSaveCompatibility::Always
        || !pictura::native_save_can_omit_merged(view)) {
        return true;
    }
    if (nativeSaveCompatibility_ == NativeSaveCompatibility::Never) {
        return false;
    }
    QMessageBox dialog(QMessageBox::Question, tr("Maximize PSD and PSB File Compatibility"),
                       tr("Include a merged image? Other applications may need it to open this file."),
                       QMessageBox::Cancel, dialogParent_);
    auto* include = dialog.addButton(tr("Include Merged Image"), QMessageBox::AcceptRole);
    auto* omit = dialog.addButton(tr("Omit Merged Image"), QMessageBox::DestructiveRole);
    dialog.setDefaultButton(include);
    dialog.setEscapeButton(QMessageBox::Cancel);
    runDialog(dialog, dialogParent_);
    if (dialog.clickedButton() == include) {
        return true;
    }
    if (dialog.clickedButton() == omit) {
        return false;
    }
    return std::nullopt;
}

bool ImportSaveCoordinator::confirmSaveProfile(const QString& path, const PictureView& view,
                                               std::optional<bool>& choice)
{
    const QString suffix = QFileInfo(path).suffix().toLower();
    const bool native = suffix == QStringLiteral("psd") || suffix == QStringLiteral("psb");
    const bool offersProfile = native || suffix == QStringLiteral("jpg")
        || suffix == QStringLiteral("jpeg") || suffix == QStringLiteral("jpe")
        || suffix == QStringLiteral("tif") || suffix == QStringLiteral("tiff");
    if (!offersProfile) {
        return true;
    }
    const bool initial = pictura::default_save_embed_profile(view, native);
    if (profileDialogForTest_) {
        bool embed = initial;
        if (!profileDialogForTest_(initial, embed)) {
            return false;
        }
        choice = embed;
        return true;
    }
    SaveOptionsDialog options(initial, true, dialogParent_);
    if (runDialog(options, dialogParent_) != QDialog::Accepted) {
        return false;
    }
    choice = options.embedProfile();
    return true;
}

bool ImportSaveCoordinator::confirmFeatureWarning(const QString& path, PictureView& view,
                                                  const QString& title)
{
    if (!saveAsNeedsFeatureWarning(path, &view)) {
        return true;
    }
    if (featureWarningForTest_) {
        return featureWarningForTest_();
    }
    const QString text = tr("The chosen format cannot hold all of the document's features. "
                            "The flattened composite will be written.");
    if (warningDriverForTest_) {
        return warningDriverForTest_(title, text);
    }
    return QMessageBox::warning(dialogParent_, title, text,
                                QMessageBox::Save | QMessageBox::Cancel)
        == QMessageBox::Save;
}

bool ImportSaveCoordinator::saveCopyWithDialog()
{
    PictureView* view = documents_.activeView();
    if (!view || !view->has_document()) {
        return false;
    }
    const QStringList& filters = saveFileFilters();
    const int sourceIndex = defaultSaveFilterIndex(view);
    QString selected = filters.at(sourceIndex);
    QString path = getSaveFileName(dialogParent_, tr("Save a Copy"),
                                   documents_.activeFilePath(), filters, &selected);
    if (path.isEmpty()) {
        return false;
    }
    const int chosen = filters.indexOf(selected);
    const int filterIndex = chosen >= 0 ? chosen : sourceIndex;
    const SaveFormatKind hint = saveFormatKindForFilter(filterIndex);
    path = pathForSaveFilter(path, filterIndex, sourceIndex);
    if (isPdfPath(path)) {
        return savePdfWithDialog(path, true);
    }
    return saveCopyAsWithOptions(path, hint);
}

bool ImportSaveCoordinator::saveCopyAsWithOptions(const QString& path, SaveFormatKind hint)
{
    PictureView* view = documents_.activeView();
    if (!view || !view->has_document()) {
        return false;
    }
    std::optional<bool> embed;
    if (!confirmFeatureWarning(path, *view, tr("Save a Copy"))
        || !promptSaveFormatOptions(path, hint) || !confirmSaveProfile(path, *view, embed)) {
        return false;
    }
    const auto includeMerged = chooseMergedImage(path, *view);
    if (!includeMerged.has_value()) {
        return false;
    }
    if (embed.has_value()) {
        pictura::set_save_embed_profile(*view, *embed);
    }
    return saveCopyAs(path, includeMerged);
}

bool ImportSaveCoordinator::saveCopyAs(const QString& path, std::optional<bool> includeMerged)
{
    const int index = documents_.activeIndex();
    PictureView* view = documents_.viewAt(index);
    if (!view || path.isEmpty()) {
        return false;
    }
    const QString name = QFileInfo(path).fileName();
    const bool native = saveRunsOnWorker(path);
    const bool saved = native
        ? runBusyJob(dialogParent_, name, tr("Saving %1…").arg(name),
                     [view, path, includeMerged] {
                         return includeMerged.has_value()
                             ? pictura::native_compatibility_save_start(*view, path, true, *includeMerged)
                             : document_api::saveCopyStart(*view, path);
                     },
                     [view] { return document_api::saveRunning(*view); },
                     [view] { return document_api::saveFinish(*view); })
        : document_api::saveCopy(*view, path);
    if (!saved) {
        const QString action = tr("save a copy of %1").arg(name);
        if (!reportMemoryRefusal(dialogParent_, action) && hooks_.showStatus) {
            hooks_.showStatus(tr("Could not save a copy of %1.").arg(name), 6000);
        }
        return false;
    }
    return true;
}

bool ImportSaveCoordinator::saveActiveAtPath(const QString& path, const QJsonObject& stepOptions)
{
    PictureView* view = documents_.activeView();
    if (!view || path.isEmpty()) {
        return false;
    }
    const SaveFormatKind kind = saveFormatKindForSuffix(QFileInfo(path).suffix());
    if (kind != SaveFormatKind::None) {
        // A step's own options overlay the remembered ones for this save only.
        QJsonObject root = saveFormatOptions_;
        const QString key = saveFormatKey(kind);
        QJsonObject format = root.value(key).toObject();
        for (auto it = stepOptions.begin(); it != stepOptions.end(); ++it) {
            format.insert(it.key(), it.value());
        }
        root.insert(key, format);
        applySaveRasterOptions(*view, kind, saveFormatOptionsFromJson(root));
        lastSaveKinds_.insert(view, kind);
    }
    return saveActiveAs(path);
}

bool ImportSaveCoordinator::saveAsWithDialog()
{
    PictureView* view = documents_.activeView();
    if (!view || !view->has_document()) {
        return false;
    }
    const QStringList& filters = saveFileFilters();
    const int sourceIndex = defaultSaveFilterIndex(view);
    QString selected = filters.at(sourceIndex);
    QString path = getSaveFileName(dialogParent_, tr("Save As"), documents_.activeFilePath(),
                                   filters, &selected);
    if (path.isEmpty()) {
        return false;
    }
    // The chosen filter owns the extension; All Formats keeps a typed suffix.
    const int chosen = filters.indexOf(selected);
    const int filterIndex = chosen >= 0 ? chosen : sourceIndex;
    const SaveFormatKind hint = saveFormatKindForFilter(filterIndex);
    path = pathForSaveFilter(path, filterIndex, sourceIndex);
    if (isPdfPath(path)) {
        const bool saved = savePdfWithDialog(path);
        if (saved && !saveActiveInProgress()) {
            recordCommand(command_ids::FileSaveAs,
                          QJsonObject{
                              {QStringLiteral("as"),
                               path.section(QLatin1Char('.'), -1).toUpper()},
                              {QStringLiteral("in"), path}});
        }
        return saved;
    }
    return saveActiveAsWithOptions(path, hint);
}

bool ImportSaveCoordinator::saveActiveAsWithOptions(const QString& path, SaveFormatKind hint)
{
    PictureView* view = documents_.activeView();
    if (!view || !view->has_document()) {
        return false;
    }
    SaveFormatKind kind = hint;
    if (kind == SaveFormatKind::None) {
        kind = saveFormatKindForSuffix(QFileInfo(path).suffix());
    }
    // The warning must run before the GIF options prompt: a cancelled save
    // leaves the document and any previous file untouched.
    const auto warn = [this](const QString& title, const QString& text) {
        if (warningDriverForTest_) {
            return warningDriverForTest_(title, text);
        }
        return QMessageBox::warning(dialogParent_, title, text,
                                    QMessageBox::Save | QMessageBox::Cancel)
            == QMessageBox::Save;
    };
    if (!confirmFeatureWarning(path, *view, tr("Save As"))) {
        return false;
    }
    if (kind == SaveFormatKind::Dcs) {
        const QString mode = pictura::image_mode(*view);
        if (mode != QStringLiteral("cmyk") && mode != QStringLiteral("multichannel")) {
            if (!warn(tr("Save As"),
                      tr("DCS separations need CMYK; the document will be converted."))) {
                return false;
            }
        }
        if (!channels_alpha_names(*view).isEmpty()) {
            if (!warn(tr("Save As"),
                      tr("DCS cannot carry named spot inks; the extra channels will be omitted."))) {
                return false;
            }
        }
    }
    if (!promptSaveFormatOptions(path, kind)) {
        return false;
    }
    std::optional<bool> embed;
    if (!confirmSaveProfile(path, *view, embed)) {
        return false;
    }
    const auto includeMerged = chooseMergedImage(path, *view);
    if (!includeMerged.has_value()) {
        return false;
    }
    if (embed.has_value()) {
        pictura::set_save_embed_profile(*view, *embed);
    }
    if (kind != SaveFormatKind::None) {
        lastSaveKinds_.insert(view, kind);
    } else {
        lastSaveKinds_.remove(view);
    }
    return saveActiveAs(path, includeMerged);
}

bool ImportSaveCoordinator::promptSaveFormatOptions(const QString& path, SaveFormatKind hint)
{
    SaveFormatKind kind = hint;
    if (kind == SaveFormatKind::None) {
        kind = saveFormatKindForSuffix(QFileInfo(path).suffix());
    }
    if (kind == SaveFormatKind::None) {
        return true;
    }
    PictureView* view = documents_.activeView();
    if (!view) {
        return true;
    }
    SaveFormatOptions options = saveFormatOptionsFromJson(saveFormatOptions_);
    if (kind == SaveFormatKind::Gif) {
        if (pictura::image_mode(*view) != QStringLiteral("indexed")) {
            if (gifDialogForTest_) {
                if (!gifDialogForTest_(view, options)) {
                    return false;
                }
            } else {
                const QString mode = pictura::image_mode(*view);
                const bool canPreview = document_api::documentDepthBits(*view) == 8
                    && (mode == QStringLiteral("rgb") || mode == QStringLiteral("grayscale"));
                IndexedColorOptions initial;
                initial.palette = options.gifPalette;
                initial.colors = options.gifColors;
                initial.dither = options.gifDither;
                initial.amount = options.gifAmount;
                initial.preview = canPreview;
                IndexedColorDialog dialog(indexed_exact_available(*view), initial, dialogParent_);
                dialog.setPreviewAvailable(canPreview);
                if (canPreview) {
                    const auto preview = [view, &dialog] {
                        if (dialog.preview()) {
                            preview_gif_indexed(*view, dialog.palette(), dialog.colors(),
                                                dialog.dither(), dialog.amount());
                        } else {
                            cancel_mode_preview(*view);
                        }
                    };
                    connect(&dialog, &IndexedColorDialog::optionsChanged, &dialog, preview);
                    preview();
                }
                const int result = gifDialogDriver_
                    ? (gifDialogDriver_(dialog) ? QDialog::Accepted : QDialog::Rejected)
                    : runDialog(dialog, dialogParent_);
                if (canPreview) {
                    cancel_mode_preview(*view);
                }
                if (result != QDialog::Accepted) {
                    return false;
                }
                options.gifPalette = dialog.palette();
                options.gifColors = dialog.colors();
                options.gifDither = dialog.dither();
                options.gifAmount = dialog.amount();
            }
            if (!runSaveFormatDialog(kind, options)) {
                return false;
            }
        }
        // An already-Indexed document has no palette dialog, and its remembered
        // row order applies without changing the document.
    } else if (!runSaveFormatDialog(kind, options)) {
        return false;
    }
    rememberSaveFormatOptions(kind, options);
    applySaveRasterOptions(*view, kind, options);
    if (hooks_.saveSession) {
        hooks_.saveSession();
        return true;
    }
    SessionState state = loadSession();
    state.saveFormatOptions = saveFormatOptions_;
    pictura::saveSession(state);
    return true;
}

bool ImportSaveCoordinator::runSaveFormatDialog(SaveFormatKind kind,
                                                SaveFormatOptions& options)
{
    if (saveFormatDialogForTest_) {
        return saveFormatDialogForTest_(kind, options);
    }
    PictureView* view = documents_.activeView();
    const QColor foreground = hooks_.foregroundColor ? hooks_.foregroundColor() : Qt::black;
    const QColor background = hooks_.backgroundColor ? hooks_.backgroundColor() : Qt::white;
    switch (kind) {
    case SaveFormatKind::Jpeg: {
        JpegOptionsDialog dialog(options, view, foreground, background, dialogParent_);
        if (runDialog(dialog, dialogParent_) != QDialog::Accepted) {
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
        PngOptionsDialog dialog(options, dialogParent_);
        if (runDialog(dialog, dialogParent_) != QDialog::Accepted) {
            return false;
        }
        options.pngCompression = dialog.compression();
        options.pngInterlace = dialog.interlace();
        return true;
    }
    case SaveFormatKind::Bmp: {
        BmpOptionsDialog dialog(options, dialogParent_);
        if (runDialog(dialog, dialogParent_) != QDialog::Accepted) {
            return false;
        }
        options.bmpFormat = dialog.format();
        options.bmpDepth = dialog.depth();
        options.bmpRle = dialog.rle();
        options.bmpFlip = dialog.flip();
        return true;
    }
    case SaveFormatKind::Tiff: {
        TiffOptionsDialog dialog(options, dialogParent_);
        if (runDialog(dialog, dialogParent_) != QDialog::Accepted) {
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
    case SaveFormatKind::Gif: {
        GifOptionsDialog dialog(options, dialogParent_);
        if (runDialog(dialog, dialogParent_) != QDialog::Accepted) {
            return false;
        }
        options.gifRowOrder = dialog.rowOrder();
        return true;
    }
    case SaveFormatKind::Eps: {
        EpsOptionsDialog dialog(options, dialogParent_);
        if (runDialog(dialog, dialogParent_) != QDialog::Accepted) {
            return false;
        }
        options.epsPreview = dialog.preview();
        options.epsEncoding = dialog.encoding();
        options.epsJpegQuality = dialog.jpegQuality();
        options.epsHalftone = dialog.halftone();
        options.epsTransfer = dialog.transfer();
        options.epsTransparentWhites = dialog.transparentWhites();
        options.epsPsColorManagement = dialog.psColorManagement();
        options.epsVectorData = dialog.vectorData();
        options.epsInterpolation = dialog.interpolation();
        return true;
    }
    case SaveFormatKind::Dcs: {
        DcsOptionsDialog dialog(options, dialogParent_);
        if (runDialog(dialog, dialogParent_) != QDialog::Accepted) {
            return false;
        }
        options.dcsVersion = dialog.version();
        options.dcsPreview = dialog.preview();
        return true;
    }
    case SaveFormatKind::Raw: {
        RawOptionsDialog dialog(options, dialogParent_);
        if (runDialog(dialog, dialogParent_) != QDialog::Accepted) {
            return false;
        }
        options.rawHeader = dialog.header();
        options.rawInterleave = dialog.interleave();
        return true;
    }
    case SaveFormatKind::Tga: {
        TgaOptionsDialog dialog(options, dialogParent_);
        if (runDialog(dialog, dialogParent_) != QDialog::Accepted) {
            return false;
        }
        options.tgaDepth = dialog.depth();
        options.tgaRle = dialog.rle();
        return true;
    }
    case SaveFormatKind::Cineon: {
        CineonOptionsDialog dialog(options, dialogParent_);
        if (runDialog(dialog, dialogParent_) != QDialog::Accepted) {
            return false;
        }
        options.cineonBits = dialog.bits();
        return true;
    }
    default:
        return true;
    }
}

void ImportSaveCoordinator::rememberSaveFormatOptions(
    SaveFormatKind kind, const SaveFormatOptions& options)
{
    const QString key = saveFormatKey(kind);
    if (key.isEmpty()) {
        return;
    }
    saveFormatOptions_.insert(key, saveFormatOptionsToJson(options).value(key).toObject());
}

void ImportSaveCoordinator::applySaveRasterOptions(PictureView& view, SaveFormatKind kind,
                                                   const SaveFormatOptions& options)
{
    if (kind == SaveFormatKind::None) {
        return;
    }
    const QJsonObject root = saveFormatOptionsToJson(options);
    const QByteArray json = QJsonDocument(root).toJson(QJsonDocument::Compact);
    pictura::set_save_options(view, static_cast<int>(kind),
                              ::rust::Str(json.constData(), static_cast<size_t>(json.size())));
    switch (kind) {
    case SaveFormatKind::Jpeg:
        pictura::set_save_raster_options(view, jpegQualityPercent(options.jpegQuality), -1,
                                         options.jpegFormat != 0, options.jpegFormat == 2,
                                         options.jpegMatte);
        break;
    case SaveFormatKind::Gif:
        pictura::set_save_gif_options(view, options.gifPalette, options.gifColors,
                                      options.gifDither, options.gifAmount,
                                      options.gifRowOrder == 1);
        break;
    case SaveFormatKind::Png:
        pictura::set_save_raster_options(view, -1, options.pngCompression, false, false, -1);
        break;
    case SaveFormatKind::Bmp:
        pictura::set_save_raster_options(view, -1, -1, false, false, -1);
        break;
    case SaveFormatKind::Tiff:
        pictura::set_save_raster_options(
            view, options.tiffCompression == 4 ? jpegQualityPercent(options.tiffJpegQuality) : -1,
            options.tiffCompression, false, false, options.tiffTransparency ? -1 : 0xFFFFFF);
        break;
    default:
        break;
    }
}

bool ImportSaveCoordinator::revertActive()
{
    const int index = documents_.activeIndex();
    PictureView* view = documents_.viewAt(index);
    if (!view) {
        return false;
    }
    const QString path = document_api::filePath(*view);
    if (path.isEmpty()) {
        return false;
    }
    // Reload in place through the Rust edge: decode first, preserve history and
    // file identity, then append one undoable Revert state.
    const bool reloaded = document_api::revert(*view, path);
    if (!reloaded) {
        return false;
    }
    lastSaveKinds_.remove(view);
    documents_.updateTabTitle(index);
    if (hooks_.requestRefresh) {
        hooks_.requestRefresh();
    }
    return true;
}

bool ImportSaveCoordinator::saveAllDocumentsForRestart()
{
    for (int i = 0; i < documents_.count(); ++i) {
        if (!documents_.isDocumentDirty(i)) {
            continue;
        }
        switch (askUnsaved(dialogParent_, documents_.documentName(i))) {
        case UnsavedChoice::Cancel:
            return false;
        case UnsavedChoice::Save:
            documents_.setActiveIndex(i);
            if (!saveActive()) {
                return false;
            }
            break;
        case UnsavedChoice::Discard:
            break;
        }
    }
    return true;
}

bool ImportSaveCoordinator::closeDocument(int index, bool interactive)
{
    if (index < 0 || index >= documents_.count()) {
        return false;
    }
    if (documents_.isLoading(index)) {
        cancelOpen(index);
        return true;
    }
    if (interactive && documents_.isDocumentDirty(index)) {
        switch (askUnsaved(dialogParent_, documents_.documentName(index))) {
        case UnsavedChoice::Cancel:
            return false;
        case UnsavedChoice::Save:
            documents_.setActiveIndex(index);
            if (!saveActive()) {
                return false;
            }
            break;
        case UnsavedChoice::Discard:
            break;
        }
    }
    lastSaveKinds_.remove(documents_.viewAt(index));
    documents_.removeDocument(index);
    return true;
}

bool ImportSaveCoordinator::closeActiveDocument(bool interactive)
{
    return closeDocument(documents_.activeIndex(), interactive);
}

bool ImportSaveCoordinator::editSmartObjectContents(const QString& layerPath)
{
    PictureView* origin = documents_.activeView();
    if (!origin || layerPath.isEmpty()
        || !origin->layer_can_edit_smart_object_contents(layerPath)) {
        return false;
    }
    const QString suffix = vector_smart_object_suffix(*origin, layerPath);
    if (!suffix.isEmpty()) {
        return editVectorContents(*origin, layerPath, suffix);
    }
    auto* temp = new QTemporaryDir();
    if (!temp->isValid()) {
        delete temp;
        return false;
    }
    const QString filename = QStringLiteral("contents.psd");
    const QString file = temp->filePath(filename);
    if (!origin->export_smart_object_contents(layerPath, file)) {
        delete temp;
        return false;
    }
    auto* editor = new PictureView(dialogParent_);
    if (!document_api::open(*editor, file)) {
        delete editor;
        delete temp;
        return false;
    }
    documents_.addDocument(editor, QString());
    editSessions_.append(SmartObjectEditSession{editor, origin, layerPath, filename, temp});
    return true;
}

bool ImportSaveCoordinator::editVectorContents(PictureView& origin, const QString& layerPath,
                                                const QString& suffix)
{
    const QString id = vector_smart_object_id(origin, layerPath);
    const auto revision = vector_edit_revision(origin, id);
    const auto generation = origin.document_generation();
    const auto temp = std::make_shared<QTemporaryDir>();
    const QString source = temp->filePath(QStringLiteral("contents.") + suffix);
    if (id.isEmpty() || revision == 0 || !temp->isValid()
        || !origin.export_smart_object_contents(layerPath, source)) {
        QMessageBox::warning(dialogParent_, tr("Edit Contents"),
                             tr("Could not prepare the artwork for editing."));
        return false;
    }
    const bool launched = vectorEditorLauncher_
        ? vectorEditorLauncher_(source)
        : QDesktopServices::openUrl(QUrl::fromLocalFile(source));
    if (!launched) {
        temp->remove();
        QMessageBox::warning(dialogParent_, tr("Edit Contents"),
                             tr("Could not open the artwork in the associated application."));
        return false;
    }
    auto* dialog = new QDialog(dialogParent_);
    dialog->setObjectName(QStringLiteral("vectorEditContents"));
    dialog->setWindowTitle(tr("Edit Contents"));
    dialog->setAttribute(Qt::WA_DeleteOnClose);
    auto* layout = new QVBoxLayout(dialog);
    auto* label = new QLabel(tr("Save the artwork in the external editor, then select Apply."),
                             dialog);
    label->setWordWrap(true);
    layout->addWidget(label);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Apply | QDialogButtonBox::Discard,
                                         dialog);
    layout->addWidget(buttons);
    auto* apply = buttons->button(QDialogButtonBox::Apply);
    auto* discard = buttons->button(QDialogButtonBox::Discard);
    apply->setObjectName(QStringLiteral("vectorEditApply"));
    discard->setObjectName(QStringLiteral("vectorEditDiscard"));
    connect(discard, &QPushButton::clicked, dialog, &QDialog::reject);
    connect(dialog, &QDialog::finished, dialog, [temp]() { temp->remove(); });

    const QPointer<PictureView> target(&origin);
    const QPointer<QDialog> sessionDialog(dialog);
    const QPointer<QPushButton> applyButton(apply);
    const QPointer<QWidget> progressParent(dialogParent_);
    connect(apply, &QPushButton::clicked, dialog,
            [target, sessionDialog, applyButton, progressParent, id, source, generation,
             expected = revision]() mutable {
        if (!target || target->document_generation() != generation) {
            QMessageBox::warning(sessionDialog, tr("Edit Contents"),
                                 tr("The original document is no longer available."));
            return;
        }
        applyButton->setEnabled(false);
        bool closed = false;
        const auto onClose = QObject::connect(
            sessionDialog.data(), &QDialog::finished, target.data(), [&closed, target](int) {
                closed = true;
                document_job_cancel(*target);
            });
        const bool applied = runSourceCommand(progressParent, *target, tr("Edit Contents"),
                                               [&](PictureView& view) {
            return start_vector_edit(view, id, expected, source);
        });
        QObject::disconnect(onClose);
        if (closed || !target || !sessionDialog) {
            return;
        }
        if (applyButton) {
            applyButton->setEnabled(true);
        }
        if (applied) {
            expected = vector_edit_revision(*target, id);
        } else {
            QMessageBox::warning(sessionDialog, tr("Edit Contents"),
                                 tr("Could not apply the edited artwork. Check that the source "
                                    "still exists and contains the selected page."));
        }
    });
    for (int i = vectorEditSessions_.size() - 1; i >= 0; --i) {
        if (!vectorEditSessions_.at(i).dialog) {
            vectorEditSessions_.removeAt(i);
        }
    }
    vectorEditSessions_.append(VectorEditSession{target, sessionDialog});
    dialog->show();
    return true;
}

} // namespace pictura
