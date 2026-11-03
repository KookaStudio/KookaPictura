#pragma once

#include <QtCore/QString>

#include <functional>

class QWidget;

namespace pictura {

// Run `start` and, while `running` is true, pump the event loop without
// accepting user input. Timers and paints keep running; a wait cursor appears
// at once and a busy dialog after 250 ms. `finish` lands the result.
bool runBusyJob(QWidget* parent, const QString& title, const QString& label,
                const std::function<bool()>& start, const std::function<bool()>& running,
                const std::function<bool()>& finish);

// Run `start` and, while `running` is true, show a progress bar fed by
// `progress` (0 to 1000) with a Cancel button that calls `cancel`, which must
// make `running` turn false. The window appears after 250 ms. `finish` lands the
// result, or the lack of one after a cancel.
bool runCancellableJob(QWidget* parent, const QString& title, const QString& label,
                       const std::function<bool()>& start, const std::function<bool()>& running,
                       const std::function<int()>& progress, const std::function<void()>& cancel,
                       const std::function<bool()>& finish);

} // namespace pictura
