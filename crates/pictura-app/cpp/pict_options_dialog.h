#pragma once

#include <QtWidgets/QDialog>

#include "save_format_options.h"

class QRadioButton;

namespace pictura {

// PICT Options: Resolution (16 or 32 bits/pixel) and Compression (None or a
// JPEG level).
class PictOptionsDialog : public QDialog {
    Q_OBJECT
public:
    explicit PictOptionsDialog(const SaveFormatOptions& initial, QWidget* parent = nullptr);
    int resolution() const;
    int compression() const;
private:
    QRadioButton* resolution_[2] = {};
    QRadioButton* compression_[5] = {};
};

} // namespace pictura
