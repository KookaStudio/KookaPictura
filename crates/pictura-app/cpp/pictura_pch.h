#pragma once

// The precompiled header of the optimized development build: the standard and
// Qt headers nearly every translation unit parses. Project headers stay out,
// so an edit to one does not rebuild the header and every unit that uses it;
// the generated cxx-qt headers stay out for the same reason.

#include <algorithm>
#include <functional>
#include <memory>
#include <optional>
#include <utility>
#include <vector>

#include <QtCore/QtCore>
#include <QtGui/QtGui>
#include <QtWidgets/QtWidgets>
