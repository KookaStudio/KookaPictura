#pragma once

#include <QtCore/QHash>
#include <QtCore/QJsonObject>
#include <QtCore/QObject>
#include <QtCore/QPointer>
#include <QtCore/QSet>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QColor>

#include <functional>
#include <memory>
#include <optional>

#include "document_window.h"
#include "recovery_store.h"
#include "save_format_options.h"

class QDialog;
class QTemporaryDir;
class QTimer;
class QWidget;

namespace pictura {

class DocumentRegistry;
class PictureView;
class IndexedColorDialog;
struct SessionState;

// Owns the file seam: the async open pipeline and its tokens/probes, the
// save/save-as/revert and PDF/EPS/Open As import paths, the recent list,
// save-format options, smart-object edit sessions, and crash recovery. The
// frame keeps the chrome and forwards its entry points here.
class ImportSaveCoordinator : public QObject {
    Q_OBJECT

public:
    struct Hooks {
        // Frame-owned view setup; open and recovered views get it before they
        // are handed to the registry.
        std::function<void(PictureView*)> applyColorSettings;
        std::function<bool()> gpuCompute;
        std::function<void()> requestRefresh;
        std::function<void()> updateWindowTitle;
        std::function<void(const QString& message, int timeoutMs)> showStatus;
        std::function<void(const QString& commandId, const QJsonObject& values)> recordCommand;
        // Persist the whole session when available; without it the coordinator
        // rewrites only its own slice of the stored state.
        std::function<void()> saveSession;
        std::function<QColor()> foregroundColor;
        std::function<QColor()> backgroundColor;
    };

    ImportSaveCoordinator(DocumentRegistry& documents, QWidget* dialogParent,
                          Hooks hooks = {}, QObject* parent = nullptr);
    ~ImportSaveCoordinator() override;

    // The session slice this collaborator owns.
    void saveState(SessionState& state) const;
    void loadState(const SessionState& state);

    // Open entry points. A started open returns its token; completion arrives
    // on documentOpenFinished.
    quint64 startOpenPath(const QString& path);
    quint64 startOpenImagePath(const QString& path);
    quint64 startOpenAsSmartObjectPath(const QString& path);
    quint64 startOpenDocumentAtPath(const QString& path, bool interactive = true,
                                    bool reportFailure = false);
    bool openPath(const QString& path);
    bool openImagePath(const QString& path);
    bool openDocumentAtPath(const QString& path, bool interactive = true);
    // A user-initiated open (File > Open, Open Recent): a successful open is
    // recorded as an Open action step.
    quint64 startUserOpen(const QString& path, bool reportFailure = false);
    // Batch's Suppress Color Profile Warnings: keep the embedded profile
    // without the mismatch/missing prompt while set.
    void setSuppressProfileWarnings(bool suppress) { suppressProfileWarnings_ = suppress; }
    OpenOutcome openDocumentAtPathOutcome(const QString& path, bool interactive = true);
    bool openAsSmartObjectPath(const QString& path);
    bool confirmProfileForOpen(PictureView* view, const QString& path);
    void wireOpenSignals(PictureView* view);
    void setOpenDelayForTest(int ms) { openDelayForTest_ = ms; }
    int openDelayForTest() const { return openDelayForTest_; }
    void waitForOpensForTest();
    void cancelAllOpens();
    void cancelOpenForView(PictureView* view);

    static bool isNativeDocumentPath(const QString& path);
    static bool saveRunsOnWorker(const QString& path);
    static bool isPdfPath(const QString& path);

    // Save / close.
    bool saveActive(bool interactive = true);
    bool saveActiveAs(const QString& path, std::optional<bool> includeMerged = std::nullopt);
    // Save a known path with the remembered options for its suffix (no dialog):
    // the path used by the control server and recorded-command replay. A
    // recorded Save As step's `stepOptions` overlay the remembered options for
    // this save without being remembered.
    bool saveActiveAtPath(const QString& path, const QJsonObject& stepOptions = {});
    bool saveAsWithDialog();
    bool saveActiveAsWithOptions(const QString& path,
                                 SaveFormatKind hint = SaveFormatKind::None);
    bool saveCopyWithDialog();
    bool saveCopyAs(const QString& path, std::optional<bool> includeMerged = std::nullopt);
    bool saveCopyAsWithOptions(const QString& path,
                               SaveFormatKind hint = SaveFormatKind::None);
    bool confirmFeatureWarning(const QString& path, PictureView& view, const QString& title);
    bool promptSaveFormatOptions(const QString& path,
                                 SaveFormatKind hint = SaveFormatKind::None);
    bool runSaveFormatDialog(SaveFormatKind kind, SaveFormatOptions& options);
    void rememberSaveFormatOptions(SaveFormatKind kind, const SaveFormatOptions& options);
    void applySaveRasterOptions(PictureView& view, SaveFormatKind kind,
                                const SaveFormatOptions& options);
    bool revertActive();
    bool saveAllDocumentsForRestart();
    bool closeDocument(int index, bool interactive);
    bool closeActiveDocument(bool interactive);
    bool editSmartObjectContents(const QString& layerPath);

    // PDF.
    bool savePdfWithDialog(const QString& path, bool copy = false);
    quint64 startOpenPdfPath(const QString& path, bool interactive = true);
    void showPdfPresets();

    // Generic PDF/EPS import and File > Open As.
    quint64 startImportEpsPath(const QString& path, bool interactive = true);
    void showOpenAsDialog();
    quint64 startOpenAsPath(const QString& path, int formatIndex);
    bool openAsPath(const QString& path, int formatIndex);

    // Recent files (the menu actions stay on the frame until the registrar).
    const QStringList& recentFiles() const { return recent_; }
    void setRecentFiles(const QStringList& files);
    void rememberRecent(const QString& path);
    void openRecent(const QString& path);

    // Crash recovery.
    void enableRecovery();
    // Start tracking `view`'s unsaved changes for the autosave tick.
    void trackRecovery(PictureView* view);
    void setAutoSaveRecovery(bool on, int minutes);
    bool autoSaveRecovery() const { return autoSaveRecovery_; }
    int autoSaveMinutes() const { return autoSaveMinutes_; }
    int nativeSaveCompatibility() const { return static_cast<int>(nativeSaveCompatibility_); }
    void setNativeSaveCompatibility(int choice);
    void offerRecovery();
    int recoverOrphans();
    void autosaveRecovery();
    void shutdownRecovery();
    QString recoverySessionDirForTest() const;

    // Test seams.
    void setSaveFormatDialogForTest(
        std::function<bool(SaveFormatKind, SaveFormatOptions&)> hook)
    {
        saveFormatDialogForTest_ = std::move(hook);
    }
    void setGifDialogForTest(std::function<bool(PictureView*, SaveFormatOptions&)> hook)
    {
        gifDialogForTest_ = std::move(hook);
    }
    void setWarningDriverForTest(std::function<bool(const QString&, const QString&)> driver)
    {
        warningDriverForTest_ = std::move(driver);
    }
    void setPdfDialogDriverForTest(std::function<bool(QDialog&)> driver)
    {
        pdfDialogDriver_ = std::move(driver);
    }
    // Test seam: answer the Embed Color Profile dialog; false cancels.
    void setProfileDialogForTest(std::function<bool(bool, bool&)> hook)
    {
        profileDialogForTest_ = std::move(hook);
    }
    // Test seam: answer the feature-loss warning; false cancels.
    void setFeatureWarningForTest(std::function<bool()> hook)
    {
        featureWarningForTest_ = std::move(hook);
    }
    // Test seam: answer the GIF Indexed Color dialog; false cancels.
    void setGifDialogDriverForTest(std::function<bool(IndexedColorDialog&)> driver)
    {
        gifDialogDriver_ = std::move(driver);
    }
    void setPdfViewerForTest(std::function<void(const QString&)> viewer)
    {
        pdfViewer_ = std::move(viewer);
    }
    void setImportDialogDriverForTest(std::function<bool(QDialog&)> driver)
    {
        importDialogDriver_ = std::move(driver);
    }

Q_SIGNALS:
    void documentOpenFinished(quint64 token, int outcome, const QString& error);

private:
    enum class NativeSaveCompatibility { Always, Ask, Never };
    std::optional<bool> chooseMergedImage(const QString& path, const PictureView& view);
    bool confirmSaveProfile(const QString& path, const PictureView& view,
                            std::optional<bool>& embed);
    // A PDF probe that has no tab yet. `token` is the open the caller waits on;
    // `password` is the one already accepted, if any.
    struct PdfProbe {
        quint64 token = 0;
        QString path;
        bool interactive = true;
        QString password;
    };

    // One open Edit Contents editor: the untitled tab, the document that owns
    // the edited layer, and the per-session temporary file holding the source.
    struct SmartObjectEditSession {
        PictureView* editor = nullptr;
        PictureView* origin = nullptr;
        QString layerPath;
        QString filename;
        QTemporaryDir* temp = nullptr;
    };

    quint64 nextOpenToken();
    int indexOfView(PictureView* view) const;
    void beginOpenCursor();
    void endOpenCursor();
    void finishLoadedDocument(int index);
    void finishOpen(quint64 token, OpenOutcome outcome, const QString& error = QString());
    OpenOutcome waitForOpen(quint64 token);
    void cancelOpen(int index);
    void restorePreviousTab(PictureView* view);
    void onOpenDetected(PictureView* view);
    void onOpenProgress(PictureView* view, int percent, const QString& phase);
    void onOpenFinished(PictureView* view, const QString& status,
                        const QStringList& warnings);
    void reportOpenFailure(int index, const QString& status);
    void handlePdfProbeFinished(PictureView* view, const PdfProbe& probe,
                                const QString& status, const QStringList& warnings);
    void startPdfProbe(const QString& path, bool interactive, const QString& password,
                       quint64 token);
    OpenOutcome importPdfPathWithToken(const QString& path, bool interactive,
                                       const QString& knownPassword, quint64 token);
    bool runPdfDialog(QDialog& dialog);
    bool runImportDialog(QDialog& dialog);
    bool askPdfPassword(const QString& path, bool refused, QString& password);
    void reportImportError(const QString& path, const QString& reason);
    void alertMissingFonts(PictureView* view);
    void onAboutToRemoveDocument(int index);

    void dropRecoverySnapshot(PictureView* view);
    void forgetRecovery(PictureView* view);
    void recordCommand(const char* id, const QJsonObject& values);

    DocumentRegistry& documents_;
    QWidget* dialogParent_ = nullptr;
    Hooks hooks_;

    QList<SmartObjectEditSession> editSessions_;
    // PDF probes that have no tab yet, keyed by their temporary view.
    QHash<PictureView*, PdfProbe> pdfProbes_;
    // Views whose open signals are already wired (a PDF probe's view becomes a
    // tab, so it is wired once).
    QSet<PictureView*> openWired_;
    // Per-open side data that must outlive the tab's removal.
    QHash<quint64, QString> openSourcePaths_;
    QHash<quint64, QStringList> openWarnings_;
    // A per-open failure message for routes that must not fail silently (Open
    // As's "format mismatch or damaged file.").
    QHash<quint64, QString> openFailureMessages_;
    QSet<quint64> reportFailureTokens_;
    QSet<quint64> finishedOpens_;
    QHash<quint64, QString> recordOpenPaths_;
    bool suppressProfileWarnings_ = false;
    // Opens that own a busy-cursor reference (the outer open, not a PDF page).
    QSet<quint64> cursorTokens_;
    quint64 nextOpenToken_ = 1;
    int openCount_ = 0;
    int openDelayForTest_ = 0;

    QJsonObject saveFormatOptions_;
    QHash<PictureView*, SaveFormatKind> lastSaveKinds_;
    std::function<bool(SaveFormatKind, SaveFormatOptions&)> saveFormatDialogForTest_;
    std::function<bool(PictureView*, SaveFormatOptions&)> gifDialogForTest_;
    std::function<bool(const QString&, const QString&)> warningDriverForTest_;
    std::function<bool(bool, bool&)> profileDialogForTest_;
    std::function<bool()> featureWarningForTest_;
    std::function<bool(IndexedColorDialog&)> gifDialogDriver_;
    std::function<bool(QDialog&)> pdfDialogDriver_;
    std::function<bool(QDialog&)> importDialogDriver_;
    std::function<void(const QString&)> pdfViewer_;

    QStringList recent_;

    std::unique_ptr<RecoveryStore> recovery_;
    QTimer* autosaveTimer_ = nullptr;
    QHash<PictureView*, int> recoveryIds_;
    QSet<PictureView*> recoveryStale_;
    QSet<PictureView*> recoverySnapshots_;
    int recoveryCounter_ = 0;
    bool autoSaveRecovery_ = true;
    int autoSaveMinutes_ = 10;
    NativeSaveCompatibility nativeSaveCompatibility_ = NativeSaveCompatibility::Always;
};

} // namespace pictura
