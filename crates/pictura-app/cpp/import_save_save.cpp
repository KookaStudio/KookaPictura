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
#include <QtCore/QDir>
#include <QtCore/QScopeGuard>
#include <QtCore/QTimer>
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

#include <algorithm>

namespace pictura {
namespace {

// Save may route through Save As; only the origin command records the step.
constexpr int kMemoryAdmission = 1;
constexpr int kScratchAdmission = 2;
constexpr int kScratchFullAdmission = 3;

QString nativeAdmissionMessage(int admission, const QString& action)
{
    switch (admission) {
    case kMemoryAdmission: return memoryRefusalMessage(action);
    case kScratchAdmission: return scratchRefusalMessage(action);
    case kScratchFullAdmission: return scratchFullRefusalMessage(action);
    default: return {};
    }
}

thread_local bool saveActiveOrigin = false;

struct SaveActiveOrigin {
    SaveActiveOrigin() { saveActiveOrigin = true; }
    ~SaveActiveOrigin() { saveActiveOrigin = false; }
};

QString destinationKey(const QString& path)
{
    const QFileInfo file(path);
    const QString parent = file.absoluteDir().canonicalPath();
    const QString key = QDir::cleanPath(QDir(parent.isEmpty() ? file.absolutePath() : parent)
                                           .filePath(file.fileName()));
#ifdef Q_OS_WIN
    return key.toCaseFolded();
#else
    return key;
#endif
}

QJsonObject saveParameters(const QString& path, bool fromSave)
{
    QJsonObject values{{QStringLiteral("in"), path}};
    if (!fromSave) {
        values.insert(QStringLiteral("as"), path.section(QLatin1Char('.'), -1).toUpper());
    }
    return values;
}

bool startNativeSave(PictureView& view, const QString& path, std::optional<bool> includeMerged)
{
    return includeMerged.has_value()
        ? pictura::native_compatibility_save_start(view, path, false, *includeMerged)
        : document_api::saveStart(view, path);
}

} // namespace

bool saveActiveInProgress()
{
    return saveActiveOrigin;
}

bool ImportSaveCoordinator::saveActive(bool interactive, bool backgroundAllowed)
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
    if (view && !pictura::compatibility_reason(*view).isEmpty()) {
        if (hooks_.showStatus) {
            hooks_.showStatus(view->open_notice(), 6000);
        }
        return false;
    }
    for (const SmartObjectEditSession& session : editSessions_) {
        if (session.editor != view) {
            continue;
        }
        const QString file = session.temp->filePath(session.filename);
        const QString destination = destinationKey(file);
        if (!canStartNativeSave(view, destination)) {
            return false;
        }
        synchronousDestinations_.insert(destination);
        const auto release = qScopeGuard([&] { synchronousDestinations_.remove(destination); });
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
        const bool saved = saveActiveAs(target, backgroundAllowed, includeMerged);
        if (saved && !backgroundSaves_.contains(view)) {
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
        const bool saved = saveAsWithDialog(backgroundAllowed);
        if (saved && !backgroundSaves_.contains(view)) {
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
    const bool saved = saveActiveAs(path, backgroundAllowed, includeMerged);
    if (saved && !backgroundSaves_.contains(view)) {
        record();
    }
    return saved;
}

bool ImportSaveCoordinator::canStartNativeSave(PictureView* view, const QString& destination)
{
    const bool conflict = backgroundSaves_.contains(view)
        || synchronousDestinations_.contains(destination)
        || std::any_of(backgroundSaves_.cbegin(), backgroundSaves_.cend(),
                       [&](const PendingSave& job) { return job.destination == destination; });
    if (conflict && hooks_.showStatus) {
        hooks_.showStatus(tr("A save is already in progress."), 6000);
    }
    return !conflict;
}

bool ImportSaveCoordinator::startBackgroundSave(PictureView* view, const QString& path,
                                                std::optional<bool> includeMerged)
{
    const QString destination = destinationKey(path);
    if (!canStartNativeSave(view, destination)) {
        return false;
    }
    if (!startNativeSave(*view, path, includeMerged)) {
        return applySaveResult(view, path, 0, true, save_last_result(*view).code, true, save_last_result(*view).admission);
    }
    const bool fromSave = saveActiveInProgress();
    const auto id = fromSave ? command_ids::FileSave : command_ids::FileSaveAs;
    const quint64 token = hooks_.reserveRecording
        ? hooks_.reserveRecording(QString::fromLatin1(id), saveParameters(path, fromSave)) : 0;
    backgroundSaves_.insert(view, PendingSave{path, destination, token});
    if (!saveTimer_) {
        saveTimer_ = new QTimer(this);
        saveTimer_->setInterval(16);
        connect(saveTimer_, &QTimer::timeout, this, &ImportSaveCoordinator::pollBackgroundSaves);
    }
    if (DocEntry* doc = documents_.docAt(indexOfView(view))) {
        doc->saveProgress = 0;
        documents_.updateTabTitle(indexOfView(view));
    }
    if (hooks_.updateSaveProgress) {
        hooks_.updateSaveProgress();
    }
    saveTimer_->start();
    return true;
}

void ImportSaveCoordinator::pollBackgroundSaves()
{
    const auto saving = backgroundSaves_.keys();
    for (PictureView* view : saving) {
        if (!document_api::saveRunning(*view)) {
            finishBackgroundSave(view);
            continue;
        }
        const int index = indexOfView(view);
        if (DocEntry* doc = documents_.docAt(index)) {
            const int percent = qBound(0, save_progress(*view), 99);
            if (doc->saveProgress != percent) {
                doc->saveProgress = percent;
                documents_.updateTabTitle(index);
                if (hooks_.updateSaveProgress) {
                    hooks_.updateSaveProgress();
                }
            }
        }
    }
}

bool ImportSaveCoordinator::finishBackgroundSave(PictureView* view)
{
    const PendingSave job = backgroundSaves_.take(view);
    if (backgroundSaves_.isEmpty()) {
        saveTimer_->stop();
    }
    const int index = indexOfView(view);
    if (DocEntry* doc = documents_.docAt(index)) {
        doc->saveProgress = -1;
    }
    const auto result = save_finish_result(*view);
    const int outcome = result.replaced ? (result.publication == 1 ? 3 : 4)
                                        : result.publication;
    if (hooks_.finishRecording && job.recordingToken) {
        hooks_.finishRecording(job.recordingToken, outcome == 1 || outcome == 3);
    }
    const bool saved = applySaveResult(view, job.path, outcome, true, result.code, true, result.admission);
    if (!saved && outcome < 3) {
        unresolvedSaveFailures_.insert(view);
    }
    if (index >= 0) {
        documents_.updateTabTitle(index);
    }
    if (hooks_.updateSaveProgress) {
        hooks_.updateSaveProgress();
    }
    return saved;
}

bool ImportSaveCoordinator::resolvePendingSave(PictureView* view)
{
    if (!backgroundSaves_.contains(view)) {
        return !saveResolutionNeedsPrompt(view);
    }
    const QString name = QFileInfo(backgroundSaves_.value(view).path).fileName();
    saveTimer_->stop();
    const auto restart = qScopeGuard([this] {
        if (!backgroundSaves_.isEmpty()) {
            saveTimer_->start();
        }
    });
    return runBusyJob(dialogParent_, name, tr("Saving %1…").arg(name),
                      [] { return true; },
                      [view] { return document_api::saveRunning(*view); },
                      [this, view] { return finishBackgroundSave(view); });
}

bool ImportSaveCoordinator::saveResolutionNeedsPrompt(PictureView* view) const
{
    return unresolvedSaveFailures_.contains(view);
}

void ImportSaveCoordinator::acknowledgeSaveResolution(PictureView* view)
{
    unresolvedSaveFailures_.remove(view);
}

bool ImportSaveCoordinator::applySaveResult(PictureView* view, const QString& path, int outcome,
                                            bool background, quint32 code, bool native, int admission)
{
    const int index = indexOfView(view);
    const QString name = QFileInfo(path).fileName();
    if (outcome == 2) {
        if (hooks_.showStatus) {
            hooks_.showStatus(tr("Could not confirm publication of %1. Save the document again.").arg(name),
                              6000);
        }
        return false;
    }
    if (outcome >= 3) {
        return false;
    }
    if (outcome != 1) {
        const QString action = tr("save the document");
        const bool legacyRefusal = !native && reportMemoryRefusal(dialogParent_, action);
        if (native && admission && hooks_.showStatus) {
            hooks_.showStatus(nativeAdmissionMessage(admission, action), 6000);
        } else if (!legacyRefusal && hooks_.showStatus) {
            const auto message = code == quint32(OperationStatus::Panic)
                ? tr("Could not save %1. The operation stopped unexpectedly.").arg(name)
                : tr("Could not save %1.").arg(name);
            hooks_.showStatus(message, 6000);
        }
        return false;
    }
    if (index < 0) {
        return true;
    }
    acknowledgeSaveResolution(view);
    documents_.setDocumentPath(index, path);
    attachRecovery(view);
    if (!document_api::isDirty(*view)) {
        if (background) {
            if (!recovery_discard_async(*view) && hooks_.showStatus) {
                hooks_.showStatus(tr("Could not remove superseded crash recovery data."), 8000);
            }
        } else {
            dropRecoverySnapshot(view);
        }
    }
    documents_.updateTabTitle(index);
    rememberRecent(path);
    if (hooks_.updateWindowTitle) {
        hooks_.updateWindowTitle();
    }
    const QString warning = pictura::native_save_warning(*view);
    if (!warning.isEmpty() && hooks_.showStatus) {
        hooks_.showStatus(warning, 0);
    }
    return true;
}

bool ImportSaveCoordinator::saveActiveAs(const QString& path, bool backgroundAllowed,
                                         std::optional<bool> includeMerged)
{
    PictureView* view = documents_.activeView();
    if (!view || path.isEmpty() || !pictura::compatibility_reason(*view).isEmpty()) {
        return false;
    }
    const bool native = saveRunsOnWorker(path);
    if (backgroundAllowed && saveInBackground_ && native) {
        return startBackgroundSave(view, path, includeMerged);
    }
    const QString destination = destinationKey(path);
    if (!canStartNativeSave(view, destination)) {
        return false;
    }
    synchronousDestinations_.insert(destination);
    const auto release = qScopeGuard([&] { synchronousDestinations_.remove(destination); });
    const QString name = QFileInfo(path).fileName();
    int outcome = 0;
    quint32 code = 0;
    int admission = 0;
    if (native) {
        runBusyJob(dialogParent_, name, tr("Saving %1…").arg(name),
                   [view, path, includeMerged, &code, &admission] {
                       if (startNativeSave(*view, path, includeMerged)) {
                           return true;
                       }
                       code = save_last_result(*view).code;
                       admission = save_last_result(*view).admission;
                       return false;
                   },
                   [view] { return document_api::saveRunning(*view); },
                   [&] {
                       const auto result = save_finish_result(*view);
                       code = result.code;
                       outcome = result.replaced ? (result.publication == 1 ? 3 : 4)
                                                 : result.publication;
                       return outcome == 1;
                   });
    } else {
        outcome = document_api::save(*view, path) ? 1 : 0;
    }
    if (!applySaveResult(view, path, outcome, false, code, native, admission)) {
        return false;
    }
    if (!saveActiveInProgress()) {
        QJsonObject step = saveParameters(path, false);
        // Preserve the format options for action and Batch playback.
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
    if (!pictura::compatibility_reason(*view).isEmpty()) {
        if (!pictura::compatibility_preview_available(*view)) {
            if (hooks_.showStatus) {
                hooks_.showStatus(view->open_notice(), 6000);
            }
            return false;
        }
        const QString title = tr("Save a Copy");
        const QStringList filters = {tr("PNG (*.PNG)")};
        QString selected = filters.front();
        QString path = getSaveFileName(dialogParent_, title, documents_.activeFilePath(),
                                       filters, &selected);
        if (path.isEmpty()) {
            return false;
        }
        if (QFileInfo(path).suffix().isEmpty()) {
            path += QStringLiteral(".png");
        }
        const QString text = tr("Save the original merged image as a PNG? The copy will have no "
                                "editable layers or document features.");
        const bool confirmed = warningDriverForTest_
            ? warningDriverForTest_(title, text)
            : QMessageBox::warning(dialogParent_, title, text, QMessageBox::Save | QMessageBox::Cancel)
                == QMessageBox::Save;
        if (!confirmed) {
            return false;
        }
        const QString failure = pictura::save_compatibility_copy(*view, path, confirmed);
        if (!failure.isEmpty() && hooks_.showStatus) {
            hooks_.showStatus(failure, 6000);
        }
        return failure.isEmpty();
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
    const QString destination = destinationKey(path);
    if (!canStartNativeSave(view, destination)) {
        return false;
    }
    synchronousDestinations_.insert(destination);
    const auto release = qScopeGuard([&] { synchronousDestinations_.remove(destination); });
    const QString name = QFileInfo(path).fileName();
    const bool native = saveRunsOnWorker(path);
    NativeSaveResult result{};
    const bool saved = native
        ? runBusyJob(dialogParent_, name, tr("Saving %1…").arg(name),
                     [view, path, includeMerged, &result] {
                         const bool started = includeMerged.has_value()
                             ? pictura::native_compatibility_save_start(*view, path, true, *includeMerged)
                             : document_api::saveCopyStart(*view, path);
                         if (!started) {
                             result = save_last_result(*view);
                         }
                         return started;
                     },
                     [view] { return document_api::saveRunning(*view); },
                     [view, &result] {
                         result = save_finish_result(*view);
                         return result.publication == 1 && !result.replaced;
                     })
        : document_api::saveCopy(*view, path);
    if (!saved) {
        if (native && result.publication == 2) {
            if (hooks_.showStatus) {
                hooks_.showStatus(tr("Could not confirm publication of the copy %1.").arg(name), 6000);
            }
            return false;
        }
        const QString action = tr("save a copy of %1").arg(name);
        const bool legacyRefusal = !native && reportMemoryRefusal(dialogParent_, action);
        if (native && result.admission && hooks_.showStatus) {
            hooks_.showStatus(nativeAdmissionMessage(result.admission, action), 6000);
        } else if (!legacyRefusal && hooks_.showStatus) {
            const auto message = result.code == quint32(OperationStatus::Panic)
                ? tr("Could not save a copy of %1. The operation stopped unexpectedly.").arg(name)
                : tr("Could not save a copy of %1.").arg(name);
            hooks_.showStatus(message, 6000);
        }
        return false;
    }
    if (!pictura::native_save_warning(*view).isEmpty() && hooks_.showStatus) {
        hooks_.showStatus(pictura::native_save_warning(*view), 0);
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

bool ImportSaveCoordinator::saveAsWithDialog(bool backgroundAllowed)
{
    PictureView* view = documents_.activeView();
    if (!view || !view->has_document() || !pictura::compatibility_reason(*view).isEmpty()) {
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
    return saveActiveAsWithOptions(path, hint, backgroundAllowed);
}

bool ImportSaveCoordinator::saveActiveAsWithOptions(const QString& path, SaveFormatKind hint,
                                                    bool backgroundAllowed)
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
    return saveActiveAs(path, backgroundAllowed, includeMerged);
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

} // namespace pictura
