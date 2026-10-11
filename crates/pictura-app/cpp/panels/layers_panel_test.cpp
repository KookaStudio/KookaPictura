#include "layers_panel.h"

#include "layers_filter_bar.h"
#include "layers_filter_proxy.h"
#include "layers_panel_internal.h"
#include "percent_field.h"

#include "pictura_app/src/cxxqt_object/impl_layers/layers_surface.cxxqt.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QMetaObject>
#include <QtCore/QMimeData>
#include <QtCore/QModelIndex>
#include <QtCore/QPoint>
#include <QtCore/QRect>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QAction>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDragMoveEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPalette>
#include <QtWidgets/QLabel>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QStyle>
#include <QtWidgets/QStyleOptionViewItem>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QTreeView>

#include <array>

#include <functional>

namespace pictura {

namespace {

void setPercentForTest(PercentField* field, int pct)
{
    if (!field) {
        return;
    }
    auto* edit = field->findChild<QLineEdit*>();
    if (!edit) {
        return;
    }
    edit->setText(QString::number(pct));
    QMetaObject::invokeMethod(edit, "editingFinished");
}

} // namespace

// --- M39 self-test hooks ----------------------------------------------------

bool LayersPanel::rowHasMaskForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(HasMaskRole).toBool();
}

bool LayersPanel::rowHasAdjustmentForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(HasAdjustmentRole).toBool();
}

bool LayersPanel::rowClippingForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(ClippingRole).toBool();
}

bool LayersPanel::rowClipBaseForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(ClipBaseRole).toBool();
}

bool LayersPanel::rowExpandableForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(ExpandableRole).toBool();
}

QString LayersPanel::rowToolTipForTest(const QString& path) const
{
    return model_ ? model_->indexForPath(path).data(Qt::ToolTipRole).toString() : QString();
}

bool LayersPanel::beginRenameForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid()) {
        return false;
    }
    tree_->setCurrentIndex(index);
    tree_->edit(index);
    return true;
}

bool LayersPanel::selectRowForTest(const QString& path)
{
    if (!model_ || !model_->indexForPath(path).isValid()) {
        return false;
    }
    selectPath(path);
    syncControls();
    return true;
}

QObject* LayersPanel::itemDelegateForTest() const
{
    return delegate_;
}

void LayersPanel::toggleSoloForTest(const QString& path)
{
    toggleSolo(path);
}

int LayersPanel::thumbSizeIndexForTest() const
{
    return thumbSizeIndex_;
}

int LayersPanel::thumbContentsForTest() const
{
    return thumbContents_;
}

bool LayersPanel::expandNewEffectsForTest() const
{
    return expandNewEffects_;
}

void LayersPanel::setOptionsForTest(int size, int contents, bool expand)
{
    thumbSizeIndex_ = qBound(0, size, 3);
    thumbContents_ = qBound(0, contents, 1);
    expandNewEffects_ = expand;
    thumbEntireDocument_ = thumbContents_ == 0;
    if (delegate_) {
        delegate_->setThumbnailSize(kThumbSizePx.at(thumbSizeIndex_));
    }
    persistOptions();
}

bool LayersPanel::addCopyOnDuplicateForTest() const
{
    return addCopyOnDuplicate_;
}

bool LayersPanel::useDefaultMasksOnFillForTest() const
{
    return useDefaultMasksOnFill_;
}

void LayersPanel::setOptionFlagsForTest(bool addCopy, bool useDefaultMasks)
{
    addCopyOnDuplicate_ = addCopy;
    useDefaultMasksOnFill_ = useDefaultMasks;
    if (view_) {
        set_layers_panel_options(*view_, addCopyOnDuplicate_, useDefaultMasksOnFill_);
    }
    persistOptions();
}

QStringList LayersPanel::rowMenuTextsForTest(const QString& kind)
{
    QMenu menu;
    populateRowMenu(menu, QString(), 0, kind);
    QStringList texts;
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator()) {
            texts.push_back(action->text());
        }
    }
    return texts;
}

bool LayersPanel::rowMenuEnabledForTest(const QString& kind, const QString& text)
{
    QMenu menu;
    populateRowMenu(menu, QString(), 0, kind);
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator() && action->text() == text) {
            return action->isEnabled();
        }
    }
    return false;
}

QString LayersPanel::rowMenuToolTipForTest(const QString& kind, const QString& text)
{
    QMenu menu;
    populateRowMenu(menu, QString(), 0, kind);
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator() && action->text() == text) {
            return action->toolTip();
        }
    }
    return QString();
}

QStringList LayersPanel::shapeRowMenuTextsForTest()
{
    QMenu menu;
    populateRowMenu(menu, QString(), 0, QStringLiteral("adjustment"), true);
    QStringList texts;
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator()) {
            texts.push_back(action->text());
        }
    }
    return texts;
}

bool LayersPanel::shapeRowMenuEnabledForTest(const QString& text)
{
    QMenu menu;
    populateRowMenu(menu, QString(), 0, QStringLiteral("adjustment"), true);
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator() && action->text() == text) {
            return action->isEnabled();
        }
    }
    return false;
}

QStringList LayersPanel::smartRowMenuTextsForTest()
{
    QMenu menu;
    populateRowMenu(menu, QString(), 0, QStringLiteral("pixel"), false, true);
    QStringList texts;
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator()) {
            texts.push_back(action->text());
        }
    }
    return texts;
}

bool LayersPanel::lockToggleEnabledForTest(int flag) const
{
    QToolButton* button = nullptr;
    switch (flag) {
    case 0:
        button = lockTransparency_;
        break;
    case 1:
        button = lockPixels_;
        break;
    case 2:
        button = lockPosition_;
        break;
    case 3:
        button = lockAll_;
        break;
    default:
        return false;
    }
    return button && button->isEnabled();
}

bool LayersPanel::lockToggleCheckedForTest(int flag) const
{
    QToolButton* button = nullptr;
    switch (flag) {
    case 0:
        button = lockTransparency_;
        break;
    case 1:
        button = lockPixels_;
        break;
    case 2:
        button = lockPosition_;
        break;
    case 3:
        button = lockAll_;
        break;
    default:
        return false;
    }
    return button && button->isChecked();
}

QStringList LayersPanel::colorLabelTextsForTest()
{
    QMenu menu;
    addColorLabelActions(&menu, 0);
    QStringList texts;
    for (QAction* action : menu.actions()) {
        texts.push_back(action->text());
    }
    return texts;
}

int LayersPanel::lockButtonCountForTest() const
{
    return 5;
}

int LayersPanel::opacityPercentForTest() const
{
    return opacity_ ? opacity_->value() : 0;
}

int LayersPanel::fillPercentForTest() const
{
    return fill_ ? fill_->value() : 0;
}

QRect LayersPanel::blendControlRectForTest() const
{
    return blend_ ? blend_->geometry() : QRect();
}

QRect LayersPanel::opacityControlRectForTest() const
{
    return opacity_ ? opacity_->geometry() : QRect();
}

int LayersPanel::opacitySizeHintWidthForTest() const
{
    return opacity_ ? opacity_->sizeHint().width() : 0;
}

void LayersPanel::setOpacityPercentForTest(int pct)
{
    setPercentForTest(opacity_, pct);
}

void LayersPanel::setFillPercentForTest(int pct)
{
    setPercentForTest(fill_, pct);
}

QStringList LayersPanel::visiblePathsForTest() const
{
    QStringList paths;
    if (!proxy_ || !tree_) {
        return paths;
    }
    std::function<void(const QModelIndex&)> walk = [this, &walk, &paths](
                                                      const QModelIndex& parent) {
        const int rows = proxy_->rowCount(parent);
        for (int row = 0; row < rows; ++row) {
            const QModelIndex index = proxy_->index(row, 0, parent);
            paths.push_back(pathForProxyIndex(index));
            if (tree_->isExpanded(index)) {
                walk(index);
            }
        }
    };
    walk(QModelIndex());
    return paths;
}

void LayersPanel::setFilterNameForTest(const QString& name, bool enabled)
{
    LayerFilter filter = filterBar_ ? filterBar_->filter() : LayerFilter{};
    filter.enabled = enabled;
    filter.name = name;
    if (filterBar_) {
        filterBar_->setFilter(filter);
    }
    applyFilter(filter);
}

void LayersPanel::setFilterKindForTest(const QStringList& kinds, bool enabled)
{
    LayerFilter filter = filterBar_ ? filterBar_->filter() : LayerFilter{};
    filter.enabled = enabled;
    filter.kinds.clear();
    for (const QString& kind : kinds) {
        filter.kinds.insert(kind);
    }
    if (filterBar_) {
        filterBar_->setFilter(filter);
    }
    applyFilter(filter);
}

void LayersPanel::setFilterModeForTest(const QString& key, bool enabled)
{
    LayerFilter filter = filterBar_ ? filterBar_->filter() : LayerFilter{};
    filter.enabled = enabled;
    filter.mode = key;
    if (filterBar_) {
        filterBar_->setFilter(filter);
    }
    applyFilter(filter);
}

void LayersPanel::setFilterColorForTest(int label, bool enabled)
{
    LayerFilter filter = filterBar_ ? filterBar_->filter() : LayerFilter{};
    filter.enabled = enabled;
    filter.color = label;
    if (filterBar_) {
        filterBar_->setFilter(filter);
    }
    applyFilter(filter);
}

void LayersPanel::setFilterAttributeForTest(const QString& attr, bool enabled)
{
    LayerFilter filter = filterBar_ ? filterBar_->filter() : LayerFilter{};
    filter.enabled = enabled;
    filter.attribute = attr;
    if (filterBar_) {
        filterBar_->setFilter(filter);
    }
    applyFilter(filter);
}

int LayersPanel::filterDimensionForTest() const
{
    return filterBar_ ? filterBar_->dimensionIndexForTest() : -1;
}

bool LayersPanel::rowHasStyleForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(HasLayerStyleRole).toBool();
}

QRect LayersPanel::rowFxRectForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return {};
    }
    const QRect vr = tree_->visualRect(index);
    return delegate_->fxRect(QRect(0, 0, vr.width(), vr.height()), index);
}

void LayersPanel::altClickRowFxForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return;
    }
    const QRect vr = tree_->visualRect(index);
    const QRect fx = delegate_->fxRect(vr, index);
    if (fx.isEmpty()) {
        return;
    }
    const QPoint at = fx.center();
    QMouseEvent press(QEvent::MouseButtonPress, at, tree_->viewport()->mapToGlobal(at),
                      Qt::LeftButton, Qt::LeftButton, Qt::AltModifier);
    QApplication::sendEvent(tree_->viewport(), &press);
}

bool LayersPanel::rowHasBlendIfForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(HasBlendIfRole).toBool();
}

QRect LayersPanel::rowBlendIfRectForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return {};
    }
    const QRect vr = tree_->visualRect(index);
    return delegate_->blendIfRect(QRect(0, 0, vr.width(), vr.height()), index);
}

QStringList LayersPanel::rowMenuTextsForPathForTest(const QString& path)
{
    const QModelIndex index = model_ ? model_->indexForPath(path) : QModelIndex();
    QMenu menu;
    populateRowMenu(menu, path, index.data(ColorRole).toInt(), index.data(KindRole).toString(),
                    index.data(LayerRowShapeRole).toBool(),
                    index.data(SmartObjectRole).toBool());
    QStringList texts;
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator()) {
            texts.push_back(action->text());
        }
    }
    return texts;
}

bool LayersPanel::rowMenuEnabledForPathForTest(const QString& path, const QString& text)
{
    const QModelIndex index = model_ ? model_->indexForPath(path) : QModelIndex();
    QMenu menu;
    populateRowMenu(menu, path, index.data(ColorRole).toInt(), index.data(KindRole).toString(),
                    index.data(LayerRowShapeRole).toBool(),
                    index.data(SmartObjectRole).toBool());
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator() && action->text() == text) {
            return action->isEnabled();
        }
    }
    return false;
}

bool LayersPanel::performRowActionForTest(const QString& id, const QString& path)
{
    return performRowAction(id, path);
}

void LayersPanel::setFilterEffectForTest(const QString& effect, bool enabled)
{
    LayerFilter filter = filterBar_ ? filterBar_->filter() : LayerFilter{};
    filter.enabled = enabled;
    filter.effect = effect;
    if (filterBar_) {
        filterBar_->setFilter(filter);
    }
    applyFilter(filter);
}

bool LayersPanel::headerOrderOkForTest() const
{
    const auto y = [this](const QWidget* widget) {
        return widget ? widget->mapTo(this, QPoint(0, 0)).y() : -1;
    };
    const int filterY = y(filterBar_);
    const int opacityY = y(opacity_);
    const int locksY = y(lockTransparency_);
    const int treeY = y(tree_);
    return filterY >= 0 && filterY < opacityY && opacityY < locksY && locksY < treeY;
}

bool LayersPanel::opacityLabelPresentForTest() const
{
    return opacity_ && !opacity_->labelText().isEmpty();
}

bool LayersPanel::fillLabelPresentForTest() const
{
    return fill_ && !fill_->labelText().isEmpty();
}

bool LayersPanel::hasPanelMenuButtonForTest() const
{
    return findChild<QToolButton*>(QStringLiteral("layersPanelMenu")) != nullptr;
}

bool LayersPanel::filterToggleOnForTest() const
{
    return filterBar_ && filterBar_->toggleOnForTest();
}

bool LayersPanel::filterToggleHasIconForTest() const
{
    return filterBar_ && filterBar_->toggleHasIconForTest();
}

int LayersPanel::eyeLeftForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return -1;
    }
    return delegate_->eyeRect(tree_->visualRect(index)).left();
}

bool LayersPanel::chevronClickExpandsForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !index.data(ExpandableRole).toBool() || !delegate_ || !tree_) {
        return false;
    }
    const bool before = tree_->isExpanded(index);
    const QPoint pos =
        delegate_->chevronRect(tree_->visualRect(index), index).center();
    QMouseEvent press(QEvent::MouseButtonPress, pos, tree_->viewport()->mapToGlobal(pos),
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &press);
    return tree_->isExpanded(index) != before;
}

QString LayersPanel::opacityLabelTextForTest() const
{
    return opacity_ ? opacity_->labelText() : QString();
}

bool LayersPanel::opacitySuffixPresentForTest() const
{
    return opacity_ && opacity_->findChild<QLabel*>(QStringLiteral("percentSuffix")) != nullptr;
}

bool LayersPanel::opacitySuffixInsideEditForTest() const
{
    if (!opacity_) {
        return false;
    }
    auto* edit = opacity_->findChild<QLineEdit*>(QStringLiteral("percentEdit"));
    auto* suffix = opacity_->findChild<QLabel*>(QStringLiteral("percentSuffix"));
    if (!edit || !suffix) {
        return false;
    }
    return edit->rect().contains(QRect(suffix->mapTo(edit, QPoint(0, 0)), suffix->size()));
}

bool LayersPanel::opacityValueFitsForTest() const
{
    if (!opacity_) {
        return false;
    }
    auto* edit = opacity_->findChild<QLineEdit*>(QStringLiteral("percentEdit"));
    auto* suffix = opacity_->findChild<QLabel*>(QStringLiteral("percentSuffix"));
    if (!edit || !suffix) {
        return false;
    }
    const int needed = edit->fontMetrics().horizontalAdvance(QStringLiteral("100"))
        + suffix->fontMetrics().horizontalAdvance(QStringLiteral("%"));
    return edit->width() >= needed;
}

int LayersPanel::dragOpacitySliderForTest(int fromX1000, int toX1000)
{
    if (!opacity_) {
        return -1;
    }
    auto* slider = opacity_->findChild<QSlider*>();
    if (!slider) {
        return -1;
    }
    const int width = qMax(qMax(1, slider->minimumWidth()), slider->width());
    const auto at = [width](int fraction) {
        return qBound(0, width * fraction / 1000, width - 1);
    };
    const int y = qMax(1, slider->height()) / 2;
    const QPoint from(at(fromX1000), y);
    QMouseEvent press(QEvent::MouseButtonPress, from, slider->mapToGlobal(from),
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(slider, &press);
    const QPoint to(at(toX1000), y);
    QMouseEvent move(QEvent::MouseMove, to, slider->mapToGlobal(to), Qt::NoButton,
                     Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(slider, &move);
    const int value = opacity_->value();
    QMouseEvent release(QEvent::MouseButtonRelease, to, slider->mapToGlobal(to),
                        Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
    QCoreApplication::sendEvent(slider, &release);
    return value;
}

int LayersPanel::lockBadgeLeftForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || index.data(LockRole).toInt() == 0 || !delegate_ || !tree_) {
        return -1;
    }
    return delegate_->lockRect(tree_->visualRect(index)).left();
}

bool LayersPanel::clickLockBadgeForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || index.data(LockRole).toInt() == 0 || !delegate_ || !tree_) {
        return false;
    }
    const QPoint pos = delegate_->lockRect(tree_->visualRect(index)).center();
    QMouseEvent press(QEvent::MouseButtonPress, pos, tree_->viewport()->mapToGlobal(pos),
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &press);
    return true;
}

QImage LayersPanel::rowThumbnailForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return QImage();
    }
    const QRect thumb = delegate_->thumbRect(tree_->visualRect(index), index);
    return thumb.isEmpty() ? QImage() : tree_->viewport()->grab(thumb).toImage();
}

bool LayersPanel::rowCheckStateForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(Qt::CheckStateRole).isValid();
}

bool LayersPanel::lockIconsPresentForTest() const
{
    const std::array<QToolButton*, 5> buttons = {
        lockTransparency_, lockPixels_, lockPosition_, lockNesting_, lockAll_};
    for (QToolButton* button : buttons) {
        if (!button || button->icon().isNull()) {
            return false;
        }
    }
    return true;
}

bool LayersPanel::treeDragEnabledForTest() const
{
    return tree_ && tree_->dragEnabled() && tree_->acceptDrops();
}

bool LayersPanel::moveForTest(const QString& path, const QString& target, int mode)
{
    return view_ && view_->move_layer_to(path, target, mode);
}

bool LayersPanel::canMoveForTest(const QString& path, const QString& target, int mode)
{
    return view_ && view_->can_move_layer_to(path, target, mode);
}

bool LayersPanel::dropOnStripButtonForTest(const QString& buttonName, const QStringList& paths)
{
    auto* button = findChild<QToolButton*>(buttonName);
    if (!button) {
        return false;
    }
    QMimeData mime;
    mime.setData(kLayerMimeType, paths.join(QLatin1Char('\n')).toUtf8());
    mime.setData(kLayerSourceMimeType, QByteArray::number(reinterpret_cast<quintptr>(view_)));
    const QPointF local(5, 5);
    QDragEnterEvent enter(local.toPoint(), Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(button, &enter);
    if (!enter.isAccepted()) {
        return false;
    }
    QDropEvent drop(local, Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(button, &drop);
    return drop.isAccepted();
}

bool LayersPanel::layerDragFlagsForTest(const QString& path) const
{
    const QModelIndex index = model_ ? model_->indexForPath(path) : QModelIndex();
    if (!index.isValid()) {
        return false;
    }
    const Qt::ItemFlags flags = model_->flags(index);
    return flags.testFlag(Qt::ItemIsDragEnabled) && flags.testFlag(Qt::ItemIsDropEnabled);
}

QColor LayersPanel::rowGutterColorForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return {};
    }
    const QRect vr = tree_->visualRect(index);
    if (vr.width() <= 4 || vr.height() <= 2) {
        return {};
    }
    QImage image(vr.size(), QImage::Format_ARGB32_Premultiplied);
    image.fill(tree_->palette().color(QPalette::Base));
    QPainter painter(&image);
    QStyleOptionViewItem option;
    option.rect = QRect(0, 0, vr.width(), vr.height());
    option.palette = tree_->palette();
    option.widget = tree_;
    option.state = QStyle::State_Enabled;
    delegate_->paint(&painter, option, index);
    painter.end();
    // The tint now lives in the eye toggle's own background, not the whole
    // gutter: sample just inside its top-left corner, above the glyph.
    const QRect eye = delegate_->eyeRect(option.rect);
    return image.pixelColor(eye.left() + 2, eye.top() + 1);
}

bool LayersPanel::editTriggersDisabledForTest() const
{
    return tree_ && tree_->editTriggers() == QAbstractItemView::NoEditTriggers;
}

bool LayersPanel::doubleClickAtForTest(const QString& path, bool atName)
{
    if (!delegate_ || !tree_) {
        return false;
    }
    // An editor a previous check left open keeps the view in EditingState, which
    // makes the next programmatic edit fail. Revert it and delete every stale
    // editor so findChild below returns only the editor this double-click makes.
    const QList<QLineEdit*> stale = tree_->viewport()->findChildren<QLineEdit*>();
    for (QLineEdit* editor : stale) {
        if (editor->isVisible()) {
            QKeyEvent escape(QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
            QCoreApplication::sendEvent(editor, &escape);
        }
        editor->deleteLater();
    }
    QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
    // A model reset during an earlier edit can leave the view in EditingState
    // with no live editor, which wedges programmatic edits. There is no editor
    // to close at this point, so normalize the state.
    if (tree_->editStateForTest() != 0) {
        tree_->resetEditStateForTest();
    }

    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid()) {
        return false;
    }
    const QRect vr = tree_->visualRect(index);
    const QPoint pos = atName ? delegate_->nameRect(vr, index).center()
                              : delegate_->eyeRect(vr).center();
    QMouseEvent dbl(QEvent::MouseButtonDblClick, pos, tree_->viewport()->mapToGlobal(pos),
                    Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &dbl);
    QLineEdit* editor = tree_->viewport()->findChild<QLineEdit*>();
    const bool opened = editor != nullptr;
    if (editor) {
        QKeyEvent escape(QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
        QCoreApplication::sendEvent(editor, &escape);
        editor->deleteLater();
        QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
    }
    return opened;
}

bool LayersPanel::layerRowDragSupportedForTest() const
{
    if (!model_) {
        return false;
    }
    return model_->mimeTypes().contains(QString::fromLatin1(kLayerMimeType))
        && model_->supportedDropActions().testFlag(Qt::MoveAction)
        && model_->canDropMimeData(nullptr, Qt::MoveAction, 0, 0, QModelIndex());
}

bool LayersPanel::dropIndicatorShownForTest() const
{
    return tree_ && tree_->dropIndicatorShownForTest();
}

int LayersPanel::dragMoveModeAtForTest(const QString& source, const QString& hover, bool above)
{
    if (!tree_) {
        return -1;
    }
    const QModelIndex index = proxyIndexForPath(hover);
    if (!index.isValid()) {
        return -1;
    }
    const QRect vr = tree_->visualRect(index);
    const QPoint pos(vr.left() + 4, above ? vr.top() + 1 : vr.bottom() - 1);
    QMimeData mime;
    mime.setData(kLayerMimeType, source.toUtf8());
    mime.setData(kLayerSourceMimeType, QByteArray::number(reinterpret_cast<quintptr>(view_)));
    QDragEnterEvent enter(pos, Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &enter);
    QDragMoveEvent move(pos, Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &move);
    return tree_->dropModeAtForTest(pos);
}

bool LayersPanel::dropAtForTest(const QString& source, const QString& hover, bool above)
{
    if (!tree_) {
        return false;
    }
    const QModelIndex index = proxyIndexForPath(hover);
    if (!index.isValid()) {
        return false;
    }
    const QRect vr = tree_->visualRect(index);
    const QPoint pos(vr.left() + 4, above ? vr.top() + 1 : vr.bottom() - 1);
    QMimeData mime;
    mime.setData(kLayerMimeType, source.toUtf8());
    mime.setData(kLayerSourceMimeType, QByteArray::number(reinterpret_cast<quintptr>(view_)));
    QDragEnterEvent enter(pos, Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &enter);
    QDragMoveEvent move(pos, Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &move);
    QDropEvent drop(pos, Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &drop);
    return drop.isAccepted();
}

bool LayersPanel::dropIntoForTest(const QString& source, const QString& hover)
{
    if (!tree_) {
        return false;
    }
    const QModelIndex index = proxyIndexForPath(hover);
    if (!index.isValid()) {
        return false;
    }
    const QPoint pos = tree_->visualRect(index).center();
    QMimeData mime;
    mime.setData(kLayerMimeType, source.toUtf8());
    mime.setData(kLayerSourceMimeType, QByteArray::number(reinterpret_cast<quintptr>(view_)));
    QDragEnterEvent enter(pos, Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &enter);
    QDragMoveEvent move(pos, Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &move);
    QDropEvent drop(pos, Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &drop);
    return drop.isAccepted();
}

bool LayersPanel::ctrlClickThumbnailForTest(const QString& path)
{
    if (!tree_ || !delegate_) {
        return false;
    }
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid()) {
        return false;
    }
    const QRect thumb = delegate_->thumbRect(tree_->visualRect(index), index);
    if (thumb.isEmpty()) {
        return false;
    }
    const QPoint pos = thumb.center();
    QMouseEvent press(QEvent::MouseButtonPress, pos, tree_->viewport()->mapToGlobal(pos),
                      Qt::LeftButton, Qt::LeftButton, Qt::ControlModifier);
    return QCoreApplication::sendEvent(tree_->viewport(), &press);
}

bool LayersPanel::inlineEditorOpenForTest() const
{
    if (!tree_) {
        return false;
    }
    for (QLineEdit* editor : tree_->viewport()->findChildren<QLineEdit*>()) {
        if (editor->isVisible()) {
            return true;
        }
    }
    return false;
}

bool LayersPanel::rootDropEnabledForTest() const
{
    return model_ && model_->flags(QModelIndex()).testFlag(Qt::ItemIsDropEnabled);
}

void LayersPanel::beginDragCursorForTest()
{
    if (tree_) {
        tree_->enterDragCursor();
    }
}

void LayersPanel::endDragCursorForTest()
{
    if (tree_) {
        tree_->leaveDragCursor();
    }
}

int LayersPanel::dragCursorShapeForTest() const
{
    return tree_ ? tree_->cursorShapeForTest() : -1;
}

bool LayersPanel::doubleClickChevronForTest(const QString& path)
{
    if (!delegate_ || !tree_) {
        return false;
    }
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !index.data(ExpandableRole).toBool()) {
        return false;
    }
    const QPoint pos =
        delegate_->chevronRect(tree_->visualRect(index), index).center();
    QMouseEvent dbl(QEvent::MouseButtonDblClick, pos, tree_->viewport()->mapToGlobal(pos),
                    Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &dbl);
    return tree_->viewport()->findChild<QLineEdit*>() != nullptr;
}

void LayersPanel::setBackgroundConvertForTest(bool accept, const QString& name, int color)
{
    bgConvertArmed_ = true;
    bgConvertAccept_ = accept;
    bgConvertName_ = name;
    bgConvertColor_ = color;
}

bool LayersPanel::lockNestingHiddenForTest() const
{
    return lockNesting_ && lockNesting_->isHidden();
}

bool LayersPanel::rowLinkedForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(LayerRowLinkedRole).toBool();
}

bool LayersPanel::rowMaskLinkedForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(MaskLinkedRole).toBool();
}

bool LayersPanel::rowMaskDisabledForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(MaskDisabledRole).toBool();
}

bool LayersPanel::clickLinkGlyphForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return false;
    }
    const QRect glyph = delegate_->linkGlyphRect(tree_->visualRect(index), index);
    if (glyph.isEmpty()) {
        return false;
    }
    const QPoint at = glyph.center();
    QMouseEvent press(QEvent::MouseButtonPress, at, tree_->viewport()->mapToGlobal(at),
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &press);
    return true;
}

bool LayersPanel::shiftClickMaskThumbnailForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return false;
    }
    const QRect mask = delegate_->maskThumbRect(tree_->visualRect(index), index);
    if (mask.isEmpty()) {
        return false;
    }
    const QPoint at = mask.center();
    QMouseEvent press(QEvent::MouseButtonPress, at, tree_->viewport()->mapToGlobal(at),
                      Qt::LeftButton, Qt::LeftButton, Qt::ShiftModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &press);
    return true;
}

bool LayersPanel::rowMaskTargetForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(MaskTargetRole).toBool();
}

bool LayersPanel::clickMaskThumbnailForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return false;
    }
    const QRect mask = delegate_->maskThumbRect(tree_->visualRect(index), index);
    if (mask.isEmpty()) {
        return false;
    }
    const QPoint at = mask.center();
    QMouseEvent press(QEvent::MouseButtonPress, at, tree_->viewport()->mapToGlobal(at),
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &press);
    return true;
}

bool LayersPanel::clickLayerThumbnailForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return false;
    }
    const QRect thumb = delegate_->thumbRect(tree_->visualRect(index), index);
    if (thumb.isEmpty()) {
        return false;
    }
    const QPoint at = thumb.center();
    QMouseEvent press(QEvent::MouseButtonPress, at, tree_->viewport()->mapToGlobal(at),
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &press);
    return true;
}

bool LayersPanel::rowHasVectorMaskForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(HasVectorMaskRole).toBool();
}

bool LayersPanel::rowVectorMaskLinkedForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(VectorMaskLinkedRole).toBool();
}

bool LayersPanel::rowVectorMaskDisabledForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(VectorMaskDisabledRole).toBool();
}

bool LayersPanel::clickVectorLinkGlyphForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return false;
    }
    const QRect glyph = delegate_->vectorLinkGlyphRect(tree_->visualRect(index), index);
    if (glyph.isEmpty()) {
        return false;
    }
    const QPoint at = glyph.center();
    QMouseEvent press(QEvent::MouseButtonPress, at, tree_->viewport()->mapToGlobal(at),
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &press);
    return true;
}

bool LayersPanel::shiftClickVectorMaskThumbnailForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return false;
    }
    const QRect mask = delegate_->vectorMaskThumbRect(tree_->visualRect(index), index);
    if (mask.isEmpty()) {
        return false;
    }
    const QPoint at = mask.center();
    QMouseEvent press(QEvent::MouseButtonPress, at, tree_->viewport()->mapToGlobal(at),
                      Qt::LeftButton, Qt::LeftButton, Qt::ShiftModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &press);
    return true;
}

bool LayersPanel::rowPlacedForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(LayerRowPlacedRole).toBool();
}

void LayersPanel::altClickBelowRowForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    const QRect row = tree_->visualRect(index);
    const QPointF at(row.center().x(), row.bottom() - 1);
    QMouseEvent press(QEvent::MouseButtonPress, at, tree_->viewport()->mapToGlobal(at),
                      Qt::LeftButton, Qt::LeftButton, Qt::AltModifier);
    QApplication::sendEvent(tree_->viewport(), &press);
}

bool LayersPanel::clickSmartFilterEyeForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !tree_ || !delegate_) {
        return false;
    }
    const QRect vr = tree_->visualRect(index);
    const QRect eye = delegate_->eyeRect(QRect(0, 0, vr.width(), vr.height()));
    const QPointF at(vr.left() + eye.center().x(), vr.center().y());
    QMouseEvent press(QEvent::MouseButtonPress, at, tree_->viewport()->mapToGlobal(at),
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &press);
    return true;
}

bool LayersPanel::rowShapeForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(LayerRowShapeRole).toBool();
}

bool LayersPanel::rowSmartObjectForTest(const QString& path) const
{
    return model_ && model_->indexForPath(path).data(SmartObjectRole).toBool();
}

QColor LayersPanel::rowLabelChipColorForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return {};
    }
    const QRect vr = tree_->visualRect(index);
    if (vr.width() <= 4 || vr.height() <= 2) {
        return {};
    }
    QImage image(vr.size(), QImage::Format_ARGB32_Premultiplied);
    image.fill(tree_->palette().color(QPalette::Base));
    QPainter painter(&image);
    QStyleOptionViewItem option;
    option.rect = QRect(0, 0, vr.width(), vr.height());
    option.palette = tree_->palette();
    option.widget = tree_;
    option.state = QStyle::State_Enabled;
    delegate_->paint(&painter, option, index);
    painter.end();
    const QRect chip = delegate_->labelChipRect(option.rect, index);
    if (chip.isEmpty()) {
        return {};
    }
    return image.pixelColor(chip.left() + 1, chip.center().y());
}

bool LayersPanel::rowNameItalicForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    return index.isValid() && delegate_ && delegate_->nameFont(tree_->font(), index).italic();
}

bool LayersPanel::rowNameUnderlineForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    return index.isValid() && delegate_ && delegate_->nameFont(tree_->font(), index).underline();
}

int LayersPanel::rowHeightForTest() const
{
    return delegate_ ? delegate_->rowHeight() : 0;
}

QRect LayersPanel::rowThumbRectForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return {};
    }
    const QRect vr = tree_->visualRect(index);
    return delegate_->thumbRect(QRect(0, 0, vr.width(), vr.height()), index);
}

QRect LayersPanel::rowEyeRectForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return {};
    }
    const QRect vr = tree_->visualRect(index);
    return delegate_->eyeRect(QRect(0, 0, vr.width(), vr.height()));
}

QRect LayersPanel::rowNameRectForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return {};
    }
    const QRect vr = tree_->visualRect(index);
    return delegate_->nameRect(QRect(0, 0, vr.width(), vr.height()), index);
}

QRect LayersPanel::rowViewportRectForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !tree_) {
        return {};
    }
    tree_->scrollTo(index);
    return tree_->visualRect(index);
}

bool LayersPanel::eyeGutterCentredForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return false;
    }
    const QRect vr = tree_->visualRect(index);
    const QRect eye = delegate_->eyeRect(QRect(0, 0, vr.width(), vr.height()));
    const int leftPad = eye.left();
    const int rightPad = LayerRowDelegate::kEyeColumn - eye.right() - 1;
    return leftPad > 0 && leftPad == rightPad;
}

bool LayersPanel::eyeSeparatorPresentForTest(const QString& path) const
{
    if (!delegate_) {
        return false;
    }
    const QImage image = rowImageForTest(path);
    if (image.isNull()) {
        return false;
    }
    const int x = LayerRowDelegate::kEyeColumn - 1;
    const int y = image.height() / 2;
    if (x < 0 || x >= image.width() || y < 0 || y >= image.height()) {
        return false;
    }
    const QColor separator = image.pixelColor(x, y);
    return separator.red() == separator.green() && separator.green() == separator.blue()
        && separator.red() < 128;
}

int LayersPanel::dropIndicatorForTest(const QString& source, const QString& hover, int where)
{    if (!tree_) {
        return -1;
    }
    const QModelIndex index = proxyIndexForPath(hover);
    if (!index.isValid()) {
        return -1;
    }
    const QRect vr = tree_->visualRect(index);
    int y = vr.center().y();
    if (where == 0) {
        y = vr.top() + 1;
    } else if (where == 1) {
        y = vr.bottom() - 1;
    }
    const QPoint pos(vr.left() + 4, y);
    QMimeData mime;
    mime.setData(kLayerMimeType, source.toUtf8());
    mime.setData(kLayerSourceMimeType, QByteArray::number(reinterpret_cast<quintptr>(view_)));
    QDragEnterEvent enter(pos, Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &enter);
    QDragMoveEvent move(pos, Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(tree_->viewport(), &move);
    return tree_->dropIndicatorKindForTest();
}

void LayersPanel::expandForTest(const QString& path)
{
    const QModelIndex index = proxyIndexForPath(path);
    if (index.isValid()) {
        tree_->expand(index);
    }
}

QImage LayersPanel::rowImageForTest(const QString& path) const
{
    const QModelIndex index = proxyIndexForPath(path);
    if (!index.isValid() || !delegate_ || !tree_) {
        return {};
    }
    const QRect vr = tree_->visualRect(index);
    if (vr.width() <= 0 || vr.height() <= 0) {
        return {};
    }
    QImage image(vr.size(), QImage::Format_ARGB32_Premultiplied);
    const QColor base(128, 128, 128);
    image.fill(base);
    QPainter painter(&image);
    QStyleOptionViewItem option;
    option.rect = QRect(0, 0, vr.width(), vr.height());
    QPalette palette = tree_->palette();
    palette.setColor(QPalette::Window, base);
    palette.setColor(QPalette::Base, base);
    palette.setColor(QPalette::Highlight, QColor(0, 0, 255));
    palette.setColor(QPalette::HighlightedText, QColor(255, 255, 255));
    option.palette = palette;
    option.widget = tree_;
    option.font = tree_->font();
    option.state = QStyle::State_Enabled;
    if (tree_->selectionModel() && tree_->selectionModel()->isSelected(index)) {
        option.state |= QStyle::State_Selected;
    }
    delegate_->paint(&painter, option, index);
    painter.end();
    return image;
}

} // namespace pictura
