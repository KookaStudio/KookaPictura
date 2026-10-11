#pragma once

#include <QtCore/QString>

#include <functional>

class QWidget;

namespace pictura {

class PictureView;

// Commit a filter whose work `start` put on a worker thread (#152): user input
// waits, a busy window with an animated bar appears if the work outlasts a
// moment, and the result lands before this returns. `name` is the filter's
// menu name. False when `start` or the worker refused.
bool runFilterCommit(QWidget* parent, PictureView* view, const QString& name,
                     const std::function<bool()>& start);

// The history label of a committed filter, e.g. "Filter: Crosshatch".
QString filterHistoryLabel(const QString& name);

} // namespace pictura
