// Save, Save As, Revert, document close prompting, and smart-object edit
// sessions. The frame forwards its entry points here.

#include "import_save_coordinator.h"

#include "api/document_api.h"

#include "bmp_options_dialog.h"
#include "busy_job.h"
#include "command_ids.h"
#include "dialogs.h"
#include "document_registry.h"
#include "jpeg_options_dialog.h"
#include "memory_refusal.h"
#include "png_options_dialog.h"
#include "save_options_dialog.h"
#include "session.h"
#include "tiff_options_dialog.h"

#include "api/engine_bridge.h"

#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QMessageBox>

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

bool ImportSaveCoordinator::saveActive()
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
    if (!path.isEmpty() && isPdfPath(path)) {
        const bool saved = savePdfWithDialog(path);
        if (saved) {
            record();
        }
        return saved;
    }
    if (path.isEmpty()
        || savePathNeedsFormatDialog(path, view, isNativeDocumentPath(path))) {
        const bool saved = saveAsWithDialog();
        if (saved) {
            record();
        }
        return saved;
    }
    const bool saved = saveActiveAs(path);
    if (saved) {
        record();
    }
    return saved;
}

bool ImportSaveCoordinator::saveActiveAs(const QString& path)
{
    const int index = documents_.activeIndex();
    PictureView* view = documents_.viewAt(index);
    if (!view || path.isEmpty()) {
        return false;
    }
    const QString name = QFileInfo(path).fileName();
    const bool saved = saveRunsOnWorker(path)
        ? runBusyJob(dialogParent_, name, tr("Saving %1…").arg(name),
                     [view, path] { return document_api::saveStart(*view, path); },
                     [view] { return document_api::saveRunning(*view); },
                     [view] { return document_api::saveFinish(*view); })
        : document_api::save(*view, path);
    if (!saved) {
        if (saveRunsOnWorker(path)) {
            reportMemoryRefusal(dialogParent_, tr("save the document"));
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
        recordCommand(command_ids::FileSaveAs,
                      QJsonObject{
                          {QStringLiteral("as"),
                           path.section(QLatin1Char('.'), -1).toUpper()},
                          {QStringLiteral("in"), path}});
    }
    return true;
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
    return saveActiveAsWithOptions(path);
}

bool ImportSaveCoordinator::saveActiveAsWithOptions(const QString& path)
{
    PictureView* view = documents_.activeView();
    if (!view || !view->has_document() || !promptSaveFormatOptions(path)) {
        return false;
    }
    const QString suffix = QFileInfo(path).suffix().toLower();
    const bool native = suffix == QStringLiteral("psd") || suffix == QStringLiteral("psb");
    const bool offersProfile = native || suffix == QStringLiteral("jpg")
        || suffix == QStringLiteral("jpeg") || suffix == QStringLiteral("jpe")
        || suffix == QStringLiteral("tif") || suffix == QStringLiteral("tiff");
    if (offersProfile) {
        SaveOptionsDialog options(pictura::default_save_embed_profile(*view, native), true,
                                  dialogParent_);
        if (runDialog(options, dialogParent_) != QDialog::Accepted) {
            return false;
        }
        pictura::set_save_embed_profile(*view, options.embedProfile());
    }
    if (saveAsNeedsFeatureWarning(path, view)) {
        const auto choice = QMessageBox::warning(
            dialogParent_, tr("Save As"),
            tr("The chosen format cannot hold all of the document's features. "
               "The flattened composite will be written."),
            QMessageBox::Save | QMessageBox::Cancel);
        if (choice != QMessageBox::Save) {
            return false;
        }
    }
    return saveActiveAs(path);
}

bool ImportSaveCoordinator::promptSaveFormatOptions(const QString& path)
{
    const SaveFormatKind kind = saveFormatKindForSuffix(QFileInfo(path).suffix());
    if (kind == SaveFormatKind::None) {
        return true;
    }
    PictureView* view = documents_.activeView();
    if (!view) {
        return true;
    }
    SaveFormatOptions options = saveFormatOptionsFromJson(saveFormatOptions_);
    if (!runSaveFormatDialog(kind, options)) {
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

} // namespace pictura
