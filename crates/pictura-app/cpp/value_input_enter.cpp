#include "value_input_enter.h"

#include "panels/numeric_field.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QEvent>
#include <QtCore/QMetaObject>
#include <QtGui/QKeyEvent>
#include <QtWidgets/QAbstractSpinBox>
#include <QtWidgets/QApplication>
#include <QtWidgets/QColorDialog>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QCompleter>
#include <QtWidgets/QDialog>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QFontComboBox>
#include <QtWidgets/QFontDialog>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QWidget>

namespace pictura {
namespace {

// Enter belongs to the focused value input: it commits the current value and
// leaves the field, so the key never reaches a tool or a dialog's default
// button. Shift+Enter commits and keeps the value selected instead.
class ValueInputEnterFilter : public QObject {
public:
    bool eventFilter(QObject* watched, QEvent* event) override
    {
        if (event->type() != QEvent::KeyPress) {
            return false;
        }
        auto* key = static_cast<QKeyEvent*>(event);
        if (key->key() != Qt::Key_Return && key->key() != Qt::Key_Enter) {
            return false;
        }
        if (key->isAutoRepeat()
            || (key->modifiers()
                & (Qt::ControlModifier | Qt::AltModifier | Qt::MetaModifier))) {
            return false;
        }
        auto* target = qobject_cast<QWidget*>(watched);
        QWidget* field = valueFieldFor(target);
        if (!field) {
            return false;
        }
        QWidget* focus = QApplication::focusWidget();
        if (!focus) {
            focus = target;
        }
        commitField(field);
        if (key->modifiers().testFlag(Qt::ShiftModifier)) {
            selectAll(field, focus);
        } else if (focus) {
            focus->clearFocus();
        } else {
            field->clearFocus();
        }
        event->accept();
        return true;
    }

private:
    // The value field that owns `target`, or nullptr when the key belongs to
    // something else. A NumericField handles its own Enter, so its edit is
    // skipped here.
    static QWidget* valueFieldFor(QWidget* target)
    {
        if (!target || stockDialog(target->window())) {
            return nullptr;
        }
        for (QWidget* widget = target; widget; widget = widget->parentWidget()) {
            if (qobject_cast<NumericField*>(widget)) {
                return nullptr;
            }
            if (auto* spin = qobject_cast<QAbstractSpinBox*>(widget)) {
                return spin->isReadOnly() ? nullptr : widget;
            }
            if (auto* combo = qobject_cast<QComboBox*>(widget);
                combo && (combo->isEditable() || qobject_cast<QFontComboBox*>(combo))) {
                return widget;
            }
            if (widget->isWindow()) {
                break;
            }
        }
        // A plain dialog text field. Line edits in the shell stay untouched:
        // search, filter, and rename fields keep their own Enter handling.
        if (auto* edit = qobject_cast<QLineEdit*>(target);
            edit && !edit->isReadOnly() && qobject_cast<QDialog*>(edit->window())) {
            return edit;
        }
        return nullptr;
    }

    // Qt's own dialogs keep their Enter: a file name or a stock color field
    // accepts the dialog, and the app does not own their commit path.
    static bool stockDialog(QWidget* window)
    {
        return qobject_cast<QFileDialog*>(window) || qobject_cast<QColorDialog*>(window)
            || qobject_cast<QFontDialog*>(window) || qobject_cast<QInputDialog*>(window);
    }

    static void commitField(QWidget* field)
    {
        if (auto* spin = qobject_cast<QAbstractSpinBox*>(field)) {
            spin->interpretText();
            return;
        }
        if (auto* combo = qobject_cast<QComboBox*>(field)) {
            acceptCompletion(combo);
            if (QLineEdit* edit = combo->lineEdit()) {
                QMetaObject::invokeMethod(edit, "editingFinished");
            }
            if (auto* fonts = qobject_cast<QFontComboBox*>(combo)) {
                commitFont(fonts);
            }
            return;
        }
        if (auto* edit = qobject_cast<QLineEdit*>(field)) {
            QMetaObject::invokeMethod(edit, "editingFinished");
        }
    }

    // Enter takes the completion the completer highlights, as a popup pick
    // would, before the owner's commit path reads the text.
    static void acceptCompletion(QComboBox* combo)
    {
        QCompleter* completer = combo->completer();
        if (!completer || !completer->popup() || !completer->popup()->isVisible()) {
            return;
        }
        const QString completion = completer->currentCompletion();
        if (!completion.isEmpty()) {
            combo->setEditText(completion);
        }
    }

    // A typed installed family becomes the current font; anything else reverts
    // to the family the combo already had.
    static void commitFont(QFontComboBox* combo)
    {
        const int index = combo->findText(combo->currentText(), Qt::MatchFixedString);
        if (index >= 0) {
            if (index != combo->currentIndex()) {
                combo->setCurrentIndex(index);
            }
            return;
        }
        combo->setEditText(combo->currentFont().family());
    }

    static void selectAll(QWidget* field, QWidget* focus)
    {
        if (auto* edit = qobject_cast<QLineEdit*>(focus)) {
            edit->selectAll();
            return;
        }
        if (auto* edit = field->findChild<QLineEdit*>()) {
            edit->selectAll();
        }
    }
};

} // namespace

void installValueInputEnter()
{
    static ValueInputEnterFilter* filter = nullptr;
    if (filter) {
        return;
    }
    QCoreApplication* app = QApplication::instance();
    if (!app) {
        return;
    }
    filter = new ValueInputEnterFilter;
    app->installEventFilter(filter);
}

} // namespace pictura
