#pragma once

#include "layers_panel_model.h"

#include <QtGui/QFontMetrics>

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
    // The color-label chip's width and the gap that follows it, before the
    // thumbnail. CS6 shows a label as a small bar next to the thumbnail.
    static constexpr int kLabelChipWidth = 3;
    static constexpr int kLabelChipGap = 3;

    /// The horizontal space a row's color-label chip consumes before the
    /// thumbnail, or 0 when the row has no label. Shared by paint, thumbRect,
    /// and nameRect so the three never disagree.
    int labelChipAdvance(const QModelIndex& index) const
    {
        return layerLabelColor(index.data(ColorRole).toInt()).isValid()
            ? kLabelChipWidth + kLabelChipGap
            : 0;
    }

    /// The color-label chip's rect at the start of a row's content, as painted;
    /// empty when the row has no label.
    QRect labelChipRect(const QRect& itemRect, const QModelIndex& index) const
    {
        if (!layerLabelColor(index.data(ColorRole).toInt()).isValid()) {
            return {};
        }
        return QRect(contentLeft(itemRect, index), itemRect.top() + 3, kLabelChipWidth,
                     qMax(1, itemRect.height() - 6));
    }

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

    /// The square box left of the right-edge badges where the image thumbnail
    /// sits, mirroring paint()'s left-to-right content walk.
    QRect imageThumbBox(const QRect& itemRect, const QModelIndex& index) const
    {
        int x = contentLeft(itemRect, index);
        x += labelChipAdvance(index);
        if (index.data(ClippingRole).toBool()) {
            const int thumb = thumbSize();
            const int side = qMax(10, thumb > 0 ? thumb - 8 : 12);
            if (!pictura::icon(QStringLiteral("layers.clipMask")).pixmap(side, side).isNull()) {
                x += side + 2;
            }
        }
        const int thumb = thumbSize();
        return QRect(x, itemRect.top() + (itemRect.height() - thumb) / 2, thumb, thumb);
    }

    /// The x just past the image thumbnail's square slot.
    int imageSlotRight(const QRect& itemRect, const QModelIndex& index) const
    {
        return imageThumbBox(itemRect, index).right() + 1;
    }

    /// Whether a row paints the fx badge: adjustment content or a layer style,
    /// but never a shape row (its own badge owns the thumbnail corner).
    bool showsFx(const QModelIndex& index) const
    {
        return !index.data(LayerRowShapeRole).toBool()
            && (index.data(HasAdjustmentRole).toBool()
                || index.data(HasLayerStyleRole).toBool());
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
        if (index.data(LockRole).toInt() != 0
            && !pictura::icon(QStringLiteral("layers.lockAll"))
                    .pixmap(lockBadgeSide(), lockBadgeSide())
                    .isNull()) {
            right -= lockBadgeSide() + 3;
        }
        return QRect(right - badge, itemRect.top() + (itemRect.height() - badge) / 2, badge,
                     badge);
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
        if (static_cast<qint64>(documentWidth) * height
            > static_cast<qint64>(documentHeight) * width) {
            height = qMax(1, qRound(static_cast<double>(box.height()) * documentHeight
                                    / documentWidth));
        } else {
            width = qMax(1, qRound(static_cast<double>(box.width()) * documentWidth
                                   / documentHeight));
        }
        return QRect(box.left() + (box.width() - width) / 2,
                     box.top() + (box.height() - height) / 2, width, height);
    }

    /// The thumbnail's hit-target, mirroring the x/y math paint() lays out, as
    /// the letterboxed (document-aspect) rect. Empty when thumbnails are off.
    QRect thumbRect(const QRect& itemRect, const QModelIndex& index) const
    {
        const int thumb = qMax(0, thumbnailSize_);
        if (thumb <= 0) {
            return {};
        }
        int x = contentLeft(itemRect, index);
        x += labelChipAdvance(index);
        if (index.data(ClippingRole).toBool()) {
            x += qMax(10, thumb - 8) + 2;
        }
        const QRect box(x, itemRect.top() + (itemRect.height() - thumb) / 2, thumb, thumb);
        return letterboxedThumb(box, index);
    }

    /// The right edge left for the mask thumbnail after the lock and fx badges,
    /// mirroring paint()'s walk so the mask and link rects agree with it.
    int badgesRight(const QRect& itemRect, const QModelIndex& index) const
    {
        const int thumb = qMax(0, thumbnailSize_);
        const int badge = qMax(12, thumb > 0 ? thumb : 16);
        int right = itemRect.right() - 3;
        if (index.data(LockRole).toInt() != 0
            && !pictura::icon(QStringLiteral("layers.lockAll"))
                    .pixmap(lockBadgeSide(), lockBadgeSide())
                    .isNull()) {
            right -= lockBadgeSide() + 3;
        }
        if (showsFx(index)
            && !pictura::icon(QStringLiteral("layers.fx")).pixmap(badge, badge).isNull()) {
            right -= badge + 3;
        }
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
        const QRect box(left, itemRect.top() + (itemRect.height() - thumb) / 2, thumb, thumb);
        return letterboxedThumb(box, index);
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
        const QRect box(left, itemRect.top() + (itemRect.height() - thumb) / 2, thumb, thumb);
        return letterboxedThumb(box, index);
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
        if (index.data(LockRole).toInt() != 0
            && !pictura::icon(QStringLiteral("layers.lockAll"))
                    .pixmap(lockBadgeSide(), lockBadgeSide())
                    .isNull()) {
            right -= lockBadgeSide() + 3;
        }
        if (showsFx(index)
            && !pictura::icon(QStringLiteral("layers.fx")).pixmap(badge, badge).isNull()) {
            right -= badge + 3;
        }
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
                const auto* view = qobject_cast<QTreeView*>(parent());
                bool haveNeighbour = false;
                if (view) {
                    const QModelIndex current = view->currentIndex();
                    haveNeighbour = (back ? view->indexAbove(current)
                                          : view->indexBelow(current))
                                        .isValid();
                }
                if (auto* editor = qobject_cast<QWidget*>(object)) {
                    emit commitData(editor);
                    emit closeEditor(editor, haveNeighbour
                                                 ? (back
                                                        ? QAbstractItemDelegate::EditPreviousItem
                                                        : QAbstractItemDelegate::EditNextItem)
                                                 : QAbstractItemDelegate::NoHint);
                }
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
        // 1 px bottom rule per row.
        painter->fillRect(QRect(rect.left(), rect.bottom(), rect.width(), 1),
                          option.palette.color(QPalette::Base).darker(115));

        painter->save();
        const QPalette& palette = option.palette;
        // A non-None color label tints the eye toggle's own background; the
        // alpha keeps the eye glyph legible.
        QColor label = layerLabelColor(index.data(ColorRole).toInt());
        if (label.isValid()) {
            label.setAlpha(90);
            painter->fillRect(eye, label);
        }
        // Eye icon, anchored at the panel's left edge for every depth.
        paintAsset(painter, eye,
                   index.data(VisibleRole).toBool() ? QStringLiteral("layers.eyeOn")
                                                    : QStringLiteral("layers.eyeOff"));
        // 1 px darker-grey separator at the gutter's right edge, between the
        // eye and the row content.
        const QColor separator = palette.color(QPalette::Base).darker(115);
        painter->fillRect(QRect(rect.left() + kEyeColumn - 1, rect.top(), 1, height), separator);

        // Content (disclosure, clipping glyph, thumbnail, name) starts after the
        // gutter; the chevron slot is reserved only for expandable rows.
        const int thumb = qMax(0, thumbnailSize_);
        int x = contentLeft(rect, index);
        QRect thumbBox;
        // A small color-label chip sits at the content start, before the
        // thumbnail (CS6 draws the label as a bar next to the thumbnail).
        const QColor chipColor = layerLabelColor(index.data(ColorRole).toInt());
        if (chipColor.isValid()) {
            painter->fillRect(labelChipRect(rect, index), chipColor);
        }
        x += labelChipAdvance(index);
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
            const QRect box(x, rect.top() + (height - thumb) / 2, thumb, thumb);
            const QRect shaped = letterboxedThumb(box, index);
            thumbBox = shaped;
            if (index.data(KindRole).toString() == QLatin1String("group")) {
                // The folder glyph is half the row thumbnail, centred in its
                // slot, so a folder row reads lighter than a layer row.
                const int gs = qMax(8, thumb / 2);
                const QPixmap glyph =
                    pictura::icon(QStringLiteral("layers.group")).pixmap(gs, gs);
                if (!glyph.isNull()) {
                    painter->drawPixmap(QRect(box.left() + (thumb - gs) / 2,
                                              box.top() + (thumb - gs) / 2, gs, gs),
                                        glyph);
                }
            } else if (index.data(KindRole).toString().startsWith(QLatin1String("smart-filter"))) {
                // The Smart Filters group and its rows show the fx badge instead
                // of a checkerboard (there is no raster thumbnail for them).
                const QPixmap glyph =
                    pictura::icon(QStringLiteral("layers.fx")).pixmap(thumb, thumb);
                if (!glyph.isNull()) {
                    painter->drawPixmap(box, glyph);
                }
            } else if (index.data(KindRole).toString() == QLatin1String("type")) {
                // CS6 shows a type layer as a T on white, not its pixels; the
                // stroke is `layers.kindType`'s glyph, dark for the white card.
                painter->fillRect(shaped, Qt::white);
                const QRectF t = QRectF(box).adjusted(thumb / 4.0, thumb / 4.0, -thumb / 4.0,
                                                      -thumb / 4.0);
                painter->save();
                painter->setRenderHint(QPainter::Antialiasing, true);
                painter->setPen(QPen(QColor(0x20, 0x20, 0x20), qMax(1.5, thumb / 12.0)));
                painter->drawLine(t.topLeft(), t.topRight());
                painter->drawLine(QPointF(t.center().x(), t.top()),
                                  QPointF(t.center().x(), t.bottom()));
                painter->restore();
            } else {
                painter->drawTiledPixmap(shaped, checkerTile());
                const QImage image = index.data(ThumbnailRole).value<QImage>();
                if (!image.isNull()) {
                    painter->drawImage(shaped, image);
                }
            }
            // 1 px black outline around the letterboxed thumbnail.
            painter->setPen(QPen(Qt::black, 1));
            painter->setBrush(Qt::NoBrush);
            painter->drawRect(shaped.adjusted(0, 0, -1, -1));
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
        }
        // Thumbnails run left-to-right from the image thumb: [image][chain][mask]
        // [chain][vector]. The chain glyph is always drawn (link-2 when the mask
        // is linked to the layer, unlink-2 otherwise).
        const QImage mask = index.data(MaskThumbnailRole).value<QImage>();
        const QRect imageRect = thumbBox;
        QRect maskRect;
        QRect vectorRect;
        if (thumb > 0 && !mask.isNull()) {
            const int side = linkSide(thumb);
            const QPixmap linkPix =
                pictura::icon(index.data(MaskLinkedRole).toBool()
                                  ? QStringLiteral("layers.link2")
                                  : QStringLiteral("layers.unlink2"))
                    .pixmap(side, side);
            const QRect link = linkGlyphRect(rect, index);
            if (!linkPix.isNull()) {
                painter->drawPixmap(link, linkPix);
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
            const int side = linkSide(thumb);
            const QPixmap linkPix =
                pictura::icon(index.data(VectorMaskLinkedRole).toBool()
                                  ? QStringLiteral("layers.link2")
                                  : QStringLiteral("layers.unlink2"))
                    .pixmap(side, side);
            const QRect link = vectorLinkGlyphRect(rect, index);
            if (!linkPix.isNull()) {
                painter->drawPixmap(link, linkPix);
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
        if (index.data(LockRole).toInt() != 0) {
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

    static void paintAsset(QPainter* painter, const QRect& rect, const QString& assetId)
    {
        // Render the SVG at the largest square that fits, at the painter's
        // device pixel ratio, so it stays crisp instead of being scaled from a
        // logical-size pixmap (or stretched into a non-square rect).
        const int side = qMin(rect.width(), rect.height());
        if (side <= 0) {
            return;
        }
        const qreal dpr = painter->device() ? painter->device()->devicePixelRatioF() : 1.0;
        const QPixmap pixmap = pictura::icon(assetId).pixmap(QSize(side, side), dpr);
        if (!pixmap.isNull()) {
            painter->drawPixmap(
                QRect(rect.left() + (rect.width() - side) / 2,
                      rect.top() + (rect.height() - side) / 2, side, side),
                pixmap);
        }
    }

    int thumbnailSize_ = 24;
};


} // namespace pictura
