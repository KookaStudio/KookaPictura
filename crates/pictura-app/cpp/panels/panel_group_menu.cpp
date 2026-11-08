#include "panel_group.h"

#include "actions_panel.h"
#include "commands.h"
#include "adjustment_catalogue.h"
#include "character_panel.h"
#include "frame.h"
#include "history_panel.h"
#include "layers_panel.h"
#include "paragraph_panel.h"
#include "properties_panel.h"

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
MenuRow chkKid(const QString& label, const QString& action)
{
    MenuRow row = chkAction(label, action);
    row.depth = 1;
    return row;
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
        {QStringLiteral("characterPanel"),
         {
             sub(QStringLiteral("Change Text Orientation")),
             kid(QStringLiteral("Standard Vertical Roman Alignment")),
             sub(QStringLiteral("OpenType")),
             chkKid(QStringLiteral("Standard Ligatures"),
                    QStringLiteral("character.standardLigatures")),
             chkKid(QStringLiteral("Contextual Alternates"),
                    QStringLiteral("character.contextualAlternates")),
             chkKid(QStringLiteral("Discretionary Ligatures"),
                    QStringLiteral("character.discretionaryLigatures")),
             chkKid(QStringLiteral("Swash"), QStringLiteral("character.swash")),
             chkKid(QStringLiteral("Stylistic Alternates"),
                    QStringLiteral("character.stylisticAlternates")),
             chkKid(QStringLiteral("Titling Alternates"),
                    QStringLiteral("character.titlingAlternates")),
             chkKid(QStringLiteral("Ordinals"), QStringLiteral("character.ordinals")),
             chkKid(QStringLiteral("Fractions"), QStringLiteral("character.fractions")),
             chkAction(QStringLiteral("Faux Bold"), QStringLiteral("character.fauxBold")),
             chkAction(QStringLiteral("Faux Italic"), QStringLiteral("character.fauxItalic")),
             chkAction(QStringLiteral("All Caps"), QStringLiteral("character.allCaps")),
             chkAction(QStringLiteral("Small Caps"), QStringLiteral("character.smallCaps")),
             chkAction(QStringLiteral("Superscript"), QStringLiteral("character.superscript")),
             chkAction(QStringLiteral("Subscript"), QStringLiteral("character.subscript")),
             chkAction(QStringLiteral("Underline"), QStringLiteral("character.underline"), true),
             chkAction(QStringLiteral("Strikethrough"), QStringLiteral("character.strikethrough")),
             chkAction(QStringLiteral("Fractional Widths"),
                       QStringLiteral("character.fractionalWidths"), true),
             chkAction(QStringLiteral("System Layout"), QStringLiteral("character.systemLayout")),
             dis(QStringLiteral("No Break"), true),
             sub(QStringLiteral("Middle Eastern Features"), true),
             imp(QStringLiteral("Reset Character"), QStringLiteral("character.reset"), true),
         }},
        {QStringLiteral("paragraphPanel"),
         {
             chkAction(QStringLiteral("Roman Hanging Punctuation"),
                       QStringLiteral("paragraph.hanging")),
             imp(QStringLiteral("Justification…"), QStringLiteral("paragraph.justification"),
                 true),
             dis(QStringLiteral("Hyphenation…")),
             chk(QStringLiteral("Latin and East Asian Layout"), true),
             chk(QStringLiteral("World-Ready Layout")),
             chk(QStringLiteral("Default Features"), true),
             chk(QStringLiteral("East Asian Features")),
             chk(QStringLiteral("Middle Eastern Features")),
             chkAction(QStringLiteral("Single-line Composer"),
                       QStringLiteral("paragraph.singleLineComposer"), true),
             chkAction(QStringLiteral("Every-line Composer"),
                       QStringLiteral("paragraph.everyLineComposer")),
             imp(QStringLiteral("Reset Paragraph"), QStringLiteral("paragraph.reset"), true),
         }},
        {QStringLiteral("adjustmentsPanel"),
         [] {
             QList<MenuRow> rows;
             for (const AdjustmentKind& entry : adjustmentCatalogue()) {
                 if (!entry.panelMenu.isEmpty()) {
                     rows << imp(entry.panelMenu, QStringLiteral("adjustment:") + entry.kind);
                 }
             }
             rows << chkAction(QStringLiteral("Add Mask by Default"),
                                 QStringLiteral("addMaskByDefault"), true)
                  << chk(QStringLiteral("Clip to Layer"));
             return rows;
         }()},
        {QStringLiteral("propertiesPanel"),
         {
             imp(QStringLiteral("Save Preset…"), QStringLiteral("savePreset")),
             imp(QStringLiteral("Load Preset…"), QStringLiteral("loadPreset")),
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

QString panelCommandId(const QString& panel, const QString& action)
{
    if (action.isEmpty()) {
        return {};
    }
    if (action.startsWith(QStringLiteral("adjustment:"))) {
        const QString kind = action.mid(11);
        for (const AdjustmentKind& entry : adjustmentCatalogue()) {
            if (entry.kind == kind) {
                return commandIdForPath({QStringLiteral("Layer"),
                                         QStringLiteral("New Adjustment Layer"), entry.title});
            }
        }
        return {};
    }
    if (panel == QStringLiteral("layersPanel")) {
        static const QHash<QString, const char*> map = {
            {QStringLiteral("newLayer"), command_ids::PanelLayersNewLayer},
            {QStringLiteral("duplicate"), command_ids::LayerDuplicateLayer},
            {QStringLiteral("delete"), command_ids::LayerDeleteLayer},
            {QStringLiteral("deleteHidden"), command_ids::LayerDeleteHiddenLayers},
            {QStringLiteral("newGroup"), command_ids::PanelLayersNewGroup},
            {QStringLiteral("group"), command_ids::LayerGroupLayers},
            {QStringLiteral("ungroup"), command_ids::LayerUngroupLayers},
            {QStringLiteral("hide"), command_ids::LayerHideLayers},
            {QStringLiteral("link"), command_ids::LayerLinkLayers},
            {QStringLiteral("selectLinked"), command_ids::LayerSelectLinked},
            {QStringLiteral("mergeDown"), command_ids::LayerMergeLayers},
            {QStringLiteral("mergeVisible"), command_ids::LayerMergeVisible},
            {QStringLiteral("mergeClippingMask"), command_ids::LayerMergeClippingMask},
            {QStringLiteral("flatten"), command_ids::LayerFlattenImage},
            {QStringLiteral("panelOptions"), command_ids::PanelLayersPanelOptions},
        };
        return map.contains(action) ? QString::fromLatin1(map.value(action)) : QString();
    }
    if (panel == QStringLiteral("historyPanel")) {
        static const QHash<QString, const char*> map = {
            {QStringLiteral("stepForward"), command_ids::PanelHistoryStepForward},
            {QStringLiteral("stepBackward"), command_ids::PanelHistoryStepBackward},
            {QStringLiteral("newSnapshot"), command_ids::PanelHistoryNewSnapshot},
        };
        return map.contains(action) ? QString::fromLatin1(map.value(action)) : QString();
    }
    if (panel == QStringLiteral("propertiesPanel")) {
        static const QHash<QString, const char*> map = {
            {QStringLiteral("savePreset"), command_ids::PanelPropertiesSavePreset},
            {QStringLiteral("loadPreset"), command_ids::PanelPropertiesLoadPreset},
        };
        return map.contains(action) ? QString::fromLatin1(map.value(action)) : QString();
    }
    if (action.startsWith(QStringLiteral("actions."))
        || action.startsWith(QStringLiteral("character."))
        || action.startsWith(QStringLiteral("paragraph."))) {
        return action;
    }
    return {};
}

void populatePanelMenu(QMenu* menu, const QString& panelName,
                       const std::function<void(const QString&)>& dispatch,
                       CommandRegistry* registry)
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
            const QString command = panelCommandId(panelName, row.action);
            const bool preference = row.action == QStringLiteral("addMaskByDefault");
            QAction* action = nullptr;
            if (!command.isEmpty() && registry) {
                action = registry->detachedAction(command, menu);
                if (action) {
                    action->setCheckable(row.checkable || action->isCheckable());
                    menu->addAction(action);
                }
            }
            if (!action) {
                action = menu->addAction(row.label);
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
                } else if (preference || command.isEmpty()) {
                    const QString id = row.action;
                    if (dispatch) {
                        QObject::connect(action, &QAction::triggered, menu,
                                         [dispatch, id]() { dispatch(id); });
                    }
                }
            }
            if (!command.isEmpty() && action && registry) {
                QObject::connect(action, &QAction::triggered, menu,
                                 [registry, command]() { registry->dispatch(command); });
            }
            continue;
        }
        if (!submenu) {
            continue;
        }
        const QString command = panelCommandId(panelName, row.action);
        QAction* action = nullptr;
        if (!command.isEmpty() && registry) {
            action = registry->detachedAction(command, submenu);
            if (action) {
                action->setCheckable(row.checkable || action->isCheckable());
                submenu->addAction(action);
            }
        }
        if (!action) {
            action = submenu->addAction(row.label);
            action->setCheckable(row.checkable);
            if (!row.action.isEmpty()) {
                action->setData(row.action);
            }
            if (row.action.isEmpty()) {
                action->setEnabled(false);
                action->setToolTip(QStringLiteral("%1 — not implemented yet").arg(row.label));
            } else if (dispatch) {
                const QString id = row.action;
                QObject::connect(action, &QAction::triggered, menu,
                                 [dispatch, id]() { dispatch(id); });
            }
        } else {
            submenu->menuAction()->setEnabled(true);
            submenu->menuAction()->setToolTip(QString());
            QObject::connect(action, &QAction::triggered, menu,
                             [registry, command]() { registry->dispatch(command); });
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

PicturaMainWindow* PanelGroup::owningFrame() const
{
    QWidget* top = window();
    if (auto* frame = qobject_cast<PicturaMainWindow*>(top)) {
        return frame;
    }
    // A group hosted in a floating overlay: the overlay's parent is the frame.
    if (top) {
        if (auto* frame = qobject_cast<PicturaMainWindow*>(top->parentWidget())) {
            return frame;
        }
    }
    return nullptr;
}

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
                      [this](const QString& id) { runPanelPreference(id); },
                      owningFrame() ? owningFrame()->registry() : nullptr);
    QObject::connect(headerMenu_, &QMenu::aboutToShow, headerMenu_, [this, panel] {
        // The menu is built once; re-read every command's state so a document
        // opened (or a selection changed) since then is reflected.
        if (PicturaMainWindow* frame = owningFrame()) {
            if (CommandRegistry* registry = frame->registry()) {
                for (QAction* action : headerMenu_->findChildren<QAction*>()) {
                    registry->applyState(action);
                }
            }
        }
        if (panel == QStringLiteral("adjustmentsPanel")) {
            LayersPanel* layersPanel = qobject_cast<LayersPanel*>(tabs_->currentWidget());
            if (!layersPanel) {
                if (QWidget* win = window()) {
                    layersPanel =
                        win->findChild<LayersPanel*>(QStringLiteral("layersPanel"));
                }
            }
            for (QAction* action : headerMenu_->findChildren<QAction*>()) {
                if (action->data().toString() == QStringLiteral("addMaskByDefault")) {
                    action->setChecked(layersPanel && layersPanel->addMaskByDefault());
                }
            }
        }
        PanelMenuHook hook = panelMenuHooks_.value(panel);
        if (!hook) {
            if (PicturaMainWindow* frame = owningFrame()) {
                hook = frame->panelMenuHook(panel);
            }
        }
        if (hook) {
            hook(headerMenu_);
        } else if (auto* actions = qobject_cast<ActionsPanel*>(tabs_->currentWidget())) {
            actions->updatePanelMenu(headerMenu_);
        }
    });
    headerButton_->setMenu(headerMenu_);
}

void PanelGroup::runPanelPreference(const QString& actionId)
{
    if (actionId == QStringLiteral("close")) {
        emit closePanelRequested(currentPanelName());
        return;
    }
    if (actionId == QStringLiteral("closeGroup")) {
        emit closeGroupRequested();
        return;
    }
    if (actionId == QStringLiteral("addMaskByDefault")) {
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
    populatePanelMenu(&menu, panelName, nullptr, nullptr);
    QStringList texts;
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator()) {
            texts.push_back(action->text());
        }
    }
    return texts;
}

} // namespace pictura
