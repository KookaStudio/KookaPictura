#pragma once

// The keys a Type session takes from the window while it is open.

#include <QtCore/QObject>

#include <functional>
#include <utility>

class QKeyEvent;
class QWidget;

namespace pictura {

// Ctrl+Enter (or keypad Enter) commits the session.
bool commitKey(const QKeyEvent& key);
// The keys a typing session takes: the window's single-letter tool shortcuts
// go quiet, as CS6's do while there is a text cursor, and Ctrl+A / C / X / V
// work on the text. Other Ctrl/Meta chords still reach their shortcuts.
bool sessionKey(const QKeyEvent& key);

// Routes the window's key events to the session while it is open: an
// application filter, because the shortcut map consults the focus widget,
// which need not be the canvas. Text fields keep their keys.
class TypeKeyFilter : public QObject {
public:
    TypeKeyFilter(std::function<QWidget*()> window, std::function<void(const QKeyEvent&)> key)
        : window_(std::move(window))
        , key_(std::move(key))
    {
    }

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    std::function<QWidget*()> window_;
    std::function<void(const QKeyEvent&)> key_;
};

} // namespace pictura
