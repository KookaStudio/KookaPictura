#pragma once

#include "layers_panel_model.h"

#include <QtGui/QFontMetrics>
#include <QtGui/QPainterPath>
#include <QtGui/QTransform>
#include <QtGui/qrgb.h>

namespace pictura {
class LayerRowDelegate : public QStyledItemDelegate {
public:
    explicit LayerRowDelegate(QObject* parent = nullptr)
        : QStyledItemDelegate(parent)
    {
    }

    int thumbnailSize() const { return thumbnailSize_; }
    void setThumbnailSize(int size) { thumbnailSize_ = qMax(0, size); }

    // Fixed eye gutter and per-level content indent, in row-local pixels.
    static constexpr int kEyeColumn = 30;
    static constexpr int kContentPad = 6;
    static constexpr int kIndent = 14;
    static constexpr int kChevronWidth = 16;
    // The gap between the thumbnail (or clip glyph) and the name text.
    static constexpr int kNameGap = 8;
    // The gap either side of a chain glyph between two thumbnails.
    static constexpr int kThumbLinkGap = 2;

    /// True when `pos` (row-local) lands in the non-selectable visibility
    /// gutter at the row's left edge.
    bool eyeColumnContains(const QRect& itemRect, const QPoint& pos) const
    {
        return pos.x() >= itemRect.left() && pos.x() < itemRect.left() + kEyeColumn;
    }

    /// The eye's hit-target inside a row's content rect (as painted): a square
    /// glyph centred in the fixed gutter with equal left/right padding.
    QRect eyeRect(const QRect& itemRect) const
    {
        const int width = qBound(12, itemRect.height(), 16);
        return QRect(itemRect.left() + (kEyeColumn - width) / 2, itemRect.top(), width,
                     itemRect.height());
    }

    /// The eye glyph's tint for `label`: a dark tint on bright fills and a
    /// light tint on dark fills, so the glyph stays legible.
    static QColor labelGlyphTint(const QColor& label)
    {
        if (!label.isValid()) {
            return {};
        }
        // ponytail: fixed 85 qGray split, retune if the label sheet changes.
        return qGray(label.rgb()) >= 85 ? QColor(20, 20, 20) : QColor(240, 240, 240);
    }

    /// Side of the (shrunken) lock badge: about two thirds of the thumbnail, so
    /// it reads as a badge rather than a second thumbnail.
    int lockBadgeSide() const
    {
        const int thumb = qMax(0, thumbnailSize_);
        return qMax(10, thumb > 0 ? (thumb * 2) / 3 : 11);
    }

    /// The lock badge's rect at a row's right edge (as painted).
    QRect lockRect(const QRect& itemRect) const
    {
        const int side = lockBadgeSide();
        return QRect(itemRect.right() - 3 - side,
                     itemRect.top() + (itemRect.height() - side) / 2, side, side);
    }

    /// The nominal thumbnail side (0 when thumbnails are off).
    int thumbSize() const { return qMax(0, thumbnailSize_); }

    /// The folder glyph's side for a group row: about three quarters of the row
    /// thumbnail, so a folder reads close to a layer thumbnail in size.
    int groupIconSide() const { return qMax(8, (thumbSize() * 3) / 4); }

    /// The vertical inset of the thumbnail slot inside the row.
    static constexpr int kThumbVerticalPad = 2;

    /// The thumbnail slot at `left`: `thumb` wide and the row's full height
    /// (minus a small vertical pad), so a tall document's thumbnail fills the
    /// row vertically rather than floating in a square box.
    QRect thumbSlot(const QRect& itemRect, int left) const
    {
        const int thumb = qMax(0, thumbnailSize_);
        const int pad = qMin(kThumbVerticalPad, qMax(0, (itemRect.height() - thumb) / 2));
        return QRect(left, itemRect.top() + pad, thumb, qMax(1, itemRect.height() - 2 * pad));
    }

    /// The square box left of the right-edge badges where the image thumbnail
    /// sits, mirroring paint()'s left-to-right content walk.
    QRect imageThumbBox(const QRect& itemRect, const QModelIndex& index) const
    {
        int x = contentLeft(itemRect, index);
        if (index.data(ClippingRole).toBool()) {
            const int thumb = thumbSize();
            const int side = qMax(10, thumb > 0 ? thumb - 8 : 12);
            if (!pictura::icon(QStringLiteral("layers.clipMask")).pixmap(side, side).isNull()) {
                x += side + 2;
            }
        }
        return thumbSlot(itemRect, x);
    }

    /// The x just past the image thumbnail's square slot.
    int imageSlotRight(const QRect& itemRect, const QModelIndex& index) const
    {
        return imageThumbBox(itemRect, index).right() + 1;
    }

    /// Whether a row paints the fx badge: a layer whose style carries at least
    /// one effect, but never a shape row (its own badge owns the thumbnail
    /// corner). A layer with no style — an adjustment or fill row in
    /// particular — and a layer whose effects are all gone draw no badge, so
    /// the badge and the row's Effects group always appear together.
    bool showsFx(const QModelIndex& index) const
    {
        return !index.data(LayerRowShapeRole).toBool()
            && index.data(HasLayerStyleRole).toBool()
            && !index.data(StyleEffectsRole).toStringList().isEmpty();
    }

    /// The fx badge's rect at a row's right edge, immediately left of the lock
    /// badge; empty when the row paints no fx badge.
    QRect fxRect(const QRect& itemRect, const QModelIndex& index) const
    {
        if (!showsFx(index)) {
            return {};
        }
        const int thumb = qMax(0, thumbnailSize_);
        const int badge = qMax(12, thumb > 0 ? thumb : 16);
        if (pictura::icon(QStringLiteral("layers.fx")).pixmap(badge, badge).isNull()) {
            return {};
        }
        int right = itemRect.right() - 3;
        if (index.data(LockBadgeRole).toInt() != 0
            && !pictura::icon(QStringLiteral("layers.lockAll"))
                    .pixmap(lockBadgeSide(), lockBadgeSide())
                    .isNull()) {
            right -= lockBadgeSide() + 3;
        }
        return QRect(right - badge, itemRect.top() + (itemRect.height() - badge) / 2, badge,
                     badge);
    }

    /// Whether a row is a member of a link set (and so paints the chain badge).
    bool showsLink(const QModelIndex& index) const
    {
        return index.data(LayerRowLinkedRole).toBool();
    }

    /// The linked-layer chain badge's side: the same as the fx/lock badge.
    int linkBadgeSide() const
    {
        const int thumb = qMax(0, thumbnailSize_);
        return qMax(12, thumb > 0 ? thumb : 16);
    }

    /// The horizontal space a row's link badge consumes at the right edge; zero
    /// when the row is unlinked or the asset is missing.
    int linkBadgeAdvance(const QModelIndex& index) const
    {
        if (!showsLink(index)) {
            return 0;
        }
        const int side = linkBadgeSide();
        return pictura::icon(QStringLiteral("layers.link2")).pixmap(side, side).isNull()
            ? 0
            : side + 3;
    }

    /// The link badge's rect at a row's right edge, immediately left of the fx
    /// badge; empty when the row is unlinked or the asset is missing.
    QRect linkBadgeRect(const QRect& itemRect, const QModelIndex& index) const
    {
        if (linkBadgeAdvance(index) == 0) {
            return {};
        }
        const int side = linkBadgeSide();
        const int thumb = qMax(0, thumbnailSize_);
        const int badge = qMax(12, thumb > 0 ? thumb : 16);
        int right = itemRect.right() - 3;
        if (index.data(LockBadgeRole).toInt() != 0
            && !pictura::icon(QStringLiteral("layers.lockAll"))
                    .pixmap(lockBadgeSide(), lockBadgeSide())
                    .isNull()) {
            right -= lockBadgeSide() + 3;
        }
        if (showsFx(index)
            && !pictura::icon(QStringLiteral("layers.fx")).pixmap(badge, badge).isNull()) {
            right -= badge + 3;
        }
        return QRect(right - side, itemRect.top() + (itemRect.height() - side) / 2, side, side);
    }

    /// The `Blend If` chip's font: the app font, a step smaller and bold, so the
    /// chip reads as a badge rather than a second name.
    static QFont blendIfFont()
    {
        QFont font = QApplication::font();
        font.setBold(true);
        if (font.pixelSize() > 0) {
            font.setPixelSize(qMax(1, font.pixelSize() - 3));
        } else if (font.pointSize() > 0) {
            font.setPointSize(qMax(1, font.pointSize() - 1));
        }
        return font;
    }

    /// The `Blend If` chip's width, from the font it draws with plus padding.
    static int blendIfChipWidth()
    {
        return QFontMetrics(blendIfFont()).horizontalAdvance(QStringLiteral("Blend If")) + 10;
    }

    /// The horizontal space a row's Blend If chip consumes at the right edge.
    int blendIfAdvance(const QModelIndex& index) const
    {
        return index.data(HasBlendIfRole).toBool() ? blendIfChipWidth() + 3 : 0;
    }

    /// The `Blend If` chip's rect at a row's right edge, immediately left of the
    /// fx/lock badges; empty when the row's Blend If is default or absent.
    QRect blendIfRect(const QRect& itemRect, const QModelIndex& index) const
    {
        if (!index.data(HasBlendIfRole).toBool()) {
            return {};
        }
        const int width = blendIfChipWidth();
        const int right = badgesRight(itemRect, index) + width + 3;
        const int height = qBound(12, itemRect.height() - 8, 18);
        return QRect(right - width, itemRect.top() + (itemRect.height() - height) / 2, width,
                     height);
    }

    /// The x where a row's content begins: the eye gutter plus the per-depth
    /// indent, plus the chevron slot only for expandable rows. Shared by
    /// thumbRect, nameRect, and paint() so the three never disagree.
    int contentLeft(const QRect& itemRect, const QModelIndex& index) const
    {
        const int depth = index.data(DepthRole).toInt();
        // A folder row uses half the content pad, so its name column starts
        // closer to the gutter; child-layer visibility controls are untouched.
        const int pad = index.data(KindRole).toString() == QLatin1String("group")
            ? kContentPad / 2
            : kContentPad;
        int x = itemRect.left() + kEyeColumn + pad + qMax(0, depth) * kIndent;
        if (index.data(ExpandableRole).toBool()) {
            x += kChevronWidth;
        }
        return x;
    }

    /// The expand/collapse chevron's hit-target for `index`, in the reserved
    /// slot just before the row content.
    QRect chevronRect(const QRect& itemRect, const QModelIndex& index) const
    {
        const int side = qMin(kChevronWidth, itemRect.height());
        int left = contentLeft(itemRect, index);
        if (index.data(ExpandableRole).toBool()) {
            left -= kChevronWidth;
        }
        return QRect(left, itemRect.top() + (itemRect.height() - side) / 2, side, side);
    }

    /// Letterbox a square thumbnail `box` to the document's aspect ratio, so a
    /// wide or tall document is not stretched. A group (folder glyph) or a
    /// zero-size document keeps the square box.
    QRect letterboxedThumb(const QRect& box, const QModelIndex& index) const
    {
        if (box.isEmpty() || index.data(KindRole).toString() == QLatin1String("group")) {
            return box;
        }
        const int documentWidth = index.data(DocumentWidthRole).toInt();
        const int documentHeight = index.data(DocumentHeightRole).toInt();
        if (documentWidth <= 0 || documentHeight <= 0) {
            return box;
        }
        int width = box.width();
        int height = box.height();
        if (static_cast<qint64>(documentWidth) * box.height()
            > static_cast<qint64>(documentHeight) * box.width()) {
            // The document is wider than the box: fit by width.
            height = qMax(1, qRound(static_cast<double>(box.width()) * documentHeight
                                    / documentWidth));
        } else {
            // The document is taller than the box: fit by height.
            width = qMax(1, qRound(static_cast<double>(box.height()) * documentWidth
                                   / documentHeight));
        }
        return QRect(box.left() + (box.width() - width) / 2,
                     box.top() + (box.height() - height) / 2, width, height);
    }

    /// The image thumbnail's drawn card rect: a type layer keeps a square (1:1)
    /// "T" card regardless of the document's aspect, every other non-group
    /// layer is letterboxed to the document. A group keeps its square box.
    QRect thumbnailCard(const QRect& box, const QModelIndex& index) const
    {
        if (!box.isEmpty() && index.data(KindRole).toString() == QLatin1String("type")) {
            const int side = qMin(box.width(), box.height());
            return QRect(box.left() + (box.width() - side) / 2,
                         box.top() + (box.height() - side) / 2, side, side);
        }
        return letterboxedThumb(box, index);
    }

    /// The thumbnail's hit-target, mirroring the x/y math paint() lays out, as
    /// the drawn card (document-aspect for a regular layer, square for a type
    /// card). Empty when thumbnails are off.
    QRect thumbRect(const QRect& itemRect, const QModelIndex& index) const
    {
        if (thumbSize() <= 0) {
            return {};
        }
        return thumbnailCard(imageThumbBox(itemRect, index), index);
    }

    /// The right edge left for the mask thumbnail after the lock and fx badges,
    /// mirroring paint()'s walk so the mask and link rects agree with it.
    int badgesRight(const QRect& itemRect, const QModelIndex& index) const
    {
        const int thumb = qMax(0, thumbnailSize_);
        const int badge = qMax(12, thumb > 0 ? thumb : 16);
        int right = itemRect.right() - 3;
        if (index.data(LockBadgeRole).toInt() != 0
            && !pictura::icon(QStringLiteral("layers.lockAll"))
                    .pixmap(lockBadgeSide(), lockBadgeSide())
                    .isNull()) {
            right -= lockBadgeSide() + 3;
        }
        if (showsFx(index)
            && !pictura::icon(QStringLiteral("layers.fx")).pixmap(badge, badge).isNull()) {
            right -= badge + 3;
        }
        right -= linkBadgeAdvance(index);
        right -= blendIfAdvance(index);
        return right;
    }

    /// Side of the link/chain glyph slot between two thumbnails.
    static int linkSide(int thumb) { return qMax(8, thumb / 2); }

    /// The link/chain glyph's hit-target immediately right of the image
    /// thumbnail (the mask link); empty when thumbnails are off or the row
    /// carries no raster mask.
    QRect linkGlyphRect(const QRect& itemRect, const QModelIndex& index) const
    {
        const int thumb = thumbSize();
        if (thumb <= 0 || index.data(MaskThumbnailRole).value<QImage>().isNull()) {
            return {};
        }
        const int side = linkSide(thumb);
        const int left = imageSlotRight(itemRect, index) + kThumbLinkGap;
        return QRect(left, itemRect.top() + (itemRect.height() - side) / 2, side, side);
    }

    /// The mask thumbnail's hit-target, immediately right of the mask link;
    /// empty when thumbnails are off or the row carries no raster mask.
    QRect maskThumbRect(const QRect& itemRect, const QModelIndex& index) const
    {
        const int thumb = thumbSize();
        if (thumb <= 0 || index.data(MaskThumbnailRole).value<QImage>().isNull()) {
            return {};
        }
        const int side = linkSide(thumb);
        const int left = imageSlotRight(itemRect, index) + kThumbLinkGap + side + kThumbLinkGap;
        return letterboxedThumb(thumbSlot(itemRect, left), index);
    }

    /// The x just past the mask thumbnail's square slot (or the image slot when
    /// the row carries no raster mask).
    int maskSlotRight(const QRect& itemRect, const QModelIndex& index) const
    {
        const int thumb = thumbSize();
        if (thumb <= 0 || index.data(MaskThumbnailRole).value<QImage>().isNull()) {
            return imageSlotRight(itemRect, index);
        }
        return imageSlotRight(itemRect, index) + kThumbLinkGap + linkSide(thumb) + kThumbLinkGap
            + thumb;
    }

    /// The vector link/chain glyph's hit-target immediately right of the mask
    /// thumbnail (or the image thumbnail without a raster mask); empty without a
    /// vector mask.
    QRect vectorLinkGlyphRect(const QRect& itemRect, const QModelIndex& index) const
    {
        const int thumb = thumbSize();
        if (thumb <= 0 || index.data(VectorMaskThumbnailRole).value<QImage>().isNull()) {
            return {};
        }
        const int side = linkSide(thumb);
        const int left = maskSlotRight(itemRect, index) + kThumbLinkGap;
        return QRect(left, itemRect.top() + (itemRect.height() - side) / 2, side, side);
    }

    /// The vector-mask thumbnail's hit-target, immediately right of the vector
    /// link; empty when thumbnails are off or the row carries no vector mask.
    QRect vectorMaskThumbRect(const QRect& itemRect, const QModelIndex& index) const
    {
        const int thumb = thumbSize();
        if (thumb <= 0 || index.data(VectorMaskThumbnailRole).value<QImage>().isNull()) {
            return {};
        }
        const int side = linkSide(thumb);
        const int left = maskSlotRight(itemRect, index) + kThumbLinkGap + side + kThumbLinkGap;
        return letterboxedThumb(thumbSlot(itemRect, left), index);
    }

    /// The x just past the last thumbnail slot (image, mask, or vector).
    int thumbsRight(const QRect& itemRect, const QModelIndex& index) const
    {
        const int thumb = thumbSize();
        if (thumb > 0 && !index.data(VectorMaskThumbnailRole).value<QImage>().isNull()) {
            return maskSlotRight(itemRect, index) + kThumbLinkGap + linkSide(thumb)
                + kThumbLinkGap + thumb;
        }
        return maskSlotRight(itemRect, index);
    }

    /// The name text's hit-target: immediately right of the thumbnail block
    /// (image, mask, vector) plus the name gap and clipping indent, capped at
    /// the right-edge lock/fx/Blend If badges. Floored to a non-zero width so a
    /// click in an empty label area still resolves to the name.
    QRect nameRect(const QRect& itemRect, const QModelIndex& index) const
    {
        const int thumb = thumbSize();
        int x = thumbsRight(itemRect, index) + kNameGap;
        int right = itemRect.right() - 3;
        const int badge = qMax(12, thumb > 0 ? thumb : 16);
        if (index.data(LockBadgeRole).toInt() != 0
            && !pictura::icon(QStringLiteral("layers.lockAll"))
                    .pixmap(lockBadgeSide(), lockBadgeSide())
                    .isNull()) {
            right -= lockBadgeSide() + 3;
        }
        if (showsFx(index)
            && !pictura::icon(QStringLiteral("layers.fx")).pixmap(badge, badge).isNull()) {
            right -= badge + 3;
        }
        right -= linkBadgeAdvance(index);
        right -= blendIfAdvance(index);
        const int nameLeft = x + (index.data(ClippingRole).toBool() ? 12 : 0);
        const int nameRight = qMax(nameLeft, right - 4);
        return QRect(nameLeft, itemRect.top(), qMax(1, nameRight - nameLeft), itemRect.height());
    }

    /// One named row height (floor 32 px) shared by sizeHint and centring; a
    /// Medium (24 px) thumbnail row lands at 36 px. A folder row is shorter.
    static constexpr int kRowHeightFloor = 32;
    static constexpr int kGroupRowShrink = 6;
    int rowHeight() const { return qMax(kRowHeightFloor, thumbnailSize_ + 12); }

    /// The row name's font: Background italic, linked/placed (and clip-base)
    /// underlined. Shared by paint and the self-test so one rule is asserted.
    QFont nameFont(const QFont& base, const QModelIndex& index) const
    {
        QFont font = base;
        font.setItalic(index.data(KindRole).toString() == QLatin1String("background"));
        font.setUnderline(index.data(ClipBaseRole).toBool()
                          || index.data(LayerRowLinkedRole).toBool()
                          || index.data(LayerRowPlacedRole).toBool());
        if (base.pixelSize() > 0) {
            font.setPixelSize(qMax(1, base.pixelSize() - 2));
        } else if (base.pointSize() > 0) {
            font.setPointSize(qMax(1, base.pointSize() - 2));
        }
        return font;
    }

    QSize sizeHint(const QStyleOptionViewItem& option, const QModelIndex& index) const override
    {
        Q_UNUSED(option);
        const int shrink =
            index.data(KindRole).toString() == QLatin1String("group") ? kGroupRowShrink : 0;
        return QSize(200, qMax(kRowHeightFloor - kGroupRowShrink, rowHeight() - shrink));
    }

    /// CS6 rename navigation. The view installs this delegate as the editor's
    /// event filter, whose base implementation commits `Tab`/`Shift+Tab` and
    /// closes the editor without moving. Handling the keys here lets the close
    /// hint move the editor to the adjacent visible row, with no wrap at the
    /// ends.
    bool eventFilter(QObject* object, QEvent* event) override
    {
        if (event->type() == QEvent::KeyPress) {
            auto* key = static_cast<QKeyEvent*>(event);
            if (key->key() == Qt::Key_Tab || key->key() == Qt::Key_Backtab) {
                const bool back = key->key() == Qt::Key_Backtab
                    || (key->modifiers() & Qt::ShiftModifier);
                auto* view = qobject_cast<QTreeView*>(parent());
                QModelIndex neighbour;
                if (view) {
                    const QModelIndex current = view->currentIndex();
                    neighbour = back ? view->indexAbove(current) : view->indexBelow(current);
                    if (neighbour.isValid()) {
                        view->setCurrentIndex(neighbour);
                    }
                }
                if (auto* editor = qobject_cast<QWidget*>(object)) {
                    emit commitData(editor);
                    emit closeEditor(editor, QAbstractItemDelegate::NoHint);
                }
                Q_UNUSED(neighbour);
                return true;
            }
        }
        return QStyledItemDelegate::eventFilter(object, event);
    }

    void paint(QPainter* painter, const QStyleOptionViewItem& option,
               const QModelIndex& index) const override
    {
        // Paint the surfaces ourselves rather than through the style: the list
        // is a flat base and every row sits one shade step lighter, so the rows
        // read as a distinct surface. A selected row takes the neutral
        // list-selection grey (the same value the stylesheet uses) instead of
        // the palette's blue Highlight, and the eye gutter is refilled with the
        // row surface so it matches an unselected row.
        const QRect rect = option.rect;
        const int height = qMax(1, rect.height());
        const bool selected = option.state & QStyle::State_Selected;
        // The item surface matches the widget surface; the gutter and a selected
        // row step off it so the row still reads. Never the palette blue.
        const QColor rowSurface = Theme::shade(option.palette.color(QPalette::Window), 1);
        const QColor selectionSurface = Theme::shade(option.palette.color(QPalette::Window), 3);
        const QColor gutterSurface = Theme::shade(option.palette.color(QPalette::Window), 0);
        painter->fillRect(rect, selected ? selectionSurface : rowSurface);
        const QRect eye = eyeRect(rect);
        // The visibility gutter never takes the selection highlight.
        painter->fillRect(QRect(rect.left(), rect.top(), kEyeColumn, height), gutterSurface);
        // A non-None color label fills the whole visibility column behind the
        // eye glyph. Paint it before the bottom rule so the rule stays visible.
        const QColor label = layerLabelColor(index.data(ColorRole).toInt());
        if (label.isValid()) {
            painter->fillRect(QRect(rect.left(), rect.top(), kEyeColumn, height), label);
        }
        // 1 px bottom rule per row.
        painter->fillRect(QRect(rect.left(), rect.bottom(), rect.width(), 1),
                          option.palette.color(QPalette::Base).darker(115));

        painter->save();
        const QPalette& palette = option.palette;
        // Eye icon, anchored at the panel's left edge for every depth.
        const QColor glyphTint = labelGlyphTint(label);
        paintAsset(painter, eye,
                   index.data(VisibleRole).toBool() ? QStringLiteral("layers.eyeOn")
                                                    : QStringLiteral("layers.eyeOff"),
                   glyphTint);
        // 1 px darker-grey separator at the gutter's right edge, between the
        // eye and the row content.
        const QColor separator = palette.color(QPalette::Base).darker(115);
        painter->fillRect(QRect(rect.left() + kEyeColumn - 1, rect.top(), 1, height), separator);

        // Content (disclosure, clipping glyph, thumbnail, name) starts after the
        // gutter; the chevron slot is reserved only for expandable rows.
        const int thumb = qMax(0, thumbnailSize_);
        int x = contentLeft(rect, index);
        QRect thumbBox;
        if (index.data(ExpandableRole).toBool()) {
            const auto* treeView = qobject_cast<const QTreeView*>(option.widget);
            const bool expanded = treeView && treeView->isExpanded(index);
            paintAsset(painter, chevronRect(rect, index),
                       expanded ? QStringLiteral("layers.disclosureDown")
                                : QStringLiteral("layers.disclosureRight"));
        }
        if (index.data(ClippingRole).toBool()) {
            const int side = qMax(10, thumb > 0 ? thumb - 8 : 12);
            const QPixmap clip =
                pictura::icon(QStringLiteral("layers.clipMask")).pixmap(side, side);
            if (!clip.isNull()) {
                painter->drawPixmap(
                    QRect(x, rect.top() + (height - side) / 2, side, side), clip);
                x += side + 2;
            }
        }
        if (thumb > 0) {
            const QRect box = thumbSlot(rect, x);
            const QRect shaped = thumbnailCard(box, index);
            thumbBox = shaped;
            if (index.data(KindRole).toString() == QLatin1String("group")) {
                // The folder glyph is about three quarters of the row thumbnail,
                // centred in its slot, so a folder reads close to a layer
                // thumbnail in size.
                const int gs = groupIconSide();
                const QPixmap glyph =
                    pictura::icon(QStringLiteral("layers.group")).pixmap(gs, gs);
                if (!glyph.isNull()) {
                    painter->drawPixmap(QRect(box.left() + (box.width() - gs) / 2,
                                              box.top() + (box.height() - gs) / 2, gs, gs),
                                        glyph);
                }
            } else if (index.data(KindRole).toString().startsWith(QLatin1String("smart-filter"))
                       || index.data(KindRole).toString().startsWith(QLatin1String("layer-effect"))) {
                // The Smart Filters and layer-effects group/rows show the fx
                // badge instead of a checkerboard (they have no raster pixels).
                const QPixmap glyph =
                    pictura::icon(QStringLiteral("layers.fx")).pixmap(thumb, thumb);
                if (!glyph.isNull()) {
                    painter->drawPixmap(box, glyph);
                }
            } else if (index.data(KindRole).toString() == QLatin1String("type")) {
                // CS6 shows a type layer as a T on white, not its pixels; the
                // stroke is `layers.kindType`'s glyph, dark for the white card.
                painter->fillRect(shaped, Qt::white);
                const QRectF t = QRectF(shaped).adjusted(thumb / 4.0, thumb / 4.0, -thumb / 4.0,
                                                        -thumb / 4.0);
                painter->save();
                painter->setRenderHint(QPainter::Antialiasing, true);
                painter->setPen(QPen(QColor(0x20, 0x20, 0x20), qMax(1.5, thumb / 12.0)));
                painter->drawLine(t.topLeft(), t.topRight());
                // A warped layer's T stands on a bent baseline, as CS6 marks
                // Warp Text.
                const bool warped = index.data(LayerRowWarpedRole).toBool();
                const double foot = warped ? t.top() + t.height() * 0.6 : t.bottom();
                painter->drawLine(QPointF(t.center().x(), t.top()),
                                  QPointF(t.center().x(), foot));
                if (warped) {
                    QPainterPath arc(t.bottomLeft());
                    arc.quadTo(QPointF(t.center().x(), t.top() + t.height() * 0.55),
                               t.bottomRight());
                    painter->setBrush(Qt::NoBrush);
                    painter->drawPath(arc);
                }
                painter->restore();
            } else if (index.data(HasAdjustmentRole).toBool()
                       && !index.data(AdjustmentKindRole).toString().isEmpty()
                       && index.data(ThumbnailRole).value<QImage>().isNull()) {
                // CS6 shows an adjustment layer as its type icon on a neutral
                // card, not a checkerboard with no pixels under it. A fill row
                // carries a content preview in ThumbnailRole, so it falls
                // through to the checkerboard+image branch below.
                painter->fillRect(shaped, palette.color(QPalette::Base));
                const QString kind = index.data(AdjustmentKindRole).toString();
                const QPixmap glyph =
                    pictura::icon(QStringLiteral("adjustments.") + kind).pixmap(thumb, thumb);
                if (!glyph.isNull()) {
                    painter->drawPixmap(box, glyph);
                }
            } else {
                painter->drawTiledPixmap(shaped, checkerTile());
                const QImage image = index.data(ThumbnailRole).value<QImage>();
                if (!image.isNull()) {
                    painter->drawImage(shaped, image);
                }
            }
            // 1 px black outline around the letterboxed thumbnail. A group's
            // folder glyph and a type layer's white "T" card keep no outline.
            const QString outlineKind = index.data(KindRole).toString();
            if (outlineKind != QLatin1String("group") && outlineKind != QLatin1String("type")
                && !outlineKind.startsWith(QLatin1String("layer-effect"))) {
                painter->setPen(QPen(Qt::black, 1));
                painter->setBrush(Qt::NoBrush);
                painter->drawRect(shaped.adjusted(0, 0, -1, -1));
            }
            // A shape layer's vector badge sits on the thumbnail's bottom-right
            // corner, as CS6 marks a shape layer.
            if (index.data(LayerRowShapeRole).toBool()) {
                const int side = qMax(8, thumb / 2);
                const QPixmap shapeBadge =
                    pictura::icon(QStringLiteral("layers.kindShape")).pixmap(side, side);
                if (!shapeBadge.isNull()) {
                    const QRect corner(box.right() - side + 3, box.bottom() - side + 3, side,
                                       side);
                    painter->fillRect(corner, palette.color(QPalette::Base));
                    painter->drawPixmap(corner, shapeBadge);
                }
            }
            // A smart-object layer shows the CS6 page badge at the same
            // lower-right corner (embedded or placed).
            if (index.data(SmartObjectRole).toBool()) {
                const int side = qMax(8, thumb / 2);
                const QPixmap soBadge =
                    pictura::icon(QStringLiteral("layers.kindSmartObject")).pixmap(side, side);
                if (!soBadge.isNull()) {
                    const QRect corner(box.right() - side + 3, box.bottom() - side + 3, side,
                                       side);
                    painter->fillRect(corner, palette.color(QPalette::Base));
                    painter->drawPixmap(corner, soBadge);
                }
            }
            // A type layer whose font is not installed carries a warning
            // triangle on the card's top-right corner, as CS6 flags it.
            if (index.data(LayerRowMissingFontRole).toBool()) {
                const double side = qMax(8, thumb / 2);
                const QRectF corner(box.right() - side + 2, box.top() - 1, side, side);
                QPainterPath triangle(QPointF(corner.center().x(), corner.top()));
                triangle.lineTo(corner.bottomRight());
                triangle.lineTo(corner.bottomLeft());
                triangle.closeSubpath();
                painter->save();
                painter->setRenderHint(QPainter::Antialiasing, true);
                painter->setPen(QPen(QColor(0x20, 0x20, 0x20), 1));
                painter->setBrush(QColor(0xf5, 0xc5, 0x18));
                painter->drawPath(triangle);
                const double x = corner.center().x();
                painter->setPen(QPen(QColor(0x20, 0x20, 0x20), qMax(1.0, side / 8.0)));
                painter->drawLine(QPointF(x, corner.top() + side * 0.35),
                                  QPointF(x, corner.top() + side * 0.65));
                painter->drawPoint(QPointF(x, corner.top() + side * 0.8));
                painter->restore();
            }
        }
        // Thumbnails run left-to-right from the image thumb: [image][chain][mask]
        // [chain][vector]. The chain glyph is always drawn (link-2 when the mask
        // is linked to the layer, unlink-2 otherwise).
        const QImage mask = index.data(MaskThumbnailRole).value<QImage>();
        const QRect imageRect = thumbBox;
        QRect maskRect;
        QRect vectorRect;
        if (thumb > 0 && !mask.isNull()) {
            const int side = linkSide(thumb) + 2;
            const QPixmap linkPix = linkPixmap(index.data(MaskLinkedRole).toBool(), side);
            const QRect link = linkGlyphRect(rect, index);
            if (!linkPix.isNull()) {
                painter->drawPixmap(QRect(link.center().x() - side / 2,
                                          link.center().y() - side / 2, side, side),
                                    linkPix);
            }
            maskRect = maskThumbRect(rect, index);
            painter->drawImage(maskRect, mask);
            if (index.data(MaskTargetRole).toBool()) {
                // The active mask thumbnail wears the CS6 focus border: a dark
                // outline outside a white line, legible over any coverage.
                painter->save();
                painter->setBrush(Qt::NoBrush);
                painter->setPen(QColor(0, 0, 0, 200));
                painter->drawRect(maskRect.adjusted(-1, -1, 0, 0));
                painter->setPen(Qt::white);
                painter->drawRect(maskRect.adjusted(0, 0, -1, -1));
                painter->restore();
            }
            if (index.data(MaskDisabledRole).toBool()) {
                paintDisabledCross(painter, maskRect, thumb);
            }
        }
        const QImage vectorMask = index.data(VectorMaskThumbnailRole).value<QImage>();
        if (thumb > 0 && !vectorMask.isNull()) {
            const int side = linkSide(thumb) + 2;
            const QPixmap linkPix = linkPixmap(index.data(VectorMaskLinkedRole).toBool(), side);
            const QRect link = vectorLinkGlyphRect(rect, index);
            if (!linkPix.isNull()) {
                painter->drawPixmap(QRect(link.center().x() - side / 2,
                                          link.center().y() - side / 2, side, side),
                                    linkPix);
            }
            vectorRect = vectorMaskThumbRect(rect, index);
            painter->drawImage(vectorRect, vectorMask);
            if (index.data(VectorMaskDisabledRole).toBool()) {
                paintDisabledCross(painter, vectorRect, thumb);
            }
        }

        // Right-aligned badges: the (shrunken) lock badge at the far edge, then
        // fx. A null icon is omitted.
        int right = rect.right() - 3;
        const QIcon fxIcon = pictura::icon(QStringLiteral("layers.fx"));
        if (index.data(LockBadgeRole).toInt() != 0) {
            const int lockSide = lockBadgeSide();
            const QPixmap lockPix =
                pictura::icon(QStringLiteral("layers.lockAll")).pixmap(lockSide, lockSide);
            if (!lockPix.isNull()) {
                painter->drawPixmap(
                    QRect(right - lockSide, rect.top() + (height - lockSide) / 2, lockSide,
                          lockSide),
                    lockPix);
                right -= lockSide + 3;
            }
        }
        const int badge = qMax(12, thumb > 0 ? thumb : 16);
        if (showsFx(index) && !fxIcon.isNull()) {
            const QPixmap badgePix = fxIcon.pixmap(badge, badge);
            if (!badgePix.isNull()) {
                painter->drawPixmap(
                    QRect(right - badge, rect.top() + (height - badge) / 2, badge, badge),
                    badgePix);
                right -= badge + 3;
            }
        }
        // A link-set member shows the chain glyph left of the fx badge, as CS6
        // marks a linked layer.
        if (linkBadgeAdvance(index) != 0) {
            const int linkSide = linkBadgeSide();
            const QPixmap linkPix =
                pictura::icon(QStringLiteral("layers.link2")).pixmap(linkSide, linkSide);
            if (!linkPix.isNull()) {
                painter->drawPixmap(
                    QRect(right - linkSide, rect.top() + (height - linkSide) / 2, linkSide,
                          linkSide),
                    linkPix);
                right -= linkSide + 3;
            }
        }
        // A customised Blend If shows a text chip, left of the fx/lock badges.
        if (index.data(HasBlendIfRole).toBool()) {
            const QRect chip = blendIfRect(rect, index);
            painter->save();
            painter->setRenderHint(QPainter::Antialiasing, true);
            painter->setPen(QPen(palette.color(QPalette::Mid), 1));
            painter->setBrush(Theme::shade(palette.color(QPalette::Window), 2));
            painter->drawRoundedRect(chip.adjusted(0, 0, -1, -1), 3, 3);
            painter->setFont(blendIfFont());
            painter->setPen(selected ? palette.color(QPalette::HighlightedText)
                                     : palette.color(QPalette::Text));
            painter->drawText(chip, Qt::AlignCenter, QStringLiteral("Blend If"));
            painter->restore();
            right -= blendIfChipWidth() + 3;
        }

        // Name, after the thumbnail block, with the base underline and the
        // extra clipping indent.
        const int nameX = thumbsRight(rect, index) + kNameGap;
        const int nameLeft = nameX + (index.data(ClippingRole).toBool() ? 12 : 0);
        const int nameRight = qMax(nameLeft, right - 4);
        painter->setFont(nameFont(option.font, index));
        painter->setPen(selected ? palette.color(QPalette::HighlightedText)
                                 : palette.color(QPalette::Text));
        const QString elided =
            painter->fontMetrics().elidedText(index.data(Qt::DisplayRole).toString(),
                                              Qt::ElideRight, nameRight - nameLeft);
        painter->drawText(QRect(nameLeft, rect.top(), nameRight - nameLeft, height),
                          Qt::AlignVCenter | Qt::AlignLeft, elided);

        // White 1 px corner brackets one pixel outside the outline, on the
        // active edit target's thumbnail only; never around a group's icon.
        const int active = index.data(ActiveThumbRole).toInt();
        if (thumb > 0 && index.data(KindRole).toString() != QLatin1String("group")
            && singularActive(selected, option.widget, index)) {
            QRect target = imageRect;
            if (active == 2 && !vectorRect.isEmpty()) {
                target = vectorRect;
            } else if (active == 1 && !maskRect.isEmpty()) {
                target = maskRect;
            }
            paintBrackets(painter, target.adjusted(-1, -1, 1, 1));
        }
        painter->restore();
    }

private:
    static bool singularActive(bool selected, const QWidget* widget, const QModelIndex& index)
    {
        if (!selected) {
            return false;
        }
        const auto* treeView = qobject_cast<const QTreeView*>(widget);
        return treeView && treeView->selectionModel()
            && treeView->selectionModel()->selectedRows(0).size() == 1;
    }

    static void paintBrackets(QPainter* painter, const QRect& r)
    {
        painter->setPen(QPen(Qt::white, 1));
        const int len = qMax(2, qMin(r.width(), r.height()) / 3);
        painter->drawLine(r.left(), r.top(), r.left() + len, r.top());
        painter->drawLine(r.left(), r.top(), r.left(), r.top() + len);
        painter->drawLine(r.right() - len, r.top(), r.right(), r.top());
        painter->drawLine(r.right(), r.top(), r.right(), r.top() + len);
        painter->drawLine(r.left(), r.bottom() - len, r.left(), r.bottom());
        painter->drawLine(r.left(), r.bottom(), r.left() + len, r.bottom());
        painter->drawLine(r.right() - len, r.bottom(), r.right(), r.bottom());
        painter->drawLine(r.right(), r.bottom() - len, r.right(), r.bottom());
    }

    /// The CS6 disabled-mask mark: a red cross over the thumbnail.
    static void paintDisabledCross(QPainter* painter, const QRect& thumb, int side)
    {
        painter->save();
        painter->setPen(QPen(QColor(0xE0, 0x20, 0x20), qMax(1, side / 10)));
        const QRect cross = thumb.adjusted(2, 2, -2, -2);
        painter->drawLine(cross.topLeft(), cross.bottomRight());
        painter->drawLine(cross.topRight(), cross.bottomLeft());
        painter->restore();
    }

    static const QPixmap& checkerTile()
    {
        static const QPixmap tile = [] {
            QPixmap pixmap(8, 8);
            pixmap.fill(QColor(0xC8, 0xC8, 0xC8));
            QPainter painter(&pixmap);
            painter.fillRect(0, 0, 4, 4, QColor(0xFF, 0xFF, 0xFF));
            painter.fillRect(4, 4, 4, 4, QColor(0xFF, 0xFF, 0xFF));
            return pixmap;
        }();
        return tile;
    }

    static void paintAsset(QPainter* painter, const QRect& rect, const QString& assetId,
                           const QColor& tint = QColor())
    {
        // Render the SVG at the largest square that fits, at the painter's
        // device pixel ratio, so it stays crisp instead of being scaled from a
        // logical-size pixmap (or stretched into a non-square rect).
        const int side = qMin(rect.width(), rect.height());
        if (side <= 0) {
            return;
        }
        const qreal dpr = painter->device() ? painter->device()->devicePixelRatioF() : 1.0;
        const QPixmap pixmap =
            (tint.isValid() ? pictura::icon(assetId, tint) : pictura::icon(assetId))
                .pixmap(QSize(side, side), dpr);
        if (!pixmap.isNull()) {
            painter->drawPixmap(
                QRect(rect.left() + (rect.width() - side) / 2,
                      rect.top() + (rect.height() - side) / 2, side, side),
                pixmap);
        }
    }

    /// The chain glyph between two thumbnails: the link/unlink icon rotated a
    /// quarter turn so it reads vertically between the side-by-side thumbnails.
    static QPixmap linkPixmap(bool linked, int side)
    {
        const QPixmap base =
            pictura::icon(linked ? QStringLiteral("layers.link2")
                                 : QStringLiteral("layers.unlink2"))
                .pixmap(side, side);
        if (base.isNull()) {
            return {};
        }
        return base.transformed(QTransform().rotate(90), Qt::SmoothTransformation);
    }

    int thumbnailSize_ = 24;
};


} // namespace pictura
