#pragma once

#include <QtWidgets/QDialog>

#include "save_format_options.h"

class QComboBox;
class QLineEdit;
class QSpinBox;

namespace pictura {

// Photoshop Raw Options: Header size and Interleave; File Type/Creator are
// macOS-only and shown disabled on Linux.
class RawOptionsDialog : public QDialog {
    Q_OBJECT
public:
    explicit RawOptionsDialog(const SaveFormatOptions& initial, QWidget* parent = nullptr);
    int header() const;
    int interleave() const;
private:
    QSpinBox* header_ = nullptr;
    QComboBox* interleave_ = nullptr;
};

} // namespace pictura
