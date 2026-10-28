#include "layers_panel.h"

#include "export_as_dialog.h"
#include "layer_style_dialog.h"
#include "layers_panel_internal.h"

#include "pictura_app/src/cxxqt_object/clipping.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layers_smart_filters.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layers_surface.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/smart_object_actions.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/vector_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"
#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"

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
    // Row-state enablement the kind mask cannot express. `Always` (the default)
    // keeps the other implemented rows unconditional.
    enum Enable {
        Always,
        StyleEdit,
        HasStyle,
        CanPasteStyle,
        ConvertToSmartCan,
        ResetTransformCan,
        ConvertToLayersCan,
        NewViaCopyCan,
        EditContentsCan,
        ReplaceContentsCan,
        ExportContentsCan,
        NoMask,
        HasMask,
        MaskDisabled,
        MaskEnabled,
        NoVectorMask,
        HasVectorMask,
        VectorMaskDisabled,
        VectorMaskEnabled,
        ClippingCan,
        Linked,
        RasterizeLayerCan,
        CanMerge,
        CanDelete,
        CanMoveUp,
        CanMoveDown,
    };
    Enable enable = Always;
};

// Row menu, in CS6 block order. `populateRowMenu` emits one separator between
// each non-empty block; Rename and the Color Label submenu bracket the table
// as bespoke head/tail. The Background uses its own brief table.
const RowSpec kRowSpecs[] = {
    // 1. Blending / adjustment edit.
    {"blendingOptions", "Blending Options…", Pixel | Group | Type, 1, true, RowSpec::StyleEdit},
    {"editAdjustment", "Edit Adjustment…", Adjustment, 1, true},
    {"copyCss", "Copy CSS", Type | Group | Adjustment, 1, false},
    {"copySvg", "Copy SVG", Type | Group | Adjustment, 1, false},
    // 2. Layer operations.
    {"duplicate", "Duplicate Layer(s)", kEveryKind, 2, true},
    {"delete", "Delete Layer(s)", kEveryKind, 2, true, RowSpec::CanDelete},
    {"groupFromLayers", "Group from Layers…", kEveryKind, 2, true},
    {"newLayer", "New Layer", kEveryKind, 2, true},
    {"newGroup", "New Group", kEveryKind, 2, true},
    {"ungroup", "Ungroup Layers", Group, 2, true},
    {"moveUp", "Move Layer Up", kEveryKind, 2, true, RowSpec::CanMoveUp},
    {"moveDown", "Move Layer Down", kEveryKind, 2, true, RowSpec::CanMoveDown},
    // 3. Export.
    {"quickExport", "Quick Export as PNG", kAllKinds, 3, true},
    {"exportAs", "Export As…", kAllKinds, 3, true},
    // 4. Convert.
    {"artboardFromLayers", "Artboard from Layers…", Type | Group | Adjustment, 4, false},
    {"frameFromLayers", "Frame from Layers…", Type | Group | Adjustment, 4, false},
    {"convertToFrame", "Convert to Frame", Type | Group | Adjustment, 4, false},
    {"convertToSmartObject", "Convert to Smart Object",
     Pixel | Background | Type | Group | Adjustment, 4, true, RowSpec::ConvertToSmartCan},
    {"resetTransform", "Reset Transform", SmartObject, 4, true, RowSpec::ResetTransformCan},
    {"convertToLayers", "Convert to Layers", SmartObject, 4, true, RowSpec::ConvertToLayersCan},
    {"newSmartObjectViaCopy", "New Smart Object via Copy", SmartObject, 4, true,
     RowSpec::NewViaCopyCan},
    {"editContents", "Edit Contents", SmartObject, 4, true, RowSpec::EditContentsCan},
    {"replaceContents", "Replace Contents…", SmartObject, 4, true, RowSpec::ReplaceContentsCan},
    {"exportContents", "Export Contents…", SmartObject, 4, true, RowSpec::ExportContentsCan},
    {"relinkToFile", "Relink to File…", SmartObject, 4, false},
    {"convertToLinked", "Convert to Linked…", SmartObject, 4, false},
    // 5. Rasterize.
    {"rasterizeType", "Rasterize Type", Type, 5, true},
    {"rasterizeShape", "Rasterize Shape", Shape, 5, true},
    {"rasterizeLayer", "Rasterize Layer", SmartObject | Adjustment, 5, true,
     RowSpec::RasterizeLayerCan},
    {"rasterizeStyle", "Rasterize Layer Style", kEveryKind, 5, false},
    // 6. Masks and clipping.
    {"addLayerMask", "Add Layer Mask", Pixel | Group | Adjustment, 6, true, RowSpec::NoMask},
    {"deleteLayerMask", "Delete Layer Mask", Pixel | Group | Adjustment, 6, true,
     RowSpec::HasMask},
    {"enableLayerMask", "Enable Layer Mask", Pixel | Group | Adjustment, 6, true,
     RowSpec::MaskDisabled},
    {"disableLayerMask", "Disable Layer Mask", Pixel | Group | Adjustment, 6, true,
     RowSpec::MaskEnabled},
    {"addVectorMask", "Add Vector Mask", Pixel | Group | Adjustment, 6, true,
     RowSpec::NoVectorMask},
    {"deleteVectorMask", "Delete Vector Mask", Pixel | Group | Adjustment, 6, true,
     RowSpec::HasVectorMask},
    {"enableVectorMask", "Enable Vector Mask", Pixel | Group | Adjustment, 6, true,
     RowSpec::VectorMaskDisabled},
    {"disableVectorMask", "Disable Vector Mask", Pixel | Group | Adjustment, 6, true,
     RowSpec::VectorMaskEnabled},
    {"clippingToggle", "Create Clipping Mask", Pixel | Adjustment, 6, true,
     RowSpec::ClippingCan},
    // 7. Link.
    {"link", "Link Layers", kEveryKind, 7, true},
    {"selectLinked", "Select Linked Layers", kEveryKind, 7, true, RowSpec::Linked},
    // 8. Layer style clipboard.
    {"copyLayerStyle", "Copy Layer Style", kEveryKind, 8, true, RowSpec::HasStyle},
    {"pasteLayerStyle", "Paste Layer Style", kEveryKind, 8, true, RowSpec::CanPasteStyle},
    {"clearLayerStyle", "Clear Layer Style", kEveryKind, 8, true, RowSpec::HasStyle},
    // 9. Shape attributes.
    {"copyShapeAttributes", "Copy Shape Attributes", Shape, 9, true},
    {"pasteShapeAttributes", "Paste Shape Attributes", Shape, 9, true},
    // 11. Merge and flatten. Block 10 is Release from Isolation, emitted
    // between the table blocks only while isolation is active.
    {"mergeDown", "Merge Down", Pixel | Group | Adjustment, 11, true, RowSpec::CanMerge},
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
    if (delegate_->eyeColumnContains(tree_->visualRect(index), pos)) {
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
    if (!tree_->selectionModel()->isSelected(index)) {
        // A row action runs on the clicked row: make it the selection first.
        tree_->selectionModel()->select(index, QItemSelectionModel::ClearAndSelect);
        tree_->setCurrentIndex(index);
    }

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
    tree_->selectionModel()->select(index, QItemSelectionModel::ClearAndSelect);
    tree_->setCurrentIndex(index);
    setActiveThumb(path, 1);
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
    const auto add = [&](const char* id, const QString& label, bool enabled, const QString& tip) {
        QAction* action = menu.addAction(label);
        action->setEnabled(enabled);
        if (!tip.isEmpty()) {
            action->setToolTip(tip);
        }
        connect(action, &QAction::triggered, this,
                [this, id, path] { performMaskAction(QLatin1String(id), path); });
    };
    add("enableLayerMask",
        disabled ? tr("Enable Layer Mask") : tr("Disable Layer Mask"), true, QString());
    menu.addSeparator();
    add("deleteLayerMask", tr("Delete Layer Mask"), true, QString());
    add("applyLayerMask", tr("Apply Layer Mask"), applyTip.isEmpty(), applyTip);
    menu.addSeparator();
    add("maskAdd", tr("Add Mask To Selection"), true, QString());
    add("maskSubtract", tr("Subtract Mask From Selection"), true, QString());
    add("maskIntersect", tr("Intersect Mask With Selection"), true, QString());
    menu.addSeparator();
    add("refineMask", tr("Refine Mask…"), true, QString());
    menu.addSeparator();
    add("maskOptions", tr("Mask Options…"), true, QString());
}

bool LayersPanel::performMaskAction(const QString& id, const QString& path)
{
    if (!view_) {
        return false;
    }
    if (id == QLatin1String("enableLayerMask")) {
        // Read the document, not the row: the model lags until the next refresh.
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

QStringList LayersPanel::maskMenuTextsForPathForTest(const QString& path)
{
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
    return performMaskAction(id, path);
}

void LayersPanel::populateBackgroundMenu(QMenu& menu, const QString& path)
{
    QAction* from = menu.addAction(tr("Layer from Background…"));
    connect(from, &QAction::triggered, this, [this, path] { openBackgroundConversion(path); });
    QAction* duplicate = menu.addAction(tr("Duplicate Layer…"));
    connect(duplicate, &QAction::triggered, this, [this] { duplicateSelection(); });
    QAction* remove = menu.addAction(tr("Delete Layer"));
    remove->setEnabled(false);
    menu.addSeparator();
    QAction* quick = menu.addAction(tr("Quick Export as PNG"));
    connect(quick, &QAction::triggered, this, [this] { quickExportPngFromView(this, view_); });
    QAction* exportAs = menu.addAction(tr("Export As…"));
    connect(exportAs, &QAction::triggered, this, [this] { exportAsFromView(this, view_); });
    menu.addSeparator();
    QAction* convert = menu.addAction(tr("Convert to Smart Object"));
    connect(convert, &QAction::triggered, this, [this, path] {
        if (view_ && view_->convert_to_smart_object(path)) {
            refresh();
        }
    });
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
            QString text = tr(row.label);
            const QString id = QLatin1String(row.id);
            const QStringList selected = targetPaths();
            const bool many = selected.size() > 1;
            if (id == QLatin1String("duplicate")) {
                text = kind == QLatin1String("group") ? tr("Duplicate Group…")
                       : many                      ? tr("Duplicate Layers…")
                                                   : tr("Duplicate Layer…");
            } else if (id == QLatin1String("delete")) {
                text = kind == QLatin1String("group") ? tr("Delete Group")
                       : many                      ? tr("Delete Layers")
                                                   : tr("Delete Layer");
            } else if (id == QLatin1String("link") && linked) {
                text = tr("Unlink Layers");
            } else if (id == QLatin1String("mergeDown")
                       && (many || kind == QLatin1String("group") || fillContent)) {
                text = tr("Merge Layers");
            } else if (id == QLatin1String("clippingToggle")) {
                const QStringList targets{path};
                text = view_ && clipping_can_release(*view_, targets)
                           ? tr("Release Clipping Mask")
                           : tr("Create Clipping Mask");
            }
            QAction* action = menu.addAction(text);
            if (!row.implemented) {
                action->setEnabled(false);
                action->setToolTip(tr("%1 — not implemented yet").arg(text));
                continue;
            }
            bool enabled = true;
            switch (row.enable) {
            case RowSpec::StyleEdit:
                enabled = view_ && layer_style_can_edit(*view_, path);
                break;
            case RowSpec::HasStyle:
                enabled = view_ && layer_style_has(*view_, path);
                break;
            case RowSpec::CanPasteStyle:
                enabled = view_ && layer_style_can_paste();
                break;
            case RowSpec::ConvertToSmartCan:
                enabled = view_ && view_->layer_can_convert_to_smart_object(path);
                break;
            case RowSpec::ResetTransformCan:
                enabled = view_ && layer_can_reset_smart_object_transform(*view_, path);
                break;
            case RowSpec::ConvertToLayersCan:
                enabled = view_ && layer_can_convert_smart_object_to_layers(*view_, path);
                break;
            case RowSpec::NewViaCopyCan:
                enabled = view_ && layer_can_new_smart_object_via_copy(*view_, path);
                break;
            case RowSpec::EditContentsCan:
                enabled = view_ && view_->layer_can_edit_smart_object_contents(path);
                break;
            case RowSpec::ReplaceContentsCan:
                enabled = view_ && view_->layer_can_replace_smart_object_contents(path);
                break;
            case RowSpec::ExportContentsCan:
                enabled = view_ && view_->layer_can_export_smart_object_contents(path);
                break;
            case RowSpec::NoMask:
                enabled = view_ && !layer_mask_present_path(*view_, path);
                break;
            case RowSpec::HasMask:
                enabled = view_ && layer_mask_present_path(*view_, path);
                break;
            case RowSpec::MaskDisabled:
                enabled = view_ && layer_mask_disabled_path(*view_, path);
                break;
            case RowSpec::MaskEnabled:
                enabled = view_ && layer_mask_present_path(*view_, path)
                          && !layer_mask_disabled_path(*view_, path);
                break;
            case RowSpec::NoVectorMask:
                enabled = view_ && !vector_mask_present_path(*view_, path);
                break;
            case RowSpec::HasVectorMask:
                enabled = view_ && vector_mask_present_path(*view_, path);
                break;
            case RowSpec::VectorMaskDisabled:
                enabled = view_ && vector_mask_disabled_path(*view_, path);
                break;
            case RowSpec::VectorMaskEnabled:
                enabled = view_ && vector_mask_present_path(*view_, path)
                          && !vector_mask_disabled_path(*view_, path);
                break;
            case RowSpec::ClippingCan: {
                const QStringList targets{path};
                enabled = view_
                          && (clipping_can_create(*view_, targets)
                              || clipping_can_release(*view_, targets));
                break;
            }
            case RowSpec::Linked:
                enabled = linked;
                break;
            case RowSpec::RasterizeLayerCan:
                enabled = view_
                          && (view_->layer_is_fill_content(path)
                              || view_->layer_can_rasterize_smart_object(path));
                break;
            case RowSpec::CanMerge:
                enabled = view_ && view_->can_merge_layers(targetPaths());
                break;
            case RowSpec::CanDelete: {
                // The row action deletes the selection when the row is in it.
                const QStringList selected = targetPaths();
                enabled = view_
                          && layers_can_delete(*view_, selected.contains(path)
                                                           ? selected
                                                           : QStringList{path});
                break;
            }
            case RowSpec::CanMoveUp:
                enabled = canMovePath(path, 1);
                break;
            case RowSpec::CanMoveDown:
                enabled = canMovePath(path, -1);
                break;
            case RowSpec::Always:
                break;
            }
            if (!enabled) {
                action->setEnabled(false);
                continue;
            }
            connect(action, &QAction::triggered, this,
                    [this, id, path] { performRowAction(id, path); });
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

bool LayersPanel::performRowAction(const QString& id, const QString& path)
{
    if (id == QLatin1String("newLayer")) {
        addLayerAt(path);
    } else if (id == QLatin1String("newGroup")) {
        addGroupAt(path);
    } else if (id == QLatin1String("duplicate")) {
        duplicateSelection();
    } else if (id == QLatin1String("delete")) {
        deleteSelection();
    } else if (id == QLatin1String("group")) {
        groupSelection();
    } else if (id == QLatin1String("ungroup")) {
        ungroupSelection();
    } else if (id == QLatin1String("moveUp")) {
        moveCurrent(1);
    } else if (id == QLatin1String("moveDown")) {
        moveCurrent(-1);
    } else if (id == QLatin1String("addLayerMask")) {
        if (view_) {
            layer_mask_add_path(*view_, path,
                                view_->has_selection() ? QStringLiteral("reveal-selection")
                                                       : QStringLiteral("reveal-all"));
        }
    } else if (id == QLatin1String("deleteLayerMask")) {
        if (view_) {
            layer_mask_delete_path(*view_, path);
        }
    } else if (id == QLatin1String("enableLayerMask")) {
        if (view_) {
            layer_mask_set_enabled_path(*view_, path, true);
        }
    } else if (id == QLatin1String("disableLayerMask")) {
        if (view_) {
            layer_mask_set_enabled_path(*view_, path, false);
        }
    } else if (id == QLatin1String("copyShapeAttributes")) {
        if (view_) {
            shape_copy_attributes(*view_, path);
        }
    } else if (id == QLatin1String("pasteShapeAttributes")) {
        if (view_) {
            shape_paste_attributes(*view_, path);
        }
    } else if (id == QLatin1String("rasterizeShape")) {
        if (view_) {
            shape_rasterize(*view_, path);
        }
    } else if (id == QLatin1String("convertToSmartObject")) {
        if (view_ && view_->convert_to_smart_object(path)) {
            refresh();
        }
    } else if (id == QLatin1String("editContents")) {
        emit smartObjectContentsRequested(QStringLiteral("edit"), path);
    } else if (id == QLatin1String("replaceContents")) {
        emit smartObjectContentsRequested(QStringLiteral("replace"), path);
    } else if (id == QLatin1String("exportContents")) {
        emit smartObjectContentsRequested(QStringLiteral("export"), path);
    } else if (id == QLatin1String("resetTransform")) {
        if (view_ && smart_object_reset_transform(*view_, path)) {
            refresh();
        }
    } else if (id == QLatin1String("convertToLayers")) {
        if (view_ && smart_object_convert_to_layers(*view_, path)) {
            refresh();
        }
    } else if (id == QLatin1String("newSmartObjectViaCopy")) {
        if (view_) {
            const QString copy = smart_object_new_via_copy(*view_, path);
            if (!copy.isEmpty()) {
                refresh();
            }
        }
    } else if (id == QLatin1String("blendingOptions")) {
        openLayerStyle(path);
    } else if (id == QLatin1String("copyLayerStyle")) {
        if (view_) {
            layer_style_copy(*view_, path);
        }
    } else if (id == QLatin1String("pasteLayerStyle")) {
        if (view_ && layer_style_paste(*view_, QStringList{path}) > 0) {
            refresh();
        }
    } else if (id == QLatin1String("clearLayerStyle")) {
        if (view_ && layer_style_clear(*view_, QStringList{path}) > 0) {
            refresh();
        }
    } else if (id == QLatin1String("groupFromLayers")) {
        openGroupFromLayersDialog();
    } else if (id == QLatin1String("link")) {
        if (view_) {
            const QStringList paths = targetPaths();
            const QModelIndex index = model_ ? model_->indexForPath(path) : QModelIndex();
            const bool linked = index.isValid() && index.data(LayerRowLinkedRole).toBool();
            if (!paths.isEmpty() && view_->link_layers(paths, !linked) > 0) {
                refresh();
            }
        }
    } else if (id == QLatin1String("selectLinked")) {
        if (view_ && !view_->select_linked(path).isEmpty()) {
            refresh();
        }
    } else if (id == QLatin1String("mergeDown")) {
        if (view_) {
            const QString merged = view_->merge_layers(targetPaths());
            if (!merged.isEmpty()) {
                refresh();
            }
        }
    } else if (id == QLatin1String("mergeVisible")) {
        if (view_ && !view_->merge_visible(path).isEmpty()) {
            refresh();
        }
    } else if (id == QLatin1String("mergeClippingMask")) {
        if (view_ && view_->merge_clipping_mask(path) > 0) {
            refresh();
        }
    } else if (id == QLatin1String("flatten")) {
        if (view_) {
            view_->flatten_image();
        }
    } else if (id == QLatin1String("rasterizeType")) {
        if (view_ && view_->rasterize_type(path)) {
            refresh();
        }
    } else if (id == QLatin1String("rasterizeLayer")) {
        if (view_) {
            const bool changed = view_->layer_is_fill_content(path)
                                     ? view_->rasterize_fill_content(path)
                                     : view_->rasterize_smart_object(path);
            if (changed) {
                refresh();
            }
        }
    } else if (id == QLatin1String("addVectorMask")) {
        if (view_) {
            vector_mask_add_path(*view_, path, QStringLiteral("reveal-all"));
        }
    } else if (id == QLatin1String("deleteVectorMask")) {
        if (view_) {
            vector_mask_delete_path(*view_, path);
        }
    } else if (id == QLatin1String("enableVectorMask")) {
        if (view_) {
            vector_mask_set_enabled_path(*view_, path, true);
        }
    } else if (id == QLatin1String("disableVectorMask")) {
        if (view_) {
            vector_mask_set_enabled_path(*view_, path, false);
        }
    } else if (id == QLatin1String("clippingToggle")) {
        if (view_) {
            clipping_toggle(*view_, path);
        }
    } else if (id == QLatin1String("editAdjustment")) {
        emit editContentRequested(path);
    } else if (id == QLatin1String("exportAs")) {
        exportAsFromView(this, view_);
    } else if (id == QLatin1String("quickExport")) {
        quickExportPngFromView(this, view_);
    } else {
        return false;
    }
    return true;
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
            QString merged;
            if (actionId == QLatin1String("mergeDown")) {
                if (!paths.isEmpty()) {
                    merged = view_->merge_layers(paths);
                }
            } else if (actionId == QLatin1String("mergeVisible")) {
                if (!path.isEmpty()) {
                    merged = view_->merge_visible(path);
                }
            } else if (!path.isEmpty()) {
                view_->merge_clipping_mask(path);
            }
            if (!merged.isEmpty()) {
                refresh();
            }
        }
    } else if (actionId == QLatin1String("flatten")) {
        if (view_) {
            view_->flatten_image();
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
