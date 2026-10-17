#pragma once

#include <QtCore/QHash>
#include <QtCore/QList>
#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QKeySequence>

#include <functional>
#include <vector>

class QAction;
class QMenuBar;

namespace pictura {

// Stable command identifiers. Command ids are the dispatch key and the key for
// future localization and custom menu sets; never derive them from labels.
namespace command_ids {
inline constexpr char FileOpen[] = "file.open";
inline constexpr char FileOpenAsSmartObject[] = "file.openAsSmartObject";
inline constexpr char FileNew[] = "file.new";
inline constexpr char FileSave[] = "file.save";
inline constexpr char FileSaveAs[] = "file.saveAs";
inline constexpr char FileExportAs[] = "file.exportAs";
inline constexpr char FileQuickExportPng[] = "file.quickExportPng";
inline constexpr char FileRevert[] = "file.revert";
inline constexpr char FilePlace[] = "file.place";
inline constexpr char FileInfo[] = "file.fileInfo";
inline constexpr char FileClose[] = "file.close";
inline constexpr char FileCloseAll[] = "file.closeAll";
inline constexpr char FileExit[] = "file.exit";
inline constexpr char EditUndo[] = "edit.undo";
inline constexpr char EditRedo[] = "edit.redo";
inline constexpr char EditStepBackward[] = "edit.stepBackward";
inline constexpr char EditStepForward[] = "edit.stepForward";
inline constexpr char EditCut[] = "edit.cut";
inline constexpr char EditCopy[] = "edit.copy";
inline constexpr char EditCopyMerged[] = "edit.copyMerged";
inline constexpr char EditPaste[] = "edit.paste";
inline constexpr char EditPasteInPlace[] = "edit.pasteSpecial.pasteInPlace";
inline constexpr char EditPasteInto[] = "edit.pasteSpecial.pasteInto";
inline constexpr char EditPasteOutside[] = "edit.pasteSpecial.pasteOutside";
inline constexpr char EditClear[] = "edit.clear";
inline constexpr char EditFill[] = "edit.fill";
inline constexpr char EditStroke[] = "edit.stroke";
inline constexpr char EditPurgeClipboard[] = "edit.purge.clipboard";
inline constexpr char EditPurgeUndo[] = "edit.purge.undo";
inline constexpr char EditPurgeHistories[] = "edit.purge.histories";
inline constexpr char EditPurgeAll[] = "edit.purge.all";
inline constexpr char EditFreeTransform[] = "edit.freeTransform";
inline constexpr char EditTransformSkew[] = "edit.transform.skew";
inline constexpr char EditTransformDistort[] = "edit.transform.distort";
inline constexpr char EditTransformPerspective[] = "edit.transform.perspective";
inline constexpr char EditTransformWarp[] = "edit.transform.warp";
inline constexpr char EditKeyboardShortcuts[] = "edit.keyboardShortcuts";
inline constexpr char EditPreferencesGeneral[] = "edit.preferences.general";
inline constexpr char EditPreferencesInterface[] = "edit.preferences.interface";
inline constexpr char EditPreferencesPerformance[] = "edit.preferences.performance";
inline constexpr char EditPreferencesUnits[] = "edit.preferences.unitsRulers";
inline constexpr char EditPreferencesGuides[] = "edit.preferences.guidesGridSlices";
inline constexpr char EditAssignProfile[] = "edit.assignProfile";
inline constexpr char EditConvertProfile[] = "edit.convertProfile";
inline constexpr char EditColorSettings[] = "edit.colorSettings";
inline constexpr char EditPdfPresets[] = "edit.pdfPresets";
inline constexpr char FilterPicturaRaw[] = "filter.picturaRaw";
inline constexpr char FilterConvertForSmartFilters[] = "filter.convertForSmartFilters";
inline constexpr char FilterLastFilter[] = "filter.last";
inline constexpr char FilterLastFilterSettings[] = "filter.lastSettings";
inline constexpr char ImageRotate90Cw[] = "image.rotate90cw";
inline constexpr char ImageRotate90Ccw[] = "image.rotate90ccw";
inline constexpr char ImageRotate180[] = "image.rotate180";
inline constexpr char ImageRotateArbitrary[] = "image.rotate.arbitrary";
inline constexpr char ImageFlipHorizontal[] = "image.flipHorizontal";
inline constexpr char ImageFlipVertical[] = "image.flipVertical";
inline constexpr char ImageCrop[] = "image.crop";
inline constexpr char ImageTrim[] = "image.trim";
inline constexpr char ImageRevealAll[] = "image.revealAll";
inline constexpr char ImageDuplicate[] = "image.duplicate";
inline constexpr char ImageModeBitmap[] = "image.mode.bitmap";
inline constexpr char ImageModeGrayscale[] = "image.mode.grayscale";
inline constexpr char ImageModeIndexed[] = "image.mode.indexed.color";
inline constexpr char ImageModeRgb[] = "image.mode.rgb.color";
inline constexpr char ImageModeCmyk[] = "image.mode.cmyk.color";
inline constexpr char ImageModeLab[] = "image.mode.lab.color";
inline constexpr char ImageMode8Bits[] = "image.mode.8.bits";
inline constexpr char ImageMode16Bits[] = "image.mode.16.bits";
inline constexpr char ImageMode32Bits[] = "image.mode.32.bits";
inline constexpr char SelectAll[] = "select.all";
inline constexpr char SelectDeselect[] = "select.deselect";
inline constexpr char SelectReselect[] = "select.reselect";
inline constexpr char SelectInverse[] = "select.inverse";
inline constexpr char SelectModifyBorder[] = "select.modify.border";
inline constexpr char SelectModifySmooth[] = "select.modify.smooth";
inline constexpr char SelectModifyExpand[] = "select.modify.expand";
inline constexpr char SelectModifyContract[] = "select.modify.contract";
inline constexpr char SelectModifyFeather[] = "select.modify.feather";
inline constexpr char SelectGrow[] = "select.grow";
inline constexpr char SelectSimilar[] = "select.similar";
inline constexpr char SelectSave[] = "select.save";
inline constexpr char SelectLoad[] = "select.load";
inline constexpr char SelectAllLayers[] = "select.allLayers";
inline constexpr char SelectDeselectLayers[] = "select.deselectLayers";
inline constexpr char SelectSimilarLayers[] = "select.similarLayers";
inline constexpr char LayerNewLayer[] = "layer.new.layer";
inline constexpr char LayerNewGroup[] = "layer.new.group";
inline constexpr char LayerNewGroupFromLayers[] = "layer.new.groupFromLayers";
inline constexpr char LayerDuplicateLayer[] = "layer.duplicate.layer";
inline constexpr char LayerGroupLayers[] = "layer.group.layers";
inline constexpr char LayerUngroupLayers[] = "layer.ungroup.layers";
inline constexpr char LayerMergeLayers[] = "layer.merge.layers";
inline constexpr char LayerMergeVisible[] = "layer.merge.visible";
inline constexpr char LayerMergeDown[] = "layer.merge.down";
inline constexpr char LayerArrangeFront[] = "layer.arrange.front";
inline constexpr char LayerArrangeForward[] = "layer.arrange.forward";
inline constexpr char LayerArrangeBackward[] = "layer.arrange.backward";
inline constexpr char LayerArrangeBack[] = "layer.arrange.back";
inline constexpr char LayerArrangeReverse[] = "layer.arrange.reverse";
inline constexpr char LayerStampVisible[] = "layer.stamp.visible";
inline constexpr char LayerStampSelected[] = "layer.stamp.selected";
inline constexpr char FileSaveForWeb[] = "file.saveForWeb";
inline constexpr char LayerMergeClippingMask[] = "layer.merge.clippingMask";
inline constexpr char LayerMattingDefringe[] = "layer.matting.defringe";
inline constexpr char LayerMattingRemoveBlack[] = "layer.matting.removeBlack";
inline constexpr char LayerMattingRemoveWhite[] = "layer.matting.removeWhite";
inline constexpr char LayerCreateClippingMask[] = "layer.create.clipping.mask";
inline constexpr char LayerReleaseClippingMask[] = "layer.release.clipping.mask";
inline constexpr char LayerMaskRevealAll[] = "layer.mask.revealAll";
inline constexpr char LayerMaskHideAll[] = "layer.mask.hideAll";
inline constexpr char LayerMaskRevealSelection[] = "layer.mask.revealSelection";
inline constexpr char LayerMaskHideSelection[] = "layer.mask.hideSelection";
inline constexpr char LayerMaskFromTransparency[] = "layer.mask.fromTransparency";
inline constexpr char LayerMaskDelete[] = "layer.mask.delete";
inline constexpr char LayerMaskApply[] = "layer.mask.apply";
inline constexpr char LayerMaskEnable[] = "layer.mask.enable";
inline constexpr char LayerMaskDisable[] = "layer.mask.disable";
inline constexpr char LayerMaskLink[] = "layer.mask.link";
inline constexpr char LayerMaskUnlink[] = "layer.mask.unlink";
inline constexpr char LayerVectorMaskRevealAll[] = "layer.vectorMask.revealAll";
inline constexpr char LayerVectorMaskHideAll[] = "layer.vectorMask.hideAll";
inline constexpr char LayerVectorMaskCurrentPath[] = "layer.vectorMask.currentPath";
inline constexpr char LayerVectorMaskDelete[] = "layer.vectorMask.delete";
inline constexpr char LayerVectorMaskEnable[] = "layer.vectorMask.enable";
inline constexpr char LayerVectorMaskDisable[] = "layer.vectorMask.disable";
inline constexpr char LayerVectorMaskLink[] = "layer.vectorMask.link";
inline constexpr char LayerVectorMaskUnlink[] = "layer.vectorMask.unlink";
inline constexpr char LayerFlattenImage[] = "layer.flatten.image";
inline constexpr char LayerNewLayerFromBackground[] = "layer.new.layerFromBackground";
inline constexpr char LayerNewBackgroundFromLayer[] = "layer.new.backgroundFromLayer";
inline constexpr char LayerNewLayerViaCopy[] = "layer.new.layerViaCopy";
inline constexpr char LayerNewLayerViaCut[] = "layer.new.layerViaCut";
inline constexpr char LayerDeleteLayer[] = "layer.delete.layer";
inline constexpr char LayerDeleteHiddenLayers[] = "layer.delete.hiddenLayers";
inline constexpr char LayerSelectSimilar[] = "layer.select.similar";
inline constexpr char LayerSelectLinked[] = "layer.select.linked";
inline constexpr char LayerLinkLayers[] = "layer.link.layers";
inline constexpr char LayerUnlinkLayers[] = "layer.unlink.layers";
inline constexpr char LayerHideLayers[] = "layer.hide.layers";
inline constexpr char LayerLockLayersAll[] = "layer.lock.layers.all";
inline constexpr char LayerLockLayersTransparency[] = "layer.lock.layers.transparency";
inline constexpr char LayerLockLayersImage[] = "layer.lock.layers.image";
inline constexpr char LayerLockLayersPosition[] = "layer.lock.layers.position";
inline constexpr char LayerLockAllInGroup[] = "layer.lock.allInGroup";
inline constexpr char LayerNewFillSolidColor[] = "layer.new.fill.solidColor";
inline constexpr char LayerNewFillGradient[] = "layer.new.fill.gradient";
inline constexpr char LayerContentOptions[] = "layer.contentOptions";
inline constexpr char LayerRasterizeFillContent[] = "layer.rasterize.fillContent";
inline constexpr char LayerRasterizeType[] = "layer.rasterize.type";
inline constexpr char LayerRasterizeShape[] = "layer.rasterize.shape";
inline constexpr char LayerRasterizeLayer[] = "layer.rasterize.layer";
inline constexpr char LayerRasterizeAllLayers[] = "layer.rasterize.allLayers";
inline constexpr char LayerRasterizeSmartObject[] = "layer.rasterize.smartObject";
inline constexpr char LayerRasterizeVectorMask[] = "layer.rasterize.vectorMask";
inline constexpr char LayerCopyShapeAttributes[] = "layer.copyShapeAttributes";
inline constexpr char LayerPasteShapeAttributes[] = "layer.pasteShapeAttributes";
inline constexpr char LayerSmartObjectConvertTo[] = "layer.smartObject.convertTo";
inline constexpr char LayerSmartObjectReplaceContents[] = "layer.smartObject.replaceContents";
inline constexpr char LayerSmartObjectEditContents[] = "layer.smartObject.editContents";
inline constexpr char LayerSmartObjectExportContents[] = "layer.smartObject.exportContents";
inline constexpr char LayerSmartObjectResetTransform[] = "layer.smartObject.resetTransform";
inline constexpr char LayerSmartObjectConvertToLayers[] = "layer.smartObject.convertToLayers";
inline constexpr char LayerSmartObjectNewViaCopy[] = "layer.smartObject.newViaCopy";
inline constexpr char LayerSmartFiltersClear[] = "layer.smartFilters.clear";
inline constexpr char ViewZoomIn[] = "view.zoomIn";
inline constexpr char ViewZoomOut[] = "view.zoomOut";
inline constexpr char ViewFitOnScreen[] = "view.fitOnScreen";
inline constexpr char ViewActualPixels[] = "view.actualPixels";
inline constexpr char ViewOptions[] = "view.options";
inline constexpr char ViewFreePan[] = "view.freePan";
inline constexpr char ViewSnapCanvasToWorkspace[] = "view.snapCanvasToWorkspace";
inline constexpr char ViewToolHints[] = "view.toolHints";
inline constexpr char ViewShowSelectionEdges[] = "view.show.selectionEdges";
inline constexpr char ViewRulers[] = "view.rulers";
inline constexpr char ViewShowGuides[] = "view.show.guides";
inline constexpr char ViewLockGuides[] = "view.lockGuides";
inline constexpr char ViewClearGuides[] = "view.clearGuides";
inline constexpr char ViewNewGuide[] = "view.newGuide";
inline constexpr char ViewScreenModeStandard[] = "view.screenMode.standard";
inline constexpr char ViewScreenModeFullWithMenuBar[] = "view.screenMode.fullWithMenuBar";
inline constexpr char ViewScreenModeFull[] = "view.screenMode.full";
inline constexpr char WindowPanelsLayers[] = "window.panels.layers";
inline constexpr char WindowPanelsTools[] = "window.panels.tools";
inline constexpr char WindowPanelsNavigator[] = "window.panels.navigator";
inline constexpr char WindowPanelsNotes[] = "window.panels.notes";
inline constexpr char WindowPanelsHistory[] = "window.panels.history";
inline constexpr char WindowPanelsColor[] = "window.panels.color";
inline constexpr char WindowPanelsSwatches[] = "window.panels.swatches";
inline constexpr char WindowPanelsInfo[] = "window.panels.info";
inline constexpr char WindowPanelsHistogram[] = "window.panels.histogram";
inline constexpr char WindowPanelsGradients[] = "window.panels.gradients";
inline constexpr char WindowPanelsPatterns[] = "window.panels.patterns";
inline constexpr char WindowPanelsProperties[] = "window.panels.properties";
inline constexpr char WindowPanelsAdjustments[] = "window.panels.adjustments";
inline constexpr char WindowPanelsChannels[] = "window.panels.channels";
inline constexpr char WindowPanelsPaths[] = "window.panels.paths";
inline constexpr char WindowPanelsActions[] = "window.panels.actions";
inline constexpr char WindowPanelsBrush[] = "window.panels.brushes";
inline constexpr char WindowPanelsCloneSource[] = "window.panels.cloneSource";
inline constexpr char WindowPanelsCharacter[] = "window.panels.character";
inline constexpr char WindowPanelsParagraph[] = "window.panels.paragraph";
// Not a CS6 panel (Photoshop CC 2015); kept for the photorust port (#65).
inline constexpr char WindowPanelsGlyphs[] = "window.panels.glyphs";
inline constexpr char WindowPanelsParagraphStyles[] = "window.panels.paragraphStyles";
inline constexpr char WindowWorkspaceEssentials[] = "window.workspace.essentials";
inline constexpr char WindowWorkspacePainting[] = "window.workspace.painting";
inline constexpr char WindowWorkspacePhotography[] = "window.workspace.photography";
inline constexpr char WindowWorkspaceTypography[] = "window.workspace.typography";
inline constexpr char WindowWorkspaceNew[] = "window.workspace.new";
inline constexpr char WindowWorkspaceDelete[] = "window.workspace.delete";
inline constexpr char WindowWorkspaceReset[] = "window.workspace.reset";
inline constexpr char WindowWorkspaceKeyboardShortcuts[] = "window.workspace.keyboardShortcuts";
inline constexpr char TypePanelsCharacter[] = "type.panels.character";
inline constexpr char TypePanelsParagraph[] = "type.panels.paragraph";
inline constexpr char TypePanelsParagraphStyles[] = "type.panels.paragraphStyles";
inline constexpr char HelpAbout[] = "help.about";
inline constexpr char HelpDeveloperDesignSystem[] = "help.developer.designSystem";
} // namespace command_ids

// One entry of the command table. An entry with an empty `id` is a separator.
struct CommandSpec {
    QString id;
    QStringList path;                 // top-level menu first, leaf last
    QString label;
    QKeySequence shortcut;
    bool implemented = false;
    bool checkable = false;
};

// A read-only snapshot of one command, for `list_commands` and any inspection
// surface. Separators are excluded.
struct CommandInfo {
    QString id;
    QStringList path;
    QString label;
    bool implemented = false;
    bool enabled = false;
    bool checked = false;
};

// Declarative command table plus menu construction and dispatch. The registry
// owns no application behavior: the frame supplies handlers, providers, and
// panel-facing actions by id.
class CommandRegistry : public QObject {
    Q_OBJECT

public:
    explicit CommandRegistry(QObject* parent = nullptr);

    void add(const CommandSpec& spec);
    void add(const QString& id, const QStringList& path, const QString& label,
             const QKeySequence& shortcut = QKeySequence(), bool implemented = false);
    void addSeparator(const QStringList& path);

    void setHandler(const QString& id, std::function<void()> handler);

    // Mark an already-added command as implemented (used to light up leaves
    // registered as stubs once their handler is wired).
    void setImplemented(const QString& id, bool implemented);

    // Extra enablement predicate for `id`; combined with `implemented` and the
    // presence of a handler.
    void setEnabledProvider(const QString& id, std::function<bool()> enabled);
    // Checked-state provider for a checkable command.
    void setCheckedProvider(const QString& id, std::function<bool()> checked);
    // Dynamic label for `id`; overrides the static label when set.
    void setLabelProvider(const QString& id, std::function<QString()> label);

    QAction* action(const QString& id) const;

    // The command's current and built-in key bindings (Edit > Keyboard
    // Shortcuts may give a command several, or none). setShortcuts() also
    // updates the live action.
    QList<QKeySequence> shortcuts(const QString& id) const;
    QList<QKeySequence> defaultShortcuts(const QString& id) const;
    void setShortcuts(const QString& id, const QList<QKeySequence>& shortcuts);
    // The command holding `shortcut`, or an empty id when none does.
    QString commandForShortcut(const QKeySequence& shortcut) const;
    // Every command whose bindings differ from its defaults.
    QHash<QString, QList<QKeySequence>> shortcutOverrides() const;

    // Build (or rebuild) `menuBar` from the command table.
    void buildMenuBar(QMenuBar* menuBar);

    // Re-evaluate enabled/checked/label for every action.
    void refresh();

    // How many times refresh() has run. Test-only probe so a self-test can
    // prove the paint region path does not fan out per dab.
    int refreshCount() const { return refreshCount_; }

    // Run the handler for `id`; returns false and does nothing when none is set.
    bool dispatch(const QString& id);

    // Titles of the top-level menus in creation order.
    QStringList topLevelTitles() const;

    // Snapshot of the command table (separators excluded) with each command's
    // implemented/enabled/checked state.
    QList<CommandInfo> describe() const;

private:
    bool hasHandler(const QString& id) const;

    struct Entry {
        CommandSpec spec;
        QAction* action = nullptr;
        QList<QKeySequence> shortcuts;
        QList<QKeySequence> defaults;
    };
    const Entry* entry(const QString& id) const;
    std::vector<Entry> entries_;
    QHash<QString, std::function<void()>> handlers_;
    QHash<QString, std::function<bool()>> enabledProviders_;
    QHash<QString, std::function<bool()>> checkedProviders_;
    QHash<QString, std::function<QString()>> labelProviders_;
    QMenuBar* menuBar_ = nullptr;
    int refreshCount_ = 0;
};

// Populate `registry` with the full documented CS6 command tree
// (docs/02-ui-ux/menus.md). Commands without a handler are added with
// `implemented = false`; the ids in `command_ids` are added as implemented.
void addDefaultCommands(CommandRegistry& registry);

// The stable id derived from a command path's segments (path-derived stubs and
// their handlers must agree on this, so both call it).
QString commandIdForPath(const QStringList& path);

} // namespace pictura
