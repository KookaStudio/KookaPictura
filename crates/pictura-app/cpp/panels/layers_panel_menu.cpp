#include "memory_refusal.h"
#include "layers_panel.h"

#include "api/engine_bridge.h"
#include "commands.h"
#include "document_command.h"

#include "export_as_dialog.h"
#include "layer_style_dialog.h"
#include "layers_panel_internal.h"
#include "layers_row_actions.h"

#include "api/layers_bridge.h"

#include <QtCore/QHash>
#include <QtCore/QItemSelectionModel>
#include <QtCore/QModelIndex>
#include <QtCore/QPoint>
#include <QtCore/QStringList>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QIcon>
#include <QtGui/QPixmap>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QMenu>
#include <QtWidgets/QTreeView>

namespace pictura {

namespace {

QString labelName(int label)
{
    switch (label) {
    case 1:
        return QStringLiteral("Red");
    case 2:
        return QStringLiteral("Orange");
    case 3:
        return QStringLiteral("Yellow");
    case 4:
        return QStringLiteral("Green");
    case 5:
        return QStringLiteral("Blue");
    case 6:
        return QStringLiteral("Violet");
    case 7:
        return QStringLiteral("Gray");
    default:
        return QStringLiteral("None");
    }
}

// The real layer path behind a synthetic smart-filter row path.
QString smartFilterBase(const QString& path)
{
    const int at = path.indexOf(QStringLiteral("/@sf"));
    return at < 0 ? path : path.left(at);
}

// The filter index in a `.../@sf/<i>` path (0 when absent).
int smartFilterIndex(const QString& path)
{
    const int at = path.lastIndexOf(QLatin1Char('/'));
    return at < 0 ? 0 : path.mid(at + 1).toInt();
}

// The chain index a filter dragged from `from` lands at when dropped on the
// filter row `target` with `mode` (0 above, 1 below, 2 onto it). Rows list the
// newest filter on top, so "above" means applied after the target; dropping
// onto a row takes its place.
int smartFilterDropIndex(int from, int target, int mode)
{
    int insert = mode == 0 ? target + 1 : mode == 1 ? target : (from < target ? target + 1 : target);
    if (from < insert) {
        --insert;
    }
    return insert;
}

// Whether `dragged` and `target` are filter rows of one smart object.
bool sameSmartFilterList(const QString& dragged, const QString& target)
{
    return dragged.contains(QStringLiteral("/@sf/")) && target.contains(QStringLiteral("/@sf/"))
        && smartFilterBase(dragged) == smartFilterBase(target);
}

} // namespace

bool LayersPanel::canDropRow(const QString& dragged, const QString& target, int mode) const
{
    if (!view_) {
        return false;
    }
    if (dragged.contains(QStringLiteral("/@sf/"))) {
        return sameSmartFilterList(dragged, target)
            && smartFilterDropIndex(smartFilterIndex(dragged), smartFilterIndex(target), mode)
                   != smartFilterIndex(dragged);
    }
    return view_->can_move_layer_to(dragged, target, mode);
}

bool LayersPanel::dropRow(const QString& dragged, const QString& target, int mode)
{
    if (!view_) {
        return false;
    }
    if (!dragged.contains(QStringLiteral("/@sf/"))) {
        return view_->move_layer_to(dragged, target, mode);
    }
    if (!sameSmartFilterList(dragged, target)) {
        return false;
    }
    const int from = smartFilterIndex(dragged);
    const int to = smartFilterDropIndex(from, smartFilterIndex(target), mode);
    if (!reorder_layer_smart_filter(*view_, smartFilterBase(dragged), from, to)) {
        return false;
    }
    // Rebuilt after the view finishes the drop.
    QTimer::singleShot(0, this, [this] { refresh(); });
    return true;
}

void LayersPanel::showContextMenu(const QPoint& pos)
{
    if (!view_) {
        return;
    }
    const QModelIndex index = tree_->indexAt(pos);
    if (!index.isValid()) {
        return;
    }
    // A pending transform resolves first: the menu's own selection move and its
    // actions both change the document.
    if (!resolvePendingTransform()) {
        return;
    }
    if (delegate_->eyeColumnContains(tree_->visualRect(index), index, pos)) {
        showEyeMenu(pos, index);
        return;
    }
    // A right-click on a raster mask thumbnail opens the mask menu (not the
    // generic row menu), as CS6 does.
    const QRect maskThumb = delegate_->maskThumbRect(tree_->visualRect(index), index);
    if (!maskThumb.isEmpty() && maskThumb.contains(pos)) {
        showMaskThumbMenu(pos, index);
        return;
    }
    const QString path = pathForProxyIndex(index);
    if (path.isEmpty()) {
        return;
    }
    // A row action runs on the clicked row: make it the anchor while keeping
    // a multi-selection the row is already part of.
    makeRowAnchor(path);

    // The synthetic Smart Filters rows have their own CS6 menus, not the
    // generic layer-row menu.
    const QString rowKind = index.data(KindRole).toString();
    if (rowKind == QLatin1String("smart-filters") || rowKind == QLatin1String("smart-filter")) {
        QMenu menu(tree_);
        if (rowKind == QLatin1String("smart-filters")) {
            populateSmartFiltersGroupMenu(menu, path);
        } else {
            populateSmartFilterMenu(menu, path);
        }
        menu.exec(tree_->viewport()->mapToGlobal(pos));
        return;
    }
    // The layer-effects rows and the fx badge carry the CS6 layer-style menu.
    if (rowKind == QLatin1String("layer-effects") || rowKind == QLatin1String("layer-effect")) {
        const int at = path.indexOf(QStringLiteral("/@fx"));
        QMenu menu(tree_);
        if (at >= 0) {
            populateLayerStyleMenu(menu, path.left(at));
        }
        menu.exec(tree_->viewport()->mapToGlobal(pos));
        return;
    }
    if (index.data(HasLayerStyleRole).toBool()) {
        const QRect fx = delegate_->fxRect(tree_->visualRect(index), index);
        if (!fx.isEmpty() && fx.contains(pos)) {
            QMenu menu(tree_);
            populateLayerStyleMenu(menu, path);
            menu.exec(tree_->viewport()->mapToGlobal(pos));
            return;
        }
    }

    QMenu menu(tree_);
    // The vector-mask thumbnail has its own menu of vector-mask commands.
    const QRect vectorThumb = delegate_->vectorMaskThumbRect(tree_->visualRect(index), index);
    if (!vectorThumb.isEmpty() && vectorThumb.contains(pos)) {
        populateVectorMaskMenu(menu, path);
        menu.exec(tree_->viewport()->mapToGlobal(pos));
        return;
    }
    const LayerRowNode* row = model_ ? model_->rowForPath(path) : nullptr;
    if (!row) {
        return;
    }
    populateRowMenu(menu, path, index.data(ColorRole).toInt(), *row);
    menu.exec(tree_->viewport()->mapToGlobal(pos));
}

void LayersPanel::showMaskThumbMenu(const QPoint& pos, const QModelIndex& index)
{
    const QString path = pathForProxyIndex(index);
    if (path.isEmpty() || !view_) {
        return;
    }
    // CS6 makes the clicked mask the edit target, so the menu's commands act on
    // it; selecting the row activates its layer (the target only routes strokes
    // while its layer is active).
    makeRowAnchor(path);
    setActiveThumb(path, ActiveThumbMask);
    mask_edit_target_set(*view_, path);
    syncEditTarget();
    QMenu menu(tree_);
    populateMaskMenu(menu, path, index);
    menu.exec(tree_->viewport()->mapToGlobal(pos));
}

void LayersPanel::populateMaskMenu(QMenu& menu, const QString& path, const QModelIndex& index)
{
    const bool disabled = index.data(MaskDisabledRole).toBool();
    const bool smart = index.data(SmartObjectRole).toBool();
    // Apply bakes the mask into the layer's own alpha, so it needs a layer that
    // renders from its pixels: not a smart object, group, or adjustment/fill.
    const QString kind = index.data(KindRole).toString();
    const bool procedural = kind == QLatin1String("group") || kind == QLatin1String("adjustment");
    const QString applyTip = smart        ? tr("Apply Layer Mask — not available for a smart object")
                             : procedural ? tr("Apply Layer Mask — needs a pixel layer")
                                          : QString();
    const auto add = [&](const char* id, const QString& label,
                         const QString& tip = QString()) {
        const QString command = QString::fromLatin1(id);
        QAction* action = registry_ ? registry_->detachedAction(command, &menu) : nullptr;
        if (!action) {
            return;
        }
        action->setText(label);
        if (!tip.isEmpty()) {
            action->setToolTip(tip);
        }
        menu.addAction(action);
        connect(action, &QAction::triggered, this, [this, command]() {
            if (registry_) {
                registry_->dispatch(command);
            }
        });
    };
    add(disabled ? command_ids::LayerMaskEnable : command_ids::LayerMaskDisable,
        disabled ? tr("Enable Layer Mask") : tr("Disable Layer Mask"));
    menu.addSeparator();
    add(command_ids::LayerMaskDelete, tr("Delete Layer Mask"));
    add(command_ids::LayerMaskApply, tr("Apply Layer Mask"), applyTip);
    menu.addSeparator();
    add(command_ids::PanelLayersMaskAdd, tr("Add Mask To Selection"));
    add(command_ids::PanelLayersMaskSubtract, tr("Subtract Mask From Selection"));
    add(command_ids::PanelLayersMaskIntersect, tr("Intersect Mask With Selection"));
    menu.addSeparator();
    add(command_ids::PanelLayersRefineMask, tr("Refine Mask…"));
    menu.addSeparator();
    add(command_ids::PanelLayersMaskOptions, tr("Mask Options…"));
}

QStringList LayersPanel::maskMenuTextsForPathForTest(const QString& path)
{
    makeRowAnchor(path);
    const QModelIndex index = model_ ? model_->indexForPath(path) : QModelIndex();
    QMenu menu;
    populateMaskMenu(menu, path, index);
    QStringList texts;
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator()) {
            texts.push_back(action->text());
        }
    }
    return texts;
}

bool LayersPanel::maskMenuEnabledForPathForTest(const QString& path, const QString& text)
{
    makeRowAnchor(path);
    const QModelIndex index = model_ ? model_->indexForPath(path) : QModelIndex();
    QMenu menu;
    populateMaskMenu(menu, path, index);
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator() && action->text() == text) {
            return action->isEnabled();
        }
    }
    return false;
}

bool LayersPanel::performMaskActionForTest(const QString& id, const QString& path)
{
    if (!view_) {
        return false;
    }
    if (id == QLatin1String("enableLayerMask")) {
        return layer_mask_set_enabled_path(*view_, path, layer_mask_disabled_path(*view_, path));
    }
    if (id == QLatin1String("deleteLayerMask")) {
        return layer_mask_delete_path(*view_, path);
    }
    if (id == QLatin1String("applyLayerMask")) {
        return layer_mask_apply_path(*view_, path);
    }
    if (id == QLatin1String("maskAdd")) {
        return view_->mask_selection(path, QStringLiteral("add"));
    }
    if (id == QLatin1String("maskSubtract")) {
        return view_->mask_selection(path, QStringLiteral("subtract"));
    }
    if (id == QLatin1String("maskIntersect")) {
        return view_->mask_selection(path, QStringLiteral("intersect"));
    }
    if (id == QLatin1String("vectorToggleEnabled")) {
        return vector_mask_set_enabled_path(*view_, path, vector_mask_disabled_path(*view_, path));
    }
    if (id == QLatin1String("vectorDelete")) {
        return vector_mask_delete_path(*view_, path);
    }
    if (id == QLatin1String("vectorRasterize")) {
        return vector_mask_rasterize_path(*view_, path);
    }
    if (id == QLatin1String("refineMask")) {
        openRefineMask(path);
        return true;
    }
    if (id == QLatin1String("maskOptions")) {
        openMaskOptions();
        return true;
    }
    return false;
}

void LayersPanel::populateBackgroundMenu(QMenu& menu, const QString& path)
{
    Q_UNUSED(path);
    // Every row is a registry command, so the Background menu agrees with the
    // menu bar and the control socket. The row anchor is already the clicked
    // Background, so the handlers act on it.
    const auto add = [this, &menu](const char* id, const QString& label, bool enabled = true) {
        const QString command = QString::fromLatin1(id);
        QAction* action = registry_ ? registry_->detachedAction(command, &menu) : nullptr;
        if (!action) {
            action = menu.addAction(label);
            action->setEnabled(false);
            return action;
        }
        action->setText(label);
        action->setEnabled(enabled && action->isEnabled());
        menu.addAction(action);
        connect(action, &QAction::triggered, this, [this, command]() {
            if (registry_) {
                registry_->dispatch(command);
            }
        });
        return action;
    };
    add(command_ids::LayerNewLayerFromBackground, tr("Layer from Background…"));
    add(command_ids::LayerDuplicateLayer, tr("Duplicate Layer…"));
    add(command_ids::LayerDeleteLayer, tr("Delete Layer"), false);
    menu.addSeparator();
    add(command_ids::FileQuickExportPng, tr("Quick Export as PNG"));
    add(command_ids::FileExportAs, tr("Export As…"));
    menu.addSeparator();
    add(command_ids::LayerSmartObjectConvertTo, tr("Convert to Smart Object"));
}

void LayersPanel::populateSmartFiltersGroupMenu(QMenu& menu, const QString& path)
{
    const QString base = smartFilterBase(path);
    const bool groupOn = view_ && layer_smart_filters_enabled(*view_, base);
    QAction* toggle =
        menu.addAction(groupOn ? tr("Disable Smart Filters") : tr("Enable Smart Filters"));
    toggle->setEnabled(view_ != nullptr);
    connect(toggle, &QAction::triggered, this, [this, base, groupOn] {
        if (view_ && set_layer_smart_filters_enabled(*view_, base, !groupOn)) {
            refresh();
        }
    });

    QAction* mask = menu.addAction(tr("Delete Filter Mask"));
    mask->setEnabled(view_ && layer_has_smart_filter_mask(*view_, base));
    connect(mask, &QAction::triggered, this, [this, base] {
        if (view_ && delete_layer_smart_filter_mask(*view_, base)) {
            refresh();
        }
    });

    QAction* clear = menu.addAction(tr("Clear Smart Filters"));
    clear->setEnabled(view_ != nullptr);
    connect(clear, &QAction::triggered, this, [this, base] {
        if (view_ && clear_layer_smart_filters(*view_, base)) {
            refresh();
        }
    });
}

void LayersPanel::populateSmartFilterMenu(QMenu& menu, const QString& path)
{
    const QString base = smartFilterBase(path);
    const int index = smartFilterIndex(path);

    // A per-filter opacity/blend-mode dialog is a follow-up; the item is shown
    // as CS6 has it but disabled.
    QAction* blend = menu.addAction(tr("Edit Smart Filter Blending Options…"));
    blend->setEnabled(false);
    blend->setToolTip(tr("Edit Smart Filter Blending Options… — not implemented yet"));

    const int editKind = view_ ? layer_smart_filter_edit_kind(*view_, base, index) : 0;
    QAction* edit = menu.addAction(tr("Edit Smart Filter…"));
    edit->setEnabled(editKind != 0);
    connect(edit, &QAction::triggered, this,
            [this, base, index] { emit smartFilterEditRequested(base, index); });

    menu.addSeparator();

    const bool visible = view_ && layer_smart_filter_visible(*view_, base, index);
    QAction* toggle =
        menu.addAction(visible ? tr("Disable Smart Filter") : tr("Enable Smart Filter"));
    connect(toggle, &QAction::triggered, this, [this, base, index, visible] {
        if (view_ && set_layer_smart_filter_visible(*view_, base, index, !visible)) {
            refresh();
        }
    });

    QAction* remove = menu.addAction(tr("Delete Smart Filter"));
    connect(remove, &QAction::triggered, this, [this, base, index] {
        if (view_ && delete_layer_smart_filter(*view_, base, index)) {
            refresh();
        }
    });
}

void LayersPanel::populateLayerStyleMenu(QMenu& menu, const QString& path)
{
    if (!view_) {
        return;
    }
    const bool visible =
        layer_style_value(*view_, path, QStringLiteral("fx.visible")) >= 0.5;
    const bool styled = layer_style_has(*view_, path);
    const bool copyable = layer_style_can_copy(*view_, path);

    QAction* toggle = menu.addAction(visible ? tr("Disable Layer Effects")
                                             : tr("Enable Layer Effects"));
    connect(toggle, &QAction::triggered, this, [this, path, visible] {
        if (layer_style_set(*view_, path, QStringLiteral("fx.visible"), visible ? 0.0 : 1.0)) {
            layer_style_commit(*view_, visible ? tr("Disable Layer Effects")
                                               : tr("Enable Layer Effects"));
        }
    });
    menu.addSeparator();

    QAction* blending = menu.addAction(tr("Blending Options…"));
    connect(blending, &QAction::triggered, this, [this, path] { openLayerStyle(path); });
    menu.addSeparator();

    for (const QString& key : LayerStyleDialog::effectKeys()) {
        QAction* effect =
            menu.addAction(LayerStyleDialog::effectTitle(key) + QStringLiteral("…"));
        connect(effect, &QAction::triggered, this,
                [this, path, key] { openLayerStyle(path, key); });
    }
    menu.addSeparator();

    QAction* copy = menu.addAction(tr("Copy Layer Style"));
    copy->setEnabled(copyable);
    connect(copy, &QAction::triggered, this, [this, path] { layer_style_copy(*view_, path); });
    QAction* paste = menu.addAction(tr("Paste Layer Style"));
    paste->setEnabled(layer_style_can_paste());
    connect(paste, &QAction::triggered, this, [this, path] {
        if (layer_style_paste(*view_, QStringList{path}) > 0) {
            refresh();
        }
    });
    QAction* clear = menu.addAction(tr("Clear Layer Style"));
    clear->setEnabled(styled);
    connect(clear, &QAction::triggered, this, [this, path] {
        if (layer_style_clear(*view_, QStringList{path}) > 0) {
            refresh();
        }
    });
    menu.addSeparator();

    for (const char* id : {command_ids::LayerStyleGlobalLight,
                           command_ids::LayerStyleCreateLayers}) {
        const QString command = QString::fromLatin1(id);
        QAction* action = registry_ ? registry_->detachedAction(command, &menu) : nullptr;
        if (!action) {
            continue;
        }
        menu.addAction(action);
        connect(action, &QAction::triggered, this, [this, command]() {
            if (registry_) {
                registry_->dispatch(command);
            }
        });
    }

    const bool anyVisible = layer_style_any_visible(*view_, true);
    QAction* hide =
        menu.addAction(anyVisible ? tr("Hide All Effects") : tr("Show All Effects"));
    connect(hide, &QAction::triggered, this, [this, anyVisible] {
        if (layer_style_set_all_visible(*view_, !anyVisible) > 0) {
            refresh();
        }
    });

    QAction* scale = menu.addAction(tr("Scale Effects…"));
    connect(scale, &QAction::triggered, this, [this, path] {
        bool ok = false;
        const int percent = QInputDialog::getInt(this, tr("Scale Layer Effects"), tr("Scale (%):"),
                                                 100, 1, 1000, 1, &ok);
        if (ok && layer_style_scale(*view_, QStringList{path}, percent) > 0) {
            refresh();
        }
    });
}

void LayersPanel::populateRowMenu(QMenu& menu, const QString& path, int color,
                                  const LayerRowNode& row)
{
    if (row.kind == QLatin1String("background")) {
        populateBackgroundMenu(menu, path);
        return;
    }

    QAction* rename = menu.addAction(tr("Rename"));
    connect(rename, &QAction::triggered, this, [this, path] {
        const QModelIndex target = proxyIndexForPath(path);
        if (target.isValid()) {
            tree_->setCurrentIndex(target);
            tree_->edit(target);
        }
    });
    menu.addSeparator();

    RowFacts facts;
    if (view_) {
        facts.fillContent = view_->layer_is_fill_content(path);
        facts.canRasterizeSmartObject = view_->layer_can_rasterize_smart_object(path);
        facts.canRelinkSmartObject = row.smartObject && !relink_source_id(*view_, path).isEmpty();
        facts.canReleaseClipping = clipping_can_release(*view_, QStringList{path});
        facts.hasStyle = layer_style_has(*view_, path);
        facts.canRasterizeStyle = layer_style_can_rasterize(*view_, path);
        facts.hasMask = layer_mask_present_path(*view_, path);
        facts.hasVectorMask = vector_mask_present_path(*view_, path);
    }
    const QVector<ResolvedRowAction> actions = resolveRowActions(row, facts);
    QHash<int, QVector<ResolvedRowAction>> byBlock;
    for (const ResolvedRowAction& action : actions) {
        byBlock[action.block].push_back(action);
    }

    // A row is offered only when its kind and row state make it applicable.
    // An empty block emits no separator, or the menu shows a gap.
    bool emitted = false;
    const auto appendBlock = [&](int block) {
        const auto it = byBlock.constFind(block);
        if (it == byBlock.constEnd() || it->isEmpty()) {
            return;
        }
        if (emitted) {
            menu.addSeparator();
        }
        emitted = true;
        for (const ResolvedRowAction& action : *it) {
            if (action.id == QLatin1String("relinkToFile")) {
                QAction* relink = menu.addAction(tr("Relink to File…"));
                relink->setEnabled(action.enabled);
                connect(relink, &QAction::triggered, this, [this, path] {
                    makeRowAnchor(path);
                    emit smartObjectContentsRequested(QStringLiteral("relink"), path);
                });
                continue;
            }
            if (action.submenu) {
                const bool layerMask = action.id == QLatin1String("addLayerMask");
                QMenu* sub = menu.addMenu(layerMask ? tr("Add Layer Mask")
                                                    : tr("Add Vector Mask"));
                const char* const layerIds[] = {
                    command_ids::LayerMaskRevealAll, command_ids::LayerMaskHideAll,
                    command_ids::LayerMaskRevealSelection,
                    command_ids::LayerMaskHideSelection,
                    command_ids::LayerMaskFromTransparency};
                const char* const vectorIds[] = {command_ids::LayerVectorMaskRevealAll,
                                                 command_ids::LayerVectorMaskHideAll,
                                                 command_ids::LayerVectorMaskCurrentPath};
                const char* const* ids = layerMask ? layerIds : vectorIds;
                const int count = layerMask ? 5 : 3;
                for (int i = 0; i < count; ++i) {
                    const QString command = QString::fromLatin1(ids[i]);
                    QAction* child = registry_ ? registry_->detachedAction(command, sub)
                                               : nullptr;
                    if (!child) {
                        continue;
                    }
                    sub->addAction(child);
                    connect(child, &QAction::triggered, this, [this, command]() {
                        if (registry_) {
                            registry_->dispatch(command);
                        }
                    });
                }
                sub->menuAction()->setEnabled(view_ && action.enabled);
                continue;
            }
            if (action.commandId.isEmpty() || !registry_) {
                QAction* stub = menu.addAction(tr(action.label.toUtf8().constData()));
                stub->setEnabled(action.enabled);
                stub->setToolTip(tr("%1 — not implemented yet").arg(stub->text()));
                continue;
            }
            QAction* qaction = registry_->detachedAction(action.commandId, &menu);
            if (!qaction) {
                continue;
            }
            if (action.id == QLatin1String("rasterizeType")
                || action.id == QLatin1String("rasterizeShape")
                || action.id == QLatin1String("rasterizeLayer")
                || action.id == QLatin1String("rasterizeStyle")
                || action.id == QLatin1String("deleteLayerMask")
                || action.id == QLatin1String("enableLayerMask")
                || action.id == QLatin1String("disableLayerMask")
                || action.id == QLatin1String("deleteVectorMask")
                || action.id == QLatin1String("enableVectorMask")
                || action.id == QLatin1String("disableVectorMask")) {
                qaction->setText(tr(action.label.toUtf8().constData()));
            }
            menu.addAction(qaction);
            const QString command = action.commandId;
            connect(qaction, &QAction::triggered, this, [this, command]() {
                if (registry_) {
                    registry_->dispatch(command);
                }
            });
        }
    };
    for (const int block : {1, 2, 3, 4, 5}) {
        appendBlock(block);
    }
    if (row.kind == QLatin1String("type")) {
        addTypeRows(menu, path, emitted);
    } else {
        appendBlock(6);
    }
    for (const int block : {7, 8, 9}) {
        appendBlock(block);
    }
    // Isolation sits between the style/shape blocks and the merge block.
    if (isolationActive()) {
        if (emitted) {
            menu.addSeparator();
        }
        emitted = true;
        QAction* release = menu.addAction(tr("Release from Isolation"));
        connect(release, &QAction::triggered, this, [this, path] { releaseIsolationFor(path); });
    }
    appendBlock(11);

    menu.addSeparator();
    addColorLabelActions(menu.addMenu(tr("Color Label")), color);
}

void LayersPanel::showEyeMenu(const QPoint& pos, const QModelIndex& index)
{
    const QString path = pathForProxyIndex(index);
    QMenu menu(tree_);
    QAction* only = menu.addAction(tr("Show/Hide This Layer Only"));
    connect(only, &QAction::triggered, this, [this, path] { toggleSolo(path); });
    QAction* all = menu.addAction(tr("Show/Hide All Layers"));
    connect(all, &QAction::triggered, this, [this] {
        if (!view_) {
            return;
        }
        clearSolo();
        view_->apply_visibility(model_->paths(), QStringLiteral("Show All Layers"));
    });
    menu.exec(tree_->viewport()->mapToGlobal(pos));
}

void LayersPanel::addColorLabelActions(QMenu* menu, int currentLabel)
{
    for (int label = 0; label <= 7; ++label) {
        QAction* action = menu->addAction(labelName(label));
        action->setCheckable(true);
        action->setChecked(currentLabel == label);
        const QPixmap swatch = labelSwatch(label);
        if (!swatch.isNull()) {
            action->setIcon(QIcon(swatch));
        }
        connect(action, &QAction::triggered, this, [this, label] {
            const QStringList paths = targetPaths();
            if (view_ && !paths.isEmpty()) {
                view_->set_layers_color(paths, label);
            }
        });
    }
}

bool LayersPanel::performPanelMenuAction(const QString& actionId)
{
    // A pending transform resolves first for every mutating entry; the two
    // non-mutating entries (a preference and the options dialog) pass through.
    const bool mutates = actionId != QLatin1String("addMaskByDefault")
        && actionId != QLatin1String("panelOptions");
    if (mutates && !resolvePendingTransform()) {
        return false;
    }
    if (actionId == QLatin1String("newLayer")) {
        openNewLayerDialog();
    } else if (actionId == QLatin1String("duplicate")) {
        duplicateSelection();
    } else if (actionId == QLatin1String("delete")) {
        deleteSelection();
    } else if (actionId == QLatin1String("newGroup")) {
        openNewGroupDialog();
    } else if (actionId == QLatin1String("group")) {
        groupSelection();
    } else if (actionId == QLatin1String("ungroup")) {
        ungroupSelection();
    } else if (actionId == QLatin1String("hide")) {
        if (view_) {
            const QStringList paths = targetPaths();
            if (!paths.isEmpty()) {
                view_->set_layers_visible(paths, false);
            }
        }
    } else if (actionId == QLatin1String("deleteHidden")) {
        if (view_) {
            view_->delete_hidden_layers();
        }
    } else if (actionId == QLatin1String("link")) {
        if (view_) {
            const QStringList paths = targetPaths();
            if (!paths.isEmpty()) {
                view_->link_layers(paths, true);
            }
        }
    } else if (actionId == QLatin1String("selectLinked")) {
        if (view_) {
            const QString path = layer_selection_anchor(*view_);
            if (!path.isEmpty()) {
                view_->select_linked(path);
                refresh();
            }
        }
    } else if (actionId == QLatin1String("moveUp")) {
        moveCurrent(1);
    } else if (actionId == QLatin1String("moveDown")) {
        moveCurrent(-1);
    } else if (actionId == QLatin1String("mergeDown") || actionId == QLatin1String("mergeVisible")
               || actionId == QLatin1String("mergeClippingMask")) {
        if (view_) {
            const QStringList paths = targetPaths();
            const QString path = view_ ? layer_selection_anchor(*view_) : QString();
            PictureView* view = view_;
            bool merged = false;
            if (actionId == QLatin1String("mergeDown")) {
                if (!paths.isEmpty()) {
                    merged = runDocumentCommand(window(), *view, tr("Merge Layers"),
                                                [view, &paths] { view->merge_layers(paths); });
                }
            } else if (actionId == QLatin1String("mergeVisible")) {
                if (!path.isEmpty()) {
                    merged = runDocumentCommand(window(), *view, tr("Merge Visible"),
                                                [view, &path] { view->merge_visible(path); });
                }
            } else if (!path.isEmpty()) {
                runDocumentCommand(window(), *view, tr("Merge Clipping Mask"),
                                   [view, &path] { view->merge_clipping_mask(path); });
            }
            if (merged) {
                refresh();
            }
        }
    } else if (actionId == QLatin1String("flatten")) {
        if (view_) {
            PictureView* view = view_;
            runDocumentCommand(window(), *view, tr("Flatten Image"),
                               [view] { view->flatten_image(); });
        }
    } else if (actionId == QLatin1String("addMaskByDefault")) {
        setAddMaskByDefault(!addMaskByDefault_);
    } else if (actionId == QLatin1String("panelOptions")) {
        openPanelOptions();
    } else if (actionId.startsWith(QLatin1String("adjustment:"))) {
        const QString kind = actionId.mid(QLatin1String("adjustment:").size());
        if (canCreateAdjustment(kind) && view_->add_adjustment(kind)) {
            refresh();
        }
    } else {
        return false;
    }
    return true;
}

} // namespace pictura
