#include "panel_group.h"

#include "actions_panel.h"
#include "history_panel.h"
#include "layers_panel.h"

#include <QtCore/QHash>
#include <QtGui/QAction>
#include <QtWidgets/QMenu>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QToolButton>

#include <functional>

namespace pictura {

namespace {
// Phase D per-panel menu table, transcribed from `docs/dev/m42-panel-menus.md`
// in the researched order with the CS6 grouping separators. `action` non-empty
// means the entry is wired to a per-panel handler (via the panel's
// `performPanelMenuAction`); empty means it ships disabled under the
// "<label> — not implemented yet" tooltip convention.
struct MenuRow {
    QString label;
    QString action;
    bool checkable = false;
    bool submenu = false;
    bool separatorBefore = false;
    int depth = 0;
};

MenuRow dis(const QString& label, bool separatorBefore = false)
{
    return {label, QString(), false, false, separatorBefore, 0};
}
MenuRow chk(const QString& label, bool separatorBefore = false)
{
    return {label, QString(), true, false, separatorBefore, 0};
}
MenuRow chkAction(const QString& label, const QString& action, bool separatorBefore = false)
{
    return {label, action, true, false, separatorBefore, 0};
}
MenuRow imp(const QString& label, const QString& action, bool separatorBefore = false)
{
    return {label, action, false, false, separatorBefore, 0};
}
MenuRow sub(const QString& label, bool separatorBefore = false)
{
    return {label, QString(), false, true, separatorBefore, 0};
}
MenuRow kid(const QString& label, const QString& action = QString())
{
    return {label, action, false, false, false, 1};
}

const QList<MenuRow>& rowsForPanel(const QString& panel)
{
    static const QHash<QString, QList<MenuRow>> table = {
        {QStringLiteral("layersPanel"),
         {
             imp(QStringLiteral("New Layer…"), QStringLiteral("newLayer")),
             dis(QStringLiteral("Copy CSS")),
             imp(QStringLiteral("Duplicate Layer/Group…"), QStringLiteral("duplicate"), true),
             imp(QStringLiteral("Delete Layer/Group"), QStringLiteral("delete")),
             imp(QStringLiteral("Delete Hidden Layers"), QStringLiteral("deleteHidden")),
             imp(QStringLiteral("New Group…"), QStringLiteral("newGroup"), true),
             dis(QStringLiteral("New Group from Layers…")),
             dis(QStringLiteral("Lock Layers…")),
             dis(QStringLiteral("Convert to Smart Object")),
             dis(QStringLiteral("Rasterize Layer")),
             imp(QStringLiteral("Group Layers"), QStringLiteral("group"), true),
             imp(QStringLiteral("Ungroup Layers"), QStringLiteral("ungroup")),
             imp(QStringLiteral("Hide Layers"), QStringLiteral("hide")),
             sub(QStringLiteral("Arrange"), true),
             kid(QStringLiteral("Bring to Front")),
             kid(QStringLiteral("Bring Forward")),
             kid(QStringLiteral("Send Backward")),
             kid(QStringLiteral("Send to Back")),
             kid(QStringLiteral("Reverse")),
             kid(QStringLiteral("Move Layer Up"), QStringLiteral("moveUp")),
             kid(QStringLiteral("Move Layer Down"), QStringLiteral("moveDown")),
             imp(QStringLiteral("Link Layers"), QStringLiteral("link"), true),
             imp(QStringLiteral("Select Linked Layers"), QStringLiteral("selectLinked")),
             imp(QStringLiteral("Merge Down"), QStringLiteral("mergeDown"), true),
             imp(QStringLiteral("Merge Visible"), QStringLiteral("mergeVisible")),
             imp(QStringLiteral("Merge Clipping Mask"), QStringLiteral("mergeClippingMask")),
             imp(QStringLiteral("Flatten Image"), QStringLiteral("flatten"), true),
             dis(QStringLiteral("Blending Options…"), true),
             dis(QStringLiteral("Animation Options")),
             imp(QStringLiteral("Panel Options…"), QStringLiteral("panelOptions"), true),
         }},
        {QStringLiteral("channelsPanel"),
         {
             dis(QStringLiteral("New Channel…")),
             dis(QStringLiteral("Duplicate Channel…")),
             dis(QStringLiteral("Delete Channel")),
             dis(QStringLiteral("New Spot Channel…"), true),
             dis(QStringLiteral("Merge Spot Channel(s)")),
             dis(QStringLiteral("Split Channels"), true),
             dis(QStringLiteral("Merge Channels…")),
             dis(QStringLiteral("Channel Options…"), true),
             dis(QStringLiteral("Panel Options…"), true),
         }},
        {QStringLiteral("pathsPanel"),
         {
             dis(QStringLiteral("New Path…")),
             dis(QStringLiteral("Duplicate Path…")),
             dis(QStringLiteral("Delete Path")),
             dis(QStringLiteral("Save Path…")),
             dis(QStringLiteral("Make Work Path…"), true),
             dis(QStringLiteral("Make Selection…")),
             dis(QStringLiteral("Fill Path…"), true),
             dis(QStringLiteral("Fill Subpath…")),
             dis(QStringLiteral("Stroke Path…")),
             dis(QStringLiteral("Stroke Subpath…")),
             dis(QStringLiteral("Clipping Path…"), true),
             dis(QStringLiteral("Panel Options…"), true),
         }},
        {QStringLiteral("colorPanel"),
         {
             sub(QStringLiteral("Sliders")),
             kid(QStringLiteral("Grayscale")),
             kid(QStringLiteral("RGB")),
             kid(QStringLiteral("HSB")),
             kid(QStringLiteral("CMYK")),
             kid(QStringLiteral("Lab")),
             kid(QStringLiteral("Web Color Sliders")),
             dis(QStringLiteral("RGB Spectrum"), true),
             dis(QStringLiteral("CMYK Spectrum")),
             dis(QStringLiteral("Grayscale Ramp")),
             dis(QStringLiteral("Current Colors")),
             chk(QStringLiteral("Make Ramp Web Safe")),
         }},
        {QStringLiteral("swatchesPanel"),
         {
             dis(QStringLiteral("New Swatch…")),
             sub(QStringLiteral("Display"), true),
             kid(QStringLiteral("Small Thumbnail")),
             kid(QStringLiteral("Large Thumbnail")),
             kid(QStringLiteral("Small List")),
             kid(QStringLiteral("Large List")),
             dis(QStringLiteral("Preset Manager…"), true),
             dis(QStringLiteral("Load Swatches…")),
             dis(QStringLiteral("Save Swatches…")),
             dis(QStringLiteral("Save Swatches For Exchange…")),
             dis(QStringLiteral("Replace Swatches…")),
             dis(QStringLiteral("Reset Swatches")),
             dis(QStringLiteral("(libraries list)"), true),
         }},
        {QStringLiteral("stylesPanel"),
         {
             dis(QStringLiteral("New Style…")),
             sub(QStringLiteral("Display"), true),
             kid(QStringLiteral("Text Only")),
             kid(QStringLiteral("Small Thumbnail")),
             kid(QStringLiteral("Large Thumbnail")),
             kid(QStringLiteral("Small List")),
             kid(QStringLiteral("Large List")),
             dis(QStringLiteral("Preset Manager…"), true),
             dis(QStringLiteral("Load Styles…")),
             dis(QStringLiteral("Save Styles…")),
             dis(QStringLiteral("Replace Styles…")),
             dis(QStringLiteral("Reset Styles")),
             dis(QStringLiteral("(libraries list)"), true),
         }},
        {QStringLiteral("navigatorPanel"),
         {
             dis(QStringLiteral("Panel Options…")),
         }},
        {QStringLiteral("histogramPanel"),
         {
             dis(QStringLiteral("Uncached Refresh")),
             chk(QStringLiteral("Compact View"), true),
             chk(QStringLiteral("Expanded View")),
             chk(QStringLiteral("All Channels View")),
             chk(QStringLiteral("Show Channels In Color"), true),
             chk(QStringLiteral("Show Statistics")),
         }},
        {QStringLiteral("infoPanel"),
         {
             dis(QStringLiteral("Panel Options…")),
             chk(QStringLiteral("Color Samplers"), true),
         }},
        // docs/02-ui-ux/panels/brushes-panel.md names these; the full CS6 list is
        // an open question there.
        {QStringLiteral("brushPanel"),
         {
             dis(QStringLiteral("New Brush Preset…")),
             dis(QStringLiteral("Rename Brush…")),
             dis(QStringLiteral("Delete Brush")),
             dis(QStringLiteral("Clear Brush Controls"), true),
             dis(QStringLiteral("Copy Texture to Other Tools")),
         }},
        {QStringLiteral("historyPanel"),
         {
             imp(QStringLiteral("Step Forward"), QStringLiteral("stepForward")),
             imp(QStringLiteral("Step Backward"), QStringLiteral("stepBackward")),
             imp(QStringLiteral("New Snapshot…"), QStringLiteral("newSnapshot"), true),
             dis(QStringLiteral("Delete")),
             dis(QStringLiteral("Clear History")),
             dis(QStringLiteral("New Document"), true),
             dis(QStringLiteral("History Options…"), true),
         }},
        {QStringLiteral("actionsPanel"),
         {
             chkAction(QStringLiteral("Button Mode"),
                       QStringLiteral("actions.buttonMode"), false),
             imp(QStringLiteral("New Action…"), QStringLiteral("actions.newAction"), true),
             imp(QStringLiteral("New Set…"), QStringLiteral("actions.newSet")),
             imp(QStringLiteral("Duplicate"), QStringLiteral("actions.duplicate")),
             imp(QStringLiteral("Delete"), QStringLiteral("actions.delete")),
             imp(QStringLiteral("Play"), QStringLiteral("actions.play")),
             imp(QStringLiteral("Start Recording"), QStringLiteral("actions.toggleRecording"),
                 true),
             imp(QStringLiteral("Record Again…"), QStringLiteral("actions.recordAgain")),
             imp(QStringLiteral("Insert Menu Item…"), QStringLiteral("actions.insertMenuItem"),
                 true),
             imp(QStringLiteral("Insert Stop…"), QStringLiteral("actions.insertStop")),
             imp(QStringLiteral("Insert Conditional…"),
                 QStringLiteral("actions.insertConditional")),
             imp(QStringLiteral("Insert Path"), QStringLiteral("actions.insertPath")),
             imp(QStringLiteral("Action Options…"), QStringLiteral("actions.options"), true),
             imp(QStringLiteral("Playback Options…"),
                 QStringLiteral("actions.playbackOptions")),
             chkAction(QStringLiteral("Allow Tool Recording"),
                       QStringLiteral("actions.allowToolRecording")),
             imp(QStringLiteral("Clear All Actions"), QStringLiteral("actions.clear"), true),
             imp(QStringLiteral("Reset Actions"), QStringLiteral("actions.reset")),
             imp(QStringLiteral("Load Actions…"), QStringLiteral("actions.load")),
             imp(QStringLiteral("Replace Actions…"), QStringLiteral("actions.replace")),
             imp(QStringLiteral("Save Actions…"), QStringLiteral("actions.save")),
             dis(QStringLiteral("(installed sets)"), true),
         }},
        {QStringLiteral("adjustmentsPanel"),
         {
             imp(QStringLiteral("Brightness-Contrast"),
                 QStringLiteral("adjustment:brightness-contrast")),
             imp(QStringLiteral("Levels"), QStringLiteral("adjustment:levels")),
             imp(QStringLiteral("Curves"), QStringLiteral("adjustment:curves")),
             imp(QStringLiteral("Exposure"), QStringLiteral("adjustment:exposure")),
             imp(QStringLiteral("Vibrance"), QStringLiteral("adjustment:vibrance")),
             imp(QStringLiteral("Hue-Saturation"), QStringLiteral("adjustment:hue-saturation")),
             imp(QStringLiteral("Color Balance"), QStringLiteral("adjustment:color-balance")),
             imp(QStringLiteral("Black & White"), QStringLiteral("adjustment:black-white")),
             imp(QStringLiteral("Photo Filter"), QStringLiteral("adjustment:photo-filter")),
             imp(QStringLiteral("Channel Mixer"), QStringLiteral("adjustment:channel-mixer")),
             imp(QStringLiteral("Color Lookup"), QStringLiteral("adjustment:color-lookup")),
             imp(QStringLiteral("Invert"), QStringLiteral("adjustment:invert")),
             imp(QStringLiteral("Posterize"), QStringLiteral("adjustment:posterize")),
             imp(QStringLiteral("Threshold"), QStringLiteral("adjustment:threshold")),
             imp(QStringLiteral("Gradient Map"), QStringLiteral("adjustment:gradient-map")),
             imp(QStringLiteral("Selective Color"), QStringLiteral("adjustment:selective-color")),
             dis(QStringLiteral("Add Mask by Default"), true),
             chk(QStringLiteral("Clip to Layer")),
         }},
        {QStringLiteral("propertiesPanel"),
         {
             dis(QStringLiteral("Save Preset…")),
             dis(QStringLiteral("Save Black & White Preset")),
             dis(QStringLiteral("Auto Options…"), true),
             dis(QStringLiteral("Curves Display Options…")),
             chk(QStringLiteral("Show Clipping For Black/White Points"), true),
             chk(QStringLiteral("Auto-Select Parameter")),
             chk(QStringLiteral("Auto-Select Targeted Adjustment Tool")),
             dis(QStringLiteral("Apply Mask"), true),
             dis(QStringLiteral("Delete Mask")),
             dis(QStringLiteral("Disable Mask")),
         }},
    };
    static const QList<MenuRow> empty;
    const auto it = table.find(panel);
    return it == table.end() ? empty : it.value();
}

void populatePanelMenu(QMenu* menu, const QString& panelName,
                       const std::function<void(const QString&)>& dispatch)
{
    QMenu* submenu = nullptr;
    for (const MenuRow& row : rowsForPanel(panelName)) {
        if (row.depth == 0) {
            submenu = nullptr;
            if (row.separatorBefore) {
                menu->addSeparator();
            }
            if (row.submenu) {
                submenu = menu->addMenu(row.label);
                submenu->menuAction()->setEnabled(false);
                submenu->menuAction()->setToolTip(
                    QStringLiteral("%1 — not implemented yet").arg(row.label));
                continue;
            }
            QAction* action = menu->addAction(row.label);
            action->setCheckable(row.checkable);
            if (!row.action.isEmpty()) {
                action->setData(row.action);
            }
            if (row.label == QStringLiteral("(installed sets)")) {
                action->setData(QStringLiteral("actions.installedPlaceholder"));
            }
            if (row.action.isEmpty()) {
                action->setEnabled(false);
                action->setToolTip(
                    QStringLiteral("%1 — not implemented yet").arg(row.label));
            } else if (dispatch) {
                const QString id = row.action;
                QObject::connect(action, &QAction::triggered, menu,
                                 [dispatch, id]() { dispatch(id); });
            }
            continue;
        }
        if (!submenu) {
            continue;
        }
        QAction* action = submenu->addAction(row.label);
        action->setCheckable(row.checkable);
        if (!row.action.isEmpty()) {
            action->setData(row.action);
        }
        if (row.action.isEmpty()) {
            action->setEnabled(false);
            action->setToolTip(QStringLiteral("%1 — not implemented yet").arg(row.label));
        } else {
            submenu->menuAction()->setEnabled(true);
            submenu->menuAction()->setToolTip(QString());
            if (dispatch) {
                const QString id = row.action;
                QObject::connect(action, &QAction::triggered, menu,
                                 [dispatch, id]() { dispatch(id); });
            }
        }
    }
    // M47: every live per-widget menu also closes the active tab and the whole
    // group. Only the live menu (with a dispatcher) carries them; the read-only
    // text listing stays the transcribed per-panel table.
    if (dispatch) {
        menu->addSeparator();
        QAction* closePanel = menu->addAction(PanelGroup::tr("Close"));
        closePanel->setData(QStringLiteral("close"));
        QObject::connect(closePanel, &QAction::triggered, menu,
                         [dispatch]() { dispatch(QStringLiteral("close")); });
        QAction* closeGroup = menu->addAction(PanelGroup::tr("Close Tab Group"));
        closeGroup->setData(QStringLiteral("closeGroup"));
        QObject::connect(closeGroup, &QAction::triggered, menu,
                         [dispatch]() { dispatch(QStringLiteral("closeGroup")); });
    }
}
} // namespace

void PanelGroup::updateHeaderMenu()
{
    if (!headerButton_) {
        return;
    }
    const QString panel = currentPanelName();
    // M49: the corner affordance is always present, even for a panel whose
    // transcribed menu is empty — it still offers Close / Close Tab Group.
    headerButton_->setVisible(true);
    if (headerCorner_) {
        headerCorner_->setVisible(true);
    }
    headerButton_->setMenu(nullptr);
    delete headerMenu_;
    headerMenu_ = nullptr;
    headerButton_->setObjectName(panel.isEmpty()
                                     ? QStringLiteral("panelWidgetMenu")
                                     : QStringLiteral("panelWidgetMenu_") + panel);
    headerButton_->setToolTip(panel.isEmpty()
                                  ? tr("Panel Menu")
                                  : tr("%1 Panel Menu").arg(titleForPanel(panel)));
    headerMenu_ = new QMenu(headerButton_);
    populatePanelMenu(headerMenu_, panel,
                      [this](const QString& id) { runPanelMenuAction(id); });
    QObject::connect(headerMenu_, &QMenu::aboutToShow, headerMenu_, [this, panel] {
        auto* layers = qobject_cast<LayersPanel*>(tabs_->currentWidget());
        for (QAction* action : headerMenu_->findChildren<QAction*>()) {
            const QString id = action->data().toString();
            if (id == QLatin1String("moveUp") || id == QLatin1String("moveDown")) {
                action->setEnabled(layers
                                   && layers->canMoveCurrent(
                                       id == QLatin1String("moveUp") ? 1 : -1));
            } else if (id == QLatin1String("mergeDown")) {
                const bool one = layers && layers->targetLayerPaths().size() == 1;
                action->setText(one ? PanelGroup::tr("Merge Down")
                                    : PanelGroup::tr("Merge Layers"));
                action->setEnabled(layers && layers->canMergeSelection());
            }
        }
        if (panelMenuHooks_.contains(panel)) {
            panelMenuHooks_.value(panel)(headerMenu_);
        } else if (auto* actions = qobject_cast<ActionsPanel*>(tabs_->currentWidget())) {
            actions->updatePanelMenu(headerMenu_);
        }
    });
    headerButton_->setMenu(headerMenu_);
}

void PanelGroup::runPanelMenuAction(const QString& actionId)
{
    // M47: the per-widget menu's close entries. The column that wires this group
    // owns the close behaviour, so the group only forwards the request.
    if (actionId == QStringLiteral("close")) {
        emit closePanelRequested(currentPanelName());
        return;
    }
    if (actionId == QStringLiteral("closeGroup")) {
        emit closeGroupRequested();
        return;
    }
    QWidget* panel = tabs_->currentWidget();
    if (!panel) {
        return;
    }
    if (auto* layers = qobject_cast<LayersPanel*>(panel)) {
        layers->performPanelMenuAction(actionId);
        return;
    }
    if (auto* history = qobject_cast<HistoryPanel*>(panel)) {
        history->performPanelMenuAction(actionId);
        return;
    }
    if (auto* actions = qobject_cast<ActionsPanel*>(panel)) {
        actions->performPanelMenuAction(actionId);
        return;
    }
    // The Adjustments panel is a placeholder with no view handle; its
    // adjustment kinds run through the Layers panel, which owns the view.
    if (actionId.startsWith(QStringLiteral("adjustment:"))) {
        if (QWidget* win = window()) {
            if (auto* layers = win->findChild<LayersPanel*>(QStringLiteral("layersPanel"))) {
                layers->performPanelMenuAction(actionId);
            }
        }
    }
}

bool PanelGroup::panelHasMenu(const QString& panelName)
{
    // The Clone Source spec lists no menu entries, so its live menu carries
    // only Close / Close Group.
    return !rowsForPanel(panelName).isEmpty() || panelName == QLatin1String("cloneSourcePanel");
}

QStringList PanelGroup::menuTextsForPanel(const QString& panelName)
{
    QMenu menu;
    populatePanelMenu(&menu, panelName, nullptr);
    QStringList texts;
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator()) {
            texts.push_back(action->text());
        }
    }
    return texts;
}

} // namespace pictura
