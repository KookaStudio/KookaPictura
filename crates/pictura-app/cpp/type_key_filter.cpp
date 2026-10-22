// Key routing for an open Type session, lifted out of tool_type.cpp.

#include "type_key_filter.h"

#include "text_input.h"

#include <QtCore/QEvent>
#include <QtGui/QKeyEvent>
#include <QtWidgets/QWidget>

namespace pictura {

bool commitKey(const QKeyEvent& key)
{
    const bool enter = key.key() == Qt::Key_Return || key.key() == Qt::Key_Enter;
    return enter
        && (key.modifiers().testFlag(Qt::ControlModifier)
            || key.modifiers().testFlag(Qt::KeypadModifier));
}

bool sessionKey(const QKeyEvent& key)
{
    if (commitKey(key)) {
        return true;
    }
    if (key.modifiers() & Qt::MetaModifier) {
        return false;
    }
    const bool ctrl = key.modifiers() & Qt::ControlModifier;
    switch (key.key()) {
    case Qt::Key_Return:
    case Qt::Key_Enter:
    case Qt::Key_Escape:
    case Qt::Key_Backspace:
    case Qt::Key_Delete:
    case Qt::Key_Left:
    case Qt::Key_Right:
    case Qt::Key_Up:
    case Qt::Key_Down:
    case Qt::Key_Home:
    case Qt::Key_End:
        return true;
    case Qt::Key_A:
    case Qt::Key_C:
    case Qt::Key_X:
    case Qt::Key_V:
        if (ctrl) {
            return true;
        }
        break;
    default:
        break;
    }
    return !ctrl && !key.text().isEmpty() && key.text().at(0).isPrint();
}

bool TypeKeyFilter::eventFilter(QObject* watched, QEvent* event)
{
    if (event->type() != QEvent::ShortcutOverride && event->type() != QEvent::KeyPress) {
        return false;
    }
    auto* widget = qobject_cast<QWidget*>(watched);
    QWidget* window = window_();
    if (!widget || !window || widget->window() != window || isTextInput(widget)) {
        return false;
    }
    auto* key = static_cast<QKeyEvent*>(event);
    if (!sessionKey(*key)) {
        return false;
    }
    if (event->type() == QEvent::ShortcutOverride) {
        event->accept();
    } else {
        key_(*key);
    }
    return true;
}

} // namespace pictura
