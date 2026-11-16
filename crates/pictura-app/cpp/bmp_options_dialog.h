#pragma once

#include <QtWidgets/QDialog>

#include "save_format_options.h"

class QCheckBox;
class QRadioButton;

namespace pictura {

// BMP Options (WF-1): File Format, Depth, Compress (RLE), Flip Row Order, and
// Advanced Modes. Every control reaches the engine writer.
class BmpOptionsDialog : public QDialog {
    Q_OBJECT

public:
    explicit BmpOptionsDialog(const SaveFormatOptions& initial, QWidget* parent = nullptr);

    int format() const; // 0 Windows, 1 OS/2
    int depth() const;  // 1/4/8/16/24/32
    bool rle() const;
    bool flip() const;
    int advanced() const; // 0 X1R5G5B5, 1 A1R5G5B5, 2 R5G6B5

private:
    QRadioButton* windows_ = nullptr;
    QRadioButton* os2_ = nullptr;
    QRadioButton* depth_[6] = {};
    QCheckBox* rle_ = nullptr;
    QCheckBox* flip_ = nullptr;
    int advanced_ = 0;
};

} // namespace pictura
