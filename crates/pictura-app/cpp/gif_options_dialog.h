#pragma once

#include <QtWidgets/QDialog>

#include "save_format_options.h"

class QRadioButton;

namespace pictura {

// GIF Options: Row Order.
class GifOptionsDialog : public QDialog {
    Q_OBJECT
public:
    explicit GifOptionsDialog(const SaveFormatOptions& initial, QWidget* parent = nullptr);
    int rowOrder() const; // 0 Normal, 1 Interlaced
private:
    QRadioButton* normal_ = nullptr;
    QRadioButton* interlaced_ = nullptr;
};

} // namespace pictura
