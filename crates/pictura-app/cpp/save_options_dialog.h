#pragma once

#include <QtWidgets/QDialog>

class QCheckBox;

namespace pictura {

// The per-save PSD/PSB/JPEG/TIFF options dialog. The only option in this
// bundle is whether the document profile is embedded.
class SaveOptionsDialog : public QDialog {
    Q_OBJECT

public:
    explicit SaveOptionsDialog(bool embedDefault, bool profileAvailable,
                               QWidget* parent = nullptr);

    bool embedProfile() const;
    void setEmbedForTest(bool on);

private:
    QCheckBox* embed_ = nullptr;
};

} // namespace pictura
