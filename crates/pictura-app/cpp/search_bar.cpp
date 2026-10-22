#include "search_bar.h"

#include <QtCore/QTimer>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtWidgets/QApplication>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QStyledItemDelegate>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>

namespace pictura {

namespace {

constexpr int kMaxVisibleRows = 10;
constexpr int kFlashPulses = 6;
constexpr int kFlashIntervalMs = 180;

// Greys a row that cannot run, and draws the entry's shortcut at the right.
class SearchResultDelegate : public QStyledItemDelegate {
public:
    using QStyledItemDelegate::QStyledItemDelegate;

protected:
    void initStyleOption(QStyleOptionViewItem* option, const QModelIndex& index) const override
    {
        QStyledItemDelegate::initStyleOption(option, index);
        if (!index.data(SearchBar::kRunnableRole).toBool()) {
            option->state &= ~QStyle::State_Enabled;
        }
    }

    void paint(QPainter* painter, const QStyleOptionViewItem& option,
               const QModelIndex& index) const override
    {
        QStyledItemDelegate::paint(painter, option, index);
        const QString shortcut = index.data(SearchBar::kShortcutRole).toString();
        if (shortcut.isEmpty()) {
            return;
        }
        QStyleOptionViewItem opt(option);
        initStyleOption(&opt, index);
        const QPalette::ColorGroup group =
            (opt.state & QStyle::State_Enabled) ? QPalette::Normal : QPalette::Disabled;
        const QPalette::ColorRole role =
            (opt.state & QStyle::State_Selected) ? QPalette::HighlightedText : QPalette::Text;
        painter->save();
        painter->setPen(opt.palette.color(group, role));
        painter->drawText(opt.rect.adjusted(0, 0, -8, 0), Qt::AlignRight | Qt::AlignVCenter,
                          shortcut);
        painter->restore();
    }
};

bool isEnter(const QKeyEvent* key)
{
    return key->key() == Qt::Key_Return || key->key() == Qt::Key_Enter;
}

bool isBarKey(const QKeyEvent* key)
{
    return isEnter(key) || key->key() == Qt::Key_Escape || key->key() == Qt::Key_Up
        || key->key() == Qt::Key_Down;
}

} // namespace

SearchBar::SearchBar(QWidget* parent) : QFrame(parent)
{
    setObjectName(QStringLiteral("searchBar"));
    setAttribute(Qt::WA_StyledBackground, true);
    hide();

    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(8, 8, 8, 6);
    layout->setSpacing(6);

    input_ = new QLineEdit(this);
    input_->setObjectName(QStringLiteral("searchInput"));
    input_->setPlaceholderText(tr("Search commands, tools, panels, and settings"));
    input_->setClearButtonEnabled(true);
    layout->addWidget(input_);

    results_ = new QListWidget(this);
    results_->setObjectName(QStringLiteral("searchResults"));
    // The field keeps the focus; a click on a row runs it without taking it.
    results_->setFocusPolicy(Qt::NoFocus);
    results_->setUniformItemSizes(true);
    results_->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    results_->setItemDelegate(new SearchResultDelegate(results_));
    layout->addWidget(results_);

    empty_ = new QLabel(tr("No matches"), this);
    empty_->setObjectName(QStringLiteral("searchEmpty"));
    layout->addWidget(empty_);

    auto* footer = new QLabel(tr("Enter run  ·  Alt+Enter show where it is  ·  Esc close"), this);
    footer->setObjectName(QStringLiteral("searchFooter"));
    layout->addWidget(footer);

    connect(input_, &QLineEdit::textChanged, this, &SearchBar::updateResults);
    connect(results_, &QListWidget::itemClicked, this,
            [this](QListWidgetItem* item) { activate(results_->row(item), false); });
}

void SearchBar::open(const QList<SearchEntry>& entries, const QRect& host)
{
    entries_ = entries;
    host_ = host;
    previousFocus_ = QApplication::focusWidget();
    closing_ = false;
    input_->clear();
    updateResults();
    qApp->installEventFilter(this);
    show();
    raise();
    input_->setFocus(Qt::ShortcutFocusReason);
}

void SearchBar::dismiss()
{
    if (closing_ || !isVisible()) {
        return;
    }
    closing_ = true;
    qApp->removeEventFilter(this);
    // Move the focus out before hiding: a hidden field that keeps it would
    // take the next Shift+Space as a typed space, and with no earlier focus
    // Qt may instead hand it to whichever text field comes next.
    if (input_->hasFocus()) {
        if (previousFocus_ && previousFocus_->isVisible() && previousFocus_ != input_) {
            previousFocus_->setFocus(Qt::OtherFocusReason);
        } else {
            window()->setFocus(Qt::OtherFocusReason);
        }
    }
    hide();
    closing_ = false;
}

const SearchEntry* SearchBar::entryAt(int row) const
{
    if (row < 0 || row >= rows_.size()) {
        return nullptr;
    }
    return &entries_.at(rows_.at(row));
}

void SearchBar::updateResults()
{
    results_->clear();
    rows_.clear();
    for (const SearchHit& hit : searchIndex(entries_, input_->text())) {
        const SearchEntry& entry = entries_.at(hit.index);
        auto* item = new QListWidgetItem(entry.path.join(QStringLiteral(" > ")), results_);
        item->setData(kRunnableRole, entry.runnable);
        item->setData(kShortcutRole, entry.shortcut);
        if (!entry.runnable) {
            item->setToolTip(entry.implemented ? tr("Not available right now")
                                               : tr("Not implemented yet"));
        }
        rows_ << hit.index;
    }
    if (!rows_.isEmpty()) {
        results_->setCurrentRow(0);
    }
    relayout();
}

void SearchBar::relayout()
{
    const bool typed = !input_->text().trimmed().isEmpty();
    const int rows = std::min(int(rows_.size()), kMaxVisibleRows);
    results_->setVisible(rows > 0);
    empty_->setVisible(typed && rows == 0);
    if (rows > 0) {
        results_->setFixedHeight(rows * results_->sizeHintForRow(0) + 2 * results_->frameWidth());
    }

    const int width = std::clamp(host_.width() * 3 / 5, std::min(360, host_.width() - 24), 640);
    setFixedWidth(std::max(width, 200));
    adjustSize();
    // Centred over the host, in its upper third.
    move(host_.center().x() - this->width() / 2, host_.top() + host_.height() / 6);
}

void SearchBar::moveSelection(int delta)
{
    if (rows_.isEmpty()) {
        return;
    }
    const int row = std::clamp(results_->currentRow() + delta, 0, int(rows_.size()) - 1);
    results_->setCurrentRow(row);
    results_->scrollToItem(results_->item(row));
}

void SearchBar::activate(int row, bool reveal)
{
    const SearchEntry* entry = entryAt(row);
    if (!entry || (!reveal && !entry->runnable)) {
        return;
    }
    const SearchEntry chosen = *entry;
    dismiss();
    if (reveal) {
        Q_EMIT revealRequested(chosen);
    } else {
        Q_EMIT runRequested(chosen);
    }
}

bool SearchBar::handleKey(QKeyEvent* event)
{
    switch (event->key()) {
    case Qt::Key_Escape:
        dismiss();
        return true;
    case Qt::Key_Up:
        moveSelection(-1);
        return true;
    case Qt::Key_Down:
        moveSelection(1);
        return true;
    case Qt::Key_Return:
    case Qt::Key_Enter:
        activate(results_->currentRow(), event->modifiers().testFlag(Qt::AltModifier));
        return true;
    default:
        return false;
    }
}

// Installed on the application only while the bar is open.
bool SearchBar::eventFilter(QObject* watched, QEvent* event)
{
    if (!isVisible()) {
        return QFrame::eventFilter(watched, event);
    }
    switch (event->type()) {
    case QEvent::ShortcutOverride:
        // The field's keys must not reach a window shortcut (Enter, arrows).
        if (watched == input_ && isBarKey(static_cast<QKeyEvent*>(event))) {
            event->accept();
            return true;
        }
        break;
    case QEvent::KeyPress:
        if (watched == input_ && handleKey(static_cast<QKeyEvent*>(event))) {
            return true;
        }
        break;
    case QEvent::FocusOut:
        if (watched == input_
            && static_cast<QFocusEvent*>(event)->reason() != Qt::PopupFocusReason) {
            dismiss();
        }
        break;
    case QEvent::MouseButtonPress:
    case QEvent::NonClientAreaMouseButtonPress: {
        const auto* mouse = static_cast<QMouseEvent*>(event);
        const QRect frame(mapToGlobal(QPoint(0, 0)), size());
        // Close without consuming, so the press still reaches its target.
        if (!frame.contains(mouse->globalPosition().toPoint())) {
            dismiss();
        }
        break;
    }
    default:
        break;
    }
    return QFrame::eventFilter(watched, event);
}

void flashWidget(QWidget* target)
{
    if (!target || !target->isVisible()) {
        return;
    }
    QWidget* window = target->window();
    auto* flash = new QWidget(window);
    flash->setObjectName(QStringLiteral("searchFlash"));
    flash->setAttribute(Qt::WA_TransparentForMouseEvents, true);
    flash->setAttribute(Qt::WA_StyledBackground, true);
    flash->setGeometry(
        QRect(target->mapTo(window, QPoint(0, 0)), target->size()).adjusted(-2, -2, 2, 2));
    flash->show();
    flash->raise();
    auto* timer = new QTimer(flash);
    timer->setInterval(kFlashIntervalMs);
    QObject::connect(timer, &QTimer::timeout, flash, [flash, pulses = 0]() mutable {
        if (++pulses >= kFlashPulses) {
            flash->deleteLater();
            return;
        }
        flash->setVisible(!flash->isVisible());
    });
    timer->start();
}

} // namespace pictura

#include "moc_search_bar.cpp"
