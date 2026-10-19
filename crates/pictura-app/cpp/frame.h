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

#include "canvas_ruler.h"
#include "panels/panel_column.h"
#include "recovery_store.h"
#include "tools.h"
#include "workspace_store.h"

class QDialog;

class QDockWidget;
class QFrame;
class QLabel;
class QListWidget;
class QResizeEvent;
class QShortcut;
class QShowEvent;
class QSplitter;
class QTabWidget;
class QTemporaryDir;
class QTimer;
class QToolButton;
class QWidget;

namespace pictura {

class CommandRegistry;
class ColorPanel;
class ChannelsPanel;
class ColorState;
class CanvasScrollBars;
class DesignSystemDialog;
class FileDropRouter;
class HistogramPanel;
class HistoryPanel;
class BrushPanel;
class CloneSourcePanel;
class CharacterPanel;
class ParagraphPanel;
class GlyphsPanel;
class ParagraphStylesPanel;
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
class PreferencesDialog;
class PropertiesPanel;
class SwatchesPanel;
class Toolbox;
class ToolHintBar;
struct FilterCommandSpec;
struct SessionState;

// True when a launch should seed the scratch white document. Only the self-test
// path needs it (headless implies self-test), so a normal launch starts on the
// empty workspace with the document commands disabled.
bool launchCreatesScratchDocument(bool selfTest, bool codecLoaded);

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
    // frame_open.cpp: the loading banner's text, and whether it is showing.
    QString openBannerTextForTest() const;
    bool openBannerVisibleForTest() const;
    // Creates two documents and drags the last tab before its neighbour through
    // the real tab bar; returns whether docs_, viewAt and the active view stayed
    // aligned with the new visual order.
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
    QColor foregroundColor() const { return foreground_; }
    bool hasPendingCrop() const;
    bool commitCrop();

    int brightnessLevel() const { return brightnessLevel_; }
    void setBrightnessLevel(int level);

    // Theme family: 0 dark / 1 light. Switching applies the palette live and
    // persists the choice; the menu-bar theme button toggles it.
    int themeFamily() const { return themeFamily_; }
    void setThemeFamily(int family);

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

    // Document operations.
    int documentCount() const { return docs_.size(); }
    int activeDocumentIndex() const;
    void setActiveDocumentIndex(int index);
    PictureView* viewAt(int index) const;
    ImageView* canvasAt(int index) const;
    QString documentPath(int index) const;
    QString documentName(int index) const;
    QString documentTabTextForTest(int index) const;
    bool isDocumentDirty(int index) const;
    QString activeFilePath() const;
    QString activeDocumentName() const;
    bool isActiveDirty() const;
    // True when no document is open, so a double-click on the empty workspace
    // opens the Open dialog.
    bool workspaceOpenArmed() const { return docs_.isEmpty(); }

    // Takes ownership of `view` and adds it as a document tab.
    int addDocument(PictureView* view, const QString& path);
    bool newDocument(const QString& name, int width, int height, const QString& mode,
                     int depth, const QString& background);
    bool openPath(const QString& path);
    bool openImagePath(const QString& path);
    // Route `path` by signature (PDF, EPS) or suffix (PSD/PSB, Qt raster
    // import). With `interactive` off (agentic control) a PDF or EPS imports
    // with the dialogs' defaults and no prompt.
    bool openDocumentAtPath(const QString& path, bool interactive = true);
    // True when `path`'s suffix routes to the native PSD/PSB reader rather than
    // the Qt image decode edge.
    static bool isNativeDocumentPath(const QString& path);
    bool openAsSmartObjectPath(const QString& path);
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
    bool openPdfPath(const QString& path, bool interactive = true);
    void showPdfPresets();
    // frame_import.cpp: a generic PDF through the password prompt and the
    // Import PDF dialog, an EPS through the Rasterize Generic EPS Format
    // dialog, and File > Open As with a chosen decoder (index into
    // openAsFileFilters()). Each opened page or image is an untitled document.
    // `knownPassword` is one `openPdfPath` already accepted.
    bool importPdfPath(const QString& path, bool interactive = true,
                       const QString& knownPassword = QString());
    // Ask for the password of a protected PDF (`refused`: the last one was
    // wrong); false on Cancel.
    bool askPdfPassword(const QString& path, bool refused, QString& password);
    bool importEpsPath(const QString& path, bool interactive = true);
    void showOpenAsDialog();
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
    bool closeDocument(int index, bool interactive);
    // The recent-files list, most recent first; setting it persists it.
    const QStringList& recentFiles() const { return recent_; }
    void setRecentFiles(const QStringList& files);
    // File > Open Recent, or null before the menu bar is built.
    QMenu* recentMenu() const;
    bool closeActiveDocument(bool interactive);
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
    // The top-right menu-bar chrome: the theme toggle and the GitHub / Discord
    // links.
    void installMenuCornerButtons();
    void updateThemeCornerIcon();

Q_SIGNALS:
    // Emitted while the loading indication is showing, immediately before a
    // blocking document read/decode begins. Test seam for the open banner.
    void documentOpenStarted(const QString& path);

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
    struct DocEntry {
        PictureView* view = nullptr;
        ImageView* canvas = nullptr;
        CanvasScrollBars* canvasHost = nullptr;
        QString path;
        // Display name for a path-less import (Open Image / Open As Smart
        // Object); preferred over the generated Untitled-N name. The tab title
        // uses it while `path` stays empty so Save cannot overwrite the source.
        QString displayName;
        int untitledNumber = 0;
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
    // frame_menus_layer_style.cpp: Layer > Layer Style, the Layer Style dialog
    // and the style commands over the Layers panel's selection.
    void wireLayerStyleMenu();
    // frame_menus_image.cpp: Image > Crop, Trim, and Duplicate.
    void wireImageMenu();
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
    // frame_menus_layer_ops.cpp: Layer > Arrange / Reverse, Merge Down, Delete
    // Layer, and the non-destructive Stamp Visible / Stamp Selected.
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
    void importSystemClipboard();
    void applyPanelSession(const SessionState& state);
    // Rebuild the widget columns from a workspace-shaped spec, moving panels
    // between groups (the session restore only reorders/hides them).
    void regroupAndApply(const QJsonArray& columns);
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
    // frame_open.cpp: the loading indication around a blocking document open.
    void beginOpenIndicator(const QString& path);
    void endOpenIndicator();
    void positionOpenBanner();
    void openRecent(const QString& path);
    void applyBrightness(int level);
    QColor canvasColorForIndex(int index) const;
    void applyWorkspaceCanvasColor();
    // The mode `cycleScreenMode` steps to, and the panel toggles: Standard and
    // Full Screen With Menu Bar flip the persisted `panelsHidden_`; Full Screen
    // Mode flips the transient `fullScreenChromeShown_`, which also controls the
    // menu bar, so the first Tab reveals the chrome.
    ScreenMode nextScreenMode(bool forward) const;
    void togglePanelsHidden();
    void setPanelColumnsVisible(bool visible);

    QList<DocEntry> docs_;
    QList<SmartObjectEditSession> editSessions_;
    QTabWidget* tabs_ = nullptr;
    // Disconnected in the destructor: the focusChanged handler must not run
    // while the window's children are being torn down.
    QMetaObject::Connection spacePanFocusConn_;
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
    PropertiesPanel* propertiesPanel_ = nullptr;
    PlaceholderPanel* adjustmentsPanel_ = nullptr;
    ChannelsPanel* channelsPanel_ = nullptr;
    PathsPanel* pathsPanel_ = nullptr;
    PlaceholderPanel* actionsPanel_ = nullptr;
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
    QToolButton* statusOptionsButton_ = nullptr;
    // The floating "Opening <file>…" banner (frame_open.cpp): a child of the
    // window, shown at the lower-left while a document load runs.
    QLabel* openBanner_ = nullptr;
    QString statusReadout_ = QStringLiteral("sizes");
    bool hintsVisible_ = true;
    // View > Free Pan: whether a canvas may pan off-centre when the document
    // fits the viewport (checked, default) or is pinned to centre. Persisted.
    bool freePan_ = true;
    // View > Snap Canvas To Workspace: when checked, a canvas whose scaled
    // document fits the viewport is pinned to the centre, panning is refused,
    // and the workspace scrollbars hide. Default off. Persisted.
    bool snapCanvasToWorkspace_ = false;
    QColor foreground_;
    QSet<QString> panelNames_;
    QStringList recent_;
    // Actions refreshRecentMenu added (file rows, separator, Clear); the
    // registry's "No Recent Files" placeholder is not among them.
    QList<QAction*> recentActions_;
    int untitledCounter_ = 0;
    int brightnessLevel_ = 1;
    int themeFamily_ = 0;
    QToolButton* themeCornerButton_ = nullptr;
    bool gpuCompute_ = true;
    int colorPolicy_ = 0;
    bool gpuAvailable_ = true;
    bool useShiftKeyForToolSwitch_ = true;
    ScreenMode screenMode_ = ScreenMode::Standard;
    int canvasColorIndex_ = 0;
    bool panelsHidden_ = false;
    // The chrome reveal Tab toggles while in Full Screen Mode (the panel
    // columns and the menu bar); transient, so leaving the mode restores the
    // Standard-mode chrome unchanged.
    bool fullScreenChromeShown_ = false;
    bool restoringPanelSession_ = false;
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
    void prepareCanvasGuides(const DocEntry& entry);
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
    bool rulersVisible_ = false;
    RulerUnit rulerUnit_ = RulerUnit::Inches;
    bool traditionalPoints_ = false;
    bool guidesVisible_ = true;
    bool guidesLocked_ = false;
    QColor guideColor_{0x4a, 0xff, 0xff};
    bool guideDashed_ = false;
    std::function<bool(QDialog&)> pdfDialogDriver_;
    std::function<bool(QDialog&)> importDialogDriver_;
    std::function<void(const QString&)> pdfViewer_;
};

} // namespace pictura
