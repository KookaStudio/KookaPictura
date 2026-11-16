#pragma once

#include <QtWidgets/QDialog>

#include "save_format_options.h"

namespace pictura {

// Cineon Options: the fixed 10-bit RGB conversion.
class CineonOptionsDialog : public QDialog {
    Q_OBJECT
public:
    explicit CineonOptionsDialog(const SaveFormatOptions& initial, QWidget* parent = nullptr);
    int bits() const { return 10; }
};

} // namespace pictura
