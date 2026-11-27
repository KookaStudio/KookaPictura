#include "history_panel.h"

#include "api/history_api.h"
#include "memory_refusal.h"
#include "icons.h"
#include "session.h"

#include <QtCore/QSize>
#include <QtCore/QTimer>
#include <QtCore/QVariant>
#include <QtGui/QFont>
#include <QtGui/QMouseEvent>
#include <QtGui/QPixmap>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
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
    const SessionState options = pictura::loadSession();
    autoFirstSnapshot_ = options.historyAutoFirstSnapshot;
    autoSnapshotOnSave_ = options.historyAutoSnapshotOnSave;
    allowNonLinear_ = options.historyAllowNonLinear;
    showSnapshotDialog_ = options.historyShowSnapshotDialog;
    visibilityUndoable_ = options.historyVisibilityUndoable;

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
    connect(snapshotButton_, &QPushButton::clicked, this,
            [this] { createSnapshot(showSnapshotDialog_); });
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
        applyOptionsToView();
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
        if (!history_api::restoreSnapshot(*view_, index)) {
            reportMemoryRefusal(this, tr("restore the snapshot"));
        }
    } else if (!history_api::jump(*view_, index)) {
        reportMemoryRefusal(this, tr("step through history"));
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

HistoryPanel::Target HistoryPanel::selectedTarget() const
{
    if (!view_) {
        return {};
    }
    if (QListWidgetItem* item = list_ ? list_->currentItem() : nullptr) {
        return {item->data(kKindRole).toInt() == kSnapshotKind,
                item->data(kIndexRole).toInt()};
    }
    return {false, history_api::index(*view_)};
}

void HistoryPanel::clearHistory(bool purge)
{
    if (!view_) {
        return;
    }
    const bool cleared =
        purge ? history_api::purgeAll(*view_) : history_api::purgeUndo(*view_);
    if (cleared) {
        refresh();
    }
}

void HistoryPanel::openHistoryOptions()
{
    QDialog dialog(this);
    dialog.setObjectName(QStringLiteral("historyOptionsDialog"));
    dialog.setWindowTitle(tr("History Options"));
    auto* layout = new QVBoxLayout(&dialog);
    auto* first = new QCheckBox(tr("Automatically Create First Snapshot"), &dialog);
    first->setObjectName(QStringLiteral("historyOptionFirstSnapshot"));
    first->setChecked(autoFirstSnapshot_);
    auto* save = new QCheckBox(tr("Automatically Create New Snapshot When Saving"), &dialog);
    save->setObjectName(QStringLiteral("historyOptionAutoSave"));
    save->setChecked(autoSnapshotOnSave_);
    auto* nonlinear = new QCheckBox(tr("Allow Non-Linear History"), &dialog);
    nonlinear->setObjectName(QStringLiteral("historyOptionNonLinear"));
    nonlinear->setChecked(allowNonLinear_);
    auto* prompt = new QCheckBox(tr("Show New Snapshot Dialog By Default"), &dialog);
    prompt->setObjectName(QStringLiteral("historyOptionSnapshotDialog"));
    prompt->setChecked(showSnapshotDialog_);
    auto* visibility = new QCheckBox(tr("Make Layer Visibility Changes Undoable"), &dialog);
    visibility->setObjectName(QStringLiteral("historyOptionVisibilityUndo"));
    visibility->setChecked(visibilityUndoable_);
    layout->addWidget(first);
    layout->addWidget(save);
    layout->addWidget(nonlinear);
    layout->addWidget(prompt);
    layout->addWidget(visibility);
    auto* buttons =
        new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    if (dialog.exec() != QDialog::Accepted) {
        return;
    }
    autoFirstSnapshot_ = first->isChecked();
    autoSnapshotOnSave_ = save->isChecked();
    allowNonLinear_ = nonlinear->isChecked();
    showSnapshotDialog_ = prompt->isChecked();
    visibilityUndoable_ = visibility->isChecked();
    persistOptions();
    applyOptionsToView();
}

void HistoryPanel::applyOptionsToView()
{
    if (view_) {
        history_api::setHistoryOptions(*view_, allowNonLinear_, visibilityUndoable_,
                                       autoSnapshotOnSave_);
    }
}

void HistoryPanel::setOptionsForTest(bool autoFirst, bool autoSave, bool nonLinear,
                                     bool showDialog, bool visibilityUndoable)
{
    autoFirstSnapshot_ = autoFirst;
    autoSnapshotOnSave_ = autoSave;
    allowNonLinear_ = nonLinear;
    showSnapshotDialog_ = showDialog;
    visibilityUndoable_ = visibilityUndoable;
    applyOptionsToView();
}

void HistoryPanel::persistOptions()
{
    SessionState state = pictura::loadSession();
    state.historyAutoFirstSnapshot = autoFirstSnapshot_;
    state.historyAutoSnapshotOnSave = autoSnapshotOnSave_;
    state.historyAllowNonLinear = allowNonLinear_;
    state.historyShowSnapshotDialog = showSnapshotDialog_;
    state.historyVisibilityUndoable = visibilityUndoable_;
    pictura::saveSession(state);
}

bool HistoryPanel::performPanelMenuAction(const QString& actionId)
{
    if (actionId == QLatin1String("stepForward")) {
        if (view_ && !history_api::redo(*view_)) {
            reportMemoryRefusal(this, tr("redo"));
        }
    } else if (actionId == QLatin1String("stepBackward")) {
        if (view_ && !history_api::undo(*view_)) {
            reportMemoryRefusal(this, tr("undo"));
        }
    } else if (actionId == QLatin1String("newSnapshot")) {
        createSnapshot(true);
    } else if (actionId == QLatin1String("clearHistory")) {
        clearHistory(QApplication::keyboardModifiers().testFlag(Qt::AltModifier));
    } else if (actionId == QLatin1String("clearHistoryPurge")) {
        clearHistory(true);
    } else if (actionId == QLatin1String("historyOptions")) {
        openHistoryOptions();
    } else {
        return false;
    }
    return true;
}

void HistoryPanel::createSnapshot(bool prompt)
{
    if (!view_) {
        return;
    }
    QString label;
    if (prompt) {
        bool accepted = false;
        label = QInputDialog::getText(
            this, tr("Create Snapshot"), tr("Label:"), QLineEdit::Normal,
            tr("Snapshot %1").arg(history_api::snapshotCount(*view_) + 1), &accepted);
        if (!accepted || label.isEmpty()) {
            return;
        }
    } else {
        label = tr("Snapshot %1").arg(history_api::snapshotCount(*view_) + 1);
    }
    if (!history_api::addSnapshot(*view_, label)) {
        reportMemoryRefusal(this, tr("create the snapshot"));
    }
}

} // namespace pictura

#include "moc_history_panel.cpp"
