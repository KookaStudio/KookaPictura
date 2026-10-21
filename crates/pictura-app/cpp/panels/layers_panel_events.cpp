#include "layers_panel.h"

#include "api/layers_api.h"
#include "dialogs.h"

#include "layer_new_dialog.h"
#include "layer_style_dialog.h"
#include "layers_filter_bar.h"
#include "layers_filter_proxy.h"
#include "layers_panel_internal.h"

#include "icons.h"
#include "layer_adjustments.h"
#include "percent_field.h"
#include "session.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/clipping.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layers_surface.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/mask_commands.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/vector_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layers_smart_filters.cxxqt.h"

#include <QtCore/QAbstractItemModel>
#include <QtCore/QEvent>
#include <QtCore/QHash>
#include <QtCore/QItemSelection>
#include <QtCore/QItemSelectionModel>
#include <QtCore/QMimeData>
#include <QtCore/QModelIndex>
#include <QtCore/QPoint>
#include <QtCore/QSet>
#include <QtCore/QSize>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/QTimer>
#include <QtCore/QVariant>
#include <QtCore/QVector>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QFont>
#include <QtGui/QIcon>
#include <QtGui/QImage>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPalette>
#include <QtGui/QPen>
#include <QtGui/QPixmap>
#include <QtWidgets/QAbstractItemView>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QHeaderView>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QStyle>
#include <QtWidgets/QStyledItemDelegate>
#include <QtWidgets/QStyleOptionViewItem>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QTreeView>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

#include <array>
#include <functional>
#include <memory>
#include <utility>


namespace pictura {

bool LayersPanel::eventFilter(QObject* watched, QEvent* event)
{
    if (auto* button = qobject_cast<QToolButton*>(watched)) {
        const QString dropAction = button->property("layerDropAction").toString();
        if (!dropAction.isEmpty()) {
            if (event->type() == QEvent::DragEnter) {
                auto* drag = static_cast<QDragEnterEvent*>(event);
                if (tree_->isOwnLayerDrag(drag->mimeData())) {
                    drag->acceptProposedAction();
                    return true;
                }
            } else if (event->type() == QEvent::Drop) {
                auto* drop = static_cast<QDropEvent*>(event);
                if (tree_->isOwnLayerDrag(drop->mimeData())) {
                    const QStringList paths =
                        QString::fromUtf8(drop->mimeData()->data(kLayerMimeType))
                            .split(QLatin1Char('\n'), Qt::SkipEmptyParts);
                    if (view_ && !paths.isEmpty()) {
                        if (dropAction == QLatin1String("delete")) {
                            view_->delete_layers(paths);
                        } else if (dropAction == QLatin1String("duplicate")) {
                            // A Background cannot be duplicated into a locked
                            // `Background copy`; convert it in place instead.
                            const QModelIndex row = model_->indexForPath(paths.first());
                            if (row.isValid()
                                && row.data(KindRole).toString()
                                    == QLatin1String("background")) {
                                openBackgroundConversion(paths.first());
                            } else {
                                view_->duplicate_layers(paths);
                            }
                        } else if (dropAction == QLatin1String("group")) {
                            view_->group_layers(paths);
                        }
                    }
                    drop->acceptProposedAction();
                    return true;
                }
            }
        }
    }
    if (watched == tree_->viewport()) {
        if (event->type() == QEvent::MouseMove || event->type() == QEvent::HoverMove) {
            auto* move = static_cast<QMouseEvent*>(event);
            const QModelIndex index = tree_->indexAt(move->position().toPoint());
            tree_->viewport()->setCursor(index.isValid() ? Qt::PointingHandCursor
                                                         : Qt::ArrowCursor);
        } else if (event->type() == QEvent::Leave) {
            tree_->viewport()->unsetCursor();
        }
    }
    if (watched == tree_->viewport() && event->type() == QEvent::MouseButtonPress) {
        auto* mouse = static_cast<QMouseEvent*>(event);
        if (mouse->button() == Qt::LeftButton) {
            const QPoint pos = mouse->position().toPoint();
            const QModelIndex index = tree_->indexAt(pos);
            if (index.isValid() && index.data(ExpandableRole).toBool()) {
                const QRect chevron =
                    delegate_->chevronRect(tree_->visualRect(index), index);
                if (chevron.contains(pos)) {
                    tree_->setExpanded(index, !tree_->isExpanded(index));
                    return true;
                }
            }
            if (index.isValid() && (mouse->modifiers() & Qt::ControlModifier)) {
                const QRect thumb = delegate_->thumbRect(tree_->visualRect(index), index);
                if (!thumb.isEmpty() && thumb.contains(pos)) {
                    // Ctrl+click the thumbnail selects the layer's pixels;
                    // consume the click so it starts no drag and opens no editor.
                    if (view_) {
                        view_->select_layer_alpha(pathForProxyIndex(index));
                    }
                    return true;
                }
            }
            // The mask's own controls: Shift-click the mask thumbnail toggles
            // its enabled bit; a plain click makes it the mask edit target (the
            // brush, fill, and filters then write its coverage); a click on the
            // link glyph toggles its linkage. All are consumed so they never
            // rename, select, or start a drag.
            if (index.isValid() && view_) {
                const QString rowPath = pathForProxyIndex(index);
                const QRect vr = tree_->visualRect(index);
                // A plain layer-thumbnail click returns to editing the layer's
                // pixels; the click still selects the row or starts a drag.
                const QRect layerThumb = delegate_->thumbRect(vr, index);
                if (!layerThumb.isEmpty() && layerThumb.contains(pos)
                    && !(mouse->modifiers() & Qt::ControlModifier)) {
                    mask_edit_target_set(*view_, QString());
                    mask_solo_set(*view_, QString());
                    syncEditTarget();
                }
                const QRect maskThumb = delegate_->maskThumbRect(vr, index);
                const Qt::KeyboardModifiers rubylith = Qt::AltModifier | Qt::ShiftModifier;
                if (!maskThumb.isEmpty() && maskThumb.contains(pos)
                    && (mouse->modifiers() & rubylith) == rubylith) {
                    // Alt+Shift-click toggles the rubylith overlay. It is
                    // display only, so it leaves the edit target alone, but it
                    // selects the row: the overlay belongs to the current layer
                    // and ends when that changes (syncMaskSolo).
                    tree_->setCurrentIndex(index);
                    mask_overlay_toggle(*view_, rowPath);
                    emit maskDisplayChanged();
                    return true;
                }
                if (!maskThumb.isEmpty() && maskThumb.contains(pos)) {
                    setActiveThumb(rowPath, 1);
                    const bool alt = mouse->modifiers().testFlag(Qt::AltModifier);
                    const bool shift = mouse->modifiers().testFlag(Qt::ShiftModifier);
                    if (shift) {
                        // Toggling the enabled bit keeps any solo on this mask:
                        // the solo shows the mask's stored coverage, not what the
                        // mask does to the layer (see ui/mask-solo-view).
                        layer_mask_set_enabled_path(*view_, rowPath,
                                                    index.data(MaskDisabledRole).toBool());
                        return false;
                    }
                    // The target belongs to the active layer, so select the row
                    // first (its selection sets the active layer).
                    tree_->setCurrentIndex(index);
                    mask_edit_target_set(*view_, rowPath);
                    if (alt) {
                        // Alt toggles the mask solo view on this thumbnail; a
                        // second Alt-click on the same mask restores.
                        const QString soloed = mask_solo_get(*view_);
                        mask_solo_set(*view_, soloed == rowPath ? QString() : rowPath);
                        refresh();
                        return true;
                    }
                    // Any other mask-thumb click restores the composite.
                    mask_solo_set(*view_, QString());
                    refresh();
                    return false;
                }
                const QRect linkGlyph = delegate_->linkGlyphRect(vr, index);
                if (!linkGlyph.isEmpty() && linkGlyph.contains(pos)) {
                    layer_mask_set_linked_path(*view_, rowPath,
                                               !index.data(MaskLinkedRole).toBool());
                    return true;
                }
                // The vector mask's own controls, mirroring the layer mask.
                const QRect vectorThumb = delegate_->vectorMaskThumbRect(vr, index);
                if (!vectorThumb.isEmpty() && vectorThumb.contains(pos)) {
                    // The vector mask becomes the target; drop any raster target.
                    mask_edit_target_set(*view_, QString());
                    mask_solo_set(*view_, QString());
                    syncEditTarget();
                    setActiveThumb(rowPath, 2);
                    if (mouse->modifiers() & Qt::ShiftModifier) {
                        vector_mask_set_enabled_path(
                            *view_, rowPath, index.data(VectorMaskDisabledRole).toBool());
                        return true;
                    }
                    return false;
                }
                const QRect vectorLink = delegate_->vectorLinkGlyphRect(vr, index);
                if (!vectorLink.isEmpty() && vectorLink.contains(pos)) {
                    vector_mask_set_linked_path(*view_, rowPath,
                                                !index.data(VectorMaskLinkedRole).toBool());
                    return true;
                }
                // A plain or Alt click on the image thumbnail makes it the edit
                // target and still selects the row (which activates the layer).
                const QRect imageThumb = delegate_->thumbRect(vr, index);
                if (!imageThumb.isEmpty() && imageThumb.contains(pos)
                    && !(mouse->modifiers() & Qt::ControlModifier)) {
                    mask_solo_set(*view_, QString());
                    setActiveThumb(rowPath, 0);
                    return false;
                }
            }
            // An Alt-click on a row's fx badge toggles every layer's effects,
            // CS6's Show/Hide All Effects (a plain click leaves it to selection).
            if (index.isValid() && view_ && (mouse->modifiers() & Qt::AltModifier)) {
                const QRect fx = delegate_->fxRect(tree_->visualRect(index), index);
                if (!fx.isEmpty() && fx.contains(pos)) {
                    layer_style_set_all_visible(*view_,
                                                !layer_style_any_visible(*view_, true));
                    return true;
                }
            }
            // Alt-click on the line between two rows (outside the eye column)
            // clips the upper layer to the lower, or releases it, as CS6 does.
            if (index.isValid() && (mouse->modifiers() & Qt::AltModifier)
                && !delegate_->eyeColumnContains(tree_->visualRect(index), pos)) {
                const QRect row = tree_->visualRect(index);
                QModelIndex upper;
                if (pos.y() >= row.bottom() - kClipLineGrab) {
                    upper = index;
                } else if (pos.y() <= row.top() + kClipLineGrab) {
                    upper = tree_->indexAbove(index);
                }
                if (upper.isValid() && view_
                    && clipping_toggle(*view_, pathForProxyIndex(upper))) {
                    return true;
                }
            }
            // The Background's lock badge unlocks it: one click makes it an
            // ordinary layer, as Photoshop's lock icon does.
            if (index.isValid() && view_
                && index.data(KindRole).toString() == QLatin1String("background")
                && delegate_->lockRect(tree_->visualRect(index)).contains(pos)) {
                const QString path = pathForProxyIndex(index);
                if (view_->convert_background(path, view_->next_layer_name(QStringLiteral("Layer")),
                                              0)) {
                    refresh();
                    selectPath(path);
                }
                return true;
            }
            // A regular layer's lock badge unlocks it with one click; the
            // Background badge above converts instead.
            if (index.isValid() && view_ && index.data(LockBadgeRole).toInt() != 0
                && index.data(KindRole).toString() != QLatin1String("background")
                && delegate_->lockRect(tree_->visualRect(index)).contains(pos)) {
                view_->set_layers_lock(QStringList{pathForProxyIndex(index)},
                                       QStringLiteral("all"), false);
                return true;
            }
            if (index.isValid() && delegate_->eyeColumnContains(tree_->visualRect(index), pos)) {
                const QString path = pathForProxyIndex(index);
                const QString kind = index.data(KindRole).toString();
                if (view_ && kind.startsWith(QLatin1String("smart-filter"))) {
                    const int sf = path.indexOf(QLatin1String("/@sf"));
                    const QString base = sf >= 0 ? path.left(sf) : path;
                    const bool on = !index.data(VisibleRole).toBool();
                    if (kind == QLatin1String("smart-filters")) {
                        set_layer_smart_filters_enabled(*view_, base, on);
                    } else {
                        const int filterIndex =
                            path.mid(path.lastIndexOf(QLatin1Char('/')) + 1).toInt();
                        set_layer_smart_filter_visible(*view_, base, filterIndex, on);
                    }
                    return true;
                }
                if (mouse->modifiers() & Qt::AltModifier) {
                    toggleSolo(path);
                    return true;
                }
                QStringList targets = targetPaths();
                if (!targets.contains(path)) {
                    targets = QStringList{path};
                }
                const bool on = !model_->visibleInDocument(path);
                if (view_ && !targets.isEmpty()) {
                    view_->set_layers_visible(targets, on);
                    // Patch the toggled rows in place so a second quick toggle
                    // reads the new value instead of a stale one.
                    for (const QString& target : targets) {
                        model_->setVisible(target, on);
                    }
                }
                return true;
            }
        }
    }
    if (watched == tree_->viewport() && event->type() == QEvent::MouseButtonDblClick) {
        auto* mouse = static_cast<QMouseEvent*>(event);
        if (mouse->button() == Qt::LeftButton) {
            const QPoint pos = mouse->position().toPoint();
            const QModelIndex index = tree_->indexAt(pos);
            if (index.isValid()) {
                if (index.data(KindRole).toString().startsWith(QLatin1String("smart-filter"))) {
                    return true;
                }
                // The mask thumbnail and link glyph own their clicks; a
                // double-click there must not rename or open a style.
                const QRect rowRect = tree_->visualRect(index);
                if (delegate_->maskThumbRect(rowRect, index).contains(pos)
                    || delegate_->linkGlyphRect(rowRect, index).contains(pos)
                    || delegate_->vectorMaskThumbRect(rowRect, index).contains(pos)
                    || delegate_->vectorLinkGlyphRect(rowRect, index).contains(pos)) {
                    return true;
                }
                const QString path = pathForProxyIndex(index);
                if (index.data(KindRole).toString() == QLatin1String("background")) {
                    // A control keeps its own action; any other content-band
                    // double-click converts the Background through the dialog.
                    const QRect vr = tree_->visualRect(index);
                    const bool control = delegate_->eyeColumnContains(vr, pos)
                        || delegate_->thumbRect(vr, index).contains(pos);
                    if (!control) {
                        openBackgroundConversion(path);
                    }
                } else if (delegate_->nameRect(tree_->visualRect(index), index).contains(pos)) {
                    tree_->edit(index);
                } else if (!delegate_->eyeColumnContains(tree_->visualRect(index), pos)
                           && !delegate_->lockRect(tree_->visualRect(index)).contains(pos)
                           && !delegate_->fxRect(tree_->visualRect(index), index).contains(pos)) {
                    openLayerStyle(path);
                }
                return true;
            }
        }
    }
    return QWidget::eventFilter(watched, event);
}

} // namespace pictura
