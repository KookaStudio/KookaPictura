#include "memory_refusal.h"
#include "layers_panel.h"
#include "commands.h"
#include "document_command.h"

#include "export_as_dialog.h"
#include "layer_style_dialog.h"
#include "layers_panel_internal.h"

#include "api/layers_bridge.h"

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

#include <algorithm>
#include <iterator>

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

enum KindMask : unsigned {
    Pixel = 1u << 0,
    Background = 1u << 1,
    Group = 1u << 2,
    Adjustment = 1u << 3,
    Type = 1u << 4,
    Shape = 1u << 5,
    SmartObject = 1u << 6,
};

constexpr unsigned kAllKinds = Pixel | Background | Group | Adjustment | Type;
constexpr unsigned kEveryKind = kAllKinds | Shape | SmartObject;

struct RowSpec {
    const char* id;
    const char* label;
    unsigned kinds;
    unsigned block;
    bool implemented;
};

// Row menu, in CS6 block order. `populateRowMenu` emits one separator between
// each non-empty block; Rename and the Color Label submenu bracket the table
// as bespoke head/tail. The Background uses its own brief table.
const RowSpec kRowSpecs[] = {
    // 1. Blending / adjustment edit.
    {"blendingOptions", "Blending Options…", Pixel | Group | Type, 1, true},
    {"editAdjustment", "Edit Adjustment…", Adjustment, 1, true},
    {"copyCss", "Copy CSS", Type | Group | Adjustment, 1, false},
    {"copySvg", "Copy SVG", Type | Group | Adjustment, 1, false},
    // 2. Layer operations.
    {"duplicate", "Duplicate Layer(s)", kEveryKind, 2, true},
    {"delete", "Delete Layer(s)", kEveryKind, 2, true},
    {"groupFromLayers", "Group from Layers…", kEveryKind, 2, true},
    {"newLayer", "New Layer", kEveryKind, 2, true},
    {"newGroup", "New Group", kEveryKind, 2, true},
    {"ungroup", "Ungroup Layers", Group, 2, true},
    {"moveUp", "Move Layer Up", kEveryKind, 2, true},
    {"moveDown", "Move Layer Down", kEveryKind, 2, true},
    // 3. Export.
    {"quickExport", "Quick Export as PNG", kAllKinds, 3, true},
    {"exportAs", "Export As…", kAllKinds, 3, true},
    // 4. Convert.
    {"artboardFromLayers", "Artboard from Layers…", Type | Group | Adjustment, 4, false},
    {"frameFromLayers", "Frame from Layers…", Type | Group | Adjustment, 4, false},
    {"convertToFrame", "Convert to Frame", Type | Group | Adjustment, 4, false},
    {"convertToSmartObject", "Convert to Smart Object",
     Pixel | Background | Type | Group | Adjustment, 4, true},
    {"resetTransform", "Reset Transform", SmartObject, 4, true},
    {"convertToLayers", "Convert to Layers", SmartObject, 4, true},
    {"newSmartObjectViaCopy", "New Smart Object via Copy", SmartObject, 4, true},
    {"editContents", "Edit Contents", SmartObject, 4, true},
    {"replaceContents", "Replace Contents…", SmartObject, 4, true},
    {"exportContents", "Export Contents…", SmartObject, 4, true},
    {"relinkToFile", "Relink to File…", SmartObject, 4, false},
    {"convertToLinked", "Convert to Linked…", SmartObject, 4, false},
    // 5. Rasterize.
    {"rasterizeType", "Rasterize Type", Type, 5, true},
    {"rasterizeShape", "Rasterize Shape", Shape, 5, true},
    {"rasterizeLayer", "Rasterize Layer", SmartObject | Adjustment, 5, true},
    {"rasterizeStyle", "Rasterize Layer Style", kEveryKind, 5, false},
    // 6. Masks and clipping.
    {"addLayerMask", "Add Layer Mask", Pixel | Group | Adjustment, 6, true},
    {"deleteLayerMask", "Delete Layer Mask", Pixel | Group | Adjustment, 6, true},
    {"enableLayerMask", "Enable Layer Mask", Pixel | Group | Adjustment, 6, true},
    {"disableLayerMask", "Disable Layer Mask", Pixel | Group | Adjustment, 6, true},
    {"addVectorMask", "Add Vector Mask", Pixel | Group | Adjustment, 6, true},
    {"deleteVectorMask", "Delete Vector Mask", Pixel | Group | Adjustment, 6, true},
    {"enableVectorMask", "Enable Vector Mask", Pixel | Group | Adjustment, 6, true},
    {"disableVectorMask", "Disable Vector Mask", Pixel | Group | Adjustment, 6, true},
    {"clippingToggle", "Create Clipping Mask", Pixel | Adjustment, 6, true},
    // 7. Link.
    {"link", "Link Layers", kEveryKind, 7, true},
    {"selectLinked", "Select Linked Layers", kEveryKind, 7, true},
    // 8. Layer style clipboard.
    {"copyLayerStyle", "Copy Layer Style", kEveryKind, 8, true},
    {"pasteLayerStyle", "Paste Layer Style", kEveryKind, 8, true},
    {"clearLayerStyle", "Clear Layer Style", kEveryKind, 8, true},
    // 9. Shape attributes.
    {"copyShapeAttributes", "Copy Shape Attributes", Shape, 9, true},
    {"pasteShapeAttributes", "Paste Shape Attributes", Shape, 9, true},
    // 11. Merge and flatten. Block 10 is Release from Isolation, emitted
    // between the table blocks only while isolation is active.
    {"mergeDown", "Merge Down", Pixel | Group | Adjustment, 11, true},
    {"mergeVisible", "Merge Visible", kEveryKind, 11, true},
    {"mergeClippingMask", "Merge Clipping Mask", kEveryKind, 11, true},
    {"flatten", "Flatten Image", kEveryKind, 11, true},
};

unsigned kindMaskFor(const QString& kind)
{
    if (kind == QLatin1String("background")) {
        return Background;
    }
    if (kind == QLatin1String("group")) {
        return Group;
    }
    if (kind == QLatin1String("adjustment")) {
        return Adjustment;
    }
    if (kind == QLatin1String("type")) {
        return Type;
    }
    // Pixel, unknown kinds, shapes, and smart objects.
    return Pixel;
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
    populateRowMenu(menu, path, index.data(ColorRole).toInt(), index.data(KindRole).toString(),
                    index.data(LayerRowShapeRole).toBool(), index.data(SmartObjectRole).toBool());
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
    copy->setEnabled(styled);
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

    QAction* globalLight = menu.addAction(tr("Global Light…"));
    globalLight->setEnabled(false);
    globalLight->setToolTip(tr("Global Light… — not implemented yet"));
    QAction* createLayer = menu.addAction(tr("Create Layer"));
    createLayer->setEnabled(false);
    createLayer->setToolTip(tr("Create Layer — not implemented yet"));

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
                                  const QString& kind, bool shape, bool smart)
{
    if (kind == QLatin1String("background")) {
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

    // A shape layer reports the `adjustment` kind (its solid fill), but its menu
    // is a pixel layer's common commands plus the shape rows — never the
    // adjustment-only rows.
    const unsigned mask = (shape ? Pixel : kindMaskFor(kind)) | (shape ? Shape : 0u)
                          | (smart ? SmartObject : 0u);
    const bool linked = !path.isEmpty() && model_
                        && model_->indexForPath(path).data(LayerRowLinkedRole).toBool();
    const bool fillContent = view_ && view_->layer_is_fill_content(path);
    // A row is offered only when its kind and row state make it applicable.
    // This is also the block predicate: an empty block must not emit its
    // separator, or the menu shows a gap between neighbours.
    const auto rowApplies = [&](const RowSpec& row) {
        if ((row.kinds & mask) == 0) {
            return false;
        }
        const QString id = QLatin1String(row.id);
        if (id == QLatin1String("rasterizeStyle")
            && !(view_ && layer_style_has(*view_, path))) {
            return false;
        }
        if (id == QLatin1String("rasterizeLayer") && !fillContent && !smart) {
            return false;
        }
        return true;
    };
    bool emitted = false;
    const auto appendBlock = [&](unsigned block) {
        const bool applies =
            std::any_of(std::begin(kRowSpecs), std::end(kRowSpecs),
                        [block, &rowApplies](const RowSpec& row) {
                            return row.block == block && rowApplies(row);
                        });
        if (!applies) {
            return;
        }
        if (emitted) {
            menu.addSeparator();
        }
        emitted = true;
        for (const RowSpec& row : kRowSpecs) {
            if (row.block != block || !rowApplies(row)) {
                continue;
            }
            const QString id = QLatin1String(row.id);
            const auto commandForRow = [&]() -> QString {
                if (id == QLatin1String("blendingOptions")) {
                    return QString::fromLatin1(command_ids::LayerStyleBlendingOptions);
                }
                if (id == QLatin1String("editAdjustment")) {
                    return QString::fromLatin1(command_ids::PanelLayersEditAdjustment);
                }
                if (id == QLatin1String("duplicate")) {
                    return QString::fromLatin1(command_ids::LayerDuplicateLayer);
                }
                if (id == QLatin1String("delete")) {
                    return QString::fromLatin1(command_ids::LayerDeleteLayer);
                }
                if (id == QLatin1String("groupFromLayers")) {
                    return QString::fromLatin1(command_ids::LayerNewGroupFromLayers);
                }
                if (id == QLatin1String("newLayer")) {
                    return QString::fromLatin1(command_ids::PanelLayersNewLayer);
                }
                if (id == QLatin1String("newGroup")) {
                    return QString::fromLatin1(command_ids::PanelLayersNewGroup);
                }
                if (id == QLatin1String("ungroup")) {
                    return QString::fromLatin1(command_ids::LayerUngroupLayers);
                }
                if (id == QLatin1String("moveUp")) {
                    return QString::fromLatin1(command_ids::PanelLayersMoveUp);
                }
                if (id == QLatin1String("moveDown")) {
                    return QString::fromLatin1(command_ids::PanelLayersMoveDown);
                }
                if (id == QLatin1String("quickExport")) {
                    return QString::fromLatin1(command_ids::FileQuickExportPng);
                }
                if (id == QLatin1String("exportAs")) {
                    return QString::fromLatin1(command_ids::FileExportAs);
                }
                if (id == QLatin1String("convertToSmartObject")) {
                    return QString::fromLatin1(command_ids::LayerSmartObjectConvertTo);
                }
                if (id == QLatin1String("resetTransform")) {
                    return QString::fromLatin1(command_ids::LayerSmartObjectResetTransform);
                }
                if (id == QLatin1String("convertToLayers")) {
                    return QString::fromLatin1(command_ids::LayerSmartObjectConvertToLayers);
                }
                if (id == QLatin1String("newSmartObjectViaCopy")) {
                    return QString::fromLatin1(command_ids::LayerSmartObjectNewViaCopy);
                }
                if (id == QLatin1String("editContents")) {
                    return QString::fromLatin1(command_ids::LayerSmartObjectEditContents);
                }
                if (id == QLatin1String("replaceContents")) {
                    return QString::fromLatin1(command_ids::LayerSmartObjectReplaceContents);
                }
                if (id == QLatin1String("exportContents")) {
                    return QString::fromLatin1(command_ids::LayerSmartObjectExportContents);
                }
                if (id == QLatin1String("rasterizeType")) {
                    return QString::fromLatin1(command_ids::LayerRasterizeType);
                }
                if (id == QLatin1String("rasterizeShape")) {
                    return QString::fromLatin1(command_ids::LayerRasterizeShape);
                }
                if (id == QLatin1String("rasterizeLayer")) {
                    if (view_ && view_->layer_can_rasterize_smart_object(path)) {
                        return QString::fromLatin1(command_ids::LayerRasterizeSmartObject);
                    }
                    if (fillContent) {
                        return QString::fromLatin1(command_ids::LayerRasterizeFillContent);
                    }
                    return QString::fromLatin1(command_ids::LayerRasterizeLayer);
                }
                if (id == QLatin1String("deleteLayerMask")) {
                    return QString::fromLatin1(command_ids::LayerMaskDelete);
                }
                if (id == QLatin1String("enableLayerMask")) {
                    return QString::fromLatin1(command_ids::LayerMaskEnable);
                }
                if (id == QLatin1String("disableLayerMask")) {
                    return QString::fromLatin1(command_ids::LayerMaskDisable);
                }
                if (id == QLatin1String("deleteVectorMask")) {
                    return QString::fromLatin1(command_ids::LayerVectorMaskDelete);
                }
                if (id == QLatin1String("enableVectorMask")) {
                    return QString::fromLatin1(command_ids::LayerVectorMaskEnable);
                }
                if (id == QLatin1String("disableVectorMask")) {
                    return QString::fromLatin1(command_ids::LayerVectorMaskDisable);
                }
                if (id == QLatin1String("clippingToggle")) {
                    const QStringList targets{path};
                    return QString::fromLatin1(
                        view_ && clipping_can_release(*view_, targets)
                            ? command_ids::LayerReleaseClippingMask
                            : command_ids::LayerCreateClippingMask);
                }
                if (id == QLatin1String("link")) {
                    return QString::fromLatin1(linked ? command_ids::LayerUnlinkLayers
                                                      : command_ids::LayerLinkLayers);
                }
                if (id == QLatin1String("selectLinked")) {
                    return QString::fromLatin1(command_ids::LayerSelectLinked);
                }
                if (id == QLatin1String("copyLayerStyle")) {
                    return QString::fromLatin1(command_ids::LayerStyleCopyLayerStyle);
                }
                if (id == QLatin1String("pasteLayerStyle")) {
                    return QString::fromLatin1(command_ids::LayerStylePasteLayerStyle);
                }
                if (id == QLatin1String("clearLayerStyle")) {
                    return QString::fromLatin1(command_ids::LayerStyleClearLayerStyle);
                }
                if (id == QLatin1String("copyShapeAttributes")) {
                    return QString::fromLatin1(command_ids::LayerCopyShapeAttributes);
                }
                if (id == QLatin1String("pasteShapeAttributes")) {
                    return QString::fromLatin1(command_ids::LayerPasteShapeAttributes);
                }
                if (id == QLatin1String("mergeDown")) {
                    return QString::fromLatin1(command_ids::LayerMergeLayers);
                }
                if (id == QLatin1String("mergeVisible")) {
                    return QString::fromLatin1(command_ids::LayerMergeVisible);
                }
                if (id == QLatin1String("mergeClippingMask")) {
                    return QString::fromLatin1(command_ids::LayerMergeClippingMask);
                }
                if (id == QLatin1String("flatten")) {
                    return QString::fromLatin1(command_ids::LayerFlattenImage);
                }
                return QString();
            };
            if (!row.implemented) {
                QAction* action = menu.addAction(tr(row.label));
                action->setEnabled(false);
                action->setToolTip(tr("%1 — not implemented yet").arg(action->text()));
                continue;
            }
            if (id == QLatin1String("addLayerMask") || id == QLatin1String("addVectorMask")) {
                const bool layerMask = id == QLatin1String("addLayerMask");
                QMenu* sub =
                    menu.addMenu(layerMask ? tr("Add Layer Mask") : tr("Add Vector Mask"));
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
                    QAction* action = registry_ ? registry_->detachedAction(command, sub)
                                                : nullptr;
                    if (!action) {
                        continue;
                    }
                    sub->addAction(action);
                    connect(action, &QAction::triggered, this, [this, command]() {
                        if (registry_) {
                            registry_->dispatch(command);
                        }
                    });
                }
                const bool present = view_ && (layerMask
                                                   ? layer_mask_present_path(*view_, path)
                                                   : vector_mask_present_path(*view_, path));
                sub->menuAction()->setEnabled(view_ && !present);
                continue;
            }
            const QString command = commandForRow();
            if (command.isEmpty() || !registry_) {
                QAction* action = menu.addAction(tr(row.label));
                action->setEnabled(false);
                action->setToolTip(tr("%1 — not implemented yet").arg(action->text()));
                continue;
            }
            QAction* action = registry_->detachedAction(command, &menu);
            if (!action) {
                continue;
            }
            if (id == QLatin1String("rasterizeType") || id == QLatin1String("rasterizeShape")
                || id == QLatin1String("rasterizeLayer")
                || id == QLatin1String("deleteLayerMask")
                || id == QLatin1String("enableLayerMask")
                || id == QLatin1String("disableLayerMask")
                || id == QLatin1String("deleteVectorMask")
                || id == QLatin1String("enableVectorMask")
                || id == QLatin1String("disableVectorMask")) {
                action->setText(tr(row.label));
            }
            menu.addAction(action);
            connect(action, &QAction::triggered, this, [this, command]() {
                if (registry_) {
                    registry_->dispatch(command);
                }
            });
        }
    };
    for (const unsigned block : {1u, 2u, 3u, 4u, 5u}) {
        appendBlock(block);
    }
    if (kind == QLatin1String("type")) {
        addTypeRows(menu, path, emitted);
    } else {
        appendBlock(6);
    }
    for (const unsigned block : {7u, 8u, 9u}) {
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
    for (const unsigned block : {11u}) {
        appendBlock(block);
    }

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
        if (view_ && view_->add_adjustment(actionId.mid(QLatin1String("adjustment:").size()))) {
            refresh();
        }
    } else {
        return false;
    }
    return true;
}

} // namespace pictura
