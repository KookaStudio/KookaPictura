#include "panel_host.h"

#include "panels/panel_group.h"
#include "panels/swatches_panel.h"
#include "session.h"

#include <QtCore/QRect>
#include <QtWidgets/QSplitter>

namespace pictura {

void PanelHost::saveState(SessionState& state)
{
    if (swatchesPanel_) {
        QJsonArray swatches;
        for (const Swatch& swatch : swatchesPanel_->grid()->swatches()) {
            QJsonObject entry;
            entry.insert(QStringLiteral("name"), swatch.name);
            entry.insert(QStringLiteral("color"), swatch.color.name());
            swatches.append(entry);
        }
        state.swatches = swatches;
        state.swatchesLoaded = true;
    }
    if (panelColumn_) {
        const bool iconic = panelColumn_->railMode();
        state.panelRailMode = iconic ? QStringLiteral("iconic") : QStringLiteral("normal");
        // Keep the last normal-mode width; an iconic column has no width of its
        // own, so the loaded value survives unchanged.
        if (!iconic) {
            const int remembered = panelColumn_->persistedWidth();
            if (remembered > 0) {
                state.railWidth = remembered;
            }
        }
        state.autoCollapseIconic = panelColumn_->autoCollapseIconic();
        state.autoShowHidden = panelColumn_->autoShowHidden();
        // Legacy flat mirror of the primary column kept for older stores.
        state.panelGroups = panelColumn_->savePanelState();
    }
    // v9: the ordered per-column layout, in central-splitter order.
    state.panelColumns = serializeWorkspaceLayout();
}

void PanelHost::applyPanelSession(const SessionState& state)
{
    if (!panelColumn_) {
        return;
    }
    restoringPanelSession_ = true;
    // A new session apply supersedes any float restore still waiting for show.
    pendingFloatRestores_.clear();

    if (!sessionHasColumnLayout(state)) {
        // No per-column layout: keep the legacy flat-state behaviour and reset
        // the tools column to its default left slot.
        const QList<PanelColumn*> live = allPanelColumns();
        for (PanelColumn* column : live) {
            if (column) {
                column->closeFlyout();
            }
        }
        for (PanelColumn* column : live) {
            if (column) {
                column->redockGroupFloats();
            }
        }
        for (PanelColumn* column : live) {
            if (!column || !column->isColumnFloating()) {
                continue;
            }
            column->cancelColumnFloat();
            if (centerSplitter_->indexOf(column) < 0) {
                centerSplitter_->insertWidget(centerSplitter_->count(), column);
            }
            column->setVisible(!panelsHidden_);
        }
        clearDynamicColumns();
        panelColumn_->restorePanelState(state.panelGroups);
        panelColumn_->setRailMode(state.panelRailMode == QStringLiteral("iconic"));
        panelColumn_->setRestoredWidth(state.railWidth > 0
                                           ? state.railWidth
                                           : PanelColumn::defaultNormalWidth());
        bool anyVisible = false;
        for (PanelGroup* group : panelColumn_->groups()) {
            if (!group) {
                continue;
            }
            const bool visible = !group->visibleTitles().isEmpty();
            group->setVisible(visible);
            anyVisible = anyVisible || visible;
        }
        if (anyVisible) {
            panelColumn_->setVisible(!panelsHidden_);
        }
        if (toolsColumn_ && centerSplitter_) {
            toolsColumn_->cancelColumnFloat();
            toolsColumn_->setParent(nullptr);
            centerSplitter_->insertWidget(0, toolsColumn_);
            toolsColumn_->setVisible(!panelsHidden_);
            reapplyColumnStretch();
        }
        for (PanelColumn* column : allPanelColumns()) {
            if (column) {
                column->setAutoCollapseIconic(state.autoCollapseIconic);
                column->setAutoShowHidden(state.autoShowHidden);
            }
        }
        restoringPanelSession_ = false;
        return;
    }

    // Normalise pre-v7/v8 entries before the one layout builder runs. The first
    // right-hand column is the primary: it inherits the legacy railWidth; every
    // other column gets the default normal width.
    QJsonArray layout = state.panelColumns;
    int primaryIndex = -1;
    int primaryOrder = (1 << 30);
    for (int i = 0; i < layout.size(); ++i) {
        const QJsonObject entry = layout.at(i).toObject();
        if (entry.value(QStringLiteral("tools")).toBool()
            || entry.value(QStringLiteral("side")).toString() == QStringLiteral("left")) {
            continue;
        }
        const int order = entry.value(QStringLiteral("order")).toInt(0);
        if (order < primaryOrder) {
            primaryOrder = order;
            primaryIndex = i;
        }
    }
    for (int i = 0; i < layout.size(); ++i) {
        QJsonObject entry = layout.at(i).toObject();
        if (entry.value(QStringLiteral("tools")).toBool()) {
            continue;
        }
        if (!entry.contains(QStringLiteral("railMode"))) {
            entry.insert(QStringLiteral("railMode"), state.panelRailMode);
        }
        if (entry.value(QStringLiteral("width")).toInt() <= 0) {
            entry.insert(QStringLiteral("width"),
                         i == primaryIndex && state.railWidth > 0
                             ? state.railWidth
                             : PanelColumn::defaultNormalWidth());
        }
        layout.replace(i, entry);
    }

    regroupAndApply(layout, false);

    // regroupAndApply creates fresh columns; re-apply the legacy global flags to
    // every docked and floating column.
    for (PanelColumn* column : allPanelColumns()) {
        if (!column) {
            continue;
        }
        column->setAutoCollapseIconic(state.autoCollapseIconic);
        column->setAutoShowHidden(state.autoShowHidden);
    }

    // The tools entry is skipped by regroupAndApply (workspace apply must leave
    // Tools untouched), so place it here and restore a floating Tools overlay
    // only on the session path.
    bool haveTools = false;
    bool toolsRight = false;
    bool toolsFloating = false;
    QRect toolsGeometry;
    for (const QJsonValue& value : state.panelColumns) {
        const QJsonObject entry = value.toObject();
        if (!entry.value(QStringLiteral("tools")).toBool()) {
            continue;
        }
        haveTools = true;
        toolsRight = entry.value(QStringLiteral("side")).toString() == QStringLiteral("right");
        toolsFloating = entry.value(QStringLiteral("floating")).toBool(false);
        const QJsonArray geometry = entry.value(QStringLiteral("floatGeometry")).toArray();
        if (geometry.size() == 4) {
            toolsGeometry = QRect(geometry.at(0).toInt(), geometry.at(1).toInt(),
                                  geometry.at(2).toInt(), geometry.at(3).toInt());
        }
        break;
    }
    if (toolsColumn_ && centerSplitter_) {
        toolsColumn_->cancelColumnFloat();
        toolsColumn_->setParent(nullptr);
        const int at = toolsRight ? centerSplitter_->count() : 0;
        centerSplitter_->insertWidget(at, toolsColumn_);
        toolsColumn_->setVisible(!panelsHidden_);
        if (toolsColumn_->toolsContentForTest()) {
            toolsColumn_->refreshToolsWidth();
        }
        reapplyColumnStretch();
        if (haveTools && toolsFloating && toolsGeometry.isValid()) {
            queueOrRestoreFloat(toolsColumn_, nullptr, toolsGeometry);
        }
    }

    restoringPanelSession_ = false;
}

}
