#include "panel_host.h"

#include "command_ids.h"
#include "commands.h"
#include "panels/panel_group.h"
#include "panels/swatches_panel.h"
#include "session.h"
#include "workspace_store.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QFile>
#include <QtCore/QRect>
#include <QtGui/QAction>
#include <QtWidgets/QMainWindow>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QSplitter>

namespace pictura {
namespace {

struct BuiltinDef {
    const char* id;
    const char* name;
};

const BuiltinDef kBuiltins[] = {
    {"essentials", "Essentials"},
    {"painting", "Painting"},
    {"photography", "Photography"},
    {"typography", "Typography"},
};

// docs: workspace names are user content; cap the length so a paste cannot
// create an unusable entry.
constexpr int kWorkspaceNameCap = 64;
const QString kDefaultWorkspace = QStringLiteral("Essentials");
const QString kBuiltinKind = QStringLiteral("builtin");
const QString kUserKind = QStringLiteral("user");

QJsonObject presetGroup(const QStringList& order, const QStringList& visible)
{
    QJsonObject group;
    group.insert(QStringLiteral("order"), QJsonArray::fromStringList(order));
    group.insert(QStringLiteral("visible"), QJsonArray::fromStringList(visible));
    group.insert(QStringLiteral("minimized"), false);
    group.insert(QStringLiteral("collapsed"), false);
    return group;
}

QJsonObject presetGroup(const QStringList& panels)
{
    return presetGroup(panels, panels);
}

QJsonObject presetColumn(const QString& railMode, int order, int width,
                         const QJsonArray& groups)
{
    QJsonObject column;
    column.insert(QStringLiteral("side"), QStringLiteral("right"));
    column.insert(QStringLiteral("order"), order);
    column.insert(QStringLiteral("width"), width);
    column.insert(QStringLiteral("railMode"), railMode);
    column.insert(QStringLiteral("groups"), groups);
    return column;
}

QJsonArray rectJson(const QRect& rect)
{
    QJsonArray array;
    array.append(rect.x());
    array.append(rect.y());
    array.append(rect.width());
    array.append(rect.height());
    return array;
}

QRect rectFromJson(const QJsonArray& array)
{
    if (array.size() != 4) {
        return QRect();
    }
    return QRect(array.at(0).toInt(), array.at(1).toInt(), array.at(2).toInt(),
                 array.at(3).toInt());
}

} // namespace

QJsonArray workspacePresets(const QString& name)
{
    // The engine treats the lowest-order right column as the leftmost, so the
    // iconic secondary is order 0 and the wide main column is order 1.
    // ponytail: the iconic width is the remembered normal width; the iconic
    // rail owns the live strip width, so 40 only seeds that memory.
    constexpr int kIconicWidth = 40;
    const QString iconic = QStringLiteral("iconic");
    const QString normal = QStringLiteral("normal");

    QJsonArray columns;
    if (name == QLatin1String("Essentials")) {
        columns.append(presetColumn(
            iconic, 0, kIconicWidth,
            QJsonArray{presetGroup({QStringLiteral("historyPanel")}),
                       presetGroup({QStringLiteral("propertiesPanel")}),
                       presetGroup({QStringLiteral("actionsPanel")}, {})}));
        columns.append(presetColumn(
            normal, 1, PanelColumn::defaultNormalWidth(),
            QJsonArray{
                presetGroup({QStringLiteral("colorPanel"), QStringLiteral("swatchesPanel")}),
                presetGroup({QStringLiteral("adjustmentsPanel"),
                             QStringLiteral("stylesPanel")}),
                presetGroup({QStringLiteral("layersPanel"), QStringLiteral("channelsPanel"),
                             QStringLiteral("pathsPanel")})}));
    } else if (name == QLatin1String("Painting")) {
        columns.append(presetColumn(iconic, 0, kIconicWidth,
                                    QJsonArray{presetGroup({QStringLiteral("historyPanel")})}));
        columns.append(presetColumn(
            normal, 1, PanelColumn::defaultNormalWidth(),
            QJsonArray{
                presetGroup({QStringLiteral("navigatorPanel"),
                             QStringLiteral("swatchesPanel")}),
                presetGroup({QStringLiteral("brushPanel"),
                             QStringLiteral("cloneSourcePanel")}),
                presetGroup({QStringLiteral("layersPanel"), QStringLiteral("channelsPanel"),
                             QStringLiteral("pathsPanel")})}));
    } else if (name == QLatin1String("Photography")) {
        columns.append(presetColumn(
            iconic, 0, kIconicWidth,
            QJsonArray{presetGroup({QStringLiteral("historyPanel")}),
                       presetGroup({QStringLiteral("actionsPanel")})}));
        columns.append(presetColumn(
            normal, 1, PanelColumn::defaultNormalWidth(),
            QJsonArray{
                presetGroup({QStringLiteral("colorPanel"), QStringLiteral("swatchesPanel")}),
                presetGroup({QStringLiteral("adjustmentsPanel"),
                             QStringLiteral("stylesPanel")}),
                presetGroup({QStringLiteral("layersPanel"), QStringLiteral("channelsPanel"),
                             QStringLiteral("pathsPanel")}),
                presetGroup({QStringLiteral("navigatorPanel"),
                             QStringLiteral("histogramPanel"),
                             QStringLiteral("infoPanel")})}));
    } else if (name == QLatin1String("Typography")) {
        columns.append(presetColumn(
            iconic, 0, kIconicWidth,
            QJsonArray{presetGroup({QStringLiteral("historyPanel")}),
                       presetGroup({QStringLiteral("propertiesPanel")})}));
        columns.append(presetColumn(
            normal, 1, PanelColumn::defaultNormalWidth(),
            QJsonArray{
                presetGroup({QStringLiteral("characterPanel"),
                             QStringLiteral("paragraphPanel")}),
                presetGroup({QStringLiteral("paragraphStylesPanel"),
                             QStringLiteral("characterStylesPanel"),
                             QStringLiteral("glyphsPanel")}),
                presetGroup({QStringLiteral("layersPanel"), QStringLiteral("channelsPanel"),
                             QStringLiteral("pathsPanel")})}));
    }
    return columns;
}

PanelColumn* PanelHost::restoreLegacyDefaultForTest()
{
    // The pre-Essentials single-column default: every group lives in the primary
    // column, matching the layout the legacy self-test panel machinery assumes.
    const QStringList color{QStringLiteral("colorPanel"), QStringLiteral("swatchesPanel"),
                            QStringLiteral("stylesPanel")};
    const QStringList adjustments{QStringLiteral("adjustmentsPanel")};
    const QStringList layers{QStringLiteral("layersPanel"), QStringLiteral("channelsPanel"),
                             QStringLiteral("pathsPanel")};
    const QStringList navigator{QStringLiteral("navigatorPanel"),
                                QStringLiteral("histogramPanel"),
                                QStringLiteral("infoPanel")};
    QJsonObject column = presetColumn(
        QStringLiteral("normal"), 0, PanelColumn::defaultNormalWidth(),
        QJsonArray{
            presetGroup(color),
            presetGroup({QStringLiteral("adjustmentsPanel"),
                         QStringLiteral("propertiesPanel")},
                        adjustments),
            presetGroup(layers),
            presetGroup(navigator),
            presetGroup({QStringLiteral("historyPanel")}, {}),
            presetGroup({QStringLiteral("actionsPanel")}, {}),
            presetGroup({QStringLiteral("gradientsPanel"), QStringLiteral("patternsPanel"),
                         QStringLiteral("notesPanel"), QStringLiteral("brushPanel"),
                         QStringLiteral("cloneSourcePanel")},
                        {}),
            presetGroup({QStringLiteral("characterPanel"), QStringLiteral("paragraphPanel"),
                         QStringLiteral("glyphsPanel"),
                         QStringLiteral("paragraphStylesPanel"),
                         QStringLiteral("characterStylesPanel")},
                        {})});
    regroupAndApply(QJsonArray{column});
    return panelColumn_;
}

void PanelHost::applyFreshSessionDefault(const SessionState& state,
                                         const QList<PanelGroup*>& staged)
{
    if (!sessionHasPanelLayout(state)) {
        regroupAndApply(workspacePresets(QStringLiteral("Essentials")), true, staged);
    }
}

void PanelHost::queueOrRestoreFloat(PanelColumn* column, PanelGroup* group,
                                    const QRect& geometry)
{
    if (!column || !geometry.isValid()) {
        return;
    }
    if (!shell_->isVisible()) {
        pendingFloatRestores_.append({column, group, geometry});
        return;
    }
    if (group) {
        column->restoreGroupFloat(group, geometry);
    } else {
        column->restoreColumnFloat(geometry);
    }
}

void PanelHost::applyPendingFloatRestores()
{
    if (!shell_->isVisible() || pendingFloatRestores_.isEmpty()) {
        return;
    }
    const QList<PendingFloatRestore> pending = pendingFloatRestores_;
    pendingFloatRestores_.clear();
    const bool wasRestoring = restoringPanelSession_;
    restoringPanelSession_ = true;
    for (const PendingFloatRestore& item : pending) {
        if (!item.column) {
            continue;
        }
        if (item.group) {
            item.column->restoreGroupFloat(item.group, item.geometry);
            continue;
        }
        if (!item.column->isColumnFloating()
            && (item.column->isToolsColumn() || !item.column->groups().isEmpty())) {
            item.column->restoreColumnFloat(item.geometry);
        }
    }
    restoringPanelSession_ = wasRestoring;
    if (workspacesInitialized_) {
        snapshotActiveWorkspace();
    }
}

void PanelHost::regroupAndApply(const QJsonArray& columns, bool persist,
                                QList<PanelGroup*> staged)
{
    if (!panelColumn_ || !centerSplitter_) {
        return;
    }
    pendingFloatRestores_.clear();
    // A mid-rebuild stateChanged() must not persist a partial layout.
    const bool wasRestoring = restoringPanelSession_;
    restoringPanelSession_ = true;
    // A host-parented flyout must not survive its group being harvested.
    for (PanelColumn* column : allPanelColumns()) {
        if (column) {
            column->closeFlyout();
        }
    }

    struct PooledPanel {
        QWidget* panel = nullptr;
        QString title;
        QIcon icon;
    };
    QList<PooledPanel> pool;
    QSet<QString> pooled;

    auto harvest = [&pool, &pooled](PanelGroup* group) {
        if (!group) {
            return;
        }
        const QList<QWidget*> panels = group->panels();
        for (QWidget* panel : panels) {
            if (!panel) {
                continue;
            }
            const QString name = panel->objectName();
            if (name.isEmpty() || pooled.contains(name)) {
                continue;
            }
            PooledPanel entry;
            // Read the tab text/icon before the panel leaves the group.
            entry.title = group->titleForPanel(name);
            entry.icon = group->iconForPanel(name);
            entry.panel = group->takePanel(name);
            if (!entry.panel) {
                continue;
            }
            pooled.insert(name);
            pool.append(entry);
        }
    };

    // Groups buildPanels staged for a fresh session first; they never joined a
    // column, so nothing else removes them.
    for (PanelGroup* group : staged) {
        harvest(group);
        group->hide();
        group->deleteLater();
    }
    // Docked groups first, then the overlays. The tools column never takes part.
    for (PanelColumn* column : allPanelColumns()) {
        if (!column || column->isToolsColumn() || column->isColumnFloating()) {
            continue;
        }
        for (PanelGroup* group : column->groups()) {
            harvest(group);
        }
    }
    // Group floats first, then whole-column overlays. A group float can belong
    // to a column that is itself floating; harvesting it before the column is
    // torn down prevents an owner pointer into a dying column.
    const QList<PanelFloat*> floats =
        shell_->findChildren<PanelFloat*>(Qt::FindDirectChildrenOnly);
    for (PanelFloat* floatWindow : floats) {
        if (!floatWindow) {
            continue;
        }
        if (PanelGroup* group = floatWindow->group()) {
            harvest(group);
            if (floatWindow->onClose) {
                floatWindow->onClose();
            }
        }
    }
    for (PanelFloat* floatWindow : floats) {
        if (!floatWindow || floatWindow->group()) {
            continue;
        }
        auto* column = qobject_cast<PanelColumn*>(floatWindow->content());
        if (!column || column->isToolsColumn()) {
            continue;
        }
        for (PanelGroup* group : column->groups()) {
            harvest(group);
        }
        column->cancelColumnFloat();
        if (column != panelColumn_) {
            column->deleteLater();
        }
    }

    clearDynamicColumns();

    // Drop the (now empty) groups the primary column still holds. Names can
    // repeat after a torn-off group docks back, so remove by name one at a time.
    const QList<PanelGroup*> stale = panelColumn_->groups();
    for (PanelGroup* group : stale) {
        if (!group) {
            continue;
        }
        if (PanelGroup* taken = panelColumn_->takeGroup(group)) {
            taken->deleteLater();
        }
    }

    // Order the stored columns by `order` (a stable selection sort: at most a
    // handful of entries).
    QList<QJsonObject> entries;
    for (const QJsonValue& value : columns) {
        if (value.isObject()) {
            entries.append(value.toObject());
        }
    }
    for (int i = 0; i < entries.size(); ++i) {
        for (int j = i + 1; j < entries.size(); ++j) {
            if (entries.at(j).value(QStringLiteral("order")).toInt()
                < entries.at(i).value(QStringLiteral("order")).toInt()) {
                entries.swapItemsAt(i, j);
            }
        }
    }

    QList<int> leftIndices;
    QList<int> rightIndices;
    for (int i = 0; i < entries.size(); ++i) {
        if (entries.at(i).value(QStringLiteral("tools")).toBool()) {
            continue;
        }
        if (entries.at(i).value(QStringLiteral("side")).toString() == QStringLiteral("left")) {
            leftIndices.append(i);
        } else {
            rightIndices.append(i);
        }
    }

    QHash<QString, int> poolIndex;
    for (int i = 0; i < pool.size(); ++i) {
        poolIndex.insert(pool.at(i).panel->objectName(), i);
    }
    QSet<QString> used;

    struct BuiltGroup {
        PanelColumn* column = nullptr;
        PanelGroup* group = nullptr;
        QJsonObject spec;
    };
    QList<BuiltGroup> built;
    QHash<int, PanelColumn*> builtColumns;

    auto buildInto = [&](PanelColumn* column, int entryIndex) {
        const QJsonObject entry = entries.at(entryIndex);
        builtColumns.insert(entryIndex, column);
        for (const QJsonValue& value : entry.value(QStringLiteral("groups")).toArray()) {
            const QJsonObject spec = value.toObject();
            auto* group = new PanelGroup(shell_);
            for (const QJsonValue& panelValue :
                 spec.value(QStringLiteral("order")).toArray()) {
                const QString name = panelValue.toString();
                const int idx = poolIndex.value(name, -1);
                if (idx < 0 || used.contains(name)) {
                    continue;
                }
                const PooledPanel& pooledPanel = pool.at(idx);
                group->addPanel(pooledPanel.panel, pooledPanel.title, pooledPanel.icon);
                used.insert(name);
            }
            if (group->titles().isEmpty()) {
                group->deleteLater();
                continue;
            }
            column->addGroup(group);
            built.append({column, group, spec});
        }
        column->setRailMode(entry.value(QStringLiteral("railMode")).toString()
                            == QStringLiteral("iconic"));
        const int width = entry.value(QStringLiteral("width")).toInt();
        if (width > 0) {
            column->setRestoredWidth(width);
        }
    };

    // The primary column survives a rebuild; re-attach it if a float detached it.
    if (centerSplitter_->indexOf(panelColumn_) < 0) {
        centerSplitter_->insertWidget(centerSplitter_->count(), panelColumn_);
    }

    // The first right-hand column reuses the primary identity; later ones are
    // anchored after the previous so the order matches the spec left-to-right.
    bool placedPrimary = false;
    PanelColumn* anchor = panelColumn_;
    // Unnamed panels join the rightmost (main) column, kept hidden, so they stay
    // reachable from a normal column rather than the iconic strip.
    PanelColumn* overflowColumn = panelColumn_;
    for (int index : rightIndices) {
        PanelColumn* column = nullptr;
        if (!placedPrimary) {
            column = panelColumn_;
            placedPrimary = true;
        } else {
            column = createPanelColumn(PanelSide::Right, anchor);
            anchor = column;
        }
        if (column) {
            buildInto(column, index);
            overflowColumn = column;
        }
    }
    // Left columns stay left of the document tabs. If the tools column owns the
    // splitter head, place them after it so the tools column keeps its position.
    PanelColumn* leftAnchor =
        toolsColumn_ && centerSplitter_->indexOf(toolsColumn_) == 0 ? toolsColumn_ : nullptr;
    for (int index : leftIndices) {
        PanelColumn* column = leftAnchor
                                  ? createPanelColumn(PanelSide::Right, leftAnchor)
                                  : createPanelColumn(PanelSide::Left);
        if (!column) {
            continue;
        }
        leftAnchor = column;
        buildInto(column, index);
    }

    // Every panel the spec did not name stays reachable from Window > Panels.
    auto* overflow = new PanelGroup(shell_);
    for (const PooledPanel& pooledPanel : pool) {
        const QString name = pooledPanel.panel->objectName();
        if (used.contains(name)) {
            continue;
        }
        overflow->addPanel(pooledPanel.panel, pooledPanel.title, pooledPanel.icon);
        used.insert(name);
    }
    if (!overflow->titles().isEmpty()) {
        overflowColumn->addGroup(overflow);
        for (QWidget* panel : overflow->panels()) {
            if (panel) {
                overflowColumn->showPanel(panel->objectName(), false);
            }
        }
    } else {
        overflow->deleteLater();
    }

    for (const BuiltGroup& item : built) {
        QSet<QString> visible;
        for (const QJsonValue& value :
             item.spec.value(QStringLiteral("visible")).toArray()) {
            visible.insert(value.toString());
        }
        for (QWidget* panel : item.group->panels()) {
            if (panel) {
                item.column->showPanel(panel->objectName(),
                                       visible.contains(panel->objectName()));
            }
        }
        item.group->setMinimized(item.spec.value(QStringLiteral("minimized")).toBool(false));
        item.group->setCollapsedToIcons(
            item.spec.value(QStringLiteral("collapsed")).toBool(false));
        item.group->setCurrentToFirstVisible();
    }

    // Detach the stored floats after the docked layout is fully built. The
    // whole column goes first so its group overlays have a live, floating owner
    // and cannot trigger empty-column cleanup before the column is restored.
    for (auto it = builtColumns.cbegin(); it != builtColumns.cend(); ++it) {
        const QJsonObject entry = entries.at(it.key());
        if (!entry.value(QStringLiteral("floating")).toBool(false)) {
            continue;
        }
        PanelColumn* column = it.value();
        if (!column || column->groups().isEmpty()) {
            continue;
        }
        const QRect rect =
            rectFromJson(entry.value(QStringLiteral("floatGeometry")).toArray());
        queueOrRestoreFloat(column, nullptr, rect);
    }
    for (const BuiltGroup& item : built) {
        if (!item.spec.value(QStringLiteral("floating")).toBool(false)) {
            continue;
        }
        const QRect rect =
            rectFromJson(item.spec.value(QStringLiteral("floatGeometry")).toArray());
        queueOrRestoreFloat(item.column, item.group, rect);
    }

    // `removeColumnIfEmpty` may have hidden the primary while its groups emptied;
    // an empty primary stays hidden rather than forcing a blank column visible.
    panelColumn_->setVisible(!panelColumn_->groups().isEmpty() && !panelsHidden_);

    refreshSharedFloor();
    reapplyColumnStretch();
    restoringPanelSession_ = wasRestoring;
    if (persist && !wasRestoring && pendingFloatRestores_.isEmpty()) {
        requestSessionSave();
    }
}

QJsonArray PanelHost::serializeWorkspaceLayout() const
{
    // v10: the ordered per-column layout, in central-splitter order, each with
    // its normal-mode width, rail mode, and floating state/geometry.
    QJsonArray columns;
    int order = 0;
    for (PanelColumn* column : allPanelColumns()) {
        if (!column) {
            continue;
        }
        // A detached (cancelled-float) column is still reachable through its
        // overlay until the event loop runs; it is not a live pane.
        if (centerSplitter_->indexOf(column) < 0 && !column->isColumnFloating()) {
            continue;
        }
        QJsonObject entry;
        entry.insert(QStringLiteral("side"),
                     column->side() == PanelSide::Left ? QStringLiteral("left")
                                                       : QStringLiteral("right"));
        entry.insert(QStringLiteral("order"), order++);
        entry.insert(QStringLiteral("width"), column->persistedWidth());
        entry.insert(QStringLiteral("railMode"),
                     column->railMode() ? QStringLiteral("iconic")
                                        : QStringLiteral("normal"));
        if (column->isColumnFloating()) {
            entry.insert(QStringLiteral("floating"), true);
            entry.insert(QStringLiteral("floatGeometry"),
                         rectJson(column->columnFloatGeometry()));
        }
        if (column->isToolsColumn()) {
            entry.insert(QStringLiteral("tools"), true);
            entry.insert(QStringLiteral("groups"), QJsonArray());
        } else {
            entry.insert(QStringLiteral("groups"), column->savePanelState());
        }
        columns.append(entry);
    }
    return columns;
}

int PanelHost::workspaceIndexOf(const QString& name) const
{
    for (int i = 0; i < workspaceStore_.workspaces.size(); ++i) {
        if (workspaceStore_.workspaces.at(i).name == name) {
            return i;
        }
    }
    return -1;
}

QString PanelHost::activeWorkspace() const
{
    return workspaceStore_.activeWorkspace;
}

void PanelHost::initWorkspaces(bool sessionHadLayout)
{
    WorkspaceStore loaded;
    const bool haveStore = loadWorkspaceStore(&loaded);
    workspaceStore_ = haveStore ? loaded : WorkspaceStore{};

    if (!haveStore || workspaceStore_.workspaces.isEmpty()) {
        // First run (or a missing/unusable store): seed the built-in presets.
        workspaceStore_.workspaces.clear();
        for (const BuiltinDef& def : kBuiltins) {
            WorkspaceRecord record;
            record.id = QString::fromLatin1(def.id);
            record.name = QString::fromLatin1(def.name);
            record.kind = kBuiltinKind;
            record.factory = workspacePresets(record.name);
            workspaceStore_.workspaces.append(record);
        }
    } else {
        // The loaded record set is authoritative; a preset deleted last session
        // stays gone. Builtins still take their factory from code so Reset works.
        for (WorkspaceRecord& record : workspaceStore_.workspaces) {
            if (record.kind == kBuiltinKind) {
                record.factory = workspacePresets(record.name);
            }
        }
    }

    if (workspaceIndexOf(workspaceStore_.activeWorkspace) < 0) {
        workspaceStore_.activeWorkspace = workspaceStore_.workspaces.isEmpty()
                                              ? kDefaultWorkspace
                                              : workspaceStore_.workspaces.first().name;
    }

    const int active = workspaceIndexOf(workspaceStore_.activeWorkspace);
    if (active >= 0) {
        WorkspaceRecord& record = workspaceStore_.workspaces[active];
        if (!haveStore || sessionHadLayout) {
            // D3: the restored live layout is the active workspace's arrangement.
            record.layout = serializeWorkspaceLayout();
            record.hasLayout = true;
        } else {
            // The session lost its layout: keep the active workspace's own saved
            // arrangement instead of overwriting it with the fresh default.
            const QJsonArray applied = record.hasLayout
                                           ? record.layout
                                           : (record.kind == kBuiltinKind
                                                  ? workspacePresets(record.name)
                                                  : record.factory);
            regroupAndApply(applied);
            record.layout = serializeWorkspaceLayout();
            record.hasLayout = true;
        }
    }

    workspacesInitialized_ = true;
    saveWorkspaceStore(workspaceStore_);
    if (commands_) {
        commands_->refresh();
    }
    attachWorkspaceSwitcher();
}

void PanelHost::attachWorkspaceSwitcher()
{
    if (!hooks_.attachWorkspaceMenu) {
        return;
    }
    hooks_.attachWorkspaceMenu(workspaceMenu());
    updateWorkspaceSwitcher();
    if (QMenu* menu = workspaceMenu()) {
        refreshWorkspaceMenu(menu);
    }
}

void PanelHost::updateWorkspaceSwitcher()
{
    if (hooks_.setActiveWorkspace) {
        const QString& name = workspaceStore_.activeWorkspace;
        const int index = workspaceIndexOf(name);
        const bool builtin = index >= 0 && workspaceStore_.workspaces.at(index).kind == kBuiltinKind;
        hooks_.setActiveWorkspace(builtin
            ? QCoreApplication::translate("PicturaCommands", name.toUtf8().constData()) : name);
    }
}

void PanelHost::snapshotActiveWorkspace()
{
    if (!workspacesInitialized_) {
        return;
    }
    const int index = workspaceIndexOf(workspaceStore_.activeWorkspace);
    if (index < 0) {
        return;
    }
    WorkspaceRecord& record = workspaceStore_.workspaces[index];
    const QJsonArray layout = serializeWorkspaceLayout();
    if (record.hasLayout && record.layout == layout) {
        return;
    }
    record.layout = layout;
    record.hasLayout = true;
    saveWorkspaceStore(workspaceStore_);
}

void PanelHost::switchWorkspace(const QString& name)
{
    if (name.isEmpty()) {
        return;
    }
    if (name == workspaceStore_.activeWorkspace) {
        // A re-select restores a toggle that was switched off without a switch.
        if (commands_) {
            commands_->refresh();
        }
        return;
    }
    const int target = workspaceIndexOf(name);
    if (target < 0) {
        return;
    }

    // Remember the outgoing workspace's live arrangement (D3).
    const int outgoing = workspaceIndexOf(workspaceStore_.activeWorkspace);
    if (outgoing >= 0) {
        workspaceStore_.workspaces[outgoing].layout = serializeWorkspaceLayout();
        workspaceStore_.workspaces[outgoing].hasLayout = true;
    }

    workspaceStore_.activeWorkspace = name;
    const WorkspaceRecord& record = workspaceStore_.workspaces.at(target);
    if (record.hasLayout && !record.layout.isEmpty()) {
        regroupAndApply(record.layout);
    } else if (record.kind == kBuiltinKind) {
        regroupAndApply(workspacePresets(name));
    } else {
        regroupAndApply(record.factory);
    }
    saveWorkspaceStore(workspaceStore_);
    if (commands_) {
        commands_->refresh();
    }
    updateWorkspaceSwitcher();
}

void PanelHost::resetWorkspace()
{
    const int index = workspaceIndexOf(workspaceStore_.activeWorkspace);
    if (index < 0) {
        return;
    }
    WorkspaceRecord& record = workspaceStore_.workspaces[index];
    const QJsonArray factory = record.kind == kBuiltinKind ? workspacePresets(record.name)
                                                           : record.factory;
    if (factory.isEmpty()) {
        return;
    }
    record.layout = factory;
    record.hasLayout = true;
    regroupAndApply(factory);
    saveWorkspaceStore(workspaceStore_);
    if (commands_) {
        commands_->refresh();
    }
    updateWorkspaceSwitcher();
}

QString PanelHost::workspaceNameError(const QString& name) const
{
    const QString trimmed = name.trimmed();
    if (trimmed.isEmpty()) {
        return tr("Enter a workspace name.");
    }
    if (trimmed.size() > kWorkspaceNameCap) {
        return tr("Workspace names cannot exceed %1 characters.").arg(kWorkspaceNameCap);
    }
    const QString duplicate = tr("A workspace named \"%1\" already exists.").arg(trimmed);
    if (workspaceIndexOf(trimmed) >= 0) {
        return duplicate;
    }
    // The four preset names stay reserved even when their record was deleted, so
    // a user workspace can never shadow a preset that may be restored.
    for (const BuiltinDef& def : kBuiltins) {
        if (trimmed == QString::fromLatin1(def.name)) {
            return duplicate;
        }
    }
    return QString();
}

bool PanelHost::createWorkspace(const QString& name)
{
    if (!workspaceNameError(name).isEmpty()) {
        return false;
    }
    const QString trimmed = name.trimmed();

    QString id;
    for (int n = 1; id.isEmpty(); ++n) {
        const QString candidate = QStringLiteral("user-") + QString::number(n);
        bool used = false;
        for (const WorkspaceRecord& record : workspaceStore_.workspaces) {
            if (record.id == candidate) {
                used = true;
                break;
            }
        }
        if (!used) {
            id = candidate;
        }
    }

    const QJsonArray layout = serializeWorkspaceLayout();
    WorkspaceRecord record;
    record.id = id;
    record.name = trimmed;
    record.kind = kUserKind;
    record.layout = layout;
    record.factory = layout;
    record.hasLayout = true;
    workspaceStore_.workspaces.append(record);
    workspaceStore_.activeWorkspace = trimmed;

    saveWorkspaceStore(workspaceStore_);
    if (commands_) {
        commands_->refresh();
    }
    updateWorkspaceSwitcher();
    return true;
}

bool PanelHost::deleteWorkspace(const QString& name)
{
    // The active workspace is load-bearing (it owns the live layout).
    if (name.isEmpty() || name == workspaceStore_.activeWorkspace) {
        return false;
    }
    const int index = workspaceIndexOf(name);
    if (index < 0) {
        return false;
    }
    const QString id = workspaceStore_.workspaces.at(index).id;
    workspaceStore_.workspaces.removeAt(index);
    // The index rewrite drops the record from every listing; the per-id file is
    // removed too so it cannot resurface.
    QFile::remove(workspaceStoreDir() + QLatin1Char('/') + id + QStringLiteral(".json"));
    saveWorkspaceStore(workspaceStore_);
    if (commands_) {
        commands_->refresh();
    }
    updateWorkspaceSwitcher();
    return true;
}

QStringList PanelHost::userWorkspaces() const
{
    QStringList names;
    for (const WorkspaceRecord& record : workspaceStore_.workspaces) {
        if (record.kind == kUserKind) {
            names.append(record.name);
        }
    }
    return names;
}

QMenu* PanelHost::workspaceMenu() const
{
    return commands_ ? commands_->menu({QStringLiteral("Window"), QStringLiteral("Workspace")})
                     : nullptr;
}

void PanelHost::refreshWorkspaceMenu(QMenu* menu)
{
    if (!menu) {
        return;
    }
    for (QAction* action : workspaceMenuActions_) {
        menu->removeAction(action);
        delete action;
    }
    workspaceMenuActions_.clear();

    // A preset the user deleted is hidden, not shown as a dead entry.
    static const struct {
        const char* command;
        const char* name;
    } presets[] = {
        {command_ids::WindowWorkspaceEssentials, "Essentials"},
        {command_ids::WindowWorkspacePainting, "Painting"},
        {command_ids::WindowWorkspacePhotography, "Photography"},
        {command_ids::WindowWorkspaceTypography, "Typography"},
    };
    for (const auto& preset : presets) {
        if (!commands_) {
            break;
        }
        if (QAction* action = commands_->action(QString::fromLatin1(preset.command))) {
            // A preset shows only while its own builtin record exists; a user
            // workspace that happens to share the name must not reveal it.
            const int index = workspaceIndexOf(QString::fromLatin1(preset.name));
            action->setVisible(index >= 0
                               && workspaceStore_.workspaces.at(index).kind == kBuiltinKind);
        }
    }

    // User workspaces join the presets section: insert before the first separator
    // (the one after the preset commands), not into the actions section.
    QAction* before = nullptr;
    for (QAction* action : menu->actions()) {
        if (action->isSeparator()) {
            before = action;
            break;
        }
    }
    if (!before && commands_) {
        before = commands_->action(command_ids::WindowWorkspaceNew);
    }
    for (const WorkspaceRecord& record : workspaceStore_.workspaces) {
        if (record.kind != kUserKind) {
            continue;
        }
        auto* action = new QAction(record.name, menu);
        action->setCheckable(true);
        action->setChecked(workspaceStore_.activeWorkspace == record.name);
        const QString name = record.name;
        connect(action, &QAction::triggered, this, [this, name]() { switchWorkspace(name); });
        menu->insertAction(before, action);
        workspaceMenuActions_.append(action);
    }

    updateWorkspaceSwitcher();
}

QStringList PanelHost::workspaceMenuLabels() const
{
    QStringList labels;
    if (QMenu* menu = workspaceMenu()) {
        for (QAction* action : menu->actions()) {
            if (!action->isSeparator()) {
                labels.append(action->text());
            }
        }
    }
    return labels;
}

int PanelHost::workspaceMenuSeparatorCount() const
{
    int count = 0;
    if (QMenu* menu = workspaceMenu()) {
        for (QAction* action : menu->actions()) {
            if (action->isSeparator()) {
                ++count;
            }
        }
    }
    return count;
}

QStringList PanelHost::deleteWorkspaceChoices() const
{
    // Exactly the list the Delete chooser offers: every workspace but the active.
    QStringList names;
    for (const WorkspaceRecord& record : workspaceStore_.workspaces) {
        if (record.name != workspaceStore_.activeWorkspace) {
            names.append(record.name);
        }
    }
    return names;
}

} // namespace pictura
