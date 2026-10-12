#pragma once

// True for the editable text widgets an application-level key must leave
// alone: line edits, spin boxes, plain/multi-line text edits, and editable
// combo boxes. Shared by the Type session's key filter and the frame's
// Delete-key routing.

#include <QtWidgets/QAbstractSpinBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QTextEdit>
#include <QtWidgets/QWidget>

namespace pictura {

inline bool isTextInput(QWidget* w)
{
    return qobject_cast<QLineEdit*>(w) || qobject_cast<QAbstractSpinBox*>(w)
        || qobject_cast<QTextEdit*>(w) || qobject_cast<QPlainTextEdit*>(w)
        || (qobject_cast<QComboBox*>(w) && static_cast<QComboBox*>(w)->isEditable());
}

} // namespace pictura
