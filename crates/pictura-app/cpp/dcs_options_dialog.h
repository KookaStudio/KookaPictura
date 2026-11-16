#pragma once

#include <QtWidgets/QDialog>

#include "save_format_options.h"

class QComboBox;

namespace pictura {

// DCS Format: version and preview.
class DcsOptionsDialog : public QDialog {
    Q_OBJECT
public:
    explicit DcsOptionsDialog(const SaveFormatOptions& initial, QWidget* parent = nullptr);
    int version() const;
    int preview() const;
private:
    QComboBox* version_ = nullptr;
    QComboBox* preview_ = nullptr;
};

} // namespace pictura
