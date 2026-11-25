#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QHash>
#include <QtCore/QJsonObject>
#include <QtCore/QList>
#include <QtCore/QPointer>
#include <QtCore/QPoint>
#include <QtCore/QRectF>
#include <QtCore/QSet>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtWidgets/QMainWindow>
#include <functional>
#include <memory>
#include <utility>
#include "canvas_ruler.h"
#include "document_registry.h"
#include "panels/panel_column.h"
#include "preferences.h"
#include "quick_mask_ui.h"
#include "save_format_options.h"
#include "status_readouts.h"
#include "menu_registrar.h"
#include "tools.h"
#include "frame_forward.h"

namespace pictura {

// True when a launch should seed the scratch white document. Only the self-test
// path needs it (headless implies self-test), so a normal launch starts on the
// empty workspace with the document commands disabled.
bool launchCreatesScratchDocument(bool selfTest, bool codecLoaded);

// True while saveActive() is routing through Save As, so the nested Save As
// records only the origin Save step.
bool saveActiveInProgress();

// Factory panel-column layouts for the built-in workspaces (design D6). The
// name is matched exactly; an unknown name returns an empty array.
QJsonArray workspacePresets(const QString& name);

// The CS6-shaped application frame: menu bar, tabbed document area, status bar,
// and dock areas. Owns the UI and the open documents; each document's state
// lives in its own cxx-qt PictureView.
class PicturaMainWindow : public QMainWindow {
    Q_OBJECT
public:
    enum class ScreenMode { Standard, FullWithMenuBar, Full };
    explicit PicturaMainWindow(QWidget* parent = nullptr, bool enableUpdateCheck = true,
                               const QString& startupUiLanguage = {});
    ~PicturaMainWindow() override;
    // Test hooks.
    CommandRegistry* registry() const;
    DocumentRegistry* documentRegistry() const { return documents_.get(); }
    bool updateChecksEnabled() const { return updateChecksEnabled_; }
    ImageView* imageView() const;       // active canvas, or nullptr with no document
    PictureView* activeView() const;    // active document, or nullptr
    // The document shown by `canvas`, or nullptr for a canvas that is not an
    // open document. Used by the drop router for canvas drops.
    PictureView* viewForCanvas(const ImageView* canvas) const;
    // Make `view`'s document active when it is open.
    void activateView(PictureView* view);
    // Channels panel > Duplicate Channel… (also a test hook).
    void openDuplicateChannelDialog();
    // Channels panel > Channel Options… (also a test hook).
    void openChannelOptionsDialog();
    // Drop the cached alpha overlay so the next refresh redraws it.
    void invalidateMaskOverlay();
    // Drop the cached alpha overlay for `view`'s document on every window.
    void invalidateMaskOverlayFor(PictureView* view);
    // Creates two documents and drags the last tab before its neighbour through
    // the real tab bar; returns whether the tab windows followed the new visual
    // order while each document kept its index and the active view stayed.
    bool reorderDocumentsForTest();
    QStringList topLevelMenuTitles() const;
    bool registerPanel(QWidget* panel, Qt::DockWidgetArea area);
    QSet<QString> panelObjectNames() const;
    PanelColumn* panelColumn() const;
    PreferencesDialog* preferencesDialog() const { return preferencesDialog_; }
    void showPreferences(const QString& page);
    void saveSessionNow() { saveSession(); }
    bool saveAllDocumentsForRestart();
    // M43 multi-column host. Columns live in the central splitter around the
    // document tabs; a column's side is its splitter order relative to the
    // tabs, never its geometry. M44 W5: `anchor` inserts the new column
    // immediately before (side Left) or after (side Right) another column.
    PanelColumn* createPanelColumn(PanelSide side, PanelColumn* anchor = nullptr);
    void removeColumnIfEmpty(PanelColumn* column);
    PanelSide sideOf(const PanelColumn* column) const;
    int columnCount() const;
    QList<PanelColumn*> panelColumns() const;
    // Every column the frame owns, including a whole column torn off into a
    // floating overlay (which is no longer a splitter pane). Used where a
    // floating column must still be reachable: drop targeting, the Window >
    // Panels owner lookup, and session serialization.
    QList<PanelColumn*> allPanelColumns() const;
    PanelColumn* columnForPanel(const QString& objectName) const;
    // Show a hidden panel or hide a shown one (the options bar's panel toggles).
    void togglePanel(const QString& objectName);
    PanelColumn* columnAtGlobal(const QPoint& globalPos) const;
    // -1 = not a new-column candidate, 0 = left, 1 = right.
    int newColumnSideAt(const QPoint& globalPos) const;
    // Like `newColumnSideAt`, but a point on the desktop never resolves a dock
    // target; a floating Tools panel stays free unless dragged into the edge.
    int newColumnSideAtInsideWorkspace(const QPoint& globalPos) const;
    // M44 W5: a point in a band beside an existing column (other than `exclude`)
    // resolves to a new column anchored on that side; returns the anchor.
    PanelColumn* columnEdgeAnchorAt(const QPoint& globalPos, const PanelColumn* exclude,
                                    int* side) const;

    // M47: move a whole column beside an anchor (side 0 = left, 1 = right) using
    // the same splitter history as `createPanelColumn`. `resolveColumnMoveTarget`
    // returns the anchor for a header drag (null for a bare workspace edge).
    PanelColumn* resolveColumnMoveTarget(const QPoint& globalPos, const PanelColumn* exclude,
                                         int* side) const;
    bool movePanelColumn(PanelColumn* column, int side, PanelColumn* anchor);

    // A widget column's invisible resize grip drives the adjacent splitter
    // handle so the existing anchor logic (`onCenterSplitterMoved`) still keeps
    // the workspace-side column fixed. The tools column has no grip.
    void beginWidgetColumnResize(PanelColumn* column, int globalX);
    void updateWidgetColumnResize(int globalX);
    void endWidgetColumnResize();
    // The tabless, atomic Tools column hosted in the central splitter (default
    // left, index 0). Its content is the `toolbox_` widget.
    PanelColumn* toolsColumn() const;
    // M43 Phase B test hooks. All drive the same resolve/commit drag path.
    int panelColumnCountForTest() const { return columnCount(); }
    QString panelColumnSideForTest(int index) const;
    bool newColumnDropForTest(const QString& panelName, const QString& side);
    // M44 W5: drop a panel beside an existing column; the new column lands
    // immediately adjacent to the anchor.
    bool newColumnBesideForTest(const QString& panelName, const QString& anchorPanel);
    bool dropIntoGroupForTest(const QString& panelName, const QString& targetPanel, int index = 1);
    bool dropBoundaryForTest(const QString& panelName, const QString& targetPanel, bool above);
    // M43 Phase C test hook: re-runs the real startup restore path so a saved
    // layout can be applied and re-applied without a second frame.
    void applyPanelSessionForTest(const SessionState& state);
    // Applies a workspace-shaped column spec through the re-group-capable path.
    void applyWorkspaceLayoutForTest(const QJsonArray& columns);
    // Workspace controller test hooks; all bypass the modal dialogs.
    bool createWorkspaceForTest(const QString& name);
    bool deleteWorkspaceForTest(const QString& name);
    QString workspaceNameErrorForTest(const QString& name) const;
    void switchWorkspaceForTest(const QString& name);
    QString activeWorkspaceForTest() const;
    QStringList userWorkspacesForTest() const;
    QJsonArray workspaceLayoutForTest() const;
    void refreshWorkspaceMenuForTest();
    QStringList workspaceMenuLabelsForTest() const;
    int workspaceMenuSeparatorCountForTest() const;
    QStringList deleteWorkspaceChoicesForTest() const;
    // Rebuilds the pre-Essentials single-column default for the legacy self-test
    // panel machinery and returns the primary column. New default coverage lives
    // in the Qt Test suites.
    PanelColumn* restoreLegacyDefaultForTest();

    // Bumped when the chrome layout changes shape (M42 removed the old dock
    // set); a persisted layout from another revision is discarded on restore so
    // stale chrome cannot reappear over the menu bar. v9 moved Tools from a dock
    // into a central-splitter column, so the old layout is discarded.
    static constexpr int kLayoutRevision = 3;
    // The central-band frame width: 2 px of `${surface}` outside, 1 px `${border}`.
    static constexpr int kWorkspaceFrameWidth = 3;
    int layoutRevisionForTest() const { return kLayoutRevision; }
    bool restoreStoredLayout(const QByteArray& layout, int revision);

    // Tool test hooks.
    ToolId activeTool() const;
    void setActiveTool(ToolId id);
    QColor foregroundColor() const;
    QColor backgroundColor() const;
    // Before a paste: take another application's newer clipboard image
    // (the Vanishing Point dialog pastes too).
    void importSystemClipboard();
    bool hasPendingCrop() const;
    bool commitCrop();
    // A pending transform locks the Layer menu and greys the Layers panel.
    bool layersLockedForTest() const { return transformLocked_; }
    LayersPanel* layersPanelForTest() const;
    int dockRetargetCountForTest() const;

    int brightnessLevel() const { return brightnessLevel_; }
    void setBrightnessLevel(int level);

    // Theme family: 0 dark / 1 light. Switching applies the palette live and
    // persists the choice; the menu-bar theme button toggles it.
    int themeFamily() const { return themeFamily_; }
    void setThemeFamily(int family);

    // UI scale in percent (75..200); applies live and persists.
    int uiScalePercent() const { return uiScalePercent_; }
    void setUiScalePercent(int percent);

    ScreenMode screenMode() const { return screenMode_; }
    void setScreenMode(ScreenMode mode);
    void cycleScreenMode(bool forward);
    // The user-facing entry to the screen modes: on a Standard -> full-screen
    // transition it shows the CS6 advisory first (Cancel aborts), then applies.
    void requestScreenMode(ScreenMode mode);

    int canvasColorIndex() const { return canvasColorIndex_; }
    void cycleCanvasColor(bool forward);
    // Test hook: the built pasteboard colour menu, without popping it.
    QMenu* canvasBackgroundMenuForTest() { return canvasBackgroundMenu(); }

    bool panelsHidden() const;
    void setPanelsHidden(bool hidden);
    void refresh();
    void saveSession();
    // Hand a URL to the desktop opener, honouring the test seam.
    void openExternalUrl(const QUrl& url);
    void wireTypeRuntime(MenuRegistrar::Runtime& runtime);
    void setTypeShowAsianOptions(bool on);
    void setTypeMiddleEastern(bool on);
    void setFontPreviewSize(int size);
    // The per-format Save As options (ImportSaveCoordinator).
    bool runSaveFormatDialog(SaveFormatKind kind, SaveFormatOptions& options);
    void rememberSaveFormatOptions(SaveFormatKind kind, const SaveFormatOptions& options);
    void applySaveRasterOptions(PictureView& view, SaveFormatKind kind,
                                const SaveFormatOptions& options);
    // Apply a GPU-compute preference change to every open document and persist.
    void applyGpuComputePreference(bool on);

    // Crash recovery (frame_recovery.cpp). Off until main enables it for an
    // interactive launch, so tests and the self-test never touch the store
    // unless they ask.
    void enableRecovery();
    // Ask what to do with the documents crashed sessions left behind: recover
    // them, discard them, or keep them for the next launch.
    void offerRecovery();
    // Reopen orphaned documents as unsaved "-Recovered" tabs (failures: see
    // document/crash-recovery); returns how many reopened.
    int recoverOrphans();
    // The autosave tick: snapshot each document with unsaved changes made since
    // its last snapshot, and drop the snapshot of each document now saved.
    void autosaveRecovery();
    // Clean shutdown: finish snapshot writes and delete this session's store.
    void shutdownRecovery();
    void setAutoSaveRecovery(bool on, int minutes);
    bool autoSaveRecovery() const;
    int autoSaveMinutes() const;
    QString recoverySessionDirForTest() const;

    // Mirror the active document's selection outline onto its canvas (or clear
    // it). Called from refresh() and on toolbar selection commits; view-only.
    void refreshSelectionOverlay();
    // Show the pen-tip model only on the active document's canvas, while the
    // Brush or Pencil tool has a pen-tip preset.
    void syncBrushTipPreview();

    // Document operations.
    int documentCount() const;
    int activeDocumentIndex() const;
    void setActiveDocumentIndex(int index);
    PictureView* viewAt(int index) const;
    ImageView* canvasAt(int index) const;
    QString documentPath(int index) const;
    QString documentName(int index) const;
    QString documentTabTextForTest(int index) const;
    QStringList selectionDocumentLabelsForTest() const;
    // Give document `index` a display name, as a typed New or Duplicate name does.
    void setDocumentNameForTest(int index, const QString& name);
    bool isDocumentDirty(int index) const;
    QString activeFilePath() const;
    QString activeDocumentName() const;
    bool isActiveDirty() const;
    // True when no document is open, so a double-click on the empty workspace
    // opens the Open dialog.
    bool workspaceOpenArmed() const;

    using OpenOutcome = pictura::OpenOutcome;

    // Takes ownership of `view` and adds it as a document window: a tab, or a
    // floating window while Open Documents As Tabs is off unless `asTab`.
    int addDocument(PictureView* view, const QString& path, bool asTab = false);
    // Preferences > Interface: Open Documents As Tabs, and Enable Floating
    // Document Window Docking (a float dropped on the tab strip docks).
    bool openDocumentsAsTabs() const { return openDocumentsAsTabs_; }
    bool floatingDocumentDocking() const { return floatingDocumentDocking_; }
    // Preferences > General: Zoom Resizes Windows (View > Zoom In / Out fit a
    // floating window to the zoomed image).
    bool zoomResizesWindows() const { return zoomResizesWindows_; }
    bool newDocument(const QString& name, int width, int height, const QString& mode,
                     int depth, const QString& background);
    // Start an open and return its token for `documentOpenFinished`.
    quint64 startOpenPath(const QString& path);
    quint64 startOpenImagePath(const QString& path);
    quint64 startOpenAsSmartObjectPath(const QString& path);
    // Route by signature or suffix. Noninteractive PDF/EPS uses dialog defaults.
    quint64 startOpenDocumentAtPath(const QString& path, bool interactive = true,
                                    bool reportFailure = false);
    // Synchronous wrappers pump a local event loop until completion.
    bool openPath(const QString& path);
    bool openImagePath(const QString& path);
    bool openDocumentAtPath(const QString& path, bool interactive = true);
    OpenOutcome openDocumentAtPathOutcome(const QString& path, bool interactive = true,
                                          bool* memoryRefused = nullptr);
    // True when `path`'s suffix routes to the native PSD/PSB reader rather than
    // the Qt image decode edge.
    static bool isNativeDocumentPath(const QString& path);
    // True when a save to `path` goes through the shared PSD/PSB worker: every path that
    // is neither a raster writer nor PDF (an unknown suffix writes a PSD, as `format_for_path`).
    static bool saveRunsOnWorker(const QString& path);
    bool openAsSmartObjectPath(const QString& path);
    // Test seam: hold an open's completion `ms` ms (loading tab, spinner, hover, cancel).
    void setOpenDelayForTest(int ms);
    void setNextNativeOpenPanicForTest();
    // Pump events until every started open has finished or been cancelled (self-test drops).
    void waitForOpensForTest();
    bool isLoadingDocument(int index) const;
    bool editSmartObjectContents(const QString& layerPath);
    // Replace `layerPath`'s embedded source from a chosen PSD/PSB, or write it to a chosen
    // destination. Both prompt for the file and refuse without a usable target.
    bool replaceSmartObjectContents(const QString& layerPath);
    bool exportSmartObjectContents(const QString& layerPath);
    // Begin a Free Transform session on `path` in the active view and show its
    // overlay. Returns false without a transformable target.
    bool beginFreeTransform(const QString& path);
    // Begin a Skew / Distort / Perspective session on `path` and show its
    // overlay. Returns false without a transformable target or an unknown mode.
    bool beginTransformMode(const QString& path, const QString& mode);
    // Make `path` the active Layers-panel row (refresh first so a just-created
    // layer is present in the model). No-op without the panel or an empty path.
    void selectLayerPath(const QString& path);
    // The active document's selected layer paths and current row, from Rust.
    QStringList selectedLayerPaths() const;
    QString layerAnchor() const;
    // Copy the layer `dragged` of the open document `source` into the active
    // document, above its current layer, as one "Duplicate Layer" state. When
    // `dragged` belongs to the source's multi-layer target every selected layer
    // crosses. False when `source` is not another open document or the bridge
    // refuses the copy.
    bool copyLayersFromDocument(const void* source, const QString& dragged);
    bool saveActive(bool interactive = true);
    bool saveInBackground() const;
    bool saveActiveAs(const QString& path);
    // Prompt for a Save As path (format-aware filters, source format preselected) and write
    // through `saveActiveAs`. Warns before flattening a layered document into a raster format.
    bool saveAsWithDialog();
    // Save a known path with remembered options (a step's overlay them), no dialog.
    bool saveActiveAsWithRememberedOptions(const QString& path,
                                           const QJsonObject& stepOptions = {});
    // Prompt for a copy path with the same filters as Save As and write it
    // without changing the document's path or modified state.
    bool saveCopyWithDialog();
    // Prompt for the destination format's options, remember and store them on the view.
    // False aborts the save; a format that opens no dialog returns true.
    bool promptSaveFormatOptions(const QString& path,
                                 SaveFormatKind hint = SaveFormatKind::None);
    // Save As with a known path: prompt for the format options, then write.
    bool saveActiveAsWithOptions(const QString& path,
                                 SaveFormatKind hint = SaveFormatKind::None);
    // Test seam: replace the format options dialog; true proceeds with (edited) options.
    void setSaveFormatDialogForTest(std::function<bool(SaveFormatKind, SaveFormatOptions&)> hook);
    void setGifDialogForTest(std::function<bool(PictureView*, SaveFormatOptions&)> hook);
    void setWarningDriverForTest(std::function<bool(const QString&, const QString&)> driver);
    // Test seam: answer the Embed Color Profile dialog and the feature warning.
    void setProfileDialogForTest(std::function<bool(bool, bool&)> hook);
    void setFeatureWarningForTest(std::function<bool()> hook);
    void setGifDialogDriverForTest(std::function<bool(IndexedColorDialog&)> driver);
    bool revertActive();
    // frame_pdf.cpp: Photoshop PDF save through the Save Adobe PDF dialog,
    // reopening a Pictura Photoshop PDF, and Edit > Adobe PDF Presets.
    static bool isPdfPath(const QString& path);
    bool savePdfWithDialog(const QString& path, bool copy = false);
    quint64 startOpenPdfPath(const QString& path, bool interactive = true);
    void showPdfPresets();
    // frame_import.cpp: a generic PDF through the password prompt and Import PDF dialog, an
    // EPS through Rasterize Generic EPS Format, and File > Open As with a chosen decoder
    // (index into openAsFileFilters()). Each opened page or image is an untitled document.
    quint64 startImportEpsPath(const QString& path, bool interactive = true);
    void showOpenAsDialog();
    quint64 startOpenAsPath(const QString& path, int formatIndex);
    bool openAsPath(const QString& path, int formatIndex);
    // Test seam: answer the import dialogs (password prompt, Import PDF,
    // Rasterize Generic EPS Format, the warnings box).
    void setImportDialogDriverForTest(std::function<bool(QDialog&)> driver);
    // Test seams: answer the PDF dialogs (Save Adobe PDF, features warning, presets manager
    // and editor) and catch View PDF After Saving instead of launching the system viewer.
    void setPdfDialogDriverForTest(std::function<bool(QDialog&)> driver);
    void setPdfViewerForTest(std::function<void(const QString&)> viewer);
    // Test seam: catch a desktop URL (Reveal in File Manager, File > Browse…)
    // instead of opening it.
    void setUrlOpenerForTest(std::function<void(const QUrl&)> opener)
    {
        urlOpener_ = std::move(opener);
    }
    // Document window test hooks (frame_test.cpp), by document index, on its first window:
    // its floating window (null unless it floats), float or dock it as the menus do, its tab
    // index (-1 unless tabbed), and its full label.
    DocumentFloatWindow* floatWindowForTest(int index) const;
    bool floatDocumentForTest(int index);
    bool dockDocumentForTest(int index, int tab);
    int tabOfDocumentForTest(int index) const;
    QString documentWindowLabelForTest(int index) const;
    // The tile host while Tile lasts (null otherwise), and document `index`'s
    // first window's tile pane (null unless it is tiled).
    DocumentTileHost* tileHostForTest() const;
    DocumentTilePane* tilePaneForTest(int index) const;
    // The tab or header context menu last opened, while it is open.
    QMenu* documentMenuForTest() const;
    // Window `ordinal` of document `index`, counting its windows in label order from 0: the
    // window count, the window's canvas, full label, tab index (-1 unless tabbed), and
    // floating window (null unless it floats); activate it.
    int windowCountForTest(int index) const;
    ImageView* windowCanvasForTest(int index, int ordinal) const;
    QString windowLabelForTest(int index, int ordinal) const;
    int windowTabForTest(int index, int ordinal) const;
    DocumentFloatWindow* windowFloatForTest(int index, int ordinal) const;
    void activateWindowForTest(int index, int ordinal);
    // While set, a floating window's dock drag (Ctrl+drag, or the carried window drag) hands
    // its drag data to `hook` instead of running the blocking `QDrag::exec`.
    void setWindowDockDragForTest(std::function<void(std::unique_ptr<QMimeData>)> hook);
    bool closeWindow(DocWindow* window, bool interactive = true);
    bool closeDocument(int index, bool interactive);
    // The recent-files list, most recent first; setting it persists it.
    const QStringList& recentFiles() const;
    void setRecentFiles(const QStringList& files);
    // File > Open Recent, or null before the menu bar is built.
    QMenu* recentMenu() const;
    bool closeActiveDocument(bool interactive);
    // Cancel every in-flight open (window close, File > Exit), discarding the loading tabs.
    void cancelAllOpens();
    void showNewDocumentDialog();
    void showOpenDialog();
    void openUserPaths(const QStringList& paths); // File > Open; records Open steps.
    ActionsPanel* actionsPanel() const;
    BatchHost& batch(); // File > Automate > Batch.
    void setSuppressProfileWarnings(bool suppress);
    void showFileInfo();
    // Assign (`convert == false`) or convert to a chosen built-in profile; no-op
    // unless the active view holds an RGB document.
    void showProfileCommand(bool convert);
    // Edit > Color Settings…: show the sRGB working space and incoming-profile
    // policy; on OK persist the choice and apply it to every open view.
    void showColorSettings();
    void wireColorManagementMenu();
    void applyColorSettings(PictureView* view);
    bool confirmProfileForOpen(PictureView* view, const QString& path);
    // Help > Developer > Design System…: the in-app storybook over the real
    // widgets (modeless; the single instance is reused).
    void showDesignSystemDialog();
    // Help > System Info: collect a snapshot and show/refresh the report.
    void showSystemInfo();
    // Help > Developer > Pictura Metrics: show or hide the floating metrics panel;
    // hidden means no timer and no sample (the panel is built on first use).
    void setDebugToolsetEnabled(bool enabled);
    bool debugToolsetEnabled() const;
    void syncDebugToolsetCheck();
    // Help > Submit Issue: copy the report and open the repository's new-issue
    // page prefilled with it.
    void submitIssue();
    // The top-right menu-bar chrome: the theme toggle and the GitHub / Discord
    // links.
    void installMenuCornerButtons();
    void updateThemeCornerIcon();

Q_SIGNALS:
    // GUI-thread completion with an OpenOutcome, status code, and failure detail.
    void documentOpenFinished(quint64 token, int outcome, const QString& error,
                              quint32 code, bool memoryRefused);

protected:
    void closeEvent(QCloseEvent* event) override;
    void keyPressEvent(QKeyEvent* event) override;
    void keyReleaseEvent(QKeyEvent* event) override;
    bool eventFilter(QObject* watched, QEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;
    void showEvent(QShowEvent* event) override;

private:
    // Keys the menu shortcuts miss: Ctrl+= / Ctrl+Shift+= / keypad Ctrl++
    // zoom in, and Delete / Backspace clear a selection. True when handled.
    bool dispatchCommandKey(QKeyEvent* event);
    // The window-level Delete / Backspace action: clear a pixel selection or
    // delete the selected layer(s), unless a type session, an editable field,
    // or the active tool owns the key. True when handled. Routed from the
    // window shortcuts so the focus location is irrelevant, and from
    // `dispatchCommandKey` so synthetic events reach the same path.
    bool handleDeleteKey();
    // The canvas right-click menu: the active tool's menu over the document,
    // or the pasteboard colour menu on the background. Non-modal, so it never
    // blocks the event loop.
    void showCanvasContextMenu(ImageView* canvas, const QPoint& globalPos);
    // The pasteboard colour menu, built from the same palette `Space+F` cycles.
    QMenu* canvasBackgroundMenu();
    void showCanvasBackgroundMenu(const QPoint& globalPos);
    // Leave isolation mode if active; true when it was. Escape routes here.
    bool releaseIsolation();

    // `toolOptions` is the session's saved tool options, restored before the
    // options bar is built so its controls start from them.
    void buildTools(int toolsColumns, bool useShiftKeyForToolSwitch,
                    const QJsonObject& toolOptions = QJsonObject());
    void buildStatusBar();
    // Filter the tab pane so a double-click on the empty workspace (no document
    // open) opens the Open dialog.
    void installWorkspaceOpenGesture();
    void wirePdfCommands();
    void openSaveSelectionDialog();
    void openLoadSelectionDialog();
    void applyTextGamma(bool enabled, double gamma);
    // frame_mask_overlay.cpp: re-render the rubylith overlay when the Layers
    // panel changes it, and draw the view's overlay on `canvas`.
    void wireMaskOverlay();
    void wireQuickMask();
    void syncQuickMask();
    void showQuickMaskOptions();
    void syncMaskOverlay(PictureView* view, ImageView* canvas);
    // PanelHost forwarders used by the remaining chrome translation units.
    void layoutWorkspaceFrame();
    void updateColumnFrameEdges();
    void refreshPanels();
    void updateStatus();
    void updateToolHint();
    void updateTabTitle(int index);
    void updateWindowTitle();
    void onAboutToRemoveDocument(int index);
    // Restore the persisted window geometry (and maximized state) at startup, or
    // fall back to the default size when the stored position is off-screen.
    void restoreSessionWindowGeometry(const SessionState& state);
    // A canvas zoom changed: refresh the footer readout and that tab's title.
    void onCanvasZoomChanged();
    // frame_canvas.cpp: a window's canvas and scroll bars, and their
    // connections; a ready document's view connections, made once.
    void setupCanvas(DocWindow& window);
    void wireCanvas(DocWindow& window);
    void wireDocument(DocEntry& doc);
    // Run `fn` on the canvas of each of `doc`'s windows: the document-wide
    // pushes (pixels, overlays, guides) reach every window, active or not.
    void forEachCanvas(const DocEntry& doc, const std::function<void(ImageView*)>& fn) const;
    // The active document's canvases, the active window's first; empty with
    // no document. A filter dialog previews on all of them.
    QList<ImageView*> activeDocumentCanvases() const;
    int indexOfView(PictureView* view) const;
    // DocumentRegistry forwarders used by the frame's presentation code.
    int indexOfDoc(const DocEntry* doc) const;
    DocWindow* windowAtTab(int tab) const;
    int tabOf(const DocWindow* window) const;
    DocWindow* windowOfCanvas(const ImageView* canvas) const;
    void presentWindows();
    void syncScreenModeWindows();
    void applyBrightness(int level);
    // Re-scale the explicit chrome that the stylesheet and app font cannot
    // reach: status bar, menu-corner icons, tool slots, panel columns/groups.
    void applyUiScaleToChrome(int oldPercent, int newPercent);
    void applyStatusBarScale();
    QColor canvasColorForIndex(int index) const;
    void applyWorkspaceCanvasColor();
    // The mode `cycleScreenMode` steps to, and the panel toggles: Standard and
    // Full Screen With Menu Bar flip the persisted `panelsHidden_`; Full Screen
    // Mode flips the transient `fullScreenChromeShown_`, which also controls the
    // menu bar, so the first Tab reveals the chrome.
    ScreenMode nextScreenMode(bool forward) const;
    void togglePanelsHidden();
    std::unique_ptr<DocumentRegistry> documents_;
    std::unique_ptr<MenuRegistrar> menu_;
    std::unique_ptr<PanelHost> panels_;
    std::unique_ptr<ImportSaveCoordinator> importSave_;
    BatchHost* batch_ = nullptr;
    // Disconnected in the destructor: the focusChanged handler must not run
    // while the window's children are being torn down.
    QMetaObject::Connection spacePanFocusConn_;
    QMetaObject::Connection deleteShortcutsFocusConn_;
    FileDropRouter* fileDropRouter_ = nullptr;
    // A pending (uncommitted, modal) transform: the Layer menu and Layers panel
    // are locked and a tool switch asks "Apply the transformation?".
    void installTransformLock();
    void syncTransformLock();
    bool transformLocked_ = false;
    // The pasteboard colour menu, created lazily and popped up non-modally.
    QMenu* backgroundMenu_ = nullptr;
    // The FG/BG pair of the edit target that is not active: the image's while
    // a mask is edited, the mask's (initially white/black) otherwise.
    QColor otherForeground_{Qt::white};
    QColor otherBackground_{Qt::black};
    PreferencesDialog* preferencesDialog_ = nullptr;
    QString startupUiLanguage_;
    HistoryLogSettings historyLog_;
    void restoreHistoryLog(const Preferences& prefs);
    QPointer<DesignSystemDialog> designSystemDialog_;
    QPointer<SystemInfoDialog> systemInfoDialog_;
    DebugToolsetPanel* debugToolset_ = nullptr;
    ToolController* tools_ = nullptr;
    QuickMaskUiState quickMaskUi_;

    OptionsBar* optionsBar_ = nullptr;
    Toolbox* toolbox_ = nullptr;
    NumericField* zoomField_ = nullptr;
    QLabel* sizeLabel_ = nullptr;
    QProgressBar* backgroundSaveProgress_ = nullptr;
    ToolHintBar* hintBar_ = nullptr;
    // The rule between the GPU readout and the hint strip; hidden with the strip.
    QFrame* hintSeparator_ = nullptr;
    QLabel* backendLabel_ = nullptr;
    QLabel* actionLabel_ = nullptr;
    // The rule before the action label; hidden with the label.
    QFrame* actionSeparator_ = nullptr;
    QToolButton* statusOptionsButton_ = nullptr;
    QString statusReadout_ = QStringLiteral("sizes");
    // Scratch Sizes and Efficiency go stale without an event: this refreshes
    // them while one is shown (#636).
    QTimer* statusReadoutTimer_ = nullptr;
    EfficiencyMeter efficiency_;
    bool hintsVisible_ = true;
    // Free Pan allows off-centre panning when the document fits the viewport.
    bool freePan_ = true;
    // Snap Canvas To Workspace centres a fitting document and hides its scrollbars.
    bool snapCanvasToWorkspace_ = false;
    int brightnessLevel_ = 1;
    int themeFamily_ = 0;
    Preferences preferencesBaseline_;
    int uiScalePercent_ = 100;
    QToolButton* themeCornerButton_ = nullptr;
    bool gpuCompute_ = true;
    int colorPolicy_ = 0;
    int cmykPolicy_ = 0;
    int workingCmyk_ = 0;
    int renderIntent_ = 1;
    // View > Proof Setup / Proof Colors (frame_proof.cpp).
    int proofDevice_ = 0;
    int proofIntent_ = 1;
    bool proofBpc_ = true;
    bool proofSimulatePaper_ = false;
    bool proofSimulateBlack_ = false;
    bool proofColors_ = false;
    bool blackPointCompensation_ = true;
    bool useDither_ = true;
    bool askMismatchOpen_ = false;
    bool askMissingOpen_ = false;
    int pickerChoice_ = 0;
    bool textGammaEnabled_ = true;
    double textGamma_ = 1.45;
    bool typeShowAsianOptions_ = false;
    bool typeMiddleEastern_ = false;
    bool typeMissingGlyphProtection_ = true;
    int fontPreviewSize_ = 2;
    QString typeDefaultsJson_;
    bool gpuAvailable_ = true;
    bool gpuCandidatesAvailable_ = false;
    bool useShiftKeyForToolSwitch_ = true;
    bool openDocumentsAsTabs_ = true;
    bool floatingDocumentDocking_ = true;
    bool zoomResizesWindows_ = false;
    bool updateChecksEnabled_ = true;
    // Set while a Hand pan is fanned out to the other windows, so the fan-out
    // never echoes.
    ScreenMode screenMode_ = ScreenMode::Standard;
    int canvasColorIndex_ = 0;
    // The chrome reveal Tab toggles while in Full Screen Mode (the panel
    // columns and the menu bar); transient, so leaving the mode restores the
    // Standard-mode chrome unchanged.
    bool fullScreenChromeShown_ = false;
    // The coordinator owns crash recovery and autosave state.
    void connectRecoveryPreferences(PreferencesDialog* dialog);
    // Frame-owned chrome/settings commands, registered through MenuRegistrar's
    // callback so the menu translation units never name PicturaMainWindow.
    void registerFrameCommands(CommandRegistry& registry);
    CommandRegistry* commandRegistry() const;

    // Ruler and guide commands, view state, and preferences (frame_guides.cpp).
    void registerGuideHandlers();
    void registerProofHandlers();
    void applyProofState(PictureView* view);
    void showProofSetupCustom();
    void setProofColors(bool on);
    QString proofSetupName() const;
    void loadGuideSession(const SessionState& state, const Preferences& prefs);
    void loadTypePreferences(const Preferences& prefs);
    void prepareCanvasGuides(const DocWindow& window);
    // Give the active document's other windows its guides; the tools keep the
    // active canvas's guides.
    void syncSiblingGuides();
    void applyGuideViewState();
    void connectGuidePreferences(PreferencesDialog* dialog);
    // View > Show, Extras, grid, layer edges and target path (frame_view.cpp).
    void loadViewSession(const SessionState& state);
    void applyViewState();
    void applyCanvasViewState(const DocWindow& window);
    void applyGridState(const DocWindow& window);
    void refreshTargetPathOverlay(const DocWindow& window);
    void refreshLayerEdges(DocEntry& doc);
    void refreshDocumentSelection(DocEntry& doc);
    void refreshViewOverlays();
    void wireViewStateSignals();
    void registerViewHandlers();
    void showExtrasOptions();
    bool extraMember(const QString& id) const;
    bool effectiveExtra(const QString& id, bool enabled, bool toolOwns = false) const;
    bool pathToolActive() const;
    // Units & Rulers: from the preferences page or a ruler's context menu.
    void setRulerUnit(RulerUnit unit);
    // frame_preferences.cpp: the Edit > Preferences leaves.
    void registerPreferenceHandlers();
    // Command and tool shortcut overrides (frame_shortcuts.cpp).
    void registerShortcutHandlers();
    void applyShortcutOverrides(const Preferences& prefs);
    void rebuildToolShortcuts();
    void showKeyboardShortcuts();
    QList<QShortcut*> toolKeyShortcuts_;
    // frame_search.cpp: Help > Search (Shift+Space), the overlay it opens, and
    // the status-bar hint that names its binding.
    void registerSearchHandlers();
    void openSearch();
    void runSearchEntry(const SearchEntry& entry);
    void revealSearchEntry(const SearchEntry& entry);
    // Show a panel wherever its column lives; the panel, or null when unknown.
    QWidget* showPanelForSearch(const QString& objectName);
    // A curated control's widget, after bringing its tool page or panel up.
    QWidget* searchControlWidget(const SearchEntry& entry);
    void buildSearchHint(QStatusBar* bar);
    void updateSearchHint();
    SearchBar* searchBar_ = nullptr;
    ToolHintBar* searchHint_ = nullptr;
    bool rulersVisible_ = false;
    RulerUnit rulerUnit_ = RulerUnit::Inches;
    bool traditionalPoints_ = false;
    bool guidesVisible_ = true;
    bool guidesLocked_ = false;
    bool smartGuidesVisible_ = true;
    QColor guideColor_{0x4a, 0xff, 0xff};
    bool guideDashed_ = false;
    // View > Show overlay state and the Extras master (frame_view.cpp).
    bool selectionEdges_ = true;
    bool showTargetPath_ = true;
    bool showLayerEdges_ = true;
    bool showGrid_ = false;
    bool showCount_ = true;
    bool showSlices_ = true;
    bool showNotes_ = true;
    bool showPixelGrid_ = true;
    bool showColorSamplers_ = true;
    bool extras_ = true;
    QStringList extrasMembers_;
    bool lockSlices_ = false;
    QString gridColor_ = QStringLiteral("#c0c0c0");
    int gridStyle_ = 0;
    int gridUnit_ = 1;
    double gridEvery_ = 1.0;
    int gridSubdivisions_ = 4;
    bool viewStateSignalsWired_ = false;
    std::function<void(const QUrl&)> urlOpener_;
};
} // namespace pictura
