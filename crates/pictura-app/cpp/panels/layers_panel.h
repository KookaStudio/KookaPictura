#pragma once

#include <QtCore/QHash>
#include <QtCore/QList>
#include <QtCore/QObject>
#include <QtCore/QPoint>
#include <QtCore/QSet>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QImage>
#include <QtWidgets/QWidget>

#include <functional>

class QComboBox;
class QColor;
class QDialog;
class QEvent;
class QImage;
class QMenu;
class QModelIndex;
class QRect;
class QTimer;
class QToolButton;
class QTreeView;

namespace pictura {

class LayerFilterBar;
class LayerRowDelegate;
struct LayerFilter;
class LayersFilterProxyModel;
class LayersModel;
class LayersTreeView;
class PercentField;
class PictureView;

class LayersPanel : public QWidget {
    Q_OBJECT

public:
    explicit LayersPanel(QWidget* parent = nullptr);

    void setView(PictureView* view);
    void refresh();

    /// Bottom-first top-level index of the selected row, or -1 when nothing is
    /// selected or the selected row is nested below a group.
    int currentLayer() const;

    /// Frozen path of the selected row, or empty when nothing is selected.
    QString currentPath() const;

    /// Frozen paths of every selected row (empty when nothing is selected).
    QStringList selectedPaths() const;

    /// Whether the current selection can be merged through the
    /// context-sensitive Merge Down / Merge Layers command.
    bool canMergeSelection() const;

    /// Replace the panel selection with `paths`, making `current` the current
    /// row (falling back to the first when `current` is not among them).
    void selectPaths(const QStringList& paths, const QString& current);

    /// Enter isolation mode: filter the panel to the selected layers (the
    /// Selected dimension) and scope Move Auto-Select to them.
    void enterIsolation();
    /// Leave isolation mode: turn the layer filter off.
    void releaseIsolation();
    /// Whether the Selected filter is currently isolating the selection.
    bool isolationActive() const;

    /// Select the row whose path matches and sync the controls; false when no
    /// such row exists. Test seam for the selection -> Properties wiring.
    bool selectRowForTest(const QString& path);

    // Self-test hooks (M39). Read the projected model and menus, drive solo and
    // inline rename, and expose the Panel Options values without synthetic
    // mouse/key input.
    bool rowHasMaskForTest(const QString& path) const;
    bool rowHasAdjustmentForTest(const QString& path) const;
    bool rowClippingForTest(const QString& path) const;
    bool rowClipBaseForTest(const QString& path) const;
    bool rowExpandableForTest(const QString& path) const;
    QString rowToolTipForTest(const QString& path) const;
    bool beginRenameForTest(const QString& path);
    QObject* itemDelegateForTest() const;
    void toggleSoloForTest(const QString& path);
    int thumbSizeIndexForTest() const;
    int thumbContentsForTest() const;
    bool expandNewEffectsForTest() const;
    bool addCopyOnDuplicateForTest() const;
    bool useDefaultMasksOnFillForTest() const;
    void setOptionsForTest(int size, int contents, bool expand);
    void setOptionFlagsForTest(bool addCopy, bool useDefaultMasks);
    QStringList rowMenuTextsForTest(const QString& kind = QStringLiteral("pixel"));
    bool rowMenuEnabledForTest(const QString& kind, const QString& text);
    QString rowMenuToolTipForTest(const QString& kind, const QString& text);
    QStringList shapeRowMenuTextsForTest();
    bool shapeRowMenuEnabledForTest(const QString& text);
    QStringList smartRowMenuTextsForTest();
    // Lock-strip toggles by flag: 0 transparency, 1 pixels, 2 position, 3 all.
    bool lockToggleEnabledForTest(int flag) const;
    bool lockToggleCheckedForTest(int flag) const;
    QStringList colorLabelTextsForTest();
    int lockButtonCountForTest() const;
    int opacityPercentForTest() const;
    int fillPercentForTest() const;
    // wpx_blend_row: the blend combo and opacity field rects in panel-local
    // coordinates (shared parent), for the control-row proportion check.
    QRect blendControlRectForTest() const;
    QRect opacityControlRectForTest() const;
    // The opacity field's own size hint: the layout pins it to this when it
    // carries no stretch, so a width above it proves it shares the row's slack.
    int opacitySizeHintWidthForTest() const;
    void setOpacityPercentForTest(int pct);
    void setFillPercentForTest(int pct);
    bool headerOrderOkForTest() const;
    bool opacityLabelPresentForTest() const;
    bool fillLabelPresentForTest() const;
    bool hasPanelMenuButtonForTest() const;
    bool filterToggleOnForTest() const;
    bool filterToggleHasIconForTest() const;
    int eyeLeftForTest(const QString& path) const;
    bool chevronClickExpandsForTest(const QString& path);
    QString opacityLabelTextForTest() const;
    bool opacitySuffixPresentForTest() const;
    bool opacitySuffixInsideEditForTest() const;
    bool opacityValueFitsForTest() const;
    int dragOpacitySliderForTest(int fromX1000, int toX1000);
    int lockBadgeLeftForTest(const QString& path) const;
    // Press the lock badge of `path`'s row; false when the row shows none.
    bool clickLockBadgeForTest(const QString& path);
    // The row's thumbnail as painted (a T card for a type layer); null if hidden.
    QImage rowThumbnailForTest(const QString& path) const;
    bool rowCheckStateForTest(const QString& path) const;
    bool lockIconsPresentForTest() const;
    bool treeDragEnabledForTest() const;
    bool moveForTest(const QString& path, const QString& target, int mode);
    bool canMoveForTest(const QString& path, const QString& target, int mode);
    bool dropOnStripButtonForTest(const QString& buttonName, const QStringList& paths);

    // Batch 1 hooks: drag flags, the label-filled visibility column, the
    // disabled edit triggers, and a synthesized double-click at the name or the
    // eye rect.
    bool layerDragFlagsForTest(const QString& path) const;
    QColor rowGutterColorForTest(const QString& path, int x = 1) const;
    bool editTriggersDisabledForTest() const;
    bool doubleClickAtForTest(const QString& path, bool atName);

    // Batch 2 hooks: drag-and-drop capability and resolved drop modes, a
    // synthesized drop, a thumbnail Ctrl-click, and the inline-editor state.
    bool layerRowDragSupportedForTest() const;
    bool dropIndicatorShownForTest() const;
    int dragMoveModeAtForTest(const QString& source, const QString& hover, bool above);
    bool dropAtForTest(const QString& source, const QString& hover, bool above);
    bool dropIntoForTest(const QString& source, const QString& hover);
    bool ctrlClickThumbnailForTest(const QString& path);
    bool inlineEditorOpenForTest() const;

    // Round-3 hooks: root drop capability, drag cursor, the rename band, the
    // Background conversion dialog, row roles/typography, and painted geometry.
    bool rootDropEnabledForTest() const;
    void beginDragCursorForTest();
    void endDragCursorForTest();
    int dragCursorShapeForTest() const;
    bool doubleClickChevronForTest(const QString& path);
    void setBackgroundConvertForTest(bool accept, const QString& name, int color);
    bool lockNestingHiddenForTest() const;
    bool rowLinkedForTest(const QString& path) const;
    bool rowMaskLinkedForTest(const QString& path) const;
    bool rowMaskDisabledForTest(const QString& path) const;
    // Click the link glyph / Shift-click the mask thumbnail of `path`'s row as a
    // user would; false when the row shows no mask.
    bool clickLinkGlyphForTest(const QString& path);
    bool shiftClickMaskThumbnailForTest(const QString& path);
    // Whether the row's mask is the view's mask edit target, and plain-clicking
    // its mask or layer thumbnail as a user would (test seam).
    bool rowMaskTargetForTest(const QString& path) const;
    bool clickMaskThumbnailForTest(const QString& path);
    // Alt-click the mask thumbnail of `path`'s row as a user would (the mask
    // solo toggle); false when the row shows no mask.
    bool altClickMaskThumbnailForTest(const QString& path);
    // Alt+Shift-click the mask thumbnail (the rubylith overlay toggle); false
    // when the row shows no mask.
    bool altShiftClickMaskThumbnailForTest(const QString& path);
    bool clickLayerThumbnailForTest(const QString& path);
    // The row's active edit target (0 image, 1 raster mask, 2 vector mask).
    int rowActiveThumbForTest(const QString& path) const;
    // The Opacity field's leading label alignment and its text's right edge.
    int opacityLabelAlignmentForTest() const;
    int opacityLabelRightForTest() const;
    // The chain glyph slot of `path`'s row, and the side the glyph is drawn at.
    QRect rowLinkGlyphRectForTest(const QString& path) const;
    int linkIconSideForTest() const;
    bool rowHasVectorMaskForTest(const QString& path) const;
    bool rowVectorMaskLinkedForTest(const QString& path) const;
    bool rowVectorMaskDisabledForTest(const QString& path) const;
    // Click the vector link glyph / Shift-click the vector thumbnail of
    // `path`'s row as a user would; false when the row shows no vector mask.
    bool clickVectorLinkGlyphForTest(const QString& path);
    bool shiftClickVectorMaskThumbnailForTest(const QString& path);
    bool rowPlacedForTest(const QString& path) const;
    bool rowShapeForTest(const QString& path) const;
    bool rowSmartObjectForTest(const QString& path) const;
    // Alt-click the line under `path`'s row, as a user would.
    void altClickBelowRowForTest(const QString& path);
    // Click the eye of a synthetic Smart Filters row (the group header or a
    // filter child); false when the path has no live row.
    bool clickSmartFilterEyeForTest(const QString& path);
    bool rowNameItalicForTest(const QString& path) const;
    bool rowNameUnderlineForTest(const QString& path) const;
    int rowHeightForTest() const;
    QImage rowImageForTest(const QString& path) const;
    QRect rowThumbRectForTest(const QString& path) const;
    QRect rowEyeRectForTest(const QString& path) const;
    QRect rowNameRectForTest(const QString& path) const;
    // The row's rect in viewport coordinates, resolved through the panel's own
    // proxy mapping (so a test clicks the same row the panel would toggle).
    QRect rowViewportRectForTest(const QString& path) const;

    // Follow-up cosmetics hooks: the centred eye gutter, its separator, and the
    // custom drop indicator kind after a synthesized hover (0 above, 1 below,
    // 2 centre). The kind is 0 none, 1 sibling line, 2 drop-into outline.
    bool eyeGutterCentredForTest(const QString& path) const;
    bool eyeSeparatorPresentForTest(const QString& path) const;
    int dropIndicatorForTest(const QString& source, const QString& hover, int where);
    void expandForTest(const QString& path);

    // Filter self-test hooks (lfs_*). Each builds a LayerFilter over the current
    // one, updates the bar, and applies it to the proxy.
    QStringList visiblePathsForTest() const;
    void setFilterNameForTest(const QString& name, bool enabled);
    void setFilterKindForTest(const QStringList& kinds, bool enabled);
    void setFilterModeForTest(const QString& key, bool enabled);
    void setFilterColorForTest(int label, bool enabled);
    void setFilterAttributeForTest(const QString& attr, bool enabled);
    int filterDimensionForTest() const;
    // Isolation-mode hook: release one row from the isolated set.
    void releaseIsolationForTest(const QString& path);
    void setFilterEnabledForTest(bool on);

    // Layer-style panel hooks: the fx badge projection, the row menu with a real
    // path, the Effect filter dimension, and an Alt-click on the fx region.
    bool rowHasStyleForTest(const QString& path) const;
    QRect rowFxRectForTest(const QString& path) const;
    void altClickRowFxForTest(const QString& path);
    bool rowHasBlendIfForTest(const QString& path) const;
    QRect rowBlendIfRectForTest(const QString& path) const;
    QStringList rowMenuTextsForPathForTest(const QString& path);
    bool rowMenuEnabledForPathForTest(const QString& path, const QString& text);
    bool performRowActionForTest(const QString& id, const QString& path);
    // The mask-thumbnail context menu: its item texts/enabled states, and a
    // direct run of one command by its action id (test seam).
    QStringList maskMenuTextsForPathForTest(const QString& path);
    bool maskMenuEnabledForPathForTest(const QString& path, const QString& text);
    bool performMaskActionForTest(const QString& id, const QString& path);
    void setFilterEffectForTest(const QString& effect, bool enabled);

    // The mask button (layers_panel_mask_menu.cpp).
    QString maskButtonToolTipForTest() const;
    bool maskButtonEnabledForTest() const;
    void clickMaskButtonForTest();
    bool deleteButtonEnabledForTest() const;
    void clickDeleteButtonForTest();
    // The texts of the menu a right-click on `path`'s vector-mask thumbnail opens.
    QStringList vectorMenuTextsForTest(const QString& path);
    // Drive the panel's modal dialogs (Refine Mask, Mask Options): the driver
    // edits the dialog and returns whether it was accepted. Empty restores
    // `runDialog`.
    void setDialogDriverForTest(std::function<bool(QDialog&)> driver);

    // Phase D: run a wired Layers per-widget menu entry by its action id.
    // Returns false for ids this panel does not own.
    bool performPanelMenuAction(const QString& actionId);

    // Open the New Layer / New Group / Group from Layers dialogs. The frame's
    // Layer > New menu and the panel's Alt-click route here; no-op without a view.
    void openNewLayerDialog();
    void openNewGroupDialog();
    void openGroupFromLayersDialog();

    /// Wrap the panel's selected layers in one group / splice selected groups
    /// into the parent. The Layer menu accelerator routes here too, so the
    /// keyboard path matches the panel and menu paths.
    void groupSelection();
    void ungroupSelection();
    // Duplicate the selected layers; Layer > Duplicate Layer routes here too.
    void duplicateSelection();
    // Delete the selected layers; Layer > Delete Layer routes here too.
    void deleteSelection();
    // Whether Move Layer Up/Down would change the current row (dry run).
    bool canMoveCurrent(int delta) const;

    // Whether a raster mask is the current mask edit target.
    bool maskEditing() const { return maskEditing_; }
    // Delete the current mask edit target's mask (no confirmation dialog). The
    // Delete key and the mask-thumbnail menu's Delete Layer Mask route here.
    void deleteActiveMask();

signals:
    void selectionChanged();
    // The edit target moved between the active layer's pixels and its mask.
    void editTargetChanged(bool mask);
    // A smart-object contents command chosen from a row's context menu, to be
    // run by the frame on `path`: `edit`, `replace`, or `export`.
    void smartObjectContentsRequested(const QString& action, const QString& path);
    // The rubylith overlay or its display options changed; the frame re-renders
    // the overlay.
    void maskDisplayChanged();

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    void syncControls();
    struct PendingPreview {
        QStringList paths;
        QStringList namePaths;
        bool fill = false;
        int value = 0;
        bool valid = false;
        quint64 generation = 0;
    };
    void schedulePreview(bool fill, int value);
    void applyPendingPreview();
    void clearPendingPreview();
    void cancelPendingPreview();
    void reconcileGestureTarget();
    QStringList propertyPaths() const;
    QStringList namePathsFor(const QStringList& paths) const;
    QStringList gestureTargets() const;
    void syncEditTarget();
    // End a mask solo view whose layer is no longer the current one.
    void syncMaskSolo();
    void selectLayer(int index);
    void selectPath(const QString& path);
    void showContextMenu(const QPoint& pos);
    void populateVectorMaskMenu(QMenu& menu, const QString& path);
    // The sole selected path when that layer has both masks: the trash then
    // deletes its vector mask. Empty otherwise.
    QString trashVectorMaskPath() const;
    void openRefineMask(const QString& path);
    void openMaskOptions();
    bool runPanelDialog(QDialog& dialog);
    // The action strip is disabled without a document; the mask and trash
    // buttons also follow the active layer and the selection.
    void syncStripButtons();
    void addMaskFromButton();
    // The trash: with both masks on the layer it deletes the vector mask after
    // a "Delete Vector Mask?" prompt; otherwise it deletes the selection.
    void deleteFromTrash();
    bool confirmDeleteVectorMask();
    void populateRowMenu(QMenu& menu, const QString& path, int color, const QString& kind,
                         bool shape = false, bool smart = false);
    bool performRowAction(const QString& id, const QString& path);
    void showEyeMenu(const QPoint& pos, const QModelIndex& index);
    void showMaskThumbMenu(const QPoint& pos, const QModelIndex& index);
    void populateMaskMenu(QMenu& menu, const QString& path, const QModelIndex& index);
    bool performMaskAction(const QString& id, const QString& path);
    void addColorLabelActions(QMenu* menu, int currentLabel);
    void openPanelOptions();
    void persistOptions();
    void openLayerStyle(const QString& path);
    void openBackgroundConversion(const QString& path);

    void applyFilter(const LayerFilter& filter);
    void releaseIsolationFor(const QString& path);
    void expandMatchingGroups();
    QModelIndex proxyIndexForPath(const QString& path) const;
    QString pathForProxyIndex(const QModelIndex& index) const;

    void addLayerAt(const QString& path);
    void addGroupAt(const QString& path);
    void moveCurrent(int delta);
    bool canMovePath(const QString& path, int delta) const;

    void toggleSolo(const QString& path);
    void restoreSolo();
    void clearSolo();
    QStringList soloPaths(const QString& path, const QHash<QString, bool>& snapshot) const;
    void setActiveThumb(const QString& path, int thumb);
    // Synthesize a left-click with `mods` on `path`'s mask thumbnail.
    bool clickMaskThumbWithModifiersForTest(const QString& path, Qt::KeyboardModifiers mods);

    PictureView* view_ = nullptr;
    LayersModel* model_ = nullptr;
    LayersFilterProxyModel* proxy_ = nullptr;
    LayerFilterBar* filterBar_ = nullptr;
    LayerRowDelegate* delegate_ = nullptr;
    LayersTreeView* tree_ = nullptr;
    QComboBox* blend_ = nullptr;
    PercentField* opacity_ = nullptr;
    PercentField* fill_ = nullptr;
    QToolButton* lockTransparency_ = nullptr;
    QToolButton* lockPixels_ = nullptr;
    QToolButton* lockPosition_ = nullptr;
    QToolButton* lockNesting_ = nullptr;
    QToolButton* lockAll_ = nullptr;
    QToolButton* maskButton_ = nullptr;
    QToolButton* deleteButton_ = nullptr;
    // The six actionable strip buttons. Link is excluded because it stays
    // disabled until its operation exists.
    QList<QToolButton*> stripButtons_;
    std::function<bool(QDialog&)> dialogDriver_;
    QSet<QString> expandedPaths_;
    // Panel-local active edit target per row path: 0 image, 1 mask, 2 vector.
    QHash<QString, int> activeThumbByPath_;
    // Whether the view's mask edit target was set at the last sync.
    bool maskEditing_ = false;
    // The isolated set captured when isolation was entered; the Selected filter
    // shows these paths until they are released.
    QSet<QString> isolatedPaths_;
    bool thumbEntireDocument_ = true;
    int thumbSizeIndex_ = 2;
    int thumbContents_ = 0;
    bool expandNewEffects_ = true;
    bool addCopyOnDuplicate_ = true;
    bool useDefaultMasksOnFill_ = true;
    bool soloActive_ = false;
    QString soloPath_;
    QHash<QString, bool> soloSnapshot_;
    bool syncing_ = false;
    PendingPreview pendingPreview_;
    PendingPreview gesture_;
    QTimer* previewTimer_ = nullptr;
    bool refreshing_ = false;
    QMetaObject::Connection viewConnection_;

    // Test seam: skip the modal Background-conversion dialog and use this result.
    bool bgConvertArmed_ = false;
    bool bgConvertAccept_ = false;
    QString bgConvertName_;
    int bgConvertColor_ = 0;
};

} // namespace pictura
