#pragma once

#include <QtCore/QStringList>

namespace pictura {

class PicturaMainWindow;

// File > Place: place each path in order, settling one Free Transform session
// per placed layer. A session ended by Escape or the transform's Cancel control
// removes that placement; any other non-interruption end keeps it. See the
// multi-file-place change.
void placeDocuments(PicturaMainWindow& frame, const QStringList& paths);

} // namespace pictura
