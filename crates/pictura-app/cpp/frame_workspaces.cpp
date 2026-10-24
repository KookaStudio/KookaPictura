#include <QtCore/QRect>
#include "frame_includes.h"

namespace pictura {
namespace {

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
            normal, 1, PanelColumn::kDefaultNormalWidth,
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
            normal, 1, PanelColumn::kDefaultNormalWidth,
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
            normal, 1, PanelColumn::kDefaultNormalWidth,
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
            normal, 1, PanelColumn::kDefaultNormalWidth,
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

PanelColumn* PicturaMainWindow::restoreLegacyDefaultForTest()
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
        QStringLiteral("normal"), 0, PanelColumn::kDefaultNormalWidth,
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

void PicturaMainWindow::applyFreshSessionDefault(const SessionState& state)
{
    bool layoutHasGroups = false;
    for (const QJsonValue& value : state.panelColumns) {
        const QJsonObject entry = value.toObject();
        if (!entry.value(QStringLiteral("tools")).toBool()
            && !entry.value(QStringLiteral("groups")).toArray().isEmpty()) {
            layoutHasGroups = true;
            break;
        }
    }
    if (state.panelGroups.isEmpty() && !layoutHasGroups) {
        regroupAndApply(workspacePresets(QStringLiteral("Essentials")));
    }
}

void PicturaMainWindow::queueOrRestoreFloat(PanelColumn* column, PanelGroup* group,
                                           const QRect& geometry)
{
    if (!column || !geometry.isValid()) {
        return;
    }
    if (!isVisible()) {
        pendingFloatRestores_.append({column, group, geometry});
        return;
    }
    if (group) {
        column->restoreGroupFloat(group, geometry);
    } else {
        column->restoreColumnFloat(geometry);
    }
}

void PicturaMainWindow::applyPendingFloatRestores()
{
    if (!isVisible() || pendingFloatRestores_.isEmpty()) {
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

void PicturaMainWindow::regroupAndApply(const QJsonArray& columns, bool persist)
{
    if (!panelColumn_ || !centerSplitter_) {
        return;
    }
    pendingFloatRestores_.clear();
    // A mid-rebuild stateChanged() must not persist a partial layout.
    const bool wasRestoring = restoringPanelSession_;
    restoringPanelSession_ = true;
    // A frame-parented flyout must not survive its group being harvested.
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
    const QList<PanelFloat*> floats = findChildren<PanelFloat*>(Qt::FindDirectChildrenOnly);
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
            auto* group = new PanelGroup(this);
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
    auto* overflow = new PanelGroup(this);
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

    PanelColumn::refreshSharedFloor(this);
    reapplyColumnStretch();
    restoringPanelSession_ = wasRestoring;
    if (persist && !wasRestoring && pendingFloatRestores_.isEmpty()) {
        saveSession();
    }
}

} // namespace pictura
