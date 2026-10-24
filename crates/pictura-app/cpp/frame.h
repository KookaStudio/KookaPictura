#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QHash>
#include <QtCore/QJsonObject>
#include <QtCore/QList>
#include <QtCore/QPointer>
#include <QtCore/QPoint>
#include <QtCore/QSet>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtWidgets/QMainWindow>

#include <functional>
#include <memory>

#include "canvas_ruler.h"
#include "document_window.h"
#include "panels/panel_column.h"
#include "recovery_store.h"
#include "tools.h"
#include "workspace_store.h"

class QDialog;

class QDockWidget;
class QFrame;
class QLabel;
class QListWidget;
class QMimeData;
class QResizeEvent;
class QShortcut;
class QShowEvent;
class QSplitter;
class QStackedWidget;
class QStatusBar;
class QTabWidget;
class QTemporaryDir;
class QTimer;
class QToolButton;
class QUrl;
class QWidget;

namespace pictura {

class ActionsEngineBridge;
class ActionsPanel;
class CommandRegistry;
class ColorPanel;
class ChannelsPanel;
class ColorState;
class CanvasScrollBars;
class DesignSystemDialog;
class DocumentFloatWindow;
class DocumentTileHost;
class DocumentTilePane;
class FileDropRouter;
class LoadingCard;
class HistogramPanel;
class HistoryPanel;
class BrushPanel;
class CloneSourcePanel;
class CharacterPanel;
class ParagraphPanel;
class GlyphsPanel;
class ParagraphStylesPanel;
class CharacterStylesPanel;
class BrushTipPreview;
class ImageView;
class InfoPanel;
class LayersPanel;
class NavigatorPanel;
class NotesPanel;
class NumericField;
class ColorRangeDialog;
class ReplaceColorDialog;
class PathsPanel;
class OptionsBar;
class PanelColumn;
class PictureView;
class PlaceholderPanel;
class AdjustmentsPanel;
class PreferencesDialog;
class PropertiesPanel;
class SearchBar;
struct SearchEntry;
class SystemInfoDialog;
class SwatchesPanel;
class Toolbox;
class ToolHintBar;
struct FilterCommandSpec;
struct SessionState;

// True when a launch should seed the scratch white document. Only the self-test
// path needs it (headless implies self-test), so a normal launch starts on the
// empty workspace with the document commands disabled.
bool launchCreatesScratchDocument(bool selfTest, bool codecLoaded);

// True while saveActive() is routing through Save As, so the nested Save As
// records only the origin Save step.
bool saveActiveInProgress();

// A Window > Panels (or Type > Panels) command and the panel it toggles.
struct PanelToggle {
    const char* command;
    const char* panel;
};
const QList<PanelToggle>& panelToggles();

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

    explicit PicturaMainWindow(QWidget* parent = nullptr);
    ~PicturaMainWindow() override;

    // Test hooks.
    CommandRegistry* registry() const { return registry_; }
    ImageView* imageView() const;       // active canvas, or nullptr with no document
    PictureView* activeView() const;    // active document, or nullptr
    // Creates two documents and drags the last tab before its neighbour through
    // the real tab bar; returns whether the tab windows followed the new visual
    // order while each document kept its index and the active view stayed.
    bool reorderDocumentsForTest();
    QStringList topLevelMenuTitles() const;
    bool registerPanel(QWidget* panel, Qt::DockWidgetArea area);
    const QSet<QString>& panelObjectNames() const { return panelNames_; }
    PanelColumn* panelColumn() const { return panelColumn_; }
    PreferencesDialog* preferencesDialog() const { return preferencesDialog_; }

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
    PanelColumn* toolsColumn() const { return toolsColumn_; }

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
    void applyPanelSessionForTest(const SessionState& state) { applyPanelSession(state); }
    // Applies a workspace-shaped column spec through the re-group-capable path.
    void applyWorkspaceLayoutForTest(const QJsonArray& columns) { regroupAndApply(columns); }
    // Workspace controller test hooks; all bypass the modal dialogs.
    bool createWorkspaceForTest(const QString& name) { return createWorkspace(name); }
    bool deleteWorkspaceForTest(const QString& name) { return deleteWorkspace(name); }
    QString workspaceNameErrorForTest(const QString& name) const
    {
        return workspaceNameError(name);
    }
    void switchWorkspaceForTest(const QString& name) { switchWorkspace(name); }
    QString activeWorkspaceForTest() const { return workspaceStore_.activeWorkspace; }
    QStringList userWorkspacesForTest() const;
    QJsonArray workspaceLayoutForTest() const { return serializeWorkspaceLayout(); }
    void refreshWorkspaceMenuForTest();
    QStringList workspaceMenuLabelsForTest() const;
    int workspaceMenuSeparatorCountForTest() const;
    QStringList deleteWorkspaceChoicesForTest() const { return deleteWorkspaceChoices(); }
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
    // Before a paste: take another application's newer clipboard image
    // (the Vanishing Point dialog pastes too).
    void importSystemClipboard();
    bool hasPendingCrop() const;
    bool commitCrop();
    // A pending transform locks the Layer menu and greys the Layers panel.
    bool layersLockedForTest() const { return transformLocked_; }
    LayersPanel* layersPanelForTest() const { return layersPanel_; }

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

    bool panelsHidden() const { return panelsHidden_; }
    void setPanelsHidden(bool hidden);

    void refresh();
    void saveSession();
    // Apply a GPU-compute preference change to every open document and persist.
    void applyGpuComputePreference(bool on);

    // Crash recovery (frame_recovery.cpp). Off until main enables it for an
    // interactive launch, so tests and the self-test never touch the store
    // unless they ask.
    void enableRecovery();
    // Ask what to do with the documents crashed sessions left behind: recover
    // them, discard them, or keep them for the next launch.
    void offerRecovery();
    // Reopen every orphaned document as an unsaved "-Recovered" tab and discard
    // the orphaned sessions; returns how many reopened.
    int recoverOrphans();
    // The autosave tick: snapshot each document with unsaved changes made since
    // its last snapshot, and drop the snapshot of each document now saved.
    void autosaveRecovery();
    // Clean shutdown: finish snapshot writes and delete this session's store.
    void shutdownRecovery();
    void setAutoSaveRecovery(bool on, int minutes);
    bool autoSaveRecovery() const { return autoSaveRecovery_; }
    int autoSaveMinutes() const { return autoSaveMinutes_; }
    QString recoverySessionDirForTest() const;

    // Mirror the active document's selection outline onto its canvas (or clear
    // it). Called from refresh() and on toolbar selection commits; view-only.
    void refreshSelectionOverlay();
    // Show the pen-tip model only on the active document's canvas, while the
    // Brush or Pencil tool has a pen-tip preset.
    void syncBrushTipPreview();

    // Document operations.
    int documentCount() const { return docs_.size(); }
    int activeDocumentIndex() const;
    void setActiveDocumentIndex(int index);
    PictureView* viewAt(int index) const;
    ImageView* canvasAt(int index) const;
    QString documentPath(int index) const;
    QString documentName(int index) const;
    QString documentTabTextForTest(int index) const;
    // Give document `index` a display name, as a typed New or Duplicate name does.
    void setDocumentNameForTest(int index, const QString& name);
    bool isDocumentDirty(int index) const;
    QString activeFilePath() const;
    QString activeDocumentName() const;
    bool isActiveDirty() const;
    // True when no document is open, so a double-click on the empty workspace
    // opens the Open dialog.
    bool workspaceOpenArmed() const { return docs_.isEmpty(); }

    // How an open ended. A user cancel is not a failure: callers must not show
    // an error for it.
    enum class OpenOutcome { Opened, Cancelled, Failed };

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
    // Start an open and return its token; completion arrives on
    // `documentOpenFinished`. The loading tab is inserted immediately (or, for
    // a PDF probe, when the worker finds editing data). `reportFailure` shows
    // the Open Recent status message on failure.
    quint64 startOpenPath(const QString& path);
    quint64 startOpenImagePath(const QString& path);
    quint64 startOpenAsSmartObjectPath(const QString& path);
    // Route `path` by signature (PDF, EPS) or suffix (PSD/PSB, Qt raster
    // import). With `interactive` off (agentic control) a PDF or EPS imports
    // with the dialogs' defaults and no prompt.
    quint64 startOpenDocumentAtPath(const QString& path, bool interactive = true,
                                    bool reportFailure = false);
    // Synchronous wrappers for the self-test, the control server, and tests:
    // start the open and pump a local event loop until it finishes.
    bool openPath(const QString& path);
    bool openImagePath(const QString& path);
    bool openDocumentAtPath(const QString& path, bool interactive = true);
    OpenOutcome openDocumentAtPathOutcome(const QString& path, bool interactive = true);
    // True when `path`'s suffix routes to the native PSD/PSB reader rather than
    // the Qt image decode edge.
    static bool isNativeDocumentPath(const QString& path);
    bool openAsSmartObjectPath(const QString& path);
    // Test seam: hold a started open's completion for `ms` milliseconds so the
    // loading tab, spinner, hover, and cancel are observable.
    void setOpenDelayForTest(int ms) { openDelayForTest_ = ms; }
    // Pump events until every started open has finished or been cancelled. The
    // self-test's drop checks need the asynchronous opens to land.
    void waitForOpensForTest();
    bool isLoadingDocument(int index) const;
    bool editSmartObjectContents(const QString& layerPath);
    // Replace the embedded source of `layerPath` from a chosen PSD/PSB file, or
    // write that source to a chosen destination. Both refuse without a
    // replaceable/exportable target and prompt for the file.
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
    // Copy the layer node at `path` of the open document whose address is
    // `source` into the active document, above its current layer, as one
    // "Duplicate Layer" state. False when `source` is not another open document
    // or the bridge refuses the copy.
    bool copyLayerFromDocument(const void* source, const QString& path);
    bool saveActive();
    bool saveActiveAs(const QString& path);
    // Prompt for a Save As path with the format-aware filters (preselecting the
    // active document's source format) and write through `saveActiveAs`. Warns
    // before flattening a layered document into a raster format.
    bool saveAsWithDialog();
    bool revertActive();
    // frame_pdf.cpp: Photoshop PDF save through the Save Adobe PDF dialog,
    // reopening a Pictura Photoshop PDF, and Edit > Adobe PDF Presets.
    static bool isPdfPath(const QString& path);
    bool savePdfWithDialog(const QString& path);
    quint64 startOpenPdfPath(const QString& path, bool interactive = true);
    void showPdfPresets();
    // frame_import.cpp: a generic PDF through the password prompt and the
    // Import PDF dialog, an EPS through the Rasterize Generic EPS Format
    // dialog, and File > Open As with a chosen decoder (index into
    // openAsFileFilters()). Each opened page or image is an untitled document.
    // Ask for the password of a protected PDF (`refused`: the last one was
    // wrong); false on Cancel.
    bool askPdfPassword(const QString& path, bool refused, QString& password);
    quint64 startImportEpsPath(const QString& path, bool interactive = true);
    void showOpenAsDialog();
    quint64 startOpenAsPath(const QString& path, int formatIndex);
    bool openAsPath(const QString& path, int formatIndex);
    // Test seam: answer the import dialogs (password prompt, Import PDF,
    // Rasterize Generic EPS Format, the warnings box).
    void setImportDialogDriverForTest(std::function<bool(QDialog&)> driver)
    {
        importDialogDriver_ = std::move(driver);
    }
    // Test seams: answer the PDF dialogs (the Save Adobe PDF dialog, the
    // features warning, the presets manager and its editor) and catch View PDF
    // After Saving instead of launching the system viewer.
    void setPdfDialogDriverForTest(std::function<bool(QDialog&)> driver)
    {
        pdfDialogDriver_ = std::move(driver);
    }
    void setPdfViewerForTest(std::function<void(const QString&)> viewer)
    {
        pdfViewer_ = std::move(viewer);
    }
    // Test seam: catch Reveal in File Manager's folder URL instead of opening
    // the desktop file manager.
    void setUrlOpenerForTest(std::function<void(const QUrl&)> opener)
    {
        urlOpener_ = std::move(opener);
    }
    // Document window test hooks (frame_document_windows.cpp), by document
    // index and acting on the document's first window: its floating window
    // (null unless it floats), float or dock it as the menus do, its tab index
    // (-1 unless tabbed), and its full label.
    DocumentFloatWindow* floatWindowForTest(int index) const;
    bool floatDocumentForTest(int index);
    bool dockDocumentForTest(int index, int tab);
    int tabOfDocumentForTest(int index) const;
    QString documentWindowLabelForTest(int index) const;
    // The tile host while Tile lasts (null otherwise), and document `index`'s
    // first window's tile pane (null unless it is tiled).
    DocumentTileHost* tileHostForTest() const { return tileHost_; }
    DocumentTilePane* tilePaneForTest(int index) const;
    // The tab or header context menu last opened, while it is open.
    QMenu* documentMenuForTest() const;
    // Window `ordinal` of document `index`, counting its windows in label
    // order from 0: how many windows the document has, the window's canvas,
    // full label, tab index (-1 unless tabbed), and floating window (null
    // unless it floats); activate it.
    int windowCountForTest(int index) const;
    ImageView* windowCanvasForTest(int index, int ordinal) const;
    QString windowLabelForTest(int index, int ordinal) const;
    int windowTabForTest(int index, int ordinal) const;
    DocumentFloatWindow* windowFloatForTest(int index, int ordinal) const;
    void activateWindowForTest(int index, int ordinal);
    // While set, a floating window's dock drag (Ctrl+drag, or the carried
    // window drag) hands its drag data to `hook` instead of running the
    // blocking `QDrag::exec`.
    void setWindowDockDragForTest(std::function<void(std::unique_ptr<QMimeData>)> hook)
    {
        windowDockDragForTest_ = std::move(hook);
    }
    bool closeDocument(int index, bool interactive);
    // The recent-files list, most recent first; setting it persists it.
    const QStringList& recentFiles() const { return recent_; }
    void setRecentFiles(const QStringList& files);
    // File > Open Recent, or null before the menu bar is built.
    QMenu* recentMenu() const;
    bool closeActiveDocument(bool interactive);
    // Cancel every in-flight open (window close, File > Exit). The loading tabs
    // disappear and their workers' results are discarded.
    void cancelAllOpens();
    void showNewDocumentDialog();
    void showOpenDialog();
    void showFileInfo();
    // Assign (`convert == false`) or convert to a chosen built-in profile; no-op
    // unless the active view holds an RGB document.
    void showProfileCommand(bool convert);
    // Edit > Color Settings…: show the sRGB working space and incoming-profile
    // policy; on OK persist the choice and apply it to every open view.
    void showColorSettings();
    // Help > Developer > Design System…: the in-app storybook over the real
    // widgets (modeless; the single instance is reused).
    void showDesignSystemDialog();
    // Help > System Info: collect a snapshot and show/refresh the report.
    void showSystemInfo();
    // Help > Submit Issue: copy the report and open the repository's new-issue
    // page prefilled with it.
    void submitIssue();
    // The top-right menu-bar chrome: the theme toggle and the GitHub / Discord
    // links.
    void installMenuCornerButtons();
    void updateThemeCornerIcon();

Q_SIGNALS:
    // Emitted on the GUI thread when a started open ends. `outcome` is
    // `OpenOutcome`; `error` is the failure message (empty for success/cancel).
    void documentOpenFinished(quint64 token, int outcome, const QString& error);

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

    void buildMenus();
    void buildUiSizeMenu();
    void updateUiSizeMenu();
    void buildPanels(const SessionState& state);
    // `toolOptions` is the session's saved tool options, restored before the
    // options bar is built so its controls start from them.
    void buildTools(int toolsColumns, bool useShiftKeyForToolSwitch,
                    const QJsonObject& toolOptions = QJsonObject());
    void buildDocumentTabs();
    void buildStatusBar();
    // Filter the tab pane so a double-click on the empty workspace (no document
    // open) opens the Open dialog.
    void installWorkspaceOpenGesture();
    void registerHandlers();
    void wirePdfCommands();
    bool runPdfDialog(QDialog& dialog);
    bool runImportDialog(QDialog& dialog);
    void reportImportError(const QString& path, const QString& reason);
    void registerSelectHandlers();
    void registerEditHandlers();
    // Wire the CS6 Filter menu: one handler per implemented filter, plus Last
    // Filter / Last Filter Settings.
    void wireFilterMenu();
    void applyFilterCommand(const FilterCommandSpec& spec);
    // Surface why a filter was refused (status bar + stderr) instead of a
    // silent no-op.
    void reportFilterRefusal(PictureView* view);
    // frame_menus_align.cpp: Layer > Align / Align Layers To Selection /
    // Distribute, and the Move tool's buttons, over the selected layers.
    void wireAlignMenu();
    void wireTypeMenu();
    void wireWarpText();
    void wireReplaceMissingFonts();
    void alertMissingFonts(PictureView* view);
    void applyTextGamma(bool enabled, double gamma);
    // frame_menus_layer_style.cpp: Layer > Layer Style, the Layer Style dialog
    // and the style commands over the Layers panel's selection.
    void wireLayerStyleMenu();
    // frame_menus_image.cpp: Image > Crop, Trim, and Duplicate.
    void wireImageMenu();
    // frame_menus_transform.cpp: Edit > Transform Scale/Rotate/Again, the
    // exact rotations, and the flips.
    void wireTransformMenu();
    // frame_menus_edit.cpp: Edit > Undo/Redo/Step, honoring the active tool's
    // modal session before the document history.
    void wireEditHistory();
    // frame_menus_image_mode.cpp: Image > Mode, the color-mode and bit-depth
    // conversions with their prompts and dialogs.
    void wireImageModeMenu();
    void convertImageMode(const QString& mode);
    void convertImageDepth(int bits);
    // frame_menus_adjust.cpp: Image > Adjustments and the Image > Auto commands.
    void wireImageAdjustments();
    // frame_menus_adjust.cpp: Layer > New Adjustment Layer, the sixteen
    // adjustment-layer kinds over the active document.
    void wireLayerAdjustments();
    // frame_menus_adjust.cpp: Layer > Layer Content Options, which opens the
    // active fill/adjustment layer's Properties page.
    void wireLayerContentOptions();
    // frame_menus_layer_ops.cpp: Layer > Arrange / Reverse, Delete Layer, and
    // the non-destructive Stamp Visible / Stamp Selected.
    void wireLayerArrangeStamp();
    // frame_menus_layer_ops.cpp: Layer > Rasterize Shape and Copy/Paste Shape
    // Attributes over the current shape layer.
    void wireShapeLayerActions();
    // frame_menus_layer_ops.cpp: the Layer > Vector Mask submenu and
    // Layer > Rasterize > Vector Mask over the active layer.
    void wireVectorMaskActions();
    // frame_menus_matting.cpp: Layer > Matting (Defringe / Remove Black Matte /
    // Remove White Matte) over the active layer.
    void wireMattingActions();
    // frame_menus_smart_object.cpp: Layer > Smart Objects > Reset Transform /
    // Convert to Layers / New Smart Object via Copy over the active layer.
    void wireAdvancedSmartObjectActions();
    // frame_menus_smart_object.cpp: run the Layers row menu's Edit / Replace /
    // Export Contents actions on the row's path.
    void wireSmartObjectRowMenu();
    // frame_mask_overlay.cpp: re-render the rubylith overlay when the Layers
    // panel changes it, and draw the view's overlay on `canvas`.
    void wireMaskOverlay();
    void syncMaskOverlay(PictureView* view, ImageView* canvas);
    // frame_menus_smart_object.cpp: Layer > Smart Filter > Clear Smart Filters.
    void wireSmartFilterActions();
    void alignSelectedLayers(int edge, bool toSelection, int alignTo = 0);
    void distributeSelectedLayers(int edge);
    void updateAlignControls();
    void exportClipboard();
    void applyPanelSession(const SessionState& state);
    // Rebuild the widget columns from a workspace-shaped spec, moving panels
    // between groups and restoring floating groups/columns. Shared by session
    // restore and workspace apply.
    void regroupAndApply(const QJsonArray& columns, bool persist = true);
    // A float restored while the frame is hidden waits for `showEvent` so its
    // stored position is interpreted against the restored window geometry.
    void queueOrRestoreFloat(PanelColumn* column, PanelGroup* group, const QRect& geometry);
    void applyPendingFloatRestores();
    // Serialize the live widget columns into the same `panelColumns` shape
    // saveSession persists, shared by the session and the workspace store.
    QJsonArray serializeWorkspaceLayout() const;
    // Load/reconcile the workspace store; `sessionHadLayout` says whether the
    // session carried a panel layout to adopt into the active workspace, so a
    // lost session state does not clobber the active workspace's saved layout.
    void initWorkspaces(bool sessionHadLayout);
    // Attach the Window > Workspace menu to the Options-bar switcher.
    void attachWorkspaceSwitcher();
    // Keep the Options-bar switcher's value in step with the active workspace.
    void updateWorkspaceSwitcher();
    // Snapshot the active workspace's live arrangement into the store; called on
    // session save so the arrangement is durable independent of state.json.
    void snapshotActiveWorkspace();
    void switchWorkspace(const QString& name);
    void resetWorkspace();
    bool createWorkspace(const QString& name);
    bool deleteWorkspace(const QString& name);
    // Human message for a rejected workspace name, or "" when it is valid.
    QString workspaceNameError(const QString& name) const;
    // Wire the Window > Workspace commands and the dynamic user workspace menu.
    void wireWorkspaceCommands();
    void refreshWorkspaceMenu(QMenu* menu);
    int workspaceIndexOf(const QString& name) const;
    QMenu* workspaceMenu() const;
    // Every workspace but the active one; the Delete chooser's list.
    QStringList deleteWorkspaceChoices() const;
    // Apply the Essentials factory layout when the session has no saved or
    // legacy panel layout (a fresh session); a stored layout is left untouched.
    void applyFreshSessionDefault(const SessionState& state);
    void wirePanelColumn(PanelColumn* column);
    void clearDynamicColumns();
    void reapplyColumnStretch();
    // Center-splitter drag: keep the workspace-side column fixed between two
    // widget columns, then coalesce a session save.
    void onCenterSplitterMoved(int pos, int index);
    // Draw the 3 px central-band frame (2 px panel + 1 px border on each edge)
    // below the menu bar and above the status bar, and suppress the outermost
    // column's frame-side border so it does not double the 1 px line.
    void layoutWorkspaceFrame();
    void updateColumnFrameEdges();
    void showPreferences(const QString& page);
    void retargetDock();
    void refreshPanels();
    void updateStatus();
    void updateToolHint();
    void updateTabTitle(int index);
    void updateWindowTitle();
    // Restore the persisted window geometry (and maximized state) at startup, or
    // fall back to the default size when the stored position is off-screen.
    void restoreSessionWindowGeometry(const SessionState& state);
    // A canvas zoom changed: refresh the footer readout and that tab's title.
    void onCanvasZoomChanged();
    void removeDocument(int index);
    void rememberRecent(const QString& path);
    // Repopulate File > Open Recent from `recent_` each time it opens.
    void refreshRecentMenu(QMenu* menu);
    // frame_open.cpp: the loading-tab lifecycle and the async open entry points.
    quint64 nextOpenToken();
    void beginOpenCursor();
    void endOpenCursor();
    void wireOpenSignals(PictureView* view);
    void beginLoadingDocument(PictureView* view, const QString& path,
                              const QString& displayName, quint64 token,
                              const QString& recentPath, OpenKind kind);
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
    void installCloseButton(DocWindow* window);
    void finishLoadingDocument(int index);
    void restorePreviousTab(PictureView* view);
    void cancelOpen(int index);
    void cancelOpenForView(PictureView* view);
    void onOpenDetected(PictureView* view);
    void onOpenFinished(PictureView* view, const QString& status,
                        const QStringList& warnings);
    void onOpenProgress(PictureView* view, int percent, const QString& phase);
    void handlePdfProbeFinished(PictureView* view, const PdfProbe& probe,
                                const QString& status, const QStringList& warnings);
    void startPdfProbe(const QString& path, bool interactive, const QString& password,
                       quint64 token);
    OpenOutcome importPdfPathWithToken(const QString& path, bool interactive,
                                       const QString& knownPassword, quint64 token);
    void finishOpen(quint64 token, OpenOutcome outcome, const QString& error = QString());
    OpenOutcome waitForOpen(quint64 token);
    int indexOfView(PictureView* view) const;
    // frame_document_windows.cpp: the document window registry.
    int indexOfDoc(const DocEntry* doc) const;
    DocWindow* windowAtTab(int tab) const;
    int tabOf(const DocWindow* window) const;
    DocWindow* windowOfCanvas(const ImageView* canvas) const;
    // Register `window` as the last tab and return its tab index.
    int addTabWindow(DocWindow* window);
    // Host a ready window the tab strip holds: a tile pane while Tile lasts,
    // otherwise as Open Documents As Tabs says (it floats while the preference
    // is off), activated as `floatWindow` / `tileWindow` do. The one place
    // that picks a new ready window's host.
    void hostReadyWindow(DocWindow* window, bool activate);
    // Make `window` the active one through its host.
    void activateDocWindow(DocWindow* window);
    // Window > Arrange > New Window For: another window of `doc` on a fresh
    // canvas, hosted as a new ready window is, and activated. Refuses a
    // loading document.
    bool addDocumentWindow(DocEntry* doc);
    // True while the tab strip's current tab decides the active window: no
    // window is active, or the active one is a tab.
    bool stripOwnsActivation() const;
    void onTabCurrentChanged(int tab);
    void onTabBarClicked(int tab);
    // Move `window` into a floating document window, at `globalGeometry` (an
    // empty rect places it in the document area), and activate it unless
    // `activate` is off and it was not active. Refuses a loading document or a
    // window that already floats.
    bool floatWindow(DocWindow* window, const QRect& globalGeometry = QRect(),
                     bool activate = true);
    // Move a floating window or tile pane into the tab strip at `index` (out
    // of range: the end) and activate it.
    bool dockWindow(DocWindow* window, int index);
    // Move `window` into a tile pane after the others, starting the tile
    // layout if none is active, and activate it unless `activate` is off and
    // it was not active. Refuses a loading document or a window already tiled.
    bool tileWindow(DocWindow* window, bool activate = true);
    // Take a tiled window's page out of its pane; the last pane leaving ends
    // the tile layout and the document area shows the tab strip again.
    void untileWindow(DocWindow* window);
    // Close `window` under the tab close rules: the last window of its
    // document closes the document (prompting when modified); another window
    // closes without a prompt.
    bool closeWindow(DocWindow* window, bool interactive = true);
    // Take `windows` out of the registry and their hosts, handing the active
    // window to the strip's current tab or another window when it leaves.
    void detachWindows(const QList<DocWindow*>& windows);
    // Delete a detached window's widgets (its page, canvas host, and float).
    void destroyWindowWidgets(DocWindow* window);
    // The document area in global coordinates, and `geometry` moved and
    // shrunk so the window's header lies inside it.
    QRect documentAreaGlobalRect() const;
    QRect clampFloatGeometry(const QRect& geometry) const;
    // A float's size and place when nothing gives one: 60% of the document
    // area, centred on it.
    QRect defaultFloatGeometry() const;
    // Keep a child-hosted float under the floating panel overlays.
    void raiseDocumentFloat(DocumentFloatWindow* floating);
    // The dock zone in global coordinates: the band along the top of the tab
    // widget, as high as the tab bar while it shows tabs and otherwise as high
    // as a tab bar would be, a header high at least. Empty while docking is
    // off, while tiled, or while the tab widget is not the document area's
    // page.
    QRect dockZoneGlobalRect() const;
    // The tab index a dock at `globalPos` inserts at; -1 for the end.
    int dockIndexAt(const QPoint& globalPos) const;
    // A float's header drag ended: dock it when released over the dock zone.
    void onFloatDragReleased(DocWindow* window, const QPoint& globalPos);
    // Present every window whose document's frame revision moved.
    void presentWindows();
    // The screen modes: in a full-screen mode only the active window shows,
    // and an active float's page fills the document area without its header,
    // as an active tile pane does; Standard shows every float and pane again
    // where it was.
    void syncScreenModeWindows();
    // frame_document_menus.cpp: the tab and header context menus, the tab
    // strip's tear-out, and the menus' actions.
    void wireDocumentMenus();
    void showTabMenu(const QPoint& barPos);
    void showHeaderMenu(DocWindow* window, const QPoint& globalPos);
    bool tearOutFilter(QEvent* event);
    // A floating window's dock drag: carry `window` as drag-and-drop under the
    // private document-window MIME type. With `carryWindow` the drag also names
    // the float's top-level, held at `grabOffset`, so the compositor moves it
    // with the pointer (Wayland). The strip filter docks it where it is
    // dropped on the dock zone and claims no other drag, so file and layer
    // drops pass on to `FileDropRouter`.
    void startWindowDockDrag(DocWindow* window, bool carryWindow = false,
                             const QPoint& grabOffset = QPoint());
    DocWindow* windowOfDockMime(const QMimeData* mime) const;
    bool windowDropFilter(QObject* watched, QEvent* event);
    // The dock line: shown for a dock drag at `globalPos` while it is in the
    // dock zone (vertical at the insertion point between tabs, horizontal
    // along the band without tabs), hidden outside. Whether it shows.
    bool updateDockIndicator(const QPoint& globalPos);
    void hideDockIndicator();
    // Dock the window a drop recorded, once no platform drag is running.
    void applyPendingDock();
    // Dock every floating window, then activate `target` (null: keep the
    // active window).
    void consolidateAllToTabs(DocWindow* target);
    void closeOtherWindows(DocWindow* keep);
    void revealInFileManager(const DocWindow* window);
    // frame_document_arrange.cpp: the Window > Arrange commands. Cascade
    // places window `slot` (in cycle order) stepped from the document area's
    // top-left, restarting there when a header would leave the area; Float All
    // places newly floated windows the same way. The Match commands copy the
    // active canvas's zoom and/or centre (as document fractions) to every other
    // window.
    void wireWindowArrange();
    QRect cascadeGeometry(int slot) const;
    void floatAllWindows();
    void cascadeWindows();
    void tileWindows();
    void matchWindows(bool zoom, bool location);
    // The Hand and Zoom tools' multi-window options. A Hand drag on `source`
    // pans every other visible window by `delta` under Scroll All Windows or
    // Shift; a Zoom tool zoom applies `factor` to every other window when
    // `allWindows` and fits a floating active window when `resizeToFit`.
    void scrollOtherWindows(const ImageView* source, const QPointF& delta,
                            Qt::KeyboardModifiers mods);
    void onToolZoomed(double factor, bool allWindows, bool resizeToFit);
    // Resize a floating `window` so its canvas shows the whole zoomed image,
    // kept inside the document area; other hosts keep their size.
    void resizeWindowToFit(DocWindow* window);
    // Ctrl+Tab / Shift+Ctrl+Tab: step the active window `step` places along
    // `windows_`, wrapping; nothing with fewer than two windows.
    void wireDocumentCycling();
    void cycleDocumentWindow(int step);
    // The active window changed: settle loading entries and transform
    // sessions of the other documents, then refresh.
    void onActiveWindowChanged();
    void reportOpenFailure(int index, const QString& status);
    void openRecent(const QString& path);
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
    void setPanelColumnsVisible(bool visible);

    // The open documents, in open order; the public index API indexes them.
    QList<DocEntry*> docs_;
    // Every document window in cycle order: the tab windows in their visual
    // order first, then the windows of the other hosts.
    QList<DocWindow*> windows_;
    // The window the commands and tools target; null with no document.
    DocWindow* activeWindow_ = nullptr;
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
    // Opens that own a busy-cursor reference (the outer open, not a PDF page).
    QSet<quint64> cursorTokens_;
    quint64 nextOpenToken_ = 1;
    int openCount_ = 0;
    int openDelayForTest_ = 0;
    // Set while the frame changes tabs itself, so a loading entry does not read
    // the programmatic switch as a user selection.
    bool suppressTabTracking_ = false;
    QTabWidget* tabs_ = nullptr;
    // The splitter's stretch pane: the stack that holds the document hosts.
    QStackedWidget* documentArea_ = nullptr;
    // The document area's tile layout while Tile lasts; null otherwise.
    DocumentTileHost* tileHost_ = nullptr;
    // Set while a window moves between hosts, so the strip's own current-tab
    // changes do not take the activation; the move settles it.
    bool movingHost_ = false;
    // A tab strip press that may become a tear-out, and the float a tear-out
    // handed the drag to.
    DocWindow* tearWindow_ = nullptr;
    QPoint tearPressPos_;
    QPointer<DocumentFloatWindow> tearFloat_;
    bool tearSynthesizing_ = false;
    std::function<void(std::unique_ptr<QMimeData>)> windowDockDragForTest_;
    // The frame's dock line, made on first use.
    QWidget* dockIndicator_ = nullptr;
    // A drop on the dock zone, docked once the drag that carries the window
    // has finished: the float must outlive a running platform drag.
    DocWindow* pendingDockWindow_ = nullptr;
    int pendingDockIndex_ = -1;
    bool windowDockDragRunning_ = false;
    QPointer<QMenu> documentMenu_;
    // Disconnected in the destructor: the focusChanged handler must not run
    // while the window's children are being torn down.
    QMetaObject::Connection spacePanFocusConn_;
    QMetaObject::Connection deleteShortcutsFocusConn_;
    FileDropRouter* fileDropRouter_ = nullptr;
    QSplitter* centerSplitter_ = nullptr;
    // The 3 px central-band frame lines (outer 2 px is the window `${surface}`).
    QWidget* workspaceFrameLeft_ = nullptr;
    QWidget* workspaceFrameRight_ = nullptr;
    QTimer* panelRefreshTimer_ = nullptr;
    QTimer* sessionSaveTimer_ = nullptr;
    // M49: while dragging the divider between two widget columns, keep the
    // workspace-side column's width fixed and let the workspace absorb the delta.
    PanelColumn* resizeAnchorColumn_ = nullptr;
    int resizeAnchorWidth_ = -1;
    bool adjustingColumns_ = false;
    // A grip drag on a widget column: the splitter handle it drives, the press
    // global x, and the pane sizes at press so each move maps from the origin.
    int resizeHandleIndex_ = -1;
    int resizeStartGlobalX_ = 0;
    QList<int> resizeStartSizes_;
    CommandRegistry* registry_ = nullptr;
    LayersPanel* layersPanel_ = nullptr;
    // A pending (uncommitted, modal) transform: the Layer menu and Layers panel
    // are locked and a tool switch asks "Apply the transformation?".
    void installTransformLock();
    void syncTransformLock();
    bool transformLocked_ = false;
    // The pasteboard colour menu, created lazily and popped up non-modally.
    QMenu* backgroundMenu_ = nullptr;
    HistoryPanel* historyPanel_ = nullptr;
    NavigatorPanel* navigatorPanel_ = nullptr;
    ColorState* colorState_ = nullptr;
    // The FG/BG pair of the edit target that is not active: the image's while
    // a mask is edited, the mask's (initially white/black) otherwise.
    QColor otherForeground_{Qt::white};
    QColor otherBackground_{Qt::black};
    ColorPanel* colorPanel_ = nullptr;
    QPointer<ColorRangeDialog> colorRangeDialog_;
    QPointer<ReplaceColorDialog> replaceColorDialog_;
    SwatchesPanel* swatchesPanel_ = nullptr;
    InfoPanel* infoPanel_ = nullptr;
    HistogramPanel* histogramPanel_ = nullptr;
    PlaceholderPanel* gradientsPanel_ = nullptr;
    PlaceholderPanel* patternsPanel_ = nullptr;
    NotesPanel* notesPanel_ = nullptr;
    BrushPanel* brushPanel_ = nullptr;
    CloneSourcePanel* cloneSourcePanel_ = nullptr;
    CharacterPanel* characterPanel_ = nullptr;
    ParagraphPanel* paragraphPanel_ = nullptr;
    GlyphsPanel* glyphsPanel_ = nullptr;
    ParagraphStylesPanel* paragraphStylesPanel_ = nullptr;
    CharacterStylesPanel* characterStylesPanel_ = nullptr;
    PropertiesPanel* propertiesPanel_ = nullptr;
    AdjustmentsPanel* adjustmentsPanel_ = nullptr;
    ChannelsPanel* channelsPanel_ = nullptr;
    PathsPanel* pathsPanel_ = nullptr;
    ActionsPanel* actionsPanel_ = nullptr;
    ActionsEngineBridge* actionsEngine_ = nullptr;
    PlaceholderPanel* stylesPanel_ = nullptr;
    PanelColumn* panelColumn_ = nullptr;
    // Named workspaces and the active workspace, independent of the session
    // store. `workspacesInitialized_` gates snapshots made during construction
    // before the store is loaded.
    WorkspaceStore workspaceStore_;
    bool workspacesInitialized_ = false;
    // Dynamic user-workspace actions the Window > Workspace menu rebuilds.
    QList<QAction*> workspaceMenuActions_;
    PreferencesDialog* preferencesDialog_ = nullptr;
    QPointer<DesignSystemDialog> designSystemDialog_;
    QPointer<SystemInfoDialog> systemInfoDialog_;
    ToolController* tools_ = nullptr;
    OptionsBar* optionsBar_ = nullptr;
    Toolbox* toolbox_ = nullptr;
    // The tabless, atomic Tools column that hosts `toolbox_` in the splitter.
    PanelColumn* toolsColumn_ = nullptr;
    NumericField* zoomField_ = nullptr;
    QLabel* sizeLabel_ = nullptr;
    ToolHintBar* hintBar_ = nullptr;
    // The rule between the GPU readout and the hint strip; hidden with the strip.
    QFrame* hintSeparator_ = nullptr;
    QLabel* backendLabel_ = nullptr;
    QLabel* actionLabel_ = nullptr;
    // The rule before the action label; hidden with the label.
    QFrame* actionSeparator_ = nullptr;
    QToolButton* statusOptionsButton_ = nullptr;
    QString statusReadout_ = QStringLiteral("sizes");
    bool hintsVisible_ = true;
    // View > Free Pan: whether a canvas may pan off-centre when the document
    // fits the viewport (checked, default) or is pinned to centre. Persisted.
    bool freePan_ = true;
    // View > Snap Canvas To Workspace: when checked, a canvas whose scaled
    // document fits the viewport is pinned to the centre, panning is refused,
    // and the workspace scrollbars hide. Default off. Persisted.
    bool snapCanvasToWorkspace_ = false;
    QSet<QString> panelNames_;
    QStringList recent_;
    // Actions refreshRecentMenu added (file rows, separator, Clear); the
    // registry's "No Recent Files" placeholder is not among them.
    QList<QAction*> recentActions_;
    int untitledCounter_ = 0;
    int brightnessLevel_ = 1;
    int themeFamily_ = 0;
    int uiScalePercent_ = 100;
    QToolButton* themeCornerButton_ = nullptr;
    bool gpuCompute_ = true;
    int colorPolicy_ = 0;
    bool textGammaEnabled_ = true;
    double textGamma_ = 1.45;
    bool typeShowAsianOptions_ = false;
    bool typeMiddleEastern_ = false;
    bool typeMissingGlyphProtection_ = true;
    bool gpuAvailable_ = true;
    bool useShiftKeyForToolSwitch_ = true;
    bool openDocumentsAsTabs_ = true;
    bool floatingDocumentDocking_ = true;
    bool zoomResizesWindows_ = false;
    // Set while a Hand pan is fanned out to the other windows, so the fan-out
    // never echoes.
    bool scrollingAllWindows_ = false;
    ScreenMode screenMode_ = ScreenMode::Standard;
    int canvasColorIndex_ = 0;
    bool panelsHidden_ = false;
    // The chrome reveal Tab toggles while in Full Screen Mode (the panel
    // columns and the menu bar); transient, so leaving the mode restores the
    // Standard-mode chrome unchanged.
    bool fullScreenChromeShown_ = false;
    bool restoringPanelSession_ = false;
    struct PendingFloatRestore {
        QPointer<PanelColumn> column;
        QPointer<PanelGroup> group;
        QRect geometry;
    };
    QList<PendingFloatRestore> pendingFloatRestores_;
    // System-clipboard mirror of the bridge clipboard (frame_menus_edit.cpp):
    // `clipboardMirrored_` while both hold the same pixels, `clipboardExported_`
    // while the system clipboard holds our own export, `clipboardSetting_` only
    // during our own write so its dataChanged is not taken for another app's.
    bool clipboardMirrored_ = false;
    bool clipboardExported_ = false;
    bool clipboardSetting_ = false;

    // Crash recovery (frame_recovery.cpp): each document's snapshot id, the
    // documents changed since their last snapshot, and those with one on disk.
    void connectRecoveryPreferences(PreferencesDialog* dialog);
    void trackRecovery(PictureView* view);
    void dropRecoverySnapshot(PictureView* view);
    void forgetRecovery(PictureView* view);
    std::unique_ptr<RecoveryStore> recovery_;
    QTimer* autosaveTimer_ = nullptr;
    QHash<PictureView*, int> recoveryIds_;
    QSet<PictureView*> recoveryStale_;
    QSet<PictureView*> recoverySnapshots_;
    int recoveryCounter_ = 0;
    bool autoSaveRecovery_ = true;
    int autoSaveMinutes_ = 10;

    // Rulers and guides (frame_guides.cpp): the View commands, each canvas's
    // rulers and guide look, and the Guides preferences. Global view state,
    // persisted in the session.
    void registerGuideHandlers();
    void loadGuideSession(const SessionState& state);
    void prepareCanvasGuides(const DocWindow& window);
    // Give the active document's other windows its guides; the tools keep the
    // active canvas's guides.
    void syncSiblingGuides();
    void applyGuideViewState();
    void connectGuidePreferences(PreferencesDialog* dialog);
    // Units & Rulers: from the preferences page or a ruler's context menu.
    void setRulerUnit(RulerUnit unit);
    // frame_preferences.cpp: the Edit > Preferences leaves.
    void registerPreferenceHandlers();
    // frame_shortcuts.cpp: Edit > Keyboard Shortcuts, the saved overrides, and
    // the per-letter tool shortcuts they rebuild.
    void registerShortcutHandlers();
    void applyShortcutOverrides(const SessionState& state);
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
    std::function<bool(QDialog&)> pdfDialogDriver_;
    std::function<bool(QDialog&)> importDialogDriver_;
    std::function<void(const QString&)> pdfViewer_;
    std::function<void(const QUrl&)> urlOpener_;
};

} // namespace pictura
