#include "layers_panel.h"

#include "export_as_dialog.h"
#include "layers_panel_internal.h"

#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/smart_object_actions.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"
#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"

#include <QtCore/QItemSelectionModel>
#include <QtCore/QModelIndex>
#include <QtCore/QPoint>
#include <QtCore/QStringList>
#include <QtGui/QAction>
#include <QtGui/QIcon>
#include <QtGui/QPixmap>
#include <QtWidgets/QMenu>
#include <QtWidgets/QTreeView>

#include <algorithm>

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

struct RowSpec {
    const char* id;
    const char* label;
    unsigned kinds;
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
    };
    Enable enable = Always;
};

// Row menu, in block order: common rows, kind-specific rows, then export rows.
// `populateRowMenu` emits one separator between each non-empty block; Rename
// and the Color Label submenu bracket the table as bespoke head/tail.
const RowSpec kRowSpecs[] = {
    // Common to every kind.
    {"newLayer", "New Layer", kAllKinds, true},
    {"newGroup", "New Group", kAllKinds, true},
    {"duplicate", "Duplicate Layer(s)", kAllKinds, true},
    {"delete", "Delete Layer(s)", kAllKinds, true},
    {"group", "Group Layers", kAllKinds, true},
    {"ungroup", "Ungroup Layers", kAllKinds, true},
    {"moveUp", "Move Layer Up", kAllKinds, true},
    {"moveDown", "Move Layer Down", kAllKinds, true},
    // Kind-specific rows.
    {"addLayerMask", "Add Layer Mask", Pixel, true},
    {"deleteLayerMask", "Delete Layer Mask", Pixel, true},
    {"enableLayerMask", "Enable Layer Mask", Pixel, true},
    {"disableLayerMask", "Disable Layer Mask", Pixel, true},
    {"blendingOptions", "Blending Options…", Pixel | Background | Group | Type, true,
     RowSpec::StyleEdit},
    {"copyLayerStyle", "Copy Layer Style", Pixel, true, RowSpec::HasStyle},
    {"pasteLayerStyle", "Paste Layer Style", Pixel, true, RowSpec::CanPasteStyle},
    {"clearLayerStyle", "Clear Layer Style", Pixel, true, RowSpec::HasStyle},
    {"editAdjustment", "Edit Adjustment…", Adjustment, false},
    {"rasterizeType", "Rasterize Type", Type, false},
    {"copyShapeAttributes", "Copy Shape Attributes", Shape, true},
    {"pasteShapeAttributes", "Paste Shape Attributes", Shape, true},
    {"rasterizeShape", "Rasterize Shape", Shape, true},
    {"convertToSmartObject", "Convert to Smart Object", Pixel, true, RowSpec::ConvertToSmartCan},
    {"resetTransform", "Reset Transform", SmartObject, true, RowSpec::ResetTransformCan},
    {"convertToLayers", "Convert to Layers", SmartObject, true, RowSpec::ConvertToLayersCan},
    {"newSmartObjectViaCopy", "New Smart Object via Copy", SmartObject, true,
     RowSpec::NewViaCopyCan},
    {"editContents", "Edit Contents", SmartObject, true, RowSpec::EditContentsCan},
    {"replaceContents", "Replace Contents…", SmartObject, true, RowSpec::ReplaceContentsCan},
    {"exportContents", "Export Contents…", SmartObject, true, RowSpec::ExportContentsCan},
    {"relinkToFile", "Relink to File…", SmartObject, false},
    {"convertToLinked", "Convert to Linked…", SmartObject, false},
    // Export writes the flattened composite; pixel and background only (smart
    // objects report the pixel kind), never groups or adjustment layers.
    {"exportAs", "Export As…", Pixel | Background, true},
    {"quickExport", "Quick Export as PNG", Pixel | Background, true},
};

constexpr int kRowCount = sizeof(kRowSpecs) / sizeof(kRowSpecs[0]);

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

} // namespace

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
    const QString path = pathForProxyIndex(index);
    if (path.isEmpty()) {
        return;
    }
    if (!tree_->selectionModel()->isSelected(index)) {
        tree_->setCurrentIndex(index);
    }

    QMenu menu(tree_);
    populateRowMenu(menu, path, index.data(ColorRole).toInt(), index.data(KindRole).toString(),
                    index.data(LayerRowShapeRole).toBool(), index.data(SmartObjectRole).toBool());
    menu.exec(tree_->viewport()->mapToGlobal(pos));
}

void LayersPanel::populateRowMenu(QMenu& menu, const QString& path, int color,
                                  const QString& kind, bool shape, bool smart)
{
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
    const RowSpec* const begin = kRowSpecs;
    const RowSpec* const end = kRowSpecs + kRowCount;
    const RowSpec* const commonEnd =
        std::find_if(begin, end, [](const RowSpec& row) { return row.kinds != kAllKinds; });
    const RowSpec* const exportBegin = std::find_if(begin, end, [](const RowSpec& row) {
        return QLatin1String(row.id) == QLatin1String("exportAs");
    });

    bool first = true;
    const auto appendBlock = [&](const RowSpec* blockBegin, const RowSpec* blockEnd) {
        const bool applies = std::any_of(blockBegin, blockEnd, [mask](const RowSpec& row) {
            return (row.kinds & mask) != 0;
        });
        if (!applies) {
            return;
        }
        if (!first) {
            menu.addSeparator();
        }
        first = false;
        for (const RowSpec* row = blockBegin; row != blockEnd; ++row) {
            if ((row->kinds & mask) == 0) {
                continue;
            }
            QAction* action = menu.addAction(tr(row->label));
            if (!row->implemented) {
                action->setEnabled(false);
                action->setToolTip(tr("%1 — not implemented yet").arg(tr(row->label)));
                continue;
            }
            bool enabled = true;
            switch (row->enable) {
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
            case RowSpec::Always:
                break;
            }
            if (!enabled) {
                action->setEnabled(false);
                continue;
            }
            const QString id = QLatin1String(row->id);
            connect(action, &QAction::triggered, this,
                    [this, id, path] { performRowAction(id, path); });
        }
    };
    appendBlock(begin, commonEnd);
    appendBlock(commonEnd, exportBegin);
    appendBlock(exportBegin, end);

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
            selectPath(path);
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
                selectPath(copy);
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
            const QStringList paths = selectedPaths();
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
            const QStringList paths = selectedPaths();
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
            const QStringList paths = selectedPaths();
            if (!paths.isEmpty()) {
                view_->link_layers(paths, true);
            }
        }
    } else if (actionId == QLatin1String("selectLinked")) {
        if (view_) {
            const QString path = currentPath();
            const QStringList linked = path.isEmpty() ? QStringList() : view_->select_linked(path);
            if (!linked.isEmpty()) {
                selectPaths(linked, path);
            }
        }
    } else if (actionId == QLatin1String("moveUp")) {
        moveCurrent(1);
    } else if (actionId == QLatin1String("moveDown")) {
        moveCurrent(-1);
    } else if (actionId == QLatin1String("mergeDown") || actionId == QLatin1String("mergeVisible")
               || actionId == QLatin1String("mergeClippingMask")) {
        if (view_) {
            const QStringList paths = selectedPaths();
            const QString path = currentPath();
            if (actionId == QLatin1String("mergeDown")) {
                if (!paths.isEmpty()) {
                    view_->merge_layers(paths);
                }
            } else if (actionId == QLatin1String("mergeVisible")) {
                if (!path.isEmpty()) {
                    view_->merge_visible(path);
                }
            } else if (!path.isEmpty()) {
                view_->merge_clipping_mask(path);
            }
        }
    } else if (actionId == QLatin1String("flatten")) {
        if (view_) {
            view_->flatten_image();
        }
    } else if (actionId == QLatin1String("panelOptions")) {
        openPanelOptions();
    } else if (actionId.startsWith(QLatin1String("adjustment:"))) {
        if (view_) {
            view_->add_adjustment(actionId.mid(QLatin1String("adjustment:").size()));
        }
    } else {
        return false;
    }
    return true;
}

} // namespace pictura
