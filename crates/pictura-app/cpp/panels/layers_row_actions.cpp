#include "layers_row_actions.h"

#include "commands.h"
#include "layers_panel_model.h"

#include <utility>

namespace pictura {

namespace {

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

enum class RowDynamic {
    None,
    RasterizeLayer,
    RasterizeStyle,
    Relink,
    ClippingToggle,
    Link,
    AddMaskSubmenu,
    AddVectorMaskSubmenu,
};

struct RowAction {
    const char* id;
    const char* label;
    unsigned kinds;
    unsigned block;
    const char* commandId;
    RowDynamic dynamic;
};

// Row menu, in CS6 block order. The panel emits one separator between each
// non-empty block; Rename and the Color Label submenu bracket the table as
// bespoke head/tail. The Background uses its own brief table.
const RowAction kRowActions[] = {
    // 1. Blending / adjustment edit.
    {"blendingOptions", "Blending Options…", Pixel | Group | Type, 1,
     command_ids::LayerStyleBlendingOptions, RowDynamic::None},
    {"editAdjustment", "Edit Adjustment…", Adjustment, 1,
     command_ids::PanelLayersEditAdjustment, RowDynamic::None},
    {"copyCss", "Copy CSS", Type | Group | Adjustment, 1, nullptr, RowDynamic::None},
    {"copySvg", "Copy SVG", Type | Group | Adjustment, 1, nullptr, RowDynamic::None},
    // 2. Layer operations.
    {"duplicate", "Duplicate Layer(s)", kEveryKind, 2, command_ids::LayerDuplicateLayer,
     RowDynamic::None},
    {"delete", "Delete Layer(s)", kEveryKind, 2, command_ids::LayerDeleteLayer,
     RowDynamic::None},
    {"groupFromLayers", "Group from Layers…", kEveryKind, 2,
     command_ids::LayerNewGroupFromLayers, RowDynamic::None},
    {"newLayer", "New Layer", kEveryKind, 2, command_ids::PanelLayersNewLayer,
     RowDynamic::None},
    {"newGroup", "New Group", kEveryKind, 2, command_ids::PanelLayersNewGroup,
     RowDynamic::None},
    {"ungroup", "Ungroup Layers", Group, 2, command_ids::LayerUngroupLayers,
     RowDynamic::None},
    {"moveUp", "Move Layer Up", kEveryKind, 2, command_ids::PanelLayersMoveUp,
     RowDynamic::None},
    {"moveDown", "Move Layer Down", kEveryKind, 2, command_ids::PanelLayersMoveDown,
     RowDynamic::None},
    // 3. Export.
    {"quickExport", "Quick Export as PNG", kAllKinds, 3, command_ids::FileQuickExportPng,
     RowDynamic::None},
    {"exportAs", "Export As…", kAllKinds, 3, command_ids::FileExportAs, RowDynamic::None},
    // 4. Convert.
    {"artboardFromLayers", "Artboard from Layers…", Type | Group | Adjustment, 4, nullptr,
     RowDynamic::None},
    {"frameFromLayers", "Frame from Layers…", Type | Group | Adjustment, 4, nullptr,
     RowDynamic::None},
    {"convertToFrame", "Convert to Frame", Type | Group | Adjustment, 4, nullptr,
     RowDynamic::None},
    {"convertToSmartObject", "Convert to Smart Object",
     Pixel | Background | Type | Group | Adjustment, 4, command_ids::LayerSmartObjectConvertTo,
     RowDynamic::None},
    {"resetTransform", "Reset Transform", SmartObject, 4,
     command_ids::LayerSmartObjectResetTransform, RowDynamic::None},
    {"convertToLayers", "Convert to Layers", SmartObject, 4,
     command_ids::LayerSmartObjectConvertToLayers, RowDynamic::None},
    {"newSmartObjectViaCopy", "New Smart Object via Copy", SmartObject, 4,
     command_ids::LayerSmartObjectNewViaCopy, RowDynamic::None},
    {"editContents", "Edit Contents", SmartObject, 4,
     command_ids::LayerSmartObjectEditContents, RowDynamic::None},
    {"replaceContents", "Replace Contents…", SmartObject, 4,
     command_ids::LayerSmartObjectReplaceContents, RowDynamic::None},
    {"exportContents", "Export Contents…", SmartObject, 4,
     command_ids::LayerSmartObjectExportContents, RowDynamic::None},
    {"relinkToFile", "Relink to File…", SmartObject, 4, nullptr, RowDynamic::Relink},
    {"convertToLinked", "Convert to Linked…", SmartObject, 4, nullptr, RowDynamic::None},
    // 5. Rasterize.
    {"rasterizeType", "Rasterize Type", Type, 5, command_ids::LayerRasterizeType,
     RowDynamic::None},
    {"rasterizeShape", "Rasterize Shape", Shape, 5, command_ids::LayerRasterizeShape,
     RowDynamic::None},
    {"rasterizeLayer", "Rasterize Layer", SmartObject | Adjustment, 5, nullptr,
     RowDynamic::RasterizeLayer},
    {"rasterizeStyle", "Rasterize Layer Style", kEveryKind, 5,
     command_ids::LayerRasterizeLayerStyle, RowDynamic::RasterizeStyle},
    // 6. Masks and clipping.
    {"addLayerMask", "Add Layer Mask", Pixel | Group | Adjustment, 6, nullptr,
     RowDynamic::AddMaskSubmenu},
    {"deleteLayerMask", "Delete Layer Mask", Pixel | Group | Adjustment, 6,
     command_ids::LayerMaskDelete, RowDynamic::None},
    {"enableLayerMask", "Enable Layer Mask", Pixel | Group | Adjustment, 6,
     command_ids::LayerMaskEnable, RowDynamic::None},
    {"disableLayerMask", "Disable Layer Mask", Pixel | Group | Adjustment, 6,
     command_ids::LayerMaskDisable, RowDynamic::None},
    {"addVectorMask", "Add Vector Mask", Pixel | Group | Adjustment, 6, nullptr,
     RowDynamic::AddVectorMaskSubmenu},
    {"deleteVectorMask", "Delete Vector Mask", Pixel | Group | Adjustment, 6,
     command_ids::LayerVectorMaskDelete, RowDynamic::None},
    {"enableVectorMask", "Enable Vector Mask", Pixel | Group | Adjustment, 6,
     command_ids::LayerVectorMaskEnable, RowDynamic::None},
    {"disableVectorMask", "Disable Vector Mask", Pixel | Group | Adjustment, 6,
     command_ids::LayerVectorMaskDisable, RowDynamic::None},
    {"clippingToggle", "Create Clipping Mask", Pixel | Adjustment, 6, nullptr,
     RowDynamic::ClippingToggle},
    // 7. Link.
    {"link", "Link Layers", kEveryKind, 7, nullptr, RowDynamic::Link},
    {"selectLinked", "Select Linked Layers", kEveryKind, 7, command_ids::LayerSelectLinked,
     RowDynamic::None},
    // 8. Layer style clipboard.
    {"copyLayerStyle", "Copy Layer Style", kEveryKind, 8,
     command_ids::LayerStyleCopyLayerStyle, RowDynamic::None},
    {"pasteLayerStyle", "Paste Layer Style", kEveryKind, 8,
     command_ids::LayerStylePasteLayerStyle, RowDynamic::None},
    {"clearLayerStyle", "Clear Layer Style", kEveryKind, 8,
     command_ids::LayerStyleClearLayerStyle, RowDynamic::None},
    // 9. Shape attributes.
    {"copyShapeAttributes", "Copy Shape Attributes", Shape, 9,
     command_ids::LayerCopyShapeAttributes, RowDynamic::None},
    {"pasteShapeAttributes", "Paste Shape Attributes", Shape, 9,
     command_ids::LayerPasteShapeAttributes, RowDynamic::None},
    // 11. Merge and flatten. Block 10 is Release from Isolation, emitted
    // between the table blocks only while isolation is active.
    {"mergeDown", "Merge Down", Pixel | Group | Adjustment, 11, command_ids::LayerMergeLayers,
     RowDynamic::None},
    {"mergeVisible", "Merge Visible", kEveryKind, 11, command_ids::LayerMergeVisible,
     RowDynamic::None},
    {"mergeClippingMask", "Merge Clipping Mask", kEveryKind, 11,
     command_ids::LayerMergeClippingMask, RowDynamic::None},
    {"flatten", "Flatten Image", kEveryKind, 11, command_ids::LayerFlattenImage,
     RowDynamic::None},
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

} // namespace

QVector<ResolvedRowAction> resolveRowActions(const LayerRowNode& row, const RowFacts& facts)
{
    // A shape layer reports the `adjustment` kind (its solid fill), but its menu
    // is a pixel layer's common commands plus the shape rows — never the
    // adjustment-only rows.
    const unsigned mask = (row.shape ? Pixel : kindMaskFor(row.kind))
        | (row.shape ? Shape : 0u) | (row.smartObject ? SmartObject : 0u);

    QVector<ResolvedRowAction> actions;
    for (const RowAction& entry : kRowActions) {
        if ((entry.kinds & mask) == 0) {
            continue;
        }
        const QLatin1String id(entry.id);
        if (id == QLatin1String("rasterizeStyle") && !facts.hasStyle) {
            continue;
        }
        if (id == QLatin1String("rasterizeLayer") && !facts.fillContent && !row.smartObject) {
            continue;
        }

        ResolvedRowAction action;
        action.id = QString::fromLatin1(entry.id);
        action.label = QString::fromUtf8(entry.label);
        action.block = static_cast<int>(entry.block);
        switch (entry.dynamic) {
        case RowDynamic::None:
            action.commandId = entry.commandId ? QString::fromLatin1(entry.commandId) : QString();
            action.enabled = entry.commandId != nullptr;
            break;
        case RowDynamic::RasterizeStyle:
            action.commandId = QString::fromLatin1(command_ids::LayerRasterizeLayerStyle);
            action.enabled = facts.canRasterizeStyle;
            break;
        case RowDynamic::Relink:
            action.enabled = facts.canRelinkSmartObject;
            break;
        case RowDynamic::RasterizeLayer: {
            const char* command = command_ids::LayerRasterizeLayer;
            if (facts.canRasterizeSmartObject) {
                command = command_ids::LayerRasterizeSmartObject;
            } else if (facts.fillContent) {
                command = command_ids::LayerRasterizeFillContent;
            }
            action.commandId = QString::fromLatin1(command);
            break;
        }
        case RowDynamic::ClippingToggle:
            action.commandId = QString::fromLatin1(
                facts.canReleaseClipping ? command_ids::LayerReleaseClippingMask
                                         : command_ids::LayerCreateClippingMask);
            break;
        case RowDynamic::Link:
            action.commandId = QString::fromLatin1(
                row.linked ? command_ids::LayerUnlinkLayers : command_ids::LayerLinkLayers);
            break;
        case RowDynamic::AddMaskSubmenu:
            action.submenu = true;
            action.enabled = !facts.hasMask;
            break;
        case RowDynamic::AddVectorMaskSubmenu:
            action.submenu = true;
            action.enabled = !facts.hasVectorMask;
            break;
        }
        actions.push_back(std::move(action));
    }
    return actions;
}

} // namespace pictura
