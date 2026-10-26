#pragma once

#include <QtWidgets/QDialog>

#include "save_format_options.h"

class QRadioButton;

namespace pictura {

// PNG Options (WF-1): Compression (None/Fast … Smallest/Slow) and Interlace.
// Compression reaches the encoder; Interlace is shown disabled because Qt's PNG
// writer exposes no interlace control.
class PngOptionsDialog : public QDialog {
    Q_OBJECT

public:
    explicit PngOptionsDialog(const SaveFormatOptions& initial, QWidget* parent = nullptr);

    int compression() const; // 0-9
    int interlace() const;   // 0 None, 1 Interlaced

private:
    QRadioButton* fast_ = nullptr;
    QRadioButton* slow_ = nullptr;
    QRadioButton* none_ = nullptr;
    QRadioButton* interlaced_ = nullptr;
};

} // namespace pictura
