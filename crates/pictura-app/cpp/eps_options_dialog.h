#pragma once

#include <QtWidgets/QDialog>

#include "save_format_options.h"

class QCheckBox;
class QComboBox;
class QSpinBox;

namespace pictura {

// Photoshop EPS Options.
class EpsOptionsDialog : public QDialog {
    Q_OBJECT
public:
    explicit EpsOptionsDialog(const SaveFormatOptions& initial, QWidget* parent = nullptr);
    int preview() const;
    int encoding() const;
    int jpegQuality() const;
    bool halftone() const;
    bool transfer() const;
    bool transparentWhites() const;
    bool psColorManagement() const;
    bool vectorData() const;
    bool interpolation() const;
private:
    QComboBox* preview_ = nullptr;
    QComboBox* encoding_ = nullptr;
    QSpinBox* quality_ = nullptr;
    QCheckBox* halftone_ = nullptr;
    QCheckBox* transfer_ = nullptr;
    QCheckBox* transparentWhites_ = nullptr;
    QCheckBox* psColorManagement_ = nullptr;
    QCheckBox* vectorData_ = nullptr;
    QCheckBox* interpolation_ = nullptr;
};

} // namespace pictura
