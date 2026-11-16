#pragma once

#include <QtWidgets/QDialog>

#include "save_format_options.h"

class QCheckBox;
class QRadioButton;

namespace pictura {

// Targa Options: Resolution and Compress (RLE).
class TgaOptionsDialog : public QDialog {
    Q_OBJECT
public:
    explicit TgaOptionsDialog(const SaveFormatOptions& initial, QWidget* parent = nullptr);
    int depth() const;
    bool rle() const;
private:
    QRadioButton* depth_[3] = {};
    QCheckBox* rle_ = nullptr;
};

} // namespace pictura
