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

} // namespace pictura
