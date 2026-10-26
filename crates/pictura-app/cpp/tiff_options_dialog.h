#pragma once

#include <QtWidgets/QDialog>

#include "save_format_options.h"

class QButtonGroup;
class QCheckBox;
class QRadioButton;
class QSpinBox;

namespace pictura {

// TIFF Options (WF-1): Image Compression (with Quality for JPEG), Pixel Order,
// Byte Order, Layer Compression, Save Image Pyramid, and Save Transparency.
// Image Compression (None/LZW/JPEG) and Save Transparency reach the encoder;
// the remaining controls are shown disabled.
class TiffOptionsDialog : public QDialog {
    Q_OBJECT

public:
    explicit TiffOptionsDialog(const SaveFormatOptions& initial, QWidget* parent = nullptr);

    int compression() const;  // Qt TIFF enum: 0 None, 1 LZW, 5 JPEG
    int jpegQuality() const;  // 0-12
    int pixelOrder() const;
    int byteOrder() const;
    int layerCompression() const;
    bool pyramid() const;
    bool transparency() const;

private:
    QButtonGroup* compression_ = nullptr;
    QSpinBox* jpegQuality_ = nullptr;
    QButtonGroup* pixelOrder_ = nullptr;
    QButtonGroup* byteOrder_ = nullptr;
    QButtonGroup* layerCompression_ = nullptr;
    QCheckBox* pyramid_ = nullptr;
    QCheckBox* transparency_ = nullptr;
};

} // namespace pictura
