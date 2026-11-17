#include "layer_comps_panel.h"

#include "api/layer_comps_api.h"
#include "api/history_api.h"
#include "icons.h"
#include "layer_comp_options_dialog.h"

#include <QtCore/QEvent>
#include <QtCore/QSize>
#include <QtCore/QVariant>
#include <QtGui/QFontMetrics>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QAction>
#include <QtGui/QPainter>
#include <QtGui/QPolygon>
#include <QtGui/QPalette>
#include <QtWidgets/QApplication>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMenu>
#include <QtWidgets/QStyle>
#include <QtWidgets/QStyledItemDelegate>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

constexpr int kIdRole = Qt::UserRole;
constexpr int kWarningRole = Qt::UserRole + 1;
constexpr int kApplyEnabledRole = Qt::UserRole + 2;
constexpr int kLastRowId = -1;

class RowDelegate : public QStyledItemDelegate {
public:
    using QStyledItemDelegate::QStyledItemDelegate;

    void paint(QPainter* painter, const QStyleOptionViewItem& option,
               const QModelIndex& index) const override
    {
        painter->save();
        painter->setRenderHint(QPainter::Antialiasing, true);
        const QRect rect = option.rect;
        const bool selected = option.state & QStyle::State_Selected;
        painter->fillRect(rect, selected ? option.palette.highlight() : option.palette.base());

        const bool enabled = index.data(kApplyEnabledRole).toBool();
        const QColor ink = selected ? option.palette.highlightedText().color()
                                    : option.palette.text().color();
        QColor apply = ink;
        if (!enabled) {
            apply.setAlpha(70);
        }
        const int cy = rect.center().y();
        QPolygon triangle;
        triangle << QPoint(rect.left() + 6, cy - 5) << QPoint(rect.left() + 6, cy + 5)
                 << QPoint(rect.left() + 15, cy);
        painter->setPen(Qt::NoPen);
        painter->setBrush(apply);
        painter->drawPolygon(triangle);

        const bool warning = index.data(kWarningRole).toBool();
        QRect textRect = rect.adjusted(22, 0, warning ? -20 : -4, 0);
        painter->setPen(ink);
        const QString text = index.data(Qt::DisplayRole).toString();
        const QString elided = QFontMetrics(option.font).elidedText(
            text, Qt::ElideRight, textRect.width());
        painter->drawText(textRect, Qt::AlignVCenter | Qt::AlignLeft, elided);

        if (warning) {
            QPolygon caution;
            caution << QPoint(rect.right() - 14, cy - 5) << QPoint(rect.right() - 4, cy - 5)
                    << QPoint(rect.right() - 9, cy + 5);
            painter->setPen(QPen(QColor(0xe0, 0x8a, 0x00), 1.2));
            painter->setBrush(QColor(0xff, 0xef, 0xc0));
            painter->drawPolygon(caution);
            painter->setPen(QColor(0x8a, 0x50, 0x00));
            painter->drawText(QRect(rect.right() - 13, cy - 4, 8, 8), Qt::AlignCenter,
                              QStringLiteral("!"));
        }
        painter->restore();
    }

    QSize sizeHint(const QStyleOptionViewItem& option, const QModelIndex& index) const override
    {
        QSize size = QStyledItemDelegate::sizeHint(option, index);
        size.setHeight(qMax(size.height(), 22));
        return size;
    }
};

} // namespace

LayerCompsPanel::LayerCompsPanel(QWidget* parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("layerCompsPanel"));
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(2);

    list_ = new QListWidget(this);
    list_->setObjectName(QStringLiteral("layerCompsList"));
    list_->setSelectionMode(QAbstractItemView::ExtendedSelection);
    list_->setItemDelegate(new RowDelegate(list_));
    list_->viewport()->installEventFilter(this);
    list_->installEventFilter(this);
    layout->addWidget(list_, 1);

    auto* buttons = new QHBoxLayout;
    buttons->setSpacing(2);
    const auto button = [this, buttons](const QString& objectName, const QString& iconId,
                                        const QString& tooltip) {
        auto* control = new QToolButton(this);
        control->setObjectName(objectName);
        control->setIcon(pictura::icon(iconId));
        control->setIconSize(QSize(16, 16));
        control->setAutoRaise(true);
        control->setToolTip(tooltip);
        buttons->addWidget(control);
        return control;
    };
    create_ = button(QStringLiteral("layerCompsCreate"), QStringLiteral("layers.newLayer"),
                     tr("Create New Layer Comp"));
    apply_ = button(QStringLiteral("layerCompsApply"), QStringLiteral("crop.apply"),
                    tr("Apply Layer Comp"));
    previous_ = button(QStringLiteral("layerCompsPrevious"), QStringLiteral("edit.stepBackward"),
                       tr("Previous Layer Comp"));
    next_ = button(QStringLiteral("layerCompsNext"), QStringLiteral("edit.stepForward"),
                   tr("Next Layer Comp"));
    update_ = button(QStringLiteral("layerCompsUpdate"), QStringLiteral("file.revert"),
                     tr("Update Layer Comp"));
    delete_ = button(QStringLiteral("layerCompsDelete"), QStringLiteral("layers.delete"),
                     tr("Delete Layer Comp"));
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(create_, &QToolButton::clicked, this, [this] {
        createComp(QApplication::keyboardModifiers().testFlag(Qt::AltModifier));
    });
    connect(apply_, &QToolButton::clicked, this, [this] { applySelected(); });
    connect(previous_, &QToolButton::clicked, this, [this] { previousComp(); });
    connect(next_, &QToolButton::clicked, this, [this] { nextComp(); });
    connect(update_, &QToolButton::clicked, this, [this] { updateSelected(); });
    connect(delete_, &QToolButton::clicked, this, [this] { deleteSelected(); });
    connect(list_, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem* item) {
        if (!item) {
            return;
        }
        if (item->data(kIdRole).toInt() < 0) {
            applyItem(item);
        } else {
            list_->setCurrentItem(item);
            openOptions();
        }
    });
    connect(list_, &QListWidget::itemSelectionChanged, this, [this] { refreshControls(); });
    connect(list_, &QListWidget::itemChanged, this, [this](QListWidgetItem* item) {
        if (updating_ || !item || !view_) {
            return;
        }
        const int id = item->data(kIdRole).toInt();
        if (id >= 0 && !layer_comps_api::rename(*view_, id, item->text())) {
            refresh();
        }
    });
    refresh();
}

void LayerCompsPanel::setView(PictureView* view)
{
    if (viewConnection_) {
        QObject::disconnect(viewConnection_);
    }
    view_ = view;
    if (view_) {
        viewConnection_ = history_api::connectChanged(*view_, this, [this] { refresh(); });
    }
    refresh();
}

int LayerCompsPanel::selectedCompId() const
{
    const QListWidgetItem* item = list_->currentItem();
    return item ? item->data(kIdRole).toInt() : -2;
}

bool LayerCompsPanel::selectedIsLast() const
{
    return selectedCompId() == kLastRowId;
}

QListWidgetItem* LayerCompsPanel::currentCompItem() const
{
    QListWidgetItem* item = list_->currentItem();
    return item && item->data(kIdRole).toInt() >= 0 ? item : nullptr;
}

void LayerCompsPanel::addLastRow()
{
    auto* item = new QListWidgetItem(tr("Last Document State"), list_);
    item->setData(kIdRole, kLastRowId);
    item->setData(kApplyEnabledRole, view_ && layer_comps_api::lastAvailable(*view_));
    item->setFlags(Qt::ItemIsEnabled | Qt::ItemIsSelectable);
    item->setToolTip(tr("Restore the state before the last layer comp was applied"));
}

void LayerCompsPanel::addCompRow(int index)
{
    if (!view_) {
        return;
    }
    const int id = layer_comps_api::id(*view_, index);
    auto* item = new QListWidgetItem(layer_comps_api::name(*view_, index), list_);
    item->setData(kIdRole, id);
    item->setData(kWarningRole, layer_comps_api::warning(*view_, index));
    item->setData(kApplyEnabledRole, true);
    item->setFlags(Qt::ItemIsEnabled | Qt::ItemIsSelectable | Qt::ItemIsEditable);
    const QString comment = layer_comps_api::comment(*view_, index);
    if (!comment.isEmpty()) {
        item->setToolTip(comment);
    }
}

void LayerCompsPanel::refresh()
{
    if (!list_) {
        return;
    }
    if (!view_) {
        updating_ = true;
        list_->clear();
        updating_ = false;
        refreshControls();
        return;
    }
    const int keep = selectedCompId();
    updating_ = true;
    list_->clear();
    addLastRow();
    const int count = view_ ? layer_comps_api::count(*view_) : 0;
    for (int index = 0; index < count; ++index) {
        addCompRow(index);
    }
    updating_ = false;

    QListWidgetItem* select = nullptr;
    for (int row = 0; row < list_->count(); ++row) {
        QListWidgetItem* item = list_->item(row);
        if (item->data(kIdRole).toInt() == keep) {
            select = item;
            break;
        }
    }
    if (!select && list_->count() > 0) {
        select = list_->item(0);
    }
    if (select) {
        list_->setCurrentItem(select);
    }

    refreshControls();
}

void LayerCompsPanel::refreshControls()
{
    const bool hasView = view_ != nullptr;
    const bool hasComp = currentCompItem() != nullptr;
    const bool last = selectedIsLast() && hasView && layer_comps_api::lastAvailable(*view_);
    create_->setEnabled(hasView);
    apply_->setEnabled(hasComp || last);
    previous_->setEnabled(hasView && layer_comps_api::count(*view_) > 0);
    next_->setEnabled(hasView && layer_comps_api::count(*view_) > 0);
    update_->setEnabled(hasComp);
    delete_->setEnabled(hasComp);
}

void LayerCompsPanel::selectCompId(int id)
{
    for (int row = 0; row < list_->count(); ++row) {
        if (list_->item(row)->data(kIdRole).toInt() == id) {
            list_->setCurrentRow(row);
            return;
        }
    }
}

bool LayerCompsPanel::createComp(bool skipDialog)
{
    if (!view_) {
        return false;
    }
    QString name = tr("Layer Comp %1").arg(layer_comps_api::count(*view_) + 1);
    QString comment;
    int bits = recordBits_;
    if (!skipDialog) {
        LayerCompOptionsDialog dialog(tr("New Layer Comp"), name, comment, bits, this);
        if (dialog.exec() != QDialog::Accepted) {
            return false;
        }
        name = dialog.name();
        comment = dialog.comment();
        bits = dialog.recordBits();
        recordBits_ = bits;
    }
    const int id = layer_comps_api::create(*view_, name, comment, bits);
    if (id < 0) {
        return false;
    }
    refresh();
    for (int row = 0; row < list_->count(); ++row) {
        if (list_->item(row)->data(kIdRole).toInt() == id) {
            list_->setCurrentRow(row);
            break;
        }
    }
    return true;
}

bool LayerCompsPanel::applyItem(QListWidgetItem* item)
{
    if (!view_ || !item) {
        return false;
    }
    const int id = item->data(kIdRole).toInt();
    bool applied = false;
    if (id == kLastRowId) {
        if (layer_comps_api::lastAvailable(*view_)) {
            applied = layer_comps_api::restoreLast(*view_);
        }
    } else if (id >= 0) {
        applied = layer_comps_api::apply(*view_, id);
    }
    refresh();
    return applied;
}

bool LayerCompsPanel::applySelected()
{
    return applyItem(list_->currentItem());
}

bool LayerCompsPanel::previousComp()
{
    if (!view_) {
        return false;
    }
    QList<int> ids;
    for (QListWidgetItem* item : list_->selectedItems()) {
        const int id = item->data(kIdRole).toInt();
        if (id >= 0) {
            ids.append(id);
        }
    }
    if (ids.size() < 2) {
        ids.clear();
        for (int row = 0; row < list_->count(); ++row) {
            const int id = list_->item(row)->data(kIdRole).toInt();
            if (id >= 0) {
                ids.append(id);
            }
        }
    }
    if (ids.isEmpty()) {
        return false;
    }
    const int current = selectedCompId();
    int index = ids.indexOf(current);
    if (index < 0) {
        index = ids.size();
    }
    if (index <= 0) {
        return false;
    }
    const int target = ids.at(index - 1);
    layer_comps_api::apply(*view_, target);
    refresh();
    selectCompId(target);
    return true;
}

bool LayerCompsPanel::nextComp()
{
    if (!view_) {
        return false;
    }
    QList<int> ids;
    for (QListWidgetItem* item : list_->selectedItems()) {
        const int id = item->data(kIdRole).toInt();
        if (id >= 0) {
            ids.append(id);
        }
    }
    if (ids.size() < 2) {
        ids.clear();
        for (int row = 0; row < list_->count(); ++row) {
            const int id = list_->item(row)->data(kIdRole).toInt();
            if (id >= 0) {
                ids.append(id);
            }
        }
    }
    if (ids.isEmpty()) {
        return false;
    }
    const int current = selectedCompId();
    int index = ids.indexOf(current);
    if (index < 0) {
        index = -1;
    }
    if (index + 1 >= ids.size()) {
        return false;
    }
    const int target = ids.at(index + 1);
    layer_comps_api::apply(*view_, target);
    refresh();
    selectCompId(target);
    return true;
}

bool LayerCompsPanel::updateSelected()
{
    const int id = selectedCompId();
    if (!view_ || id < 0) {
        return false;
    }
    return layer_comps_api::update(*view_, id);
}

bool LayerCompsPanel::deleteSelected()
{
    const int id = selectedCompId();
    if (!view_ || id < 0) {
        return false;
    }
    const bool deleted = layer_comps_api::remove(*view_, id);
    refresh();
    return deleted;
}

void LayerCompsPanel::renameSelected()
{
    if (QListWidgetItem* item = currentCompItem()) {
        list_->editItem(item);
    }
}

void LayerCompsPanel::openOptions()
{
    QListWidgetItem* item = currentCompItem();
    if (!view_ || !item) {
        return;
    }
    const int id = item->data(kIdRole).toInt();
    for (int index = 0; index < layer_comps_api::count(*view_); ++index) {
        if (layer_comps_api::id(*view_, index) != id) {
            continue;
        }
        LayerCompOptionsDialog dialog(tr("Layer Comp Options"),
                                      layer_comps_api::name(*view_, index),
                                      layer_comps_api::comment(*view_, index),
                                      layer_comps_api::record(*view_, index), this);
        if (dialog.exec() == QDialog::Accepted) {
            layer_comps_api::setOptions(*view_, id, dialog.name(), dialog.comment(),
                                        dialog.recordBits());
            refresh();
        }
        return;
    }
}

bool LayerCompsPanel::duplicateSelected()
{
    const int id = selectedCompId();
    if (!view_ || id < 0) {
        return false;
    }
    return layer_comps_api::duplicate(*view_, id) >= 0;
}

bool LayerCompsPanel::clearWarningSelected()
{
    const int id = selectedCompId();
    if (!view_ || id < 0) {
        return false;
    }
    return layer_comps_api::clearWarning(*view_, id);
}

bool LayerCompsPanel::clearAllWarnings()
{
    return view_ && layer_comps_api::clearAllWarnings(*view_);
}

bool LayerCompsPanel::anyWarning() const
{
    if (!view_) {
        return false;
    }
    for (int index = 0; index < layer_comps_api::count(*view_); ++index) {
        if (layer_comps_api::warning(*view_, index)) {
            return true;
        }
    }
    return false;
}

void LayerCompsPanel::showWarningMenu(const QPoint& globalPos)
{
    QMenu menu(this);
    QAction* explain = menu.addAction(tr("This comp cannot be fully restored because captured layers are missing."));
    explain->setEnabled(false);
    menu.addSeparator();
    QAction* clear = menu.addAction(tr("Clear Layer Comp Warning"));
    QAction* clearAll = menu.addAction(tr("Clear All Layer Comp Warnings"));
    clear->setEnabled(selectedCompId() >= 0);
    clearAll->setEnabled(anyWarning());
    QAction* chosen = menu.exec(globalPos);
    if (chosen == clear) {
        clearWarningSelected();
    } else if (chosen == clearAll) {
        clearAllWarnings();
    }
}

bool LayerCompsPanel::eventFilter(QObject* watched, QEvent* event)
{
    if (watched == list_ && event->type() == QEvent::KeyPress) {
        auto* key = static_cast<QKeyEvent*>(event);
        if (key->key() == Qt::Key_Delete) {
            return deleteSelected();
        }
    }
    if (watched == list_->viewport() && event->type() == QEvent::MouseButtonDblClick) {
        auto* mouse = static_cast<QMouseEvent*>(event);
        const QPoint point = mouse->position().toPoint();
        QListWidgetItem* item = list_->itemAt(point);
        if (item && item->data(kIdRole).toInt() >= 0 && point.x() > 20
            && point.x() < list_->viewport()->width() - 22) {
            list_->editItem(item);
            return true;
        }
    }
    if (watched == list_->viewport() && event->type() == QEvent::MouseButtonRelease) {
        auto* mouse = static_cast<QMouseEvent*>(event);
        const QPoint point = mouse->position().toPoint();
        QListWidgetItem* item = list_->itemAt(point);
        if (item && mouse->button() == Qt::LeftButton) {
            if (point.x() <= 20) {
                applyItem(item);
                return true;
            }
            if (item->data(kWarningRole).toBool()
                && point.x() >= list_->viewport()->width() - 22) {
                list_->setCurrentItem(item);
                showWarningMenu(mouse->globalPosition().toPoint());
                return true;
            }
        }
    }
    return QWidget::eventFilter(watched, event);
}

void LayerCompsPanel::updatePanelMenu(QMenu* menu)
{
    if (!menu) {
        return;
    }
    menu->clear();
    const bool hasComp = currentCompItem() != nullptr;
    QAction* newComp = menu->addAction(tr("New Layer Comp…"));
    QAction* options = menu->addAction(tr("Layer Comp Options…"));
    QAction* rename = menu->addAction(tr("Rename Layer Comp"));
    QAction* duplicate = menu->addAction(tr("Duplicate Layer Comp"));
    QAction* remove = menu->addAction(tr("Delete Layer Comp"));
    menu->addSeparator();
    QAction* clear = menu->addAction(tr("Clear Layer Comp Warning"));
    QAction* clearAll = menu->addAction(tr("Clear All Layer Comp Warnings"));
    options->setEnabled(hasComp);
    rename->setEnabled(hasComp);
    duplicate->setEnabled(hasComp);
    remove->setEnabled(hasComp);
    clear->setEnabled(hasComp && list_->currentItem()
                      && list_->currentItem()->data(kWarningRole).toBool());
    clearAll->setEnabled(anyWarning());

    connect(newComp, &QAction::triggered, this, [this] { createComp(false); });
    connect(options, &QAction::triggered, this, [this] { openOptions(); });
    connect(rename, &QAction::triggered, this, [this] { renameSelected(); });
    connect(duplicate, &QAction::triggered, this, [this] { duplicateSelected(); });
    connect(remove, &QAction::triggered, this, [this] { deleteSelected(); });
    connect(clear, &QAction::triggered, this, [this] { clearWarningSelected(); });
    connect(clearAll, &QAction::triggered, this, [this] { clearAllWarnings(); });
}

} // namespace pictura

#include "moc_layer_comps_panel.cpp"
