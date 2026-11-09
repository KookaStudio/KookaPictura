#pragma once

#include <QtCore/QString>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QSpinBox>

class QSlider;

namespace pictura {

class PictureView;

// Photoshop's Edit > Fade: Opacity, Mode, and a Preview checkbox for the most
// recently applied filter, adjustment, paint stroke, or erase. OK records one
// `Fade <effect>` history state; Cancel restores the pre-dialog pixels.
class FadeDialog : public QDialog {
    Q_OBJECT

public:
    explicit FadeDialog(PictureView& view, QWidget* parent = nullptr);
    ~FadeDialog() override;

    int opacity() const;
    QString modeKey() const;
    // Commit the dialog's values. False when the record is gone or the mode is
    // refused.
    bool apply();

    void reject() override;

private:
    void preview();

    PictureView* view_ = nullptr;
    QComboBox* mode_ = nullptr;
    QSlider* opacitySlider_ = nullptr;
    QSpinBox* opacity_ = nullptr;
    QCheckBox* preview_ = nullptr;
    bool applied_ = false;
};

} // namespace pictura
