#pragma once

#include "layers_panel.h"

#include "blend_modes.h"

#include "icons.h"
#include "theme.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QAbstractItemModel>
#include <QtCore/QHash>
#include <QtCore/QModelIndex>
#include <QtCore/QRect>
#include <QtCore/QSize>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/QVariant>
#include <QtCore/QVector>
#include <QtGui/QColor>
#include <QtGui/QCursor>
#include <QtGui/QFont>
#include <QtGui/QIcon>
#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtGui/QPainter>
#include <QtGui/QPalette>
#include <QtGui/QPen>
#include <QtGui/QPixmap>
#include <QtGui/QRegion>
#include <QtWidgets/QApplication>
#include <QtWidgets/QStyle>
#include <QtCore/QMimeData>
#include <QtGui/QDrag>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDragLeaveEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QPaintEvent>
#include <QtWidgets/QStyledItemDelegate>
#include <QtWidgets/QStyleOptionViewItem>
#include <QtWidgets/QTreeView>

#include <array>
#include <functional>
#include <memory>
#include <vector>

namespace pictura {

/// MIME type carrying dragged layer paths between the tree and the strip buttons.
inline constexpr char kLayerMimeType[] = "application/x-pictura-layer";
/// MIME type tagging a layer drag with its document (the source PictureView's
/// address, compared but never dereferenced until a frame lookup vouches for it).
inline constexpr char kLayerSourceMimeType[] = "application/x-pictura-layer-source";

inline const void* layerDragSource(const QMimeData* mime)
{
    return mime ? reinterpret_cast<const void*>(static_cast<quintptr>(
                      mime->data(kLayerSourceMimeType).toULongLong()))
                : nullptr;
}

// A layer drag's payload: the dragged panel `paths` tagged with their document.
inline QMimeData* makeLayerDragMime(const void* source, const QStringList& paths)
{
    auto* mime = new QMimeData();
    mime->setData(kLayerMimeType, paths.join(QLatin1Char('\n')).toUtf8());
    mime->setData(kLayerSourceMimeType, QByteArray::number(reinterpret_cast<quintptr>(source)));
    return mime;
}

// One bridge row, as read by refresh(). The model owns a tree of these.
struct LayerRow {
    QString path;
    int depth = 0;
    QString name;
    QString kind;
    bool visible = true;
    QString blend;
    int opacity = 255;
    int fill = 255;
    int lockBits = 0;
    int color = 0;
    bool clipping = false;
    bool hasMask = false;
    bool maskLinked = false;
    bool maskDisabled = false;
    /// The raster mask is the view's mask edit target (the active thumbnail).
    bool maskTarget = false;
    bool hasVectorMask = false;
    bool vectorMaskLinked = false;
    bool vectorMaskDisabled = false;
    bool hasAdjustment = false;
    bool expandable = false;
    int childCount = 0;
    bool linked = false;
    bool placed = false;
    bool shape = false;
    bool smartObject = false;
    bool hasStyle = false;
    QStringList styleEffects;
    bool hasBlendIf = false;
    QImage thumbnail;
    QImage maskThumbnail;
    QImage vectorMaskThumbnail;
    int documentWidth = 0;
    int documentHeight = 0;
    /// A display-only row (the Smart Filters group and its children) with no
    /// real layer behind its path: not editable, draggable, or a drop target.
    bool synthetic = false;
    /// The active edit target within the row: 0 image, 1 raster mask, 2 vector
    /// mask. Panel-local; only the delegate's brackets read it.
    int activeThumb = 0;
};

QString layerTooltip(const LayerRow& layer);

// CS6 sheet color for a label index (1..7); an invalid color for 0/unknown.
QColor layerLabelColor(int label);

QPixmap labelSwatch(int label);

// How close (px) to the line between two rows an Alt-click must land to clip
// or release the upper layer.
constexpr int kClipLineGrab = 4;

// Panel Options thumbnail sizes by enum order (None/Small/Medium/Large).
constexpr std::array<int, 4> kThumbSizePx{0, 16, 24, 32};

// Frozen per-row roles (see `docs/dev/m39-panel-anatomy.md` §3.6). ClipBaseRole
// is panel-local: a row whose sibling displayed immediately above it is clipped
// underlines its name as the clipping base.
enum LayerRole {
    PathRole = Qt::UserRole + 1,
    DepthRole,
    KindRole,
    VisibleRole,
    BlendRole,
    OpacityRole,
    FillRole,
    LockRole,
    ColorRole,
    ClippingRole,
    ClipBaseRole,
    HasMaskRole,
    HasAdjustmentRole,
    ExpandableRole,
    ChildCountRole,
    ThumbnailRole,
    MaskThumbnailRole,
    LayerRowLinkedRole,
    LayerRowPlacedRole,
    DocumentWidthRole,
    DocumentHeightRole,
    LayerRowShapeRole,
    SyntheticRole,
    MaskDisabledRole,
    MaskLinkedRole,
    MaskTargetRole,
    SmartObjectRole,
    HasVectorMaskRole,
    VectorMaskThumbnailRole,
    VectorMaskLinkedRole,
    VectorMaskDisabledRole,
    HasLayerStyleRole,
    StyleEffectsRole,
    HasBlendIfRole,
    ActiveThumbRole,
};

struct Node {
    LayerRow row;
    bool clipBase = false;
    Node* parent = nullptr;
    int rowInParent = 0;
    std::vector<std::unique_ptr<Node>> children;
};

class LayersModel : public QAbstractItemModel {
public:
    explicit LayersModel(QObject* parent = nullptr)
        : QAbstractItemModel(parent)
        , root_(std::make_unique<Node>())
    {
    }

    void setView(PictureView* view) { view_ = view; }

    void setRows(QVector<LayerRow> rows)
    {
        beginResetModel();
        root_ = std::make_unique<Node>();
        byPath_.clear();

        // Two-pass: materialize every node first, then attach by parent path.
        // A forward reference (child before parent) or a partial list therefore
        // cannot silently drop the row to `root_` mid-build, and well-formed
        // pre-order input still attaches in encounter order.
        std::vector<std::unique_ptr<Node>> nodes;
        nodes.reserve(static_cast<size_t>(rows.size()));
        for (LayerRow& row : rows) {
            auto node = std::make_unique<Node>();
            node->row = std::move(row);
            byPath_.insert(node->row.path, node.get());
            nodes.push_back(std::move(node));
        }
        for (auto& node : nodes) {
            const int slash = node->row.path.lastIndexOf(QLatin1Char('/'));
            Node* parent = root_.get();
            if (slash >= 0) {
                if (Node* found = byPath_.value(node->row.path.left(slash), nullptr)) {
                    parent = found;
                }
            }
            node->parent = parent;
            node->rowInParent = static_cast<int>(parent->children.size());
            parent->children.push_back(std::move(node));
        }
        for (Node* node : byPath_) {
            const Node* parent = node->parent;
            if (parent && node->rowInParent > 0
                && parent->children[node->rowInParent - 1]->row.clipping) {
                node->clipBase = true;
            }
        }
        endResetModel();
    }

    QModelIndex index(int row, int column,
                      const QModelIndex& parent = QModelIndex()) const override
    {
        if (!hasIndex(row, column, parent)) {
            return {};
        }
        Node* node = parentNode(parent);
        return createIndex(row, column, node->children[row].get());
    }

    QModelIndex parent(const QModelIndex& child) const override
    {
        if (!child.isValid()) {
            return {};
        }
        Node* node = static_cast<Node*>(child.internalPointer());
        if (!node || !node->parent || node->parent == root_.get()) {
            return {};
        }
        return createIndex(node->parent->rowInParent, 0, node->parent);
    }

    int rowCount(const QModelIndex& parent = QModelIndex()) const override
    {
        if (parent.column() > 0) {
            return 0;
        }
        const Node* node = parentNode(parent);
        return node ? static_cast<int>(node->children.size()) : 0;
    }

    int columnCount(const QModelIndex& parent = QModelIndex()) const override
    {
        return parent.column() > 0 ? 0 : 1;
    }

    QVariant data(const QModelIndex& index, int role) const override
    {
        const Node* node = nodeFor(index);
        if (!node) {
            return {};
        }
        const LayerRow& row = node->row;
        switch (role) {
        case Qt::DisplayRole:
        case Qt::EditRole:
            return row.name;
        case Qt::ToolTipRole:
            return layerTooltip(row);
        case PathRole:
            return row.path;
        case DepthRole:
            return row.depth;
        case KindRole:
            return row.kind;
        case VisibleRole:
            return row.visible;
        case BlendRole:
            return row.blend;
        case OpacityRole:
            return row.opacity;
        case FillRole:
            return row.fill;
        case LockRole:
            return row.lockBits;
        case ColorRole:
            return row.color;
        case ClippingRole:
            return row.clipping;
        case ClipBaseRole:
            return node->clipBase;
        case HasMaskRole:
            return row.hasMask;
        case HasAdjustmentRole:
            return row.hasAdjustment;
        case ExpandableRole:
            return row.expandable;
        case ChildCountRole:
            return row.childCount;
        case ThumbnailRole:
            return row.thumbnail;
        case MaskThumbnailRole:
            return row.maskThumbnail;
        case LayerRowLinkedRole:
            return row.linked;
        case LayerRowPlacedRole:
            return row.placed;
        case LayerRowShapeRole:
            return row.shape;
        case DocumentWidthRole:
            return row.documentWidth;
        case DocumentHeightRole:
            return row.documentHeight;
        case SyntheticRole:
            return row.synthetic;
        case MaskDisabledRole:
            return row.maskDisabled;
        case MaskLinkedRole:
            return row.maskLinked;
        case MaskTargetRole:
            return row.maskTarget;
        case SmartObjectRole:
            return row.smartObject;
        case HasVectorMaskRole:
            return row.hasVectorMask;
        case VectorMaskThumbnailRole:
            return row.vectorMaskThumbnail;
        case VectorMaskLinkedRole:
            return row.vectorMaskLinked;
        case VectorMaskDisabledRole:
            return row.vectorMaskDisabled;
        case HasLayerStyleRole:
            return row.hasStyle;
        case StyleEffectsRole:
            return row.styleEffects;
        case HasBlendIfRole:
            return row.hasBlendIf;
        case ActiveThumbRole:
            return row.activeThumb;
        default:
            return {};
        }
    }

    Qt::ItemFlags flags(const QModelIndex& index) const override
    {
        if (!index.isValid()) {
            // The invalid parent is the top-level drop surface; without the
            // flag Qt computes no AboveItem/BelowItem indicator for root rows.
            return Qt::ItemIsDropEnabled;
        }
        const Node* node = static_cast<const Node*>(index.internalPointer());
        if (node && node->row.synthetic) {
            // A synthetic Smart Filters row selects and expands but neither
            // renames, drags, nor accepts a drop.
            return Qt::ItemIsEnabled | Qt::ItemIsSelectable;
        }
        return Qt::ItemIsEnabled | Qt::ItemIsSelectable | Qt::ItemIsEditable
            | Qt::ItemIsDragEnabled | Qt::ItemIsDropEnabled;
    }

    // Drag-and-drop capability virtuals. Without these the view's private
    // `canDrop()` is false, so `QTreeView::dragMoveEvent` never computes a real
    // `dropIndicatorPosition` and every drop reads the stale position.
    QStringList mimeTypes() const override
    {
        return {QString::fromLatin1(kLayerMimeType)};
    }

    Qt::DropActions supportedDropActions() const override { return Qt::MoveAction; }

    bool canDropMimeData(const QMimeData*, Qt::DropAction, int, int,
                         const QModelIndex&) const override
    {
        return true;
    }

    bool setData(const QModelIndex& index, const QVariant& value, int role) override
    {
        Node* node = index.isValid() ? static_cast<Node*>(index.internalPointer()) : nullptr;
        if (!node || !view_) {
            return false;
        }
        if (role == Qt::EditRole) {
            if (node->row.synthetic) {
                return false;
            }
            const QString name = value.toString().trimmed();
            if (name.isEmpty() || name == node->row.name) {
                return false;
            }
            return view_->set_layer_name_path(node->row.path, name);
        }
        return false;
    }

    /// Patch one row's visibility in place and notify, so a rapid second toggle
    /// reads the new value instead of a stale one from a pending model reset.
    void setVisible(const QString& path, bool visible)
    {
        Node* node = byPath_.value(path, nullptr);
        if (!node || node->row.visible == visible) {
            return;
        }
        node->row.visible = visible;
        const QModelIndex idx = indexForPath(path);
        if (idx.isValid()) {
            emit dataChanged(idx, idx, {VisibleRole});
        }
    }

    /// The live visibility of the layer at `path`, read from the document rather
    /// than `row.visible`, which a pending deferred refresh can leave stale.
    bool visibleInDocument(const QString& path) const
    {
        if (!view_) {
            return false;
        }
        for (int i = 0; i < view_->layer_row_count(); ++i) {
            if (view_->layer_row_path(i) == path) {
                return view_->layer_row_visible(i);
            }
        }
        return false;
    }

    /// Patch one row's active-thumb target in place and notify, so the
    /// delegate redraws the brackets without a model reset.
    void setActiveThumb(const QString& path, int thumb)
    {
        Node* node = byPath_.value(path, nullptr);
        if (!node || node->row.activeThumb == thumb) {
            return;
        }
        node->row.activeThumb = thumb;
        const QModelIndex idx = indexForPath(path);
        if (idx.isValid()) {
            emit dataChanged(idx, idx, {ActiveThumbRole});
        }
    }

    QVariant headerData(int section, Qt::Orientation orientation, int role) const override
    {
        if (orientation != Qt::Horizontal || role != Qt::DisplayRole) {
            return {};
        }
        return section == 0 ? QStringLiteral("Name") : QVariant();
    }

    QString pathForIndex(const QModelIndex& index) const
    {
        const Node* node = nodeFor(index);
        return node ? node->row.path : QString();
    }

    QModelIndex indexForPath(const QString& path) const
    {
        Node* node = byPath_.value(path, nullptr);
        if (!node || node == root_.get()) {
            return {};
        }
        return createIndex(node->rowInParent, 0, node);
    }

    /// Every node's path and visibility, for the solo snapshot.
    QHash<QString, bool> visibilityByPath() const
    {
        QHash<QString, bool> result;
        for (auto it = byPath_.cbegin(); it != byPath_.cend(); ++it) {
            result.insert(it.key(), it.value()->row.visible);
        }
        return result;
    }

    QStringList paths() const { return byPath_.keys(); }

private:
    Node* parentNode(const QModelIndex& parent) const
    {
        return parent.isValid() ? static_cast<Node*>(parent.internalPointer()) : root_.get();
    }

    const Node* nodeFor(const QModelIndex& index) const
    {
        if (!index.isValid()) {
            return nullptr;
        }
        return static_cast<const Node*>(index.internalPointer());
    }

    PictureView* view_ = nullptr;
    std::unique_ptr<Node> root_;
    QHash<QString, Node*> byPath_;
};

// A QTreeView that draws no branch indicators (the row delegate owns the
// nesting indentation and the disclosure icon, so the eye stays anchored at the
// panel's left edge) and that owns a self-contained layer drag/drop gesture: a
// drag carries the current row's path and a drop resolves the row under the
// cursor to a target path plus a mode (0 above, 1 below, 2 into).
class LayersTreeView : public QTreeView {
public:
    explicit LayersTreeView(QWidget* parent = nullptr)
        : QTreeView(parent)
    {
        setDragEnabled(true);
        setAcceptDrops(true);
        // The view draws the CS6 drop indicator itself; Qt's stock primitive is
        // off so only the custom line/outline shows.
        setDropIndicatorShown(false);
        setDragDropMode(QAbstractItemView::DragDrop);
        setDefaultDropAction(Qt::MoveAction);
    }

    void setDragPathsProvider(std::function<QStringList()> provider)
    {
        dragPaths_ = std::move(provider);
    }
    void setPathResolver(std::function<QString(const QModelIndex&)> resolver)
    {
        pathForIndex_ = std::move(resolver);
    }
    void setDropHandler(std::function<bool(const QString&, const QString&, int)> handler)
    {
        dropHandler_ = std::move(handler);
    }
    // Dry-run predicate consulted before a drop is highlighted or committed.
    // Returns true when the (dragged, target, mode) move is acceptable.
    void setDropValidator(std::function<bool(const QString&, const QString&, int)> validator)
    {
        dropValidator_ = std::move(validator);
    }
    // The document a drag starts from; a layer drag from another document is
    // not a drop on this tree (the frame copies it across instead).
    void setDragSource(const void* source) { dragSource_ = source; }
    bool isOwnLayerDrag(const QMimeData* mime) const
    {
        return mime->hasFormat(kLayerMimeType) && layerDragSource(mime) == dragSource_;
    }

    // Closed-hand cursor for the duration of a drag, restored when it ends.
    void enterDragCursor()
    {
        dragCursor_ = viewport()->cursor();
        viewport()->setCursor(Qt::ClosedHandCursor);
    }
    void leaveDragCursor() { viewport()->setCursor(dragCursor_); }
    int cursorShapeForTest() const { return static_cast<int>(viewport()->cursor().shape()); }

    // Self-test hooks: state() is protected on QAbstractItemView, and a model
    // reset can leave the view in EditingState with no live editor, which wedges
    // the next edit.
    int editStateForTest() const { return static_cast<int>(state()); }
    void resetEditStateForTest() { setState(NoState); }
    int dropIndicatorForTest() const { return static_cast<int>(dropIndicatorPosition()); }
    /// Whether the custom CS6 indicator currently has a valid target.
    bool dropIndicatorShownForTest() const { return dropMode_ >= 0; }
    /// 0 = none, 1 = sibling line, 2 = drop-into outline.
    int dropIndicatorKindForTest() const
    {
        if (dropMode_ < 0 || dropRect_.isEmpty()) {
            return 0;
        }
        return dropMode_ == 2 ? 2 : 1;
    }
    int dropModeAtForTest(const QPoint& pos) const
    {
        int mode = 0;
        dropTargetFor(pos, &mode);
        return mode;
    }

protected:
    void drawBranches(QPainter*, const QRect&, const QModelIndex&) const override {}

    // The CS6 drop indicator: a thin blue line at a sibling edge, or a thin
    // blue outline around a group row for a drop-into. Drawn over the base
    // paint so Qt's own primitive (disabled in the ctor) never competes.
    void paintEvent(QPaintEvent* event) override
    {
        QTreeView::paintEvent(event);
        if (dropMode_ < 0 || dropRect_.isEmpty()) {
            return;
        }
        QPainter painter(viewport());
        painter.setPen(QPen(QColor(0x33, 0x99, 0xDD), 1));
        painter.setBrush(Qt::NoBrush);
        if (dropMode_ == 2) {
            painter.drawRect(dropRect_.adjusted(0, 0, -1, -1));
        } else {
            const int y = dropMode_ == 0 ? dropRect_.top() : dropRect_.bottom();
            painter.drawLine(dropRect_.left(), y, dropRect_.right(), y);
        }
    }

    void startDrag(Qt::DropActions) override
    {
        if (!dragPaths_) {
            return;
        }
        const QStringList paths = dragPaths_();
        if (paths.isEmpty()) {
            return;
        }
        auto* drag = new QDrag(this);
        drag->setMimeData(makeLayerDragMime(dragSource_, paths));
        enterDragCursor();
        // Copy is offered for a drop on another document's tab or canvas.
        drag->exec(Qt::MoveAction | Qt::CopyAction, Qt::MoveAction);
        leaveDragCursor();
    }

    void dragEnterEvent(QDragEnterEvent* event) override
    {
        if (event->mimeData()->hasFormat(kLayerMimeType) && !isOwnLayerDrag(event->mimeData())) {
            event->ignore();
            return;
        }
        if (event->mimeData()->hasFormat(kLayerMimeType)) {
            // Enter the dragging state so the drop indicator paints (the base
            // implementation would do this, but we handle the drag ourselves).
            setState(QAbstractItemView::DraggingState);
            event->acceptProposedAction();
        } else {
            QTreeView::dragEnterEvent(event);
        }
    }

    void dragLeaveEvent(QDragLeaveEvent* event) override
    {
        clearDropIndicator();
        QTreeView::dragLeaveEvent(event);
    }

    void dragMoveEvent(QDragMoveEvent* event) override
    {
        if (event->mimeData()->hasFormat(kLayerMimeType) && !isOwnLayerDrag(event->mimeData())) {
            clearDropIndicator();
            event->ignore();
            return;
        }
        if (!event->mimeData()->hasFormat(kLayerMimeType) || !dropValidator_ || !pathForIndex_) {
            QTreeView::dragMoveEvent(event);
            return;
        }
        // Let Qt resolve the row under the cursor and its drop position first;
        // then veto invalid targets so the indicator only marks a legal drop.
        QTreeView::dragMoveEvent(event);
        int mode = 0;
        const QPoint pos = event->position().toPoint();
        const QString target = dropTargetFor(pos, &mode);
        const QStringList dragged =
            QString::fromUtf8(event->mimeData()->data(kLayerMimeType))
                .split(QLatin1Char('\n'), Qt::SkipEmptyParts);
        if (!dragged.isEmpty() && dropValidator_(dragged.first(), target, mode)) {
            const QModelIndex index = indexAt(pos);
            dropRect_ = index.isValid() ? visualRect(index)
                                        : QRect(0, pos.y(), viewport()->width(), 1);
            dropMode_ = mode;
            viewport()->update();
            event->acceptProposedAction();
        } else {
            clearDropIndicator();
            event->ignore();
        }
    }

    void dropEvent(QDropEvent* event) override
    {
        clearDropIndicator();
        if (event->mimeData()->hasFormat(kLayerMimeType) && !isOwnLayerDrag(event->mimeData())) {
            event->ignore();
            return;
        }
        if (!event->mimeData()->hasFormat(kLayerMimeType) || !dropHandler_ || !pathForIndex_) {
            QTreeView::dropEvent(event);
            return;
        }
        int mode = 0;
        const QString target = dropTargetFor(event->position().toPoint(), &mode);
        const QStringList dragged =
            QString::fromUtf8(event->mimeData()->data(kLayerMimeType))
                .split(QLatin1Char('\n'), Qt::SkipEmptyParts);
        // ponytail: only the current row is dragged; a multi-row drag would need
        // path re-sequencing as each move shifts the survivors.
        if (dragged.isEmpty() || (dropValidator_ && !dropValidator_(dragged.first(), target, mode))) {
            event->ignore();
            return;
        }
        dropHandler_(dragged.first(), target, mode);
        event->setDropAction(Qt::MoveAction);
        event->accept();
    }

private:
    // Resolve the drop row under `pos` to a target path plus mode (0 above,
    // 1 below, 2 into an item); an invalid index is the empty-root target.
    QString dropTargetFor(const QPoint& pos, int* mode) const
    {
        const QModelIndex index = indexAt(pos);
        *mode = dropPositionFor(pos, index);
        return index.isValid() ? pathForIndex_(index) : QString();
    }

    // The stock indicator is disabled, and Qt leaves `dropIndicatorPosition`
    // stale then, so the bands are resolved here. Only a group accepts a drop
    // into, so it keeps a centre band between quarter-height edges; any other
    // row splits at its midpoint so the whole row is a sibling target.
    int dropPositionFor(const QPoint& pos, const QModelIndex& index) const
    {
        if (!index.isValid()) {
            return 1;
        }
        const QRect rect = visualRect(index);
        const int offset = pos.y() - rect.top();
        if (index.data(KindRole).toString() != QLatin1String("group")) {
            return offset < rect.height() / 2 ? 0 : 1;
        }
        const int edge = qMax(2, rect.height() / 4);
        if (offset < edge) {
            return 0;
        }
        if (rect.bottom() - pos.y() < edge) {
            return 1;
        }
        return 2;
    }

    void clearDropIndicator()
    {
        if (dropMode_ < 0 && dropRect_.isNull()) {
            return;
        }
        dropMode_ = -1;
        dropRect_ = QRect();
        viewport()->update();
    }

    std::function<QStringList()> dragPaths_;
    std::function<QString(const QModelIndex&)> pathForIndex_;
    std::function<bool(const QString&, const QString&, int)> dropHandler_;
    std::function<bool(const QString&, const QString&, int)> dropValidator_;
    const void* dragSource_ = nullptr;
    QCursor dragCursor_;
    QRect dropRect_;
    int dropMode_ = -1;
};

} // namespace pictura
