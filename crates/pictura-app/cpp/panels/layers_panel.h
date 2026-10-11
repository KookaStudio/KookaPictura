#pragma once

#include <QtCore/QHash>
#include <QtCore/QObject>
#include <QtCore/QPoint>
#include <QtCore/QSet>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QImage>
#include <QtWidgets/QWidget>

class QComboBox;
class QColor;
class QEvent;
class QImage;
class QMenu;
class QModelIndex;
class QRect;
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

    /// Replace the panel selection with `paths`, making `current` the current
    /// row (falling back to the first when `current` is not among them).
    void selectPaths(const QStringList& paths, const QString& current);

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

    // Batch 1 hooks: drag flags, the tinted eye gutter, the disabled edit
    // triggers, and a synthesized double-click at the name or the eye rect.
    bool layerDragFlagsForTest(const QString& path) const;
    QColor rowGutterColorForTest(const QString& path) const;
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
    bool clickLayerThumbnailForTest(const QString& path);
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
    QColor rowLabelChipColorForTest(const QString& path) const;
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
    void setFilterEffectForTest(const QString& effect, bool enabled);

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
    // Delete the selected layers; Layer > Delete Layer routes here too.
    void deleteSelection();

signals:
    void selectionChanged();

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    void syncControls();
    void selectLayer(int index);
    void selectPath(const QString& path);
    void showContextMenu(const QPoint& pos);
    void populateRowMenu(QMenu& menu, const QString& path, int color, const QString& kind,
                         bool shape = false, bool smart = false);
    bool performRowAction(const QString& id, const QString& path);
    void showEyeMenu(const QPoint& pos, const QModelIndex& index);
    void addColorLabelActions(QMenu* menu, int currentLabel);
    void openPanelOptions();
    void persistOptions();
    void openLayerStyle(const QString& path);
    void openBackgroundConversion(const QString& path);

    void applyFilter(const LayerFilter& filter);
    void expandMatchingGroups();
    QModelIndex proxyIndexForPath(const QString& path) const;
    QString pathForProxyIndex(const QModelIndex& index) const;

    void addLayerAt(const QString& path);
    void addGroupAt(const QString& path);
    void duplicateSelection();
    void moveCurrent(int delta);

    void toggleSolo(const QString& path);
    void restoreSolo();
    void clearSolo();
    QStringList soloPaths(const QString& path, const QHash<QString, bool>& snapshot) const;
    void setActiveThumb(const QString& path, int thumb);

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
    QSet<QString> expandedPaths_;
    // Panel-local active edit target per row path: 0 image, 1 mask, 2 vector.
    QHash<QString, int> activeThumbByPath_;
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
    QMetaObject::Connection viewConnection_;

    // Test seam: skip the modal Background-conversion dialog and use this result.
    bool bgConvertArmed_ = false;
    bool bgConvertAccept_ = false;
    QString bgConvertName_;
    int bgConvertColor_ = 0;
};

} // namespace pictura
