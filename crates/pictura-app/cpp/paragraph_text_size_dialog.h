#pragma once

#include <QtCore/QSizeF>

class QWidget;

namespace pictura {

// The Type tools' Alt-click "Paragraph Text Size" dialog: the box Width and
// Height in pixels. Edits `size` and returns true on OK.
bool execParagraphTextSizeDialog(QSizeF& size, QWidget* parent);

} // namespace pictura
