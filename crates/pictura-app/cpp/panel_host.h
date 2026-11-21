#pragma once

#include <QtCore/QJsonArray>
#include <QtCore/QList>
#include <QtCore/QObject>
#include <QtCore/QPoint>
#include <QtCore/QPointer>
#include <QtCore/QRect>
#include <QtCore/QSet>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/Qt>
#include <functional>

#include "panels/panel_column.h"
#include "panels/panel_menu_hooks.h"
#include "workspace_store.h"

class QAction;
class QMenu;
class QSplitter;
class QTimer;
class QWidget;

namespace pictura {

struct Preferences;

class ActionsEngineBridge;
class ActionsPanel;
class AdjustmentsPanel;
class BrushPanel;
class ChannelsPanel;
class CharacterPanel;
class CharacterStylesPanel;
class CloneSourcePanel;
class ColorPanel;
class ColorState;
class CommandRegistry;
class DocumentRegistry;
class GlyphsPanel;
class HistogramPanel;
class HistoryPanel;
class ImageView;
class InfoPanel;
class LayerCompsPanel;
class LayersPanel;
class NavigatorPanel;
class NotesPanel;
class PanelColumn;
class PanelGroup;
class PanelFloat;
class ParagraphPanel;
class ParagraphStylesPanel;
class PathsPanel;
class PictureView;
class PlaceholderPanel;
class PropertiesPanel;
class SwatchesPanel;
struct SessionState;

// Owns the panel widgets, their groups, the panel columns, the tools column and
// the central splitter, plus the workspace store, the panel menu hooks, the
// panels-hidden state and panel binding/refresh. Constructible without
// the main window; the shell supplies the cross-domain callbacks.
class PanelHost : public QObject, public PanelMenuHookStore {
    Q_OBJECT
public:
    struct Hooks {
        std::function<void()> saveSession;
        std::function<void()> saveSelectionOptions;
        std::function<void()> alphaVisibilityChanged;
        std::function<void()> channelOptionsRequested;
        std::function<void()> interfaceOptionsRequested;
        std::function<PictureView*()> activeView;
        std::function<ImageView*()> activeCanvas;
        std::function<void(const QString&, int)> showStatus;
        // A document switch: cancel the frame's colour-range/replace-colour
        // dialogs when they hold a view the panels are leaving.
        std::function<void(PictureView*)> cancelDocumentDialogs;
        std::function<void(const QString&)> convertImageMode;
        std::function<void(int)> convertImageDepth;
        std::function<void(const QString&, int)> editSmartFilter;
        std::function<void(QMenu*)> attachWorkspaceMenu;
        std::function<void(const QString&)> setActiveWorkspace;
    };

    PanelHost(QWidget* shell, DocumentRegistry* documents, CommandRegistry* commands,
              ColorState* colorState, ActionsEngineBridge* actionsEngine, Hooks hooks,
              QObject* parent = nullptr);
    ~PanelHost() override;

    QWidget* shellWidget() const { return shell_; }
    // The widget the columns dock into (the central splitter).
    QWidget* workspaceWidget() const;
    QSplitter* centerSplitter() const { return centerSplitter_; }
    CommandRegistry* commands() const { return commands_; }
    ColorState* colorState() const { return colorState_; }
    ActionsEngineBridge* actionsEngine() const { return actionsEngine_; }

    bool panelsHidden() const { return panelsHidden_; }
    const QSet<QString>& panelObjectNames() const { return panelNames_; }
    PanelColumn* panelColumn() const { return panelColumn_; }
    PanelColumn* toolsColumn() const { return toolsColumn_; }
    int dockRetargetCount() const { return dockRetargetCount_; }
    bool isRestoringSession() const { return restoringPanelSession_; }

    LayersPanel* layersPanel() const { return layersPanel_; }
    HistoryPanel* historyPanel() const { return historyPanel_; }
    NavigatorPanel* navigatorPanel() const { return navigatorPanel_; }
    ColorPanel* colorPanel() const { return colorPanel_; }
    SwatchesPanel* swatchesPanel() const { return swatchesPanel_; }
    PlaceholderPanel* stylesPanel() const { return stylesPanel_; }
    InfoPanel* infoPanel() const { return infoPanel_; }
    HistogramPanel* histogramPanel() const { return histogramPanel_; }
    PlaceholderPanel* gradientsPanel() const { return gradientsPanel_; }
    PlaceholderPanel* patternsPanel() const { return patternsPanel_; }
    NotesPanel* notesPanel() const { return notesPanel_; }
    PropertiesPanel* propertiesPanel() const { return propertiesPanel_; }
    AdjustmentsPanel* adjustmentsPanel() const { return adjustmentsPanel_; }
    ChannelsPanel* channelsPanel() const { return channelsPanel_; }
    PathsPanel* pathsPanel() const { return pathsPanel_; }
    LayerCompsPanel* layerCompsPanel() const { return layerCompsPanel_; }
    ActionsPanel* actionsPanel() const { return actionsPanel_; }
    BrushPanel* brushPanel() const { return brushPanel_; }
    CloneSourcePanel* cloneSourcePanel() const { return cloneSourcePanel_; }
    CharacterPanel* characterPanel() const { return characterPanel_; }
    ParagraphPanel* paragraphPanel() const { return paragraphPanel_; }
    GlyphsPanel* glyphsPanel() const { return glyphsPanel_; }
    ParagraphStylesPanel* paragraphStylesPanel() const { return paragraphStylesPanel_; }
    CharacterStylesPanel* characterStylesPanel() const { return characterStylesPanel_; }
    QList<QWidget*> panelWidgets() const;

    QList<PanelGroup*> buildPanels(const SessionState& state, const Preferences& prefs);
    bool registerPanel(QWidget* panel, Qt::DockWidgetArea area);
    PanelColumn* createToolsColumn();
    PanelColumn* adoptToolsColumn(PanelColumn* column);

    // Column operations (the former frame_columns.cpp set).
    QList<PanelColumn*> panelColumns() const;
    QList<PanelColumn*> allPanelColumns() const;
    int columnCount() const;
    PanelSide sideOf(const PanelColumn* column) const;
    PanelColumn* createPanelColumn(PanelSide side, PanelColumn* anchor = nullptr);
    void removeColumnIfEmpty(PanelColumn* column);
    PanelColumn* columnForPanel(const QString& objectName) const;
    PanelColumn* columnAtGlobal(const QPoint& globalPos) const;
    int newColumnSideAt(const QPoint& globalPos) const;
    int newColumnSideAtInsideWorkspace(const QPoint& globalPos) const;
    PanelColumn* columnEdgeAnchorAt(const QPoint& globalPos, const PanelColumn* exclude,
                                    int* side) const;
    PanelColumn* resolveColumnMoveTarget(const QPoint& globalPos, const PanelColumn* exclude,
                                         int* side) const;
    bool movePanelColumn(PanelColumn* column, int side, PanelColumn* anchor);
    void clearDynamicColumns();
    void wirePanelColumn(PanelColumn* column);
    void reapplyColumnStretch();
    void onCenterSplitterMoved(int pos, int index);
    void beginWidgetColumnResize(PanelColumn* column, int globalX);
    void updateWidgetColumnResize(int globalX);
    void endWidgetColumnResize();
    void refreshSharedFloor();

    // Workspace state and layout (the former frame_workspace(s|_controller)
    // operations).
    void applyPanelSession(const SessionState& state);
    void applyFreshSessionDefault(const SessionState& state, const QList<PanelGroup*>& staged);
    void regroupAndApply(const QJsonArray& columns, bool persist = true,
                         QList<PanelGroup*> staged = {});
    QJsonArray serializeWorkspaceLayout() const;
    QJsonArray applyWorkspaceLayoutForTest(const QJsonArray& columns)
    {
        regroupAndApply(columns);
        return serializeWorkspaceLayout();
    }
    void applyWorkspaceLayout(const QJsonArray& columns) { regroupAndApply(columns); }
    void queueOrRestoreFloat(PanelColumn* column, PanelGroup* group, const QRect& geometry);
    void applyPendingFloatRestores();
    PanelColumn* restoreLegacyDefaultForTest();
    void saveState(SessionState& state);

    void initWorkspaces(bool sessionHadLayout);
    void attachWorkspaceSwitcher();
    void updateWorkspaceSwitcher();
    void snapshotActiveWorkspace();
    void switchWorkspace(const QString& name);
    void resetWorkspace();
    bool createWorkspace(const QString& name);
    bool deleteWorkspace(const QString& name);
    QString workspaceNameError(const QString& name) const;
    QString activeWorkspace() const;
    QStringList userWorkspaces() const;
    QJsonArray workspaceLayout() const { return serializeWorkspaceLayout(); }
    QStringList deleteWorkspaceChoices() const;
    QMenu* workspaceMenu() const;
    void refreshWorkspaceMenu(QMenu* menu);
    QStringList workspaceMenuLabels() const;
    int workspaceMenuSeparatorCount() const;

    // Panel visibility, binding and refresh.
    void setPanelsHidden(bool hidden);
    void setPanelColumnsVisible(bool visible);
    void togglePanel(const QString& objectName);
    void layoutWorkspaceFrame();
    void updateColumnFrameEdges();
    void handleShellResize();
    void schedulePanelRefresh();
    void stopPanelRefreshTimer();
    void setPanelsFreshlyBound(bool bound) { panelsFreshlyBound_ = bound; }
    void setPanelsRefreshedByChanged(bool refreshed) { panelsRefreshedByChanged_ = refreshed; }
    void setRegistryRefreshPending(bool pending) { registryRefreshPending_ = pending; }
    void retargetDock();
    bool dockNeedsRebind() const;
    void refreshDock(bool includeSelfRefreshing, bool includeLayers);
    void refreshPanels();

Q_SIGNALS:
    void layoutChanged();
    void panelVisibilityChanged();
    void layerEditTargetChanged(bool mask);
    void layerSelectionChanged();

private:
    void requestSessionSave();
    void buildFrameLines();
    void wireStaticPanels();
    void wireActionsPanel(const SessionState& state);
    int workspaceIndexOf(const QString& name) const;

    QWidget* shell_ = nullptr;
    DocumentRegistry* documents_ = nullptr;
    CommandRegistry* commands_ = nullptr;
    ColorState* colorState_ = nullptr;
    ActionsEngineBridge* actionsEngine_ = nullptr;
    Hooks hooks_;

    QSplitter* centerSplitter_ = nullptr;
    QWidget* workspaceFrameLeft_ = nullptr;
    QWidget* workspaceFrameRight_ = nullptr;
    PanelColumn* panelColumn_ = nullptr;
    PanelColumn* toolsColumn_ = nullptr;
    QTimer* panelRefreshTimer_ = nullptr;
    QTimer* sessionSaveTimer_ = nullptr;

    QSet<QString> panelNames_;
    bool panelsHidden_ = false;
    bool restoringPanelSession_ = false;
    bool buildingPanels_ = false;
    bool workspacesInitialized_ = false;
    int dockRetargetCount_ = 0;
    PictureView* dockView_ = nullptr;
    ImageView* dockCanvas_ = nullptr;
    quint64 dockGeneration_ = 0;
    bool panelsRefreshedByChanged_ = false;
    bool panelsFreshlyBound_ = false;
    bool refreshingDock_ = false;
    bool registryRefreshPending_ = false;

    PanelColumn* resizeAnchorColumn_ = nullptr;
    int resizeAnchorWidth_ = -1;
    bool adjustingColumns_ = false;
    int resizeHandleIndex_ = -1;
    int resizeStartGlobalX_ = 0;
    QList<int> resizeStartSizes_;

    struct PendingFloatRestore {
        QPointer<PanelColumn> column;
        QPointer<PanelGroup> group;
        QRect geometry;
    };
    QList<PendingFloatRestore> pendingFloatRestores_;

    WorkspaceStore workspaceStore_;
    QList<QAction*> workspaceMenuActions_;

    LayersPanel* layersPanel_ = nullptr;
    HistoryPanel* historyPanel_ = nullptr;
    NavigatorPanel* navigatorPanel_ = nullptr;
    ColorPanel* colorPanel_ = nullptr;
    SwatchesPanel* swatchesPanel_ = nullptr;
    InfoPanel* infoPanel_ = nullptr;
    HistogramPanel* histogramPanel_ = nullptr;
    PlaceholderPanel* gradientsPanel_ = nullptr;
    PlaceholderPanel* patternsPanel_ = nullptr;
    NotesPanel* notesPanel_ = nullptr;
    PropertiesPanel* propertiesPanel_ = nullptr;
    AdjustmentsPanel* adjustmentsPanel_ = nullptr;
    ChannelsPanel* channelsPanel_ = nullptr;
    PathsPanel* pathsPanel_ = nullptr;
    LayerCompsPanel* layerCompsPanel_ = nullptr;
    ActionsPanel* actionsPanel_ = nullptr;
    PlaceholderPanel* stylesPanel_ = nullptr;
    BrushPanel* brushPanel_ = nullptr;
    CloneSourcePanel* cloneSourcePanel_ = nullptr;
    CharacterPanel* characterPanel_ = nullptr;
    ParagraphPanel* paragraphPanel_ = nullptr;
    GlyphsPanel* glyphsPanel_ = nullptr;
    ParagraphStylesPanel* paragraphStylesPanel_ = nullptr;
    CharacterStylesPanel* characterStylesPanel_ = nullptr;
};

} // namespace pictura
