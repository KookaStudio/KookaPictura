#include "history_panel.h"

#include "api/history_api.h"
#include "icons.h"

#include <QtCore/QSize>
#include <QtCore/QTimer>
#include <QtCore/QVariant>
#include <QtGui/QFont>
#include <QtGui/QMouseEvent>
#include <QtGui/QPixmap>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

namespace pictura {

namespace {

constexpr int kKindRole = Qt::UserRole;
constexpr int kIndexRole = Qt::UserRole + 1;
constexpr int kStateKind = 0;
constexpr int kSnapshotKind = 1;
constexpr int kSourceIconSize = 16;
// The left column that chooses the History Brush source: the icon plus the
// item's leading margin.
constexpr int kSourceColumnWidth = kSourceIconSize + 6;

// The History Brush marker on the source row, or a blank of the same size so
// every label lines up.
QIcon sourceIcon(bool source)
{
    if (source) {
        return pictura::icon(QStringLiteral("tool.historybrush"));
    }
    QPixmap blank(kSourceIconSize, kSourceIconSize);
    blank.fill(Qt::transparent);
    return QIcon(blank);
}

} // namespace

HistoryPanel::HistoryPanel(QWidget* parent)
    : QWidget(parent)
{
    QWidget* body = this;
    auto* layout = new QVBoxLayout(body);
    list_ = new QListWidget(body);
    list_->setIconSize(QSize(kSourceIconSize, kSourceIconSize));
    list_->viewport()->installEventFilter(this);
    layout->addWidget(list_, 1);
    snapshotButton_ = new QPushButton(tr("Create Snapshot"), body);
    snapshotButton_->setObjectName(QStringLiteral("snapshotButton"));
    snapshotButton_->setIcon(pictura::icon(QStringLiteral("history.snapshot")));
    snapshotButton_->setIconSize(QSize(20, 20));
    snapshotButton_->setToolTip(tr("Create New Snapshot"));
    layout->addWidget(snapshotButton_);

    connect(list_, &QListWidget::itemClicked, this, [this](QListWidgetItem* item) { activate(item); });
    connect(snapshotButton_, &QPushButton::clicked, this, [this] { createSnapshot(); });
}

void HistoryPanel::setView(PictureView* view)
{
    if (viewConnection_) {
        QObject::disconnect(viewConnection_);
    }
    view_ = view;
    if (view_) {
        viewConnection_ = history_api::connectChanged(*view_, this, [this] {
            QTimer::singleShot(0, this, [this] { refresh(); });
        });
    }
    refresh();
}

void HistoryPanel::refresh()
{
    list_->clear();
    if (!view_) {
        return;
    }

    const int sourceSnapshot = history_api::brushSourceSnapshot(*view_);
    const int sourceState = history_api::brushSourceState(*view_);
    const int snapshots = history_api::snapshotCount(*view_);
    for (int i = 0; i < snapshots; ++i) {
        auto* item = new QListWidgetItem(sourceIcon(i == sourceSnapshot),
                                         history_api::snapshotLabel(*view_, i), list_);
        item->setData(kKindRole, kSnapshotKind);
        item->setData(kIndexRole, i);
        item->setToolTip(tr("Snapshot"));
    }

    const int count = history_api::count(*view_);
    const int current = history_api::index(*view_);
    QListWidgetItem* currentItem = nullptr;
    for (int i = 0; i < count; ++i) {
        const bool isCurrent = i == current;
        auto* item = new QListWidgetItem(
            sourceIcon(i == sourceState),
            (isCurrent ? QStringLiteral("> ") : QString()) + history_api::label(*view_, i), list_);
        item->setData(kKindRole, kStateKind);
        item->setData(kIndexRole, i);
        if (isCurrent) {
            QFont font = item->font();
            font.setBold(true);
            item->setFont(font);
            currentItem = item;
        }
    }
    if (currentItem) {
        list_->setCurrentItem(currentItem);
    }
}

void HistoryPanel::activate(QListWidgetItem* item)
{
    if (!view_ || !item) {
        return;
    }
    const int index = item->data(kIndexRole).toInt();
    if (item->data(kKindRole).toInt() == kSnapshotKind) {
        history_api::restoreSnapshot(*view_, index);
    } else {
        history_api::jump(*view_, index);
    }
}

bool HistoryPanel::eventFilter(QObject* watched, QEvent* event)
{
    if (watched == list_->viewport() && event->type() == QEvent::MouseButtonPress && view_) {
        const auto* press = static_cast<QMouseEvent*>(event);
        const QPoint pos = press->position().toPoint();
        QListWidgetItem* item = list_->itemAt(pos);
        if (item && pos.x() - list_->visualItemRect(item).left() < kSourceColumnWidth) {
            history_api::setBrushSource(*view_, item->data(kKindRole).toInt() == kSnapshotKind,
                                        item->data(kIndexRole).toInt());
            refresh();
            return true;
        }
    }
    return QWidget::eventFilter(watched, event);
}

bool HistoryPanel::performPanelMenuAction(const QString& actionId)
{
    if (actionId == QLatin1String("stepForward")) {
        if (view_) {
            history_api::redo(*view_);
        }
    } else if (actionId == QLatin1String("stepBackward")) {
        if (view_) {
            history_api::undo(*view_);
        }
    } else if (actionId == QLatin1String("newSnapshot")) {
        createSnapshot();
    } else {
        return false;
    }
    return true;
}

void HistoryPanel::createSnapshot()
{
    if (!view_) {
        return;
    }
    bool accepted = false;
    const QString label = QInputDialog::getText(this, tr("Create Snapshot"), tr("Label:"),
                                                QLineEdit::Normal, tr("Snapshot"), &accepted);
    if (!accepted || label.isEmpty()) {
        return;
    }
    history_api::addSnapshot(*view_, label);
}

} // namespace pictura

#include "moc_history_panel.cpp"
