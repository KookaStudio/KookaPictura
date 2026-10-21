#include "notes_panel.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/annotations.cxxqt.h"

#include <QtCore/QEvent>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {
constexpr int kNote = 1;
} // namespace

NotesPanel::NotesPanel(QWidget* parent)
    : QWidget(parent)
{
    setWindowTitle(QStringLiteral("Notes"));
    auto* layout = new QVBoxLayout(this);
    text_ = new QPlainTextEdit(this);
    text_->setObjectName(QStringLiteral("notesText"));
    text_->installEventFilter(this);
    layout->addWidget(text_, 1);

    auto* strip = new QHBoxLayout();
    prev_ = new QToolButton(this);
    prev_->setText(QStringLiteral("◀"));
    prev_->setToolTip(tr("Previous note"));
    next_ = new QToolButton(this);
    next_->setText(QStringLiteral("▶"));
    next_->setToolTip(tr("Next note"));
    counter_ = new QLabel(this);
    delete_ = new QToolButton(this);
    delete_->setText(tr("Delete"));
    delete_->setToolTip(tr("Delete this note"));
    strip->addWidget(prev_);
    strip->addWidget(next_);
    strip->addWidget(counter_, 1);
    strip->addWidget(delete_);
    layout->addLayout(strip);

    const auto step = [this](int delta) {
        const int count = noteCount();
        if (count > 0) {
            emit noteRequested(((current_ < 0 ? 0 : current_ + delta) + count) % count);
        }
    };
    connect(prev_, &QToolButton::clicked, this, [step]() { step(-1); });
    connect(next_, &QToolButton::clicked, this, [step]() { step(1); });
    connect(delete_, &QToolButton::clicked, this, [this]() {
        if (view_ && current_ >= 0) {
            const int index = current_;
            current_ = -1;
            remove_marker(*view_, kNote, index);
            emit noteRequested(-1);
        }
    });
    refresh();
}

void NotesPanel::setView(PictureView* view)
{
    if (view_ != view) {
        commitText();
        view_ = view;
        current_ = -1;
    }
    refresh();
}

void NotesPanel::showNote(int index)
{
    if (index != current_) {
        commitText();
        current_ = index;
    }
    refresh();
    if (current_ >= 0) {
        text_->setPlainText(note_text(*view_, current_));
        text_->setFocus();
    }
}

void NotesPanel::commitText()
{
    if (view_ && current_ >= 0 && current_ < noteCount()) {
        set_note_text(*view_, current_, text_->toPlainText());
    }
}

int NotesPanel::noteCount() const
{
    return view_ && view_->has_document() ? marker_count(*view_, kNote) : 0;
}

void NotesPanel::refresh()
{
    const int count = noteCount();
    if (current_ >= count) {
        current_ = -1;
    }
    const bool open = current_ >= 0;
    counter_->setText(open ? tr("%1 of %2").arg(current_ + 1).arg(count)
                           : (count > 0 ? tr("%1 notes").arg(count) : tr("No notes")));
    text_->setEnabled(open);
    delete_->setEnabled(open);
    prev_->setEnabled(count > 0);
    next_->setEnabled(count > 0);
    // Leave the text alone while it is being typed; an undo reloads it.
    if (!open) {
        text_->clear();
    } else if (!text_->hasFocus()) {
        text_->setPlainText(note_text(*view_, current_));
    }
}

bool NotesPanel::eventFilter(QObject* watched, QEvent* event)
{
    if (watched == text_ && event->type() == QEvent::FocusOut) {
        commitText();
    }
    return QWidget::eventFilter(watched, event);
}

} // namespace pictura

#include "moc_notes_panel.cpp"
